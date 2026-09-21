use std::convert::TryInto;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use blake2b_simd::Params as Blake2bParams;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use group::cofactor::CofactorGroup;
use group::GroupEncoding;
use image::GenericImageView;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::crypto::verus_id_signature::{
    compute_verus_data_identity_hash, parse_identity_signature,
    verify_identity_signature_against_addresses,
};
use crate::core::crypto::wif_encoding::Network;
use crate::types::WalletError;

pub const AVATAR_VDXF_KEY: &str = "iMMRVtGBNkr7V2hUNd4LLFiPQyzGrxAhx1";
pub const HEADER_VDXF_KEY: &str = "iP9hXXzqYXQhnY9EXikYjqraGBCrvaysAe";
pub const DESCRIPTION_VDXF_KEY: &str = "iAvXhoTu7EtDcGiUsyd1BysHMo1bTNDrTd";
pub const REMOVE_VDXF_KEY: &str = "i5Zkx5Z7tEfh42xtKfwbJ5LgEWE9rEgpFY";
const DATA_DESCRIPTOR_VDXF_KEY: &str = "i4GC1YGEVD21afWudGoFJVdnfjJ5XWnCQv";
const CROSS_CHAIN_REFERENCE_VDXF_KEY: &str = "iP3euVSzNcXUrLNHnQnR9G6q8jeYuGSxgw";
const SIGNATURE_DATA_VDXF_KEY: &str = "i7PcVF9wwPtQ6p6jDtCVpohX65pTZuP2ah";

pub const AVATAR_MIME: &str = "image/webp";
pub const LEGACY_IMAGE_MIME: &str = "image/jpeg";
pub const DESCRIPTION_MIME: &str = "text/plain; charset=utf-8";
pub const MAX_AVATAR_BYTES: usize = 32 * 1024;
// Application budgets, not protocol maxima.
pub const MAX_HEADER_BYTES: usize = 32 * 1024;
pub const MAX_DESCRIPTION_BYTES: usize = 1024;
// Two 32 KiB images plus the description and signed descriptor overhead.
pub const MAX_EVIDENCE_PARTS: usize = 16;
const MAX_DESCRIPTOR_BYTES: usize = 2 * 1024 * 1024;
const SAPLING_KDF_PERSONALIZATION: &[u8; 16] = b"Zcash_SaplingKDF";

