use std::convert::TryInto;

use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{Message, Secp256k1, SecretKey};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::channels::vrpc::VrpcProvider;
use crate::core::crypto::wif_encoding::{decode_wif, generate_p2pkh_address, Network};
use crate::types::WalletError;

const GENERIC_ENVELOPE_FLAG_SIGNED: u64 = 1;
const GENERIC_ENVELOPE_FLAG_HAS_REQUEST_ID: u64 = 2;
const GENERIC_ENVELOPE_FLAG_HAS_CREATED_AT: u64 = 4;
const GENERIC_ENVELOPE_FLAG_MULTI_DETAILS: u64 = 8;
const GENERIC_ENVELOPE_FLAG_IS_TESTNET: u64 = 16;
const GENERIC_ENVELOPE_FLAG_HAS_SALT: u64 = 32;
const GENERIC_ENVELOPE_FLAG_HAS_APP_OR_DELEGATED_ID: u64 = 64;

const VERIFIABLE_SIGNATURE_FLAG_HAS_VDXF_KEYS: u64 = 1;
const VERIFIABLE_SIGNATURE_FLAG_HAS_VDXF_KEY_NAMES: u64 = 2;
const VERIFIABLE_SIGNATURE_FLAG_HAS_BOUND_HASHES: u64 = 4;
const VERIFIABLE_SIGNATURE_FLAG_HAS_STATEMENTS: u64 = 8;

const COMPACT_ADDRESS_TYPE_FQN: u64 = 1;
const COMPACT_ADDRESS_TYPE_I_ADDRESS: u64 = 2;
const COMPACT_ADDRESS_TYPE_X_ADDRESS: u64 = 3;
const ORDINAL_VDXF_OBJECT_RESERVED_BYTE_I_ADDR: u64 = 102;
const ORDINAL_VDXF_OBJECT_RESERVED_BYTE_VDXF_ID_STRING: u64 = 103;
const ORDINAL_VDXF_OBJECT_RESERVED_BYTE_ID_OR_CURRENCY: u64 = 104;

const HASH_TYPE_SHA256: u64 = 5;
const VERIFIABLE_SIGNATURE_VERSION_V2: u64 = 2;
const I_ADDRESS_VERSION: u16 = 102;
const X_ADDRESS_VERSION: u16 = 137;
const SIGNATURE_TIME_DIFF_THRESHOLD_SECS: i64 = 3600;
const VERUS_DATA_SIGNATURE_PREFIX: &[u8] = b"\x13Verus signed data:\n";

#[derive(Debug, Clone)]
pub struct ParsedGenericEnvelope {
    pub prefix_without_signature: Vec<u8>,
    pub tail_after_signature: Vec<u8>,
    pub signature_data: ParsedVerifiableSignatureData,
    pub request_id: Option<String>,
    pub created_at: Option<u64>,
    pub is_testnet: bool,
    pub app_or_delegated_id: Option<String>,
    pub details: Vec<ParsedGenericDetail>,
}