#[derive(Debug, Clone)]
pub(crate) struct DataDescriptor {
    pub flags: u64,
    pub object_data: Vec<u8>,
    pub mime_type: Option<String>,
    pub salt: Vec<u8>,
    pub epk: Vec<u8>,
    pub ivk: Vec<u8>,
    pub ssk: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct EvidenceReference {
    pub txid_le: [u8; 32],
    pub vout: u32,
    pub object_num: u64,
    pub subobject: u64,
}

#[derive(Debug, Clone)]
struct EvidenceData {
    data_type: u64,
    index: u64,
    total_length: usize,
    start: usize,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct SignatureData {
    system_id: [u8; 20],
    hash_type: u64,
    signature_hash: [u8; 32],
    identity_id: [u8; 20],
    signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedProfilePayload {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub digest: [u8; 32],
    pub evidence_vout: usize,
    pub evidence_parts: usize,
}

fn invalid_profile() -> WalletError {
    WalletError::IdentityProfileUnavailable
}

fn read_slice<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    length: usize,
) -> Result<&'a [u8], WalletError> {
    let end = offset.checked_add(length).ok_or_else(invalid_profile)?;
    let value = bytes.get(*offset..end).ok_or_else(invalid_profile)?;
    *offset = end;
    Ok(value)
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> Result<u8, WalletError> {
    Ok(read_slice(bytes, offset, 1)?[0])
}

fn read_u16_le(bytes: &[u8], offset: &mut usize) -> Result<u16, WalletError> {
    Ok(u16::from_le_bytes(
        read_slice(bytes, offset, 2)?
            .try_into()
            .map_err(|_| invalid_profile())?,
    ))
}

fn read_u32_le(bytes: &[u8], offset: &mut usize) -> Result<u32, WalletError> {
    Ok(u32::from_le_bytes(
        read_slice(bytes, offset, 4)?
            .try_into()
            .map_err(|_| invalid_profile())?,
    ))
}

fn read_compact_size(bytes: &[u8], offset: &mut usize) -> Result<usize, WalletError> {
    let marker = read_u8(bytes, offset)?;
    let value = match marker {
        0..=252 => u64::from(marker),
        253 => u64::from(u16::from_le_bytes(
            read_slice(bytes, offset, 2)?
                .try_into()
                .map_err(|_| invalid_profile())?,
        )),
        254 => u64::from(u32::from_le_bytes(
            read_slice(bytes, offset, 4)?
                .try_into()
                .map_err(|_| invalid_profile())?,
        )),
        255 => u64::from_le_bytes(
            read_slice(bytes, offset, 8)?
                .try_into()
                .map_err(|_| invalid_profile())?,
        ),
    };
    usize::try_from(value).map_err(|_| invalid_profile())
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64, WalletError> {
    let mut value = 0u64;
    loop {
        let byte = read_u8(bytes, offset)?;
        value = value
            .checked_mul(128)
            .and_then(|current| current.checked_add(u64::from(byte & 0x7f)))
            .ok_or_else(invalid_profile)?;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        value = value.checked_add(1).ok_or_else(invalid_profile)?;
    }
}

fn read_var_bytes(
    bytes: &[u8],
    offset: &mut usize,
    maximum: usize,
) -> Result<Vec<u8>, WalletError> {
    let length = read_compact_size(bytes, offset)?;
    if length > maximum {
        return Err(invalid_profile());
    }
    Ok(read_slice(bytes, offset, length)?.to_vec())
}

fn read_string(bytes: &[u8], offset: &mut usize, maximum: usize) -> Result<String, WalletError> {
    String::from_utf8(read_var_bytes(bytes, offset, maximum)?).map_err(|_| invalid_profile())
}

fn decode_hash160(address: &str) -> Result<[u8; 20], WalletError> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| invalid_profile())?;
    if decoded.len() != 21 || decoded[0] != 102 {
        return Err(invalid_profile());
    }
    decoded[1..].try_into().map_err(|_| invalid_profile())
}

fn parse_hex_field(value: Option<&Value>, maximum: usize) -> Result<Vec<u8>, WalletError> {
    let raw = value.and_then(Value::as_str).ok_or_else(invalid_profile)?;
    let bytes = hex::decode(raw).map_err(|_| invalid_profile())?;
    if bytes.len() > maximum {
        return Err(invalid_profile());
    }
    Ok(bytes)
}

pub(crate) fn descriptor_from_json(value: &Value) -> Result<DataDescriptor, WalletError> {
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_profile)?;
    let flags = value
        .get("flags")
        .and_then(Value::as_u64)
        .ok_or_else(invalid_profile)?;
    if version != 1 || flags & !0xff != 0 {
        return Err(invalid_profile());
    }
    let object_data = parse_hex_field(value.get("objectdata"), MAX_DESCRIPTOR_BYTES)?;
    let optional_hex = |name: &str, maximum: usize| -> Result<Vec<u8>, WalletError> {
        match value.get(name) {
            Some(Value::String(_)) => parse_hex_field(value.get(name), maximum),
            None => Ok(Vec::new()),
            _ => Err(invalid_profile()),
        }
    };
    Ok(DataDescriptor {
        flags,
        object_data,
        mime_type: value
            .get("mimetype")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        salt: optional_hex("salt", 64)?,
        epk: optional_hex("epk", 32)?,
        ivk: optional_hex("ivk", 32)?,
        ssk: optional_hex("ssk", 32)?,
    })
}

fn parse_data_descriptor(bytes: &[u8]) -> Result<DataDescriptor, WalletError> {
    let mut offset = 0usize;
    let version = read_varint(bytes, &mut offset)?;
    let flags = read_varint(bytes, &mut offset)?;
    if version != 1 || flags & !0xff != 0 {
        return Err(invalid_profile());
    }
    if flags & 0x80 != 0 {
        read_slice(bytes, &mut offset, 20)?;
    }
    let object_data = read_var_bytes(bytes, &mut offset, MAX_DESCRIPTOR_BYTES)?;
    if flags & 0x20 != 0 {
        read_string(bytes, &mut offset, 64)?;
    }
    let mime_type = if flags & 0x40 != 0 {
        Some(read_string(bytes, &mut offset, 128)?)
    } else {
        None
    };
    let salt = if flags & 0x02 != 0 {
        read_var_bytes(bytes, &mut offset, 64)?
    } else {
        Vec::new()
    };
    let epk = if flags & 0x04 != 0 {
        read_var_bytes(bytes, &mut offset, 32)?
    } else {
        Vec::new()
    };
    let ivk = if flags & 0x08 != 0 {
        read_var_bytes(bytes, &mut offset, 32)?
    } else {
        Vec::new()
    };
    let ssk = if flags & 0x10 != 0 {
        read_var_bytes(bytes, &mut offset, 32)?
    } else {
        Vec::new()
    };
    if offset != bytes.len() {
        return Err(invalid_profile());
    }
    Ok(DataDescriptor {
        flags,
        object_data,
        mime_type,
        salt,
        epk,
        ivk,
        ssk,
    })
}

fn parse_tagged_data(bytes: &[u8], expected_key: &str) -> Result<Vec<u8>, WalletError> {
    let mut offset = 0usize;
    let key = read_slice(bytes, &mut offset, 20)?;
    if key != decode_hash160(expected_key)? {
        return Err(invalid_profile());
    }
    if read_varint(bytes, &mut offset)? != 1 {
        return Err(invalid_profile());
    }
    let data = read_var_bytes(bytes, &mut offset, MAX_DESCRIPTOR_BYTES)?;
    if offset != bytes.len() {
        return Err(invalid_profile());
    }
    Ok(data)
}

fn decrypt_descriptor(descriptor: &DataDescriptor) -> Result<Vec<u8>, WalletError> {
    if descriptor.flags & 0x01 == 0 {
        return Ok(descriptor.object_data.clone());
    }
    let key = if descriptor.ssk.len() == 32 {
        descriptor.ssk.clone()
    } else {
        let epk: [u8; 32] = descriptor
            .epk
            .as_slice()
            .try_into()
            .map_err(|_| invalid_profile())?;
        let ivk: [u8; 32] = descriptor
            .ivk
            .as_slice()
            .try_into()
            .map_err(|_| invalid_profile())?;
        let epk_point =
            Option::<jubjub::ExtendedPoint>::from(jubjub::ExtendedPoint::from_bytes(&epk))
                .ok_or_else(invalid_profile)?;
        let ivk_scalar =
            Option::<jubjub::Fr>::from(jubjub::Fr::from_bytes(&ivk)).ok_or_else(invalid_profile)?;
        let shared = (epk_point * ivk_scalar).clear_cofactor().to_bytes();
        Blake2bParams::new()
            .hash_length(32)
            .personal(SAPLING_KDF_PERSONALIZATION)
            .to_state()
            .update(&shared)
            .update(&epk)
            .finalize()
            .as_bytes()
            .to_vec()
    };
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|_| invalid_profile())?;
    let nonce: Nonce = [0u8; 12].into();
    cipher
        .decrypt(&nonce, descriptor.object_data.as_ref())
        .map_err(|_| invalid_profile())
}

fn unwrap_encrypted_descriptor(descriptor: DataDescriptor) -> Result<DataDescriptor, WalletError> {
    if descriptor.flags & 0x01 == 0 {
        return Ok(descriptor);
    }

    let decrypted = decrypt_descriptor(&descriptor)?;
    let inner = parse_tagged_data(&decrypted, DATA_DESCRIPTOR_VDXF_KEY)?;
    parse_data_descriptor(&inner)
}

pub(crate) fn reference_from_descriptor(
    descriptor: &DataDescriptor,
) -> Result<EvidenceReference, WalletError> {
    let decrypted = decrypt_descriptor(descriptor)?;
    let inner = parse_tagged_data(&decrypted, DATA_DESCRIPTOR_VDXF_KEY)?;
    let nested = parse_data_descriptor(&inner)?;
    let reference_bytes = parse_tagged_data(&nested.object_data, CROSS_CHAIN_REFERENCE_VDXF_KEY)?;
    let mut offset = 0usize;
    if read_u8(&reference_bytes, &mut offset)? != 0 {
        return Err(invalid_profile());
    }
    if read_varint(&reference_bytes, &mut offset)? != 1 {
        return Err(invalid_profile());
    }
    let flags = read_varint(&reference_bytes, &mut offset)?;
    if flags & 1 == 0 || flags & (2 | 4) != 0 {
        return Err(invalid_profile());
    }
    let txid_le: [u8; 32] = read_slice(&reference_bytes, &mut offset, 32)?
        .try_into()
        .map_err(|_| invalid_profile())?;
    let vout = read_u32_le(&reference_bytes, &mut offset)?;
    let object_num = read_varint(&reference_bytes, &mut offset)?;
    let subobject = read_varint(&reference_bytes, &mut offset)?;
    if offset != reference_bytes.len() || object_num > 7 || subobject > 7 {
        return Err(invalid_profile());
    }
    Ok(EvidenceReference {
        txid_le,
        vout,
        object_num,
        subobject,
    })
}