#[derive(Debug, Clone)]
pub struct ParsedGenericDetail {
    pub ordinal: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ParsedVerifiableSignatureData {
    pub signer_system_id: String,
    pub signer_identity_id: String,
    pub signer_system_hash: [u8; 20],
    pub signer_identity_hash: [u8; 20],
    pub signature_version: u64,
    pub hash_type: u64,
    pub extra_hash_data: Vec<u8>,
    pub signature_as_vch: Vec<u8>,
    prefix_without_signature_value: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ParsedIdentitySignature {
    pub version: u8,
    pub hash_type: u8,
    pub block_height: u32,
    pub compact_signatures: Vec<[u8; 65]>,
}

#[derive(Debug, Clone)]
struct ParsedCompactAddress {
    address: String,
    hash160: [u8; 20],
    raw_bytes: Vec<u8>,
}

impl ParsedVerifiableSignatureData {
    pub fn to_buffer_with_signature(&self, signature_as_vch: &[u8]) -> Vec<u8> {
        let mut out = self.prefix_without_signature_value.clone();
        write_var_slice(&mut out, signature_as_vch);
        out
    }
}

pub fn parse_generic_envelope_hex(hex_value: &str) -> Result<ParsedGenericEnvelope, WalletError> {
    let bytes = hex::decode(hex_value).map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
    parse_generic_envelope_bytes(&bytes)
}

pub fn parse_generic_envelope_bytes(bytes: &[u8]) -> Result<ParsedGenericEnvelope, WalletError> {
    let mut offset = 0usize;
    let (_, version_len) = read_compact_size(bytes, offset)?;
    offset += version_len;

    let (flags, flags_len) = read_compact_size(bytes, offset)?;
    offset += flags_len;

    if (flags & GENERIC_ENVELOPE_FLAG_SIGNED) == 0 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let prefix_without_signature = bytes[..offset].to_vec();
    let (signature_data, signature_end) = parse_verifiable_signature_data(bytes, offset)?;
    let tail_after_signature = bytes[signature_end..].to_vec();

    let mut scan = signature_end;
    let request_id = if (flags & GENERIC_ENVELOPE_FLAG_HAS_REQUEST_ID) != 0 {
        let (value, next) = parse_compact_address(bytes, scan)?;
        scan = next;
        Some(value.address)
    } else {
        None
    };

    let created_at = if (flags & GENERIC_ENVELOPE_FLAG_HAS_CREATED_AT) != 0 {
        let (value, len) = read_compact_size(bytes, scan)?;
        scan += len;
        Some(value)
    } else {
        None
    };

    if (flags & GENERIC_ENVELOPE_FLAG_HAS_SALT) != 0 {
        let (_, next) = read_var_slice(bytes, scan)?;
        scan = next;
    }

    let app_or_delegated_id = if (flags & GENERIC_ENVELOPE_FLAG_HAS_APP_OR_DELEGATED_ID) != 0 {
        let (value, next) = parse_compact_address(bytes, scan)?;
        scan = next;
        Some(value.address)
    } else {
        None
    };

    let details = if (flags & GENERIC_ENVELOPE_FLAG_MULTI_DETAILS) != 0 {
        let (count, count_len) = read_compact_size(bytes, scan)?;
        scan += count_len;

        let mut details = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let (detail, next) = parse_generic_detail(bytes, scan)?;
            scan = next;
            details.push(detail);
        }

        details
    } else {
        let (detail, next) = parse_generic_detail(bytes, scan)?;
        scan = next;
        vec![detail]
    };

    let _ = scan;

    Ok(ParsedGenericEnvelope {
        prefix_without_signature,
        tail_after_signature,
        signature_data,
        request_id,
        created_at,
        is_testnet: (flags & GENERIC_ENVELOPE_FLAG_IS_TESTNET) != 0,
        app_or_delegated_id,
        details,
    })
}

pub fn parse_identity_signature(bytes: &[u8]) -> Result<ParsedIdentitySignature, WalletError> {
    if bytes.is_empty() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let mut offset = 0usize;
    let version = read_u8(bytes, &mut offset)?;
    let hash_type = if version >= 2 {
        read_u8(bytes, &mut offset)?
    } else {
        HASH_TYPE_SHA256 as u8
    };

    if version != 1 && version != 2 {
        return Err(WalletError::GenericRequestUnsupportedSignature);
    }
    if version == 2 && hash_type as u64 != HASH_TYPE_SHA256 {
        return Err(WalletError::GenericRequestUnsupportedSignature);
    }

    let block_height = read_u32_le(bytes, &mut offset)?;
    let signature_count = read_u8(bytes, &mut offset)? as usize;

    let mut compact_signatures = Vec::with_capacity(signature_count);
    for _ in 0..signature_count {
        let (signature_slice, next) = read_var_slice(bytes, offset)?;
        offset = next;
        if signature_slice.len() != 65 {
            return Err(WalletError::GenericRequestInvalidEnvelope);
        }
        compact_signatures.push(
            signature_slice
                .try_into()
                .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?,
        );
    }

    Ok(ParsedIdentitySignature {
        version,
        hash_type,
        block_height,
        compact_signatures,
    })
}

pub fn get_raw_envelope_sha256(parsed: &ParsedGenericEnvelope) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&parsed.prefix_without_signature);
    hasher.update(&parsed.tail_after_signature);
    hasher.finalize().into()
}

pub fn compute_identity_signature_hash(
    signature_data: &ParsedVerifiableSignatureData,
    signed_block_height: u32,
    raw_envelope_sha256: [u8; 32],
) -> Result<[u8; 32], WalletError> {
    if signature_data.hash_type != HASH_TYPE_SHA256 {
        return Err(WalletError::GenericRequestUnsupportedSignature);
    }

    let mut hasher = Sha256::new();
    if signature_data.signature_version == VERIFIABLE_SIGNATURE_VERSION_V2 {
        if !signature_data.extra_hash_data.is_empty() {
            hasher.update(&signature_data.extra_hash_data);
        }
        hasher.update(signature_data.signer_system_hash);
        hasher.update(signed_block_height.to_le_bytes());
        hasher.update(signature_data.signer_identity_hash);
        hasher.update(VERUS_DATA_SIGNATURE_PREFIX);
        hasher.update(raw_envelope_sha256);
        return Ok(hasher.finalize().into());
    }

    Err(WalletError::GenericRequestUnsupportedSignature)
}

pub fn sign_identity_hash(
    identity_hash: [u8; 32],
    signed_block_height: u32,
    wif: &str,
    network: Network,
) -> Result<Vec<u8>, WalletError> {
    let private_key = decode_wif(wif, network)?;
    let secret_key = SecretKey::from_slice(&private_key)
        .map_err(|_| WalletError::GenericRequestUnsupportedSignature)?;
    let secp = Secp256k1::new();
    let message = Message::from_digest(identity_hash);
    let recoverable = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (recovery_id, compact_body) = recoverable.serialize_compact();

    let header = 27u8
        .checked_add(recovery_id.to_i32() as u8)
        .and_then(|value| value.checked_add(4))
        .ok_or(WalletError::GenericRequestUnsupportedSignature)?;

    let mut compact_signature = [0u8; 65];
    compact_signature[0] = header;
    compact_signature[1..].copy_from_slice(&compact_body);

    let mut serialized = Vec::with_capacity(1 + 1 + 4 + 1 + 1 + compact_signature.len());
    serialized.push(VERIFIABLE_SIGNATURE_VERSION_V2 as u8);
    serialized.push(HASH_TYPE_SHA256 as u8);
    serialized.extend_from_slice(&signed_block_height.to_le_bytes());
    serialized.push(1u8);
    write_var_slice(&mut serialized, &compact_signature);
    Ok(serialized)
}

pub fn verify_identity_signature_against_addresses(
    identity_hash: [u8; 32],
    identity_signature: &ParsedIdentitySignature,
    allowed_addresses: &[String],
    network: Network,
) -> bool {
    if allowed_addresses.is_empty() {
        return false;
    }

    let secp = Secp256k1::new();
    let message = Message::from_digest(identity_hash);

    identity_signature.compact_signatures.iter().all(|compact| {
        recover_address_from_compact_signature(&secp, &message, compact, network)
            .map(|address| {
                allowed_addresses
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&address))
            })
            .unwrap_or(false)
    })
}

pub async fn verify_generic_request_signature_with_provider(
    provider: &VrpcProvider,
    parsed: &ParsedGenericEnvelope,
    network: Network,
) -> Result<bool, WalletError> {
    let identity_signature = parse_identity_signature(&parsed.signature_data.signature_as_vch)?;

    if parsed.created_at.is_none() {
        return Ok(false);
    }

    let block_raw = provider
        .getblock(&identity_signature.block_height.to_string())
        .await?;
    let block_time = extract_block_time(&block_raw)?;
    let created_at = parsed.created_at.unwrap_or_default() as i64;
    if (block_time as i64 - created_at).abs() > SIGNATURE_TIME_DIFF_THRESHOLD_SECS {
        return Ok(false);
    }

    let signer_identity = provider
        .getidentity(&parsed.signature_data.signer_identity_id)
        .await?;
    let primary_addresses = extract_primary_addresses(&signer_identity);
    let minimum_signatures = extract_minimum_signatures(&signer_identity).unwrap_or(1);
    if minimum_signatures != 1 {
        return Ok(false);
    }

    let raw_envelope_sha256 = get_raw_envelope_sha256(parsed);
    let identity_hash = compute_identity_signature_hash(
        &parsed.signature_data,
        identity_signature.block_height,
        raw_envelope_sha256,
    )?;

    Ok(verify_identity_signature_against_addresses(
        identity_hash,
        &identity_signature,
        &primary_addresses,
        network,
    ))
}