fn parse_evidence_data(hex_value: &str) -> Result<EvidenceData, WalletError> {
    let bytes = hex::decode(hex_value).map_err(|_| invalid_profile())?;
    let mut offset = 0usize;
    let version = read_varint(&bytes, &mut offset)?;
    if version != 1 || read_varint(&bytes, &mut offset)? != version {
        return Err(invalid_profile());
    }
    let data_type = read_varint(&bytes, &mut offset)?;
    let (index, total_length, start) = if data_type == 2 {
        (
            read_varint(&bytes, &mut offset)?,
            usize::try_from(read_varint(&bytes, &mut offset)?).map_err(|_| invalid_profile())?,
            usize::try_from(read_varint(&bytes, &mut offset)?).map_err(|_| invalid_profile())?,
        )
    } else if data_type == 1 {
        read_slice(&bytes, &mut offset, 20)?;
        (0, 0, 0)
    } else {
        return Err(invalid_profile());
    };
    let data = read_var_bytes(&bytes, &mut offset, MAX_DESCRIPTOR_BYTES)?;
    if offset != bytes.len() {
        return Err(invalid_profile());
    }
    Ok(EvidenceData {
        data_type,
        index,
        total_length,
        start,
        data,
    })
}

fn chainobject_hexes(output: &Value) -> Result<Vec<String>, WalletError> {
    let objects = output
        .pointer("/scriptPubKey/notaryevidence/evidence/chainobjects")
        .and_then(Value::as_array)
        .ok_or_else(invalid_profile)?;
    if objects.len() > MAX_EVIDENCE_PARTS {
        return Err(invalid_profile());
    }
    objects
        .iter()
        .map(|entry| {
            entry
                .pointer("/value/hex")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .ok_or_else(invalid_profile)
        })
        .collect()
}

fn parse_notary_evidence_chainobjects(bytes: &[u8]) -> Result<Vec<EvidenceData>, WalletError> {
    let mut offset = 0usize;
    if read_u8(bytes, &mut offset)? != 1 {
        return Err(invalid_profile());
    }
    read_u8(bytes, &mut offset)?; // evidence type
    read_slice(bytes, &mut offset, 20)?;
    read_slice(bytes, &mut offset, 32)?;
    read_u32_le(bytes, &mut offset)?;
    read_u8(bytes, &mut offset)?;
    if read_u32_le(bytes, &mut offset)? != 1 {
        return Err(invalid_profile());
    }
    let count = usize::try_from(read_varint(bytes, &mut offset)?).map_err(|_| invalid_profile())?;
    if count == 0 || count > MAX_EVIDENCE_PARTS {
        return Err(invalid_profile());
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if read_u16_le(bytes, &mut offset)? != 10 {
            return Err(invalid_profile());
        }
        let version = read_varint(bytes, &mut offset)?;
        if version != 1 || read_varint(bytes, &mut offset)? != version {
            return Err(invalid_profile());
        }
        let data_type = read_varint(bytes, &mut offset)?;
        let (index, total_length, chunk_start) = if data_type == 2 {
            (
                read_varint(bytes, &mut offset)?,
                usize::try_from(read_varint(bytes, &mut offset)?).map_err(|_| invalid_profile())?,
                usize::try_from(read_varint(bytes, &mut offset)?).map_err(|_| invalid_profile())?,
            )
        } else if data_type == 1 {
            read_slice(bytes, &mut offset, 20)?;
            (0, 0, 0)
        } else {
            return Err(invalid_profile());
        };
        let data = read_var_bytes(bytes, &mut offset, MAX_DESCRIPTOR_BYTES)?;
        out.push(EvidenceData {
            data_type,
            index,
            total_length,
            start: chunk_start,
            data,
        });
    }
    if offset != bytes.len() {
        return Err(invalid_profile());
    }
    Ok(out)
}