pub async fn validate_identity_control_for_active_wallet(
    provider: &VrpcProvider,
    signer_identity_id: &str,
    active_wallet_primary_address: &str,
) -> Result<(), WalletError> {
    let signer_identity = provider.getidentity(signer_identity_id).await?;
    let primary_addresses = extract_primary_addresses(&signer_identity);
    let minimum_signatures = extract_minimum_signatures(&signer_identity).unwrap_or(1);

    if minimum_signatures != 1 {
        return Err(WalletError::IdentityUnsupportedAuthority);
    }

    if !primary_addresses
        .iter()
        .any(|address| address.eq_ignore_ascii_case(active_wallet_primary_address))
    {
        return Err(WalletError::IdentityUnsupportedAuthority);
    }

    Ok(())
}

fn parse_verifiable_signature_data(
    bytes: &[u8],
    offset: usize,
) -> Result<(ParsedVerifiableSignatureData, usize), WalletError> {
    let start = offset;
    let mut cursor = offset;

    let (_version, version_len) = read_varint(bytes, cursor)?;
    cursor += version_len;

    let (flags, flags_len) = read_compact_size(bytes, cursor)?;
    cursor += flags_len;

    let (signature_version, signature_version_len) = read_compact_size(bytes, cursor)?;
    cursor += signature_version_len;

    let (hash_type, hash_type_len) = read_compact_size(bytes, cursor)?;
    cursor += hash_type_len;

    let (signer_system, next) = parse_compact_address(bytes, cursor)?;
    cursor = next;
    let (signer_identity, next) = parse_compact_address(bytes, cursor)?;
    cursor = next;

    let extra_hash_start = cursor;
    if (flags & VERIFIABLE_SIGNATURE_FLAG_HAS_VDXF_KEYS) != 0 {
        cursor = skip_fixed_array(bytes, cursor, 20)?;
    }
    if (flags & VERIFIABLE_SIGNATURE_FLAG_HAS_VDXF_KEY_NAMES) != 0 {
        cursor = skip_vector(bytes, cursor)?;
    }
    if (flags & VERIFIABLE_SIGNATURE_FLAG_HAS_BOUND_HASHES) != 0 {
        cursor = skip_vector(bytes, cursor)?;
    }
    if (flags & VERIFIABLE_SIGNATURE_FLAG_HAS_STATEMENTS) != 0 {
        cursor = skip_vector(bytes, cursor)?;
    }
    let extra_hash_data = bytes[extra_hash_start..cursor].to_vec();

    let prefix_without_signature_value = bytes[start..cursor].to_vec();
    let (signature_as_vch, signature_end) = read_var_slice(bytes, cursor)?;

    Ok((
        ParsedVerifiableSignatureData {
            signer_system_id: signer_system.address,
            signer_identity_id: signer_identity.address,
            signer_system_hash: signer_system.hash160,
            signer_identity_hash: signer_identity.hash160,
            signature_version,
            hash_type,
            extra_hash_data,
            signature_as_vch: signature_as_vch.to_vec(),
            prefix_without_signature_value,
        },
        signature_end,
    ))
}