fn evidence_objects_for_reference(
    decoded_tx: &Value,
    reference: &EvidenceReference,
) -> Result<Vec<EvidenceData>, WalletError> {
    if reference.txid_le.iter().any(|byte| *byte != 0) {
        return Err(invalid_profile());
    }
    let outputs = decoded_tx
        .get("vout")
        .and_then(Value::as_array)
        .ok_or_else(invalid_profile)?;
    let first = outputs
        .get(reference.vout as usize)
        .ok_or_else(invalid_profile)?;
    let first_hexes = chainobject_hexes(first)?;
    let first_evidence = parse_evidence_data(first_hexes.first().ok_or_else(invalid_profile)?)?;
    if first_evidence.data_type == 1 {
        return first_hexes
            .iter()
            .map(|value| parse_evidence_data(value))
            .collect();
    }

    let total_length = first_evidence.total_length;
    if total_length == 0 || total_length > MAX_DESCRIPTOR_BYTES {
        return Err(invalid_profile());
    }
    let mut assembled = Vec::with_capacity(total_length);
    for part_index in 0..MAX_EVIDENCE_PARTS {
        let output = outputs
            .get(reference.vout as usize + part_index)
            .ok_or_else(invalid_profile)?;
        let hexes = chainobject_hexes(output)?;
        if hexes.len() != 1 {
            return Err(invalid_profile());
        }
        let part = parse_evidence_data(&hexes[0])?;
        if part.data_type != 2
            || part.index != part_index as u64
            || part.total_length != total_length
            || part.start != assembled.len()
        {
            return Err(invalid_profile());
        }
        assembled.extend_from_slice(&part.data);
        if assembled.len() == total_length {
            return parse_notary_evidence_chainobjects(&assembled);
        }
        if assembled.len() > total_length {
            return Err(invalid_profile());
        }
    }
    Err(invalid_profile())
}

fn parse_signature_data(bytes: &[u8]) -> Result<SignatureData, WalletError> {
    let mut offset = 0usize;
    if read_varint(bytes, &mut offset)? != 1 {
        return Err(invalid_profile());
    }
    let system_id = read_slice(bytes, &mut offset, 20)?
        .try_into()
        .map_err(|_| invalid_profile())?;
    let hash_type = read_varint(bytes, &mut offset)?;
    let signature_hash: [u8; 32] = read_var_bytes(bytes, &mut offset, 32)?
        .try_into()
        .map_err(|_| invalid_profile())?;
    let identity_id = read_slice(bytes, &mut offset, 20)?
        .try_into()
        .map_err(|_| invalid_profile())?;
    read_varint(bytes, &mut offset)?; // signature type
    let vdxf_keys = read_compact_size(bytes, &mut offset)?;
    read_slice(
        bytes,
        &mut offset,
        vdxf_keys.checked_mul(20).ok_or_else(invalid_profile)?,
    )?;
    let name_count = read_compact_size(bytes, &mut offset)?;
    for _ in 0..name_count {
        read_var_bytes(bytes, &mut offset, 256)?;
    }
    let bound_hash_count = read_compact_size(bytes, &mut offset)?;
    read_slice(
        bytes,
        &mut offset,
        bound_hash_count
            .checked_mul(32)
            .ok_or_else(invalid_profile)?,
    )?;
    let signature = read_var_bytes(bytes, &mut offset, 512)?;
    if offset != bytes.len() || hash_type != 5 || signature_hash.iter().all(|byte| *byte == 0) {
        return Err(invalid_profile());
    }
    Ok(SignatureData {
        system_id,
        hash_type,
        signature_hash,
        identity_id,
        signature,
    })
}

fn resolve_descriptor_object(
    descriptor: &DataDescriptor,
    decoded_tx: &Value,
    expected_tag: &str,
) -> Result<(DataDescriptor, EvidenceReference), WalletError> {
    let reference = reference_from_descriptor(descriptor)?;
    let objects = evidence_objects_for_reference(decoded_tx, &reference)?;
    let object_index = usize::try_from(reference.subobject).map_err(|_| invalid_profile())?;
    let object = objects.get(object_index).ok_or_else(invalid_profile)?;
    if object.data_type != 1 {
        return Err(invalid_profile());
    }
    let tagged = parse_tagged_data(&object.data, expected_tag)?;
    let mut nested = parse_data_descriptor(&tagged)?;
    if nested.flags & 0x01 != 0 && nested.ivk.is_empty() {
        nested.ivk = descriptor.ivk.clone();
    }
    Ok((unwrap_encrypted_descriptor(nested)?, reference))
}

// Called only after full evidence reconstruction and authentication succeeded.
fn evidence_part_count(
    decoded: &Value,
    reference: &EvidenceReference,
) -> Result<usize, WalletError> {
    let outputs = decoded
        .get("vout")
        .and_then(Value::as_array)
        .ok_or_else(invalid_profile)?;
    let first = outputs
        .get(reference.vout as usize)
        .ok_or_else(invalid_profile)?;
    let hexes = chainobject_hexes(first)?;
    let part = parse_evidence_data(hexes.first().ok_or_else(invalid_profile)?)?;
    if part.data_type == 1 {
        return Ok(1);
    }
    let mut length = 0usize;
    for index in 0..MAX_EVIDENCE_PARTS {
        let hexes = chainobject_hexes(
            outputs
                .get(reference.vout as usize + index)
                .ok_or_else(invalid_profile)?,
        )?;
        let next = parse_evidence_data(hexes.first().ok_or_else(invalid_profile)?)?;
        length = length
            .checked_add(next.data.len())
            .ok_or_else(invalid_profile)?;
        if length == part.total_length {
            return Ok(index + 1);
        }
    }
    Err(invalid_profile())
}