fn parse_compact_address(
    bytes: &[u8],
    offset: usize,
) -> Result<(ParsedCompactAddress, usize), WalletError> {
    let start = offset;
    let mut cursor = offset;

    let (_, version_len) = read_compact_size(bytes, cursor)?;
    cursor += version_len;

    let (address_type, address_type_len) = read_compact_size(bytes, cursor)?;
    cursor += address_type_len;

    let (address, hash160) = match address_type {
        COMPACT_ADDRESS_TYPE_I_ADDRESS => {
            let hash = read_fixed_hash160(bytes, &mut cursor)?;
            (to_base58_check(&hash, I_ADDRESS_VERSION), hash)
        }
        COMPACT_ADDRESS_TYPE_X_ADDRESS => {
            let hash = read_fixed_hash160(bytes, &mut cursor)?;
            (to_base58_check(&hash, X_ADDRESS_VERSION), hash)
        }
        COMPACT_ADDRESS_TYPE_FQN => {
            return Err(WalletError::GenericRequestUnsupportedSignature);
        }
        _ => return Err(WalletError::GenericRequestInvalidEnvelope),
    };

    Ok((
        ParsedCompactAddress {
            address,
            hash160,
            raw_bytes: bytes[start..cursor].to_vec(),
        },
        cursor,
    ))
}

fn parse_generic_detail(
    bytes: &[u8],
    offset: usize,
) -> Result<(ParsedGenericDetail, usize), WalletError> {
    let mut cursor = offset;

    let (ordinal, ordinal_len) = read_compact_size(bytes, cursor)?;
    cursor += ordinal_len;

    match ordinal {
        ORDINAL_VDXF_OBJECT_RESERVED_BYTE_I_ADDR => {
            cursor = skip_fixed_hash160(bytes, cursor)?;
        }
        ORDINAL_VDXF_OBJECT_RESERVED_BYTE_VDXF_ID_STRING
        | ORDINAL_VDXF_OBJECT_RESERVED_BYTE_ID_OR_CURRENCY => {
            let (_, next) = read_var_slice(bytes, cursor)?;
            cursor = next;
        }
        _ => {}
    }

    let (_, version_len) = read_varint(bytes, cursor)?;
    cursor += version_len;

    let (data, next) = read_var_slice(bytes, cursor)?;
    cursor = next;

    Ok((
        ParsedGenericDetail {
            ordinal,
            data: data.to_vec(),
        },
        cursor,
    ))
}