pub(crate) fn resolve_profile_payload(
    descriptors: &[DataDescriptor],
    decoded_tx: &Value,
    expected_mimes: &[&str],
    expected_system_id: &str,
    expected_identity_id: &str,
    allowed_primary_addresses: &[String],
    network: Network,
) -> Result<ResolvedProfilePayload, WalletError> {
    if descriptors.len() != 2 {
        return Err(invalid_profile());
    }
    let (payload_descriptor, payload_reference) =
        resolve_descriptor_object(&descriptors[0], decoded_tx, DATA_DESCRIPTOR_VDXF_KEY)?;
    let (signature_descriptor, signature_reference) =
        resolve_descriptor_object(&descriptors[1], decoded_tx, SIGNATURE_DATA_VDXF_KEY)?;
    if payload_reference.vout != signature_reference.vout
        || payload_reference.object_num != signature_reference.object_num
    {
        return Err(invalid_profile());
    }
    if payload_descriptor.flags & 0x01 != 0
        || !expected_mimes.contains(&payload_descriptor.mime_type.as_deref().unwrap_or(""))
    {
        return Err(invalid_profile());
    }
    let signature_bytes = if signature_descriptor.flags & 0x01 != 0 {
        decrypt_descriptor(&signature_descriptor)?
    } else {
        signature_descriptor.object_data.clone()
    };
    let signature_data_bytes =
        parse_tagged_data(&signature_bytes, SIGNATURE_DATA_VDXF_KEY).unwrap_or(signature_bytes);
    let signature_data = parse_signature_data(&signature_data_bytes)?;
    if signature_data.hash_type != 5
        || signature_data.system_id != decode_hash160(expected_system_id)?
        || signature_data.identity_id != decode_hash160(expected_identity_id)?
    {
        return Err(invalid_profile());
    }
    let mut digest_input = payload_descriptor.object_data.clone();
    digest_input.extend_from_slice(&payload_descriptor.salt);
    let digest: [u8; 32] = Sha256::digest(&digest_input).into();
    if digest != signature_data.signature_hash {
        return Err(invalid_profile());
    }
    let parsed_signature =
        parse_identity_signature(&signature_data.signature).map_err(|_| invalid_profile())?;
    let identity_hash = compute_verus_data_identity_hash(
        signature_data.system_id,
        signature_data.identity_id,
        parsed_signature.block_height,
        signature_data.signature_hash,
    );
    if !verify_identity_signature_against_addresses(
        identity_hash,
        &parsed_signature,
        allowed_primary_addresses,
        network,
    ) {
        return Err(invalid_profile());
    }
    Ok(ResolvedProfilePayload {
        bytes: payload_descriptor.object_data,
        mime_type: payload_descriptor.mime_type.ok_or_else(invalid_profile)?,
        digest,
        evidence_vout: payload_reference.vout as usize,
        evidence_parts: evidence_part_count(decoded_tx, &payload_reference)?,
    })
}

pub(crate) fn validate_avatar_bytes(bytes: &[u8], mime: &str) -> Result<(), WalletError> {
    validate_image_bytes(bytes, mime, (256, 256), MAX_AVATAR_BYTES)
        .map_err(|_| WalletError::IdentityProfileInvalidAvatar)
}

pub(crate) fn validate_header_bytes(bytes: &[u8], mime: &str) -> Result<(), WalletError> {
    validate_image_bytes(bytes, mime, (960, 160), MAX_HEADER_BYTES)
        .map_err(|_| WalletError::IdentityProfileInvalidHeader)
}

fn validate_image_bytes(
    bytes: &[u8],
    mime: &str,
    dimensions: (u32, u32),
    maximum: usize,
) -> Result<(), ()> {
    if bytes.is_empty() || bytes.len() > maximum {
        return Err(());
    }
    let format = match mime {
        LEGACY_IMAGE_MIME if bytes.starts_with(&[0xff, 0xd8, 0xff]) => image::ImageFormat::Jpeg,
        AVATAR_MIME
            if bytes.len() >= 12
                && &bytes[..4] == b"RIFF"
                && &bytes[8..12] == b"WEBP"
                && u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| ())?) as usize
                    == bytes.len() - 8 =>
        {
            image::ImageFormat::WebP
        }
        _ => return Err(()),
    };
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(8 * 1024 * 1024);
    limits.max_image_width = Some(dimensions.0);
    limits.max_image_height = Some(dimensions.1);
    reader.limits(limits);
    let image = reader.decode().map_err(|_| ())?;
    if image.dimensions() != dimensions {
        return Err(());
    }
    Ok(())
}

pub(crate) fn avatar_base64(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use group::Group;

    #[test]
    fn rejects_varints_that_overflow_u64() {
        assert_eq!(read_varint(&[0x80, 0x00], &mut 0).unwrap(), 128);
        let mut oversized = vec![0x80; 11];
        oversized.push(0);
        assert!(read_varint(&oversized, &mut 0).is_err());
    }

    #[test]
    fn avatar_validation_accepts_only_bounded_square_jpegs() {
        fn jpeg(width: u32, height: u32) -> Vec<u8> {
            let image = image::RgbImage::new(width, height);
            let mut bytes = Vec::new();
            image::codecs::jpeg::JpegEncoder::new(&mut bytes)
                .encode_image(&image)
                .unwrap();
            bytes
        }

        assert!(validate_avatar_bytes(&jpeg(256, 256), LEGACY_IMAGE_MIME).is_ok());
        assert!(validate_avatar_bytes(&jpeg(257, 256), LEGACY_IMAGE_MIME).is_err());
        assert!(validate_avatar_bytes(&jpeg(255, 256), LEGACY_IMAGE_MIME).is_err());
        assert!(validate_avatar_bytes(&[0xff, 0xd8, 0xff], LEGACY_IMAGE_MIME).is_err());
        assert!(validate_avatar_bytes(&vec![0; MAX_AVATAR_BYTES + 1], LEGACY_IMAGE_MIME).is_err());
    }

    #[test]
    fn header_validation_accepts_only_bounded_six_to_one_jpegs() {
        let jpeg = |width, height| {
            let image = image::RgbImage::new(width, height);
            let mut bytes = Vec::new();
            image::codecs::jpeg::JpegEncoder::new(&mut bytes)
                .encode_image(&image)
                .unwrap();
            bytes
        };
        assert!(validate_header_bytes(&jpeg(960, 160), LEGACY_IMAGE_MIME).is_ok());
        assert!(validate_header_bytes(&jpeg(256, 256), LEGACY_IMAGE_MIME).is_err());
        assert!(validate_header_bytes(&jpeg(960, 161), LEGACY_IMAGE_MIME).is_err());
        assert!(validate_header_bytes(&jpeg(959, 160), LEGACY_IMAGE_MIME).is_err());
        assert!(validate_header_bytes(&[0xff, 0xd8, 0xff], LEGACY_IMAGE_MIME).is_err());
        assert!(validate_header_bytes(&vec![0; MAX_HEADER_BYTES + 1], LEGACY_IMAGE_MIME).is_err());
    }

    #[test]
    fn webp_validation_binds_mime_container_decode_dimensions_and_budget() {
        let webp = |width, height| {
            let mut bytes = Vec::new();
            image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
                .encode(
                    &vec![128; width as usize * height as usize * 3],
                    width,
                    height,
                    image::ExtendedColorType::Rgb8,
                )
                .unwrap();
            bytes
        };
        let avatar = webp(256, 256);
        assert!(validate_avatar_bytes(&avatar, AVATAR_MIME).is_ok());
        assert!(validate_avatar_bytes(&avatar, LEGACY_IMAGE_MIME).is_err());
        assert!(validate_avatar_bytes(&avatar, "image/png").is_err());
        assert!(validate_header_bytes(&webp(960, 160), AVATAR_MIME).is_ok());
        assert!(validate_avatar_bytes(&webp(257, 256), AVATAR_MIME).is_err());
        assert!(validate_header_bytes(&avatar, AVATAR_MIME).is_err());
        let mut corrupt = avatar.clone();
        corrupt[12..].fill(255);
        assert!(validate_avatar_bytes(&corrupt, AVATAR_MIME).is_err());
        let mut wrong_length = avatar.clone();
        wrong_length[4] ^= 1;
        assert!(validate_avatar_bytes(&wrong_length, AVATAR_MIME).is_err());
        let mut oversized = avatar;
        oversized.resize(MAX_AVATAR_BYTES + 1, 0);
        assert!(validate_avatar_bytes(&oversized, AVATAR_MIME).is_err());
    }

    fn push_short_bytes(out: &mut Vec<u8>, value: &[u8]) {
        assert!(value.len() < 253);
        out.push(value.len() as u8);
        out.extend_from_slice(value);
    }

    #[test]
    fn unwraps_descriptor_with_viewing_key_published_on_the_link() {
        let object_data = b"public profile";
        let mime_type = b"text/plain; charset=utf-8";
        let salt = [7u8; 32];
        let mut inner_descriptor = vec![1, 0x42];
        push_short_bytes(&mut inner_descriptor, object_data);
        push_short_bytes(&mut inner_descriptor, mime_type);
        push_short_bytes(&mut inner_descriptor, &salt);

        let mut tagged = decode_hash160(DATA_DESCRIPTOR_VDXF_KEY)
            .expect("descriptor key")
            .to_vec();
        tagged.push(1);
        push_short_bytes(&mut tagged, &inner_descriptor);

        let epk_point = jubjub::ExtendedPoint::generator();
        let epk = epk_point.to_bytes();
        let ivk = jubjub::Fr::from(1u64).to_bytes();
        let shared = (epk_point * jubjub::Fr::from(1u64))
            .clear_cofactor()
            .to_bytes();
        let key = Blake2bParams::new()
            .hash_length(32)
            .personal(SAPLING_KDF_PERSONALIZATION)
            .to_state()
            .update(&shared)
            .update(&epk)
            .finalize();
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_bytes()).expect("cipher");
        let nonce: Nonce = [0u8; 12].into();
        let encrypted = cipher.encrypt(&nonce, tagged.as_ref()).expect("encrypt");

        let descriptor = DataDescriptor {
            flags: 0x05,
            object_data: encrypted,
            mime_type: None,
            salt: Vec::new(),
            epk: epk.to_vec(),
            ivk: ivk.to_vec(),
            ssk: Vec::new(),
        };
        let unwrapped = unwrap_encrypted_descriptor(descriptor).expect("unwrap descriptor");

        assert_eq!(unwrapped.flags, 0x42);
        assert_eq!(unwrapped.object_data, object_data);
        assert_eq!(
            unwrapped.mime_type.as_deref(),
            Some("text/plain; charset=utf-8")
        );
        assert_eq!(unwrapped.salt, salt);
    }

    #[test]
    fn decrypts_confirmed_vrsctest_avatar_reference() {
        let value = serde_json::json!({
            "version": 1,
            "flags": 13,
            "epk": "b5f641f249a3d6c99114c2c76f8c434098aab65c10f871ff2afd4d73a5dcad13",
            "ivk": "e886cfb59585dd19c829eeb55ecf921f3e490da476cda0966819a829ba008207",
            "objectdata": "1c6bddf6e531141703addc2fb50d888c177b56baffa5e7b8b905074d0dcbff3ed542210e15a1d8833c49ef43a90f85367e3c9880aabb224a9c3f2a3d3c5073b529cf3144c9288c76693fa8a2a90605287828b9dc394ce0b3c5895fe1569e97bbd8e2581e3efb0b8dd76996e48bfd281a60546dbc659d219a87f8378f66"
        });
        let descriptor = descriptor_from_json(&value).expect("descriptor");
        let reference = reference_from_descriptor(&descriptor).expect("reference");
        assert_eq!(reference.vout, 1);
        assert_eq!(reference.object_num, 0);
        assert_eq!(reference.subobject, 0);
        assert!(reference.txid_le.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn rejects_external_or_cross_system_reference_flags() {
        let mut descriptor = descriptor_from_json(&serde_json::json!({
            "version": 1,
            "flags": 13,
            "epk": "b5f641f249a3d6c99114c2c76f8c434098aab65c10f871ff2afd4d73a5dcad13",
            "ivk": "e886cfb59585dd19c829eeb55ecf921f3e490da476cda0966819a829ba008207",
            "objectdata": "1c6bddf6e531141703addc2fb50d888c177b56baffa5e7b8b905074d0dcbff3ed542210e15a1d8833c49ef43a90f85367e3c9880aabb224a9c3f2a3d3c5073b529cf3144c9288c76693fa8a2a90605287828b9dc394ce0b3c5895fe1569e97bbd8e2581e3efb0b8dd76996e48bfd281a60546dbc659d219a87f8378f66"
        })).expect("descriptor");
        *descriptor.object_data.last_mut().expect("ciphertext") ^= 1;
        assert!(reference_from_descriptor(&descriptor).is_err());
    }
}