fn recover_address_from_compact_signature(
    secp: &Secp256k1<secp256k1::All>,
    message: &Message,
    compact_signature: &[u8; 65],
    network: Network,
) -> Result<String, WalletError> {
    let flag = compact_signature[0];
    if !(27..=34).contains(&flag) {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let recovery_value = (flag - 27) & 0x03;
    let recovery_id = RecoveryId::from_i32(recovery_value as i32)
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
    let signature = RecoverableSignature::from_compact(&compact_signature[1..], recovery_id)
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
    let public_key = secp
        .recover_ecdsa(message, &signature)
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
    generate_p2pkh_address(&public_key, network)
}

pub(crate) fn extract_primary_addresses(raw_identity: &Value) -> Vec<String> {
    raw_identity
        .get("identity")
        .and_then(|identity| {
            identity
                .get("primaryaddresses")
                .or_else(|| identity.get("primaryAddresses"))
        })
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_minimum_signatures(raw_identity: &Value) -> Option<u64> {
    let value = raw_identity.get("identity").and_then(|identity| {
        identity
            .get("minimumsignatures")
            .or_else(|| identity.get("minimumSignatures"))
    })?;

    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    if let Some(number) = value.as_i64() {
        return (number >= 0).then_some(number as u64);
    }
    value.as_str()?.trim().parse::<u64>().ok()
}

fn extract_block_time(block_raw: &Value) -> Result<u64, WalletError> {
    if let Some(number) = block_raw.get("time").and_then(Value::as_u64) {
        return Ok(number);
    }
    if let Some(number) = block_raw.as_u64() {
        return Ok(number);
    }
    if let Some(number) = block_raw.get("time").and_then(Value::as_i64) {
        return (number >= 0)
            .then_some(number as u64)
            .ok_or(WalletError::OperationFailed);
    }
    Err(WalletError::OperationFailed)
}

pub fn extract_chain_height(info_raw: &Value) -> Result<u32, WalletError> {
    if let Some(number) = info_raw.get("longestchain").and_then(Value::as_u64) {
        return u32::try_from(number).map_err(|_| WalletError::OperationFailed);
    }
    if let Some(number) = info_raw.get("blocks").and_then(Value::as_u64) {
        return u32::try_from(number).map_err(|_| WalletError::OperationFailed);
    }
    if let Some(number) = info_raw.get("longestchain").and_then(Value::as_i64) {
        return if number >= 0 {
            u32::try_from(number as u64).map_err(|_| WalletError::OperationFailed)
        } else {
            Err(WalletError::OperationFailed)
        };
    }
    Err(WalletError::OperationFailed)
}

fn to_base58_check(hash: &[u8; 20], version: u16) -> String {
    let payload = if version > u8::MAX as u16 {
        let mut payload = Vec::with_capacity(22);
        payload.extend_from_slice(&version.to_be_bytes());
        payload.extend_from_slice(hash);
        payload
    } else {
        let mut payload = Vec::with_capacity(21);
        payload.push(version as u8);
        payload.extend_from_slice(hash);
        payload
    };

    bs58::encode(payload).with_check().into_string()
}

fn read_u8(bytes: &[u8], offset: &mut usize) -> Result<u8, WalletError> {
    let value = bytes
        .get(*offset)
        .copied()
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    *offset += 1;
    Ok(value)
}

fn read_u32_le(bytes: &[u8], offset: &mut usize) -> Result<u32, WalletError> {
    if bytes.len() < *offset + 4 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let mut raw = [0u8; 4];
    raw.copy_from_slice(&bytes[*offset..*offset + 4]);
    *offset += 4;
    Ok(u32::from_le_bytes(raw))
}

fn read_fixed_hash160(bytes: &[u8], offset: &mut usize) -> Result<[u8; 20], WalletError> {
    if bytes.len() < *offset + 20 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let mut hash = [0u8; 20];
    hash.copy_from_slice(&bytes[*offset..*offset + 20]);
    *offset += 20;
    Ok(hash)
}

fn skip_fixed_hash160(bytes: &[u8], offset: usize) -> Result<usize, WalletError> {
    let end = offset
        .checked_add(20)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if bytes.len() < end {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    Ok(end)
}

fn read_compact_size(bytes: &[u8], offset: usize) -> Result<(u64, usize), WalletError> {
    let first = *bytes
        .get(offset)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;

    match first {
        0x00..=0xfc => Ok((first as u64, 1)),
        0xfd => {
            if bytes.len() < offset + 3 {
                return Err(WalletError::GenericRequestInvalidEnvelope);
            }
            let raw = u16::from_le_bytes([bytes[offset + 1], bytes[offset + 2]]) as u64;
            Ok((raw, 3))
        }
        0xfe => {
            if bytes.len() < offset + 5 {
                return Err(WalletError::GenericRequestInvalidEnvelope);
            }
            let raw = u32::from_le_bytes([
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
            ]) as u64;
            Ok((raw, 5))
        }
        0xff => {
            if bytes.len() < offset + 9 {
                return Err(WalletError::GenericRequestInvalidEnvelope);
            }
            let raw = u64::from_le_bytes([
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
                bytes[offset + 8],
            ]);
            Ok((raw, 9))
        }
    }
}

fn read_varint(bytes: &[u8], offset: usize) -> Result<(u64, usize), WalletError> {
    let mut value = 0u64;
    let mut cursor = offset;

    loop {
        let current = *bytes
            .get(cursor)
            .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
        cursor += 1;
        value = (value << 7) | u64::from(current & 0x7f);
        if (current & 0x80) != 0 {
            value = value
                .checked_add(1)
                .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
        } else {
            return Ok((value, cursor - offset));
        }
    }
}

fn read_var_slice(bytes: &[u8], offset: usize) -> Result<(&[u8], usize), WalletError> {
    let (length, length_size) = read_compact_size(bytes, offset)?;
    let start = offset + length_size;
    let end = start
        .checked_add(length as usize)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let slice = bytes
        .get(start..end)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    Ok((slice, end))
}

fn skip_vector(bytes: &[u8], offset: usize) -> Result<usize, WalletError> {
    let (count, count_size) = read_compact_size(bytes, offset)?;
    let mut cursor = offset + count_size;
    for _ in 0..count {
        let (_, next) = read_var_slice(bytes, cursor)?;
        cursor = next;
    }
    Ok(cursor)
}

fn skip_fixed_array(bytes: &[u8], offset: usize, item_len: usize) -> Result<usize, WalletError> {
    let (count, count_size) = read_compact_size(bytes, offset)?;
    let total_len = (count as usize)
        .checked_mul(item_len)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let start = offset + count_size;
    let end = start
        .checked_add(total_len)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if bytes.len() < end {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    Ok(end)
}

pub(crate) fn write_compact_size(out: &mut Vec<u8>, value: usize) {
    match value {
        0x00..=0xfc => out.push(value as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(value as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(value as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&(value as u64).to_le_bytes());
        }
    }
}

pub(crate) fn write_var_slice(out: &mut Vec<u8>, value: &[u8]) {
    write_compact_size(out, value.len());
    out.extend_from_slice(value);
}

pub(crate) fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    let mut encoded = Vec::new();

    loop {
        let mut current = (value & 0x7f) as u8;
        if !encoded.is_empty() {
            current |= 0x80;
        }
        encoded.push(current);

        if value <= 0x7f {
            break;
        }

        value = (value >> 7)
            .checked_sub(1)
            .expect("value is greater than 0x7f when subtracting one");
    }

    encoded.reverse();
    out.extend_from_slice(&encoded);
}

pub(crate) fn encode_compact_i_address(address: &str) -> Result<Vec<u8>, WalletError> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;

    if decoded.len() != 21 || decoded[0] != I_ADDRESS_VERSION as u8 {
        return Err(WalletError::InvalidAddress);
    }

    let mut encoded = Vec::with_capacity(22);
    write_compact_size(&mut encoded, 1);
    write_compact_size(&mut encoded, COMPACT_ADDRESS_TYPE_I_ADDRESS as usize);
    encoded.extend_from_slice(&decoded[1..]);
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_identity_signature_hash, get_raw_envelope_sha256, parse_generic_envelope_hex,
    };

    const SIGNED_REQUEST_HEX: &str = "0195001002050102a6ef9ea235635e328124ff3429db9f9e91b64e2d0102333e45170feadf2565b8512f1fe92af448ccb8e24902058bbc0e0001411f7d22d03c6ff11539f76be72aeab046ca23d3a8c31e686e809cd7e436e545d6ab0e1bb78bcae64cdff930c86220a481c43ab4856c49914deaf40a8763d68e4212fe2b8dab690201170101025d1d62a4f01ffca0cc061665d2e33e47c328fb6601021468747470733a2f2f7777772e76657275732e696f";
    const EXPECTED_RAW_SHA256: &str =
        "eebf8a0602c116bd35d14319883dcb547a493df882b734b509fe5f86b851edd7";
    const EXPECTED_IDENTITY_HASH: &str =
        "a07b699e8c927fbc1dc680e9d1ed61ed139c70efa48b2839f0522e7b21fb46ef";
    const SIGNED_BLOCK_HEIGHT: u32 = 965_771;

    #[test]
    fn generic_request_hashes_match_verus_typescript_primitives() {
        let parsed = parse_generic_envelope_hex(SIGNED_REQUEST_HEX).expect("request parses");

        let raw_sha256 = get_raw_envelope_sha256(&parsed);
        assert_eq!(hex::encode(raw_sha256), EXPECTED_RAW_SHA256);

        let identity_hash = compute_identity_signature_hash(
            &parsed.signature_data,
            SIGNED_BLOCK_HEIGHT,
            raw_sha256,
        )
        .expect("identity hash");
        assert_eq!(hex::encode(identity_hash), EXPECTED_IDENTITY_HASH);
    }
}
