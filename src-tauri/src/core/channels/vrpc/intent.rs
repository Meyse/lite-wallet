use std::collections::BTreeMap;
use std::io::Cursor;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use bitcoin::consensus::Decodable;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::channels::vrpc::common::VrpcInputRef;
use crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex as decode_verus_tx;
use crate::types::WalletError;
use zcash_client_backend::encoding::decode_payment_address;

const OP_CHECKCRYPTOCONDITION: u8 = 0xcc;
const OP_DROP: u8 = 0x75;
const EVAL_NONE: u8 = 0;
const EVAL_RESERVE_TRANSFER: u8 = 8;
const EVAL_RESERVE_OUTPUT: u8 = 9;
const EVAL_IDENTITY_PRIMARY: u8 = 14;
const EVAL_IDENTITY_REVOKE: u8 = 15;
const EVAL_IDENTITY_RECOVER: u8 = 16;
// Canonical EVAL_IDENTITY_RECOVER contract destination from VerusCoin
// src/cc/CCcustom.cpp (IdentityRecoverAddr). Tokenized identity recovery must
// not accept an arbitrary caller-selected public-key hash in this slot.
const IDENTITY_RECOVER_CONTRACT_ADDRESS: &str = "RRw9rJMPwdNqC1wgXn5vryJwMDyBgpXjYT";
const FLAG_IDENTITY_REVOKED: u32 = 0x8000;
const FLAG_IDENTITY_TOKENIZED_CONTROL: u32 = 0x4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IdentityControlIntent {
    pub version: u32,
    pub flags: u32,
    pub primary_addresses: Vec<Vec<u8>>,
    pub minimum_signatures: u32,
    pub parent: Vec<u8>,
    pub name: Vec<u8>,
    pub content_multimap: Vec<(Vec<u8>, Vec<Vec<u8>>)>,
    pub legacy_content_map: Vec<(Vec<u8>, Vec<u8>)>,
    pub content_map: Vec<(Vec<u8>, Vec<u8>)>,
    pub revocation_authority: Vec<u8>,
    pub recovery_authority: Vec<u8>,
    pub private_addresses: Vec<Vec<u8>>,
    pub system_id: Option<Vec<u8>>,
    pub unlock_after: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SemanticDestination {
    pub destination_type: u8,
    pub destination_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum VrpcOutputIntent {
    NativePayment {
        destination: SemanticDestination,
        amount_sats: u64,
    },
    TokenPayment {
        destination: SemanticDestination,
        currency_id: Vec<u8>,
        amount_sats: u64,
        output_value_sats: u64,
    },
    ReserveTransfer {
        system_currency_id: Vec<u8>,
        source_currency_id: Vec<u8>,
        amount_sats: u64,
        flags: u64,
        fee_currency_id: Vec<u8>,
        fee_sats: u64,
        destination: SemanticDestination,
        dest_currency_id: Option<Vec<u8>>,
        second_reserve_id: Option<Vec<u8>>,
        dest_system_id: Option<Vec<u8>>,
        output_value_sats: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecodedReserveTransfer {
    source_currency_id: Vec<u8>,
    amount_sats: u64,
    flags: u64,
    fee_currency_id: Vec<u8>,
    fee_sats: u64,
    destination: SemanticDestination,
    dest_currency_id: Vec<u8>,
    second_reserve_id: Option<Vec<u8>>,
    dest_system_id: Option<Vec<u8>>,
}

#[derive(Debug)]
struct DecodedOutput {
    value: u64,
    script: Vec<u8>,
}

#[derive(Debug)]
struct OptCcParams {
    version: u8,
    eval_code: u8,
    required_signatures: u8,
    destinations: Vec<SemanticDestination>,
    vdata: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct CcScriptParams {
    master: OptCcParams,
    condition: OptCcParams,
}

pub(crate) fn decode_base58_destination(address: &str) -> Result<SemanticDestination, WalletError> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;
    if decoded.len() != 21 {
        return Err(WalletError::InvalidAddress);
    }
    let destination_type = match decoded[0] {
        60 => 2,
        102 => 4,
        _ => return Err(WalletError::InvalidAddress),
    };
    Ok(SemanticDestination {
        destination_type,
        destination_bytes: decoded[1..].to_vec(),
    })
}

pub(crate) fn decode_semantic_destination(
    address: &str,
) -> Result<SemanticDestination, WalletError> {
    let trimmed = address.trim();
    if trimmed.starts_with('R') || trimmed.starts_with('i') {
        return decode_base58_destination(trimmed);
    }
    if let Some(raw) = trimmed.strip_prefix("0x") {
        if raw.len() != 40 {
            return Err(WalletError::InvalidAddress);
        }
        return Ok(SemanticDestination {
            destination_type: 9,
            destination_bytes: hex::decode(raw).map_err(|_| WalletError::InvalidAddress)?,
        });
    }
    let payment =
        decode_sapling_payment_address(trimmed).map_err(|_| WalletError::InvalidAddress)?;
    Ok(SemanticDestination {
        destination_type: 7,
        destination_bytes: payment.to_bytes().to_vec(),
    })
}

pub(crate) fn decode_currency_id(currency_id: &str) -> Result<Vec<u8>, WalletError> {
    let decoded = bs58::decode(currency_id.trim())
        .with_check(None)
        .into_vec()
        .map_err(|_| WalletError::OperationFailed)?;
    if decoded.len() != 21 || decoded[0] != 102 {
        return Err(WalletError::OperationFailed);
    }
    Ok(decoded[1..].to_vec())
}

pub(crate) fn validate_transaction_intent(
    tx_hex: &str,
    intent: &VrpcOutputIntent,
    change_address: &str,
    input_total_sats: i64,
    expected_fee_sats: i64,
    inputs: &[VrpcInputRef],
) -> Result<(), WalletError> {
    let outputs = decode_outputs(tx_hex)?;
    let change = decode_base58_destination(change_address)?;
    if change.destination_type != 2 {
        return Err(WalletError::OperationFailed);
    }
    let change_script = p2pkh_script(&change.destination_bytes)?;
    let mut matched = 0usize;
    let mut output_total = 0u64;
    let input_reserves = reserve_values_from_inputs(inputs)?;
    let mut output_reserves = BTreeMap::new();

    for output in outputs {
        output_total = output_total
            .checked_add(output.value)
            .ok_or(WalletError::OperationFailed)?;
        if output_matches_intent(&output, intent)? {
            matched += 1;
            add_output_reserve_values(&output, &mut output_reserves)?;
            continue;
        }
        if output.script == change_script {
            continue;
        }
        if !add_change_reserve_values(&output, &change, &mut output_reserves)? {
            return Err(WalletError::OperationFailed);
        }
    }

    if matched != 1 {
        return Err(WalletError::OperationFailed);
    }
    let output_total = i64::try_from(output_total).map_err(|_| WalletError::OperationFailed)?;
    let actual_fee = input_total_sats
        .checked_sub(output_total)
        .ok_or(WalletError::OperationFailed)?;
    if actual_fee != expected_fee_sats || actual_fee <= 0 {
        return Err(WalletError::OperationFailed);
    }
    validate_reserve_conservation(&input_reserves, &output_reserves, intent)?;
    Ok(())
}

/// Bind fields that verusd derives rather than accepting directly from the
/// user request. The unfunded template must already match every user-selected
/// reserve-transfer field; after this step, funding and signing validate the
/// daemon-derived destination currency exactly as well.
pub(crate) fn bind_derived_reserve_fields(
    tx_hex: &str,
    intent: &mut VrpcOutputIntent,
) -> Result<(), WalletError> {
    let VrpcOutputIntent::ReserveTransfer {
        dest_currency_id, ..
    } = &*intent
    else {
        return Ok(());
    };
    if dest_currency_id.is_some() {
        return Ok(());
    }

    let outputs = decode_outputs(tx_hex)?;
    let mut derived = None;
    for output in outputs {
        if !output_matches_intent(&output, intent)? {
            continue;
        }
        let params = parse_cc_params(&output.script)?;
        validate_single_condition_master(&params)?;
        if params.condition.eval_code != EVAL_RESERVE_TRANSFER || params.condition.vdata.len() != 1
        {
            continue;
        }
        let decoded = parse_reserve_transfer(&params.condition.vdata[0])?;
        if derived.replace(decoded.dest_currency_id).is_some() {
            return Err(WalletError::OperationFailed);
        }
    }
    let derived = derived.ok_or(WalletError::OperationFailed)?;
    if let VrpcOutputIntent::ReserveTransfer {
        dest_currency_id, ..
    } = intent
    {
        *dest_currency_id = Some(derived);
    }
    Ok(())
}

pub(crate) fn identity_control_intent_from_json(
    identity: &Value,
) -> Result<IdentityControlIntent, WalletError> {
    let version = json_u32(identity, &["version"])?;
    let flags = json_u32(identity, &["flags"])?;
    let minimum_signatures = json_u32(identity, &["minimumsignatures", "minimumSignatures"])?;
    let parent = json_address_hash(identity, &["parent"])?;
    let name = identity
        .get("name")
        .and_then(Value::as_str)
        .map(|value| value.as_bytes().to_vec())
        .ok_or(WalletError::IdentityBuildFailed)?;
    let content_multimap = json_content_multimap(identity)?;
    let content_map = json_content_map(identity)?;
    let legacy_content_map = if version < 3 {
        content_map.clone()
    } else {
        Vec::new()
    };
    let primary_addresses = json_array(identity, &["primaryaddresses", "primaryAddresses"])?
        .iter()
        .map(|value| {
            let address = value.as_str().ok_or(WalletError::IdentityBuildFailed)?;
            decode_primary_destination(address)
        })
        .collect::<Result<Vec<_>, WalletError>>()?;
    let revocation_authority =
        json_address_hash(identity, &["revocationauthority", "revocationAuthority"])?;
    let recovery_authority =
        json_address_hash(identity, &["recoveryauthority", "recoveryAuthority"])?;
    let private_addresses = match identity
        .get("privateaddresses")
        .or_else(|| identity.get("privateAddresses"))
        .or_else(|| identity.get("privateaddress"))
        .or_else(|| identity.get("privateAddress"))
    {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| decode_private_address(value.as_str()))
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::String(value)) if !value.trim().is_empty() => {
            vec![decode_private_address(Some(value))?]
        }
        _ => Vec::new(),
    };
    let system_id = if version >= 2 {
        Some(json_address_hash(identity, &["systemid", "systemId"])?)
    } else {
        None
    };
    let unlock_after = if version >= 2 {
        Some(json_u32(
            identity,
            &["timelock", "unlockafter", "unlockAfter"],
        )?)
    } else {
        None
    };
    Ok(IdentityControlIntent {
        version,
        flags,
        primary_addresses,
        minimum_signatures,
        parent,
        name,
        content_multimap,
        legacy_content_map,
        content_map,
        revocation_authority,
        recovery_authority,
        private_addresses,
        system_id,
        unlock_after,
    })
}

fn json_content_multimap(identity: &Value) -> Result<Vec<(Vec<u8>, Vec<Vec<u8>>)>, WalletError> {
    let Some(value) = identity
        .get("contentmultimap")
        .or_else(|| identity.get("contentMultiMap"))
    else {
        return Ok(Vec::new());
    };
    let object = value.as_object().ok_or(WalletError::IdentityBuildFailed)?;
    let mut entries = Vec::with_capacity(object.len());
    for (key, value) in object {
        let key = decode_base58_destination(key)
            .map_err(|_| WalletError::IdentityBuildFailed)?
            .destination_bytes;
        let values = match value {
            Value::Array(values) => values
                .iter()
                .map(normalize_vdxf_univalue)
                .collect::<Result<Vec<_>, _>>()?,
            value => vec![normalize_vdxf_univalue(value)?],
        };
        if values.is_empty() || values.iter().any(Vec::is_empty) {
            return Err(WalletError::IdentityBuildFailed);
        }
        entries.push((key, values));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

fn decode_primary_destination(value: &str) -> Result<Vec<u8>, WalletError> {
    let trimmed = value.trim();
    if trimmed.len() == 66 {
        let bytes = hex::decode(trimmed).map_err(|_| WalletError::IdentityBuildFailed)?;
        if bytes.len() != 33 || !matches!(bytes.first(), Some(2 | 3)) {
            return Err(WalletError::IdentityBuildFailed);
        }
        return Ok(bytes);
    }
    Ok(decode_base58_destination(trimmed)?.destination_bytes)
}

pub(crate) fn normalize_vdxf_univalue(value: &Value) -> Result<Vec<u8>, WalletError> {
    if let Some(raw) = value.as_str() {
        return decode_identity_hex(raw);
    }
    if let Some(values) = value.as_array() {
        let mut encoded = Vec::new();
        for value in values {
            encoded.extend_from_slice(&normalize_vdxf_univalue(value)?);
        }
        if encoded.is_empty() {
            return Err(WalletError::IdentityBuildFailed);
        }
        return Ok(encoded);
    }
    let object = value.as_object().ok_or(WalletError::IdentityBuildFailed)?;
    if object.len() != 1 {
        return Err(WalletError::IdentityBuildFailed);
    }
    let (key, value) = object
        .iter()
        .next()
        .ok_or(WalletError::IdentityBuildFailed)?;
    match key.as_str() {
        "serializedhex" => {
            decode_identity_hex(value.as_str().ok_or(WalletError::IdentityBuildFailed)?)
        }
        "serializedbase64" => BASE64_STANDARD
            .decode(
                value
                    .as_str()
                    .ok_or(WalletError::IdentityBuildFailed)?
                    .trim(),
            )
            .map_err(|_| WalletError::IdentityBuildFailed),
        "message" => {
            let message = value.as_str().ok_or(WalletError::IdentityBuildFailed)?;
            if message.is_empty() {
                return Err(WalletError::IdentityBuildFailed);
            }
            Ok(message.as_bytes().to_vec())
        }
        "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c" => {
            let bytes = value
                .as_str()
                .ok_or(WalletError::IdentityBuildFailed)?
                .as_bytes();
            encode_length_prefixed_vdxf_value(key, bytes)
        }
        "iKMhRLX1JHQihVZx2t2pAWW2uzmK6AzwW3" => {
            let bytes =
                decode_identity_hex(value.as_str().ok_or(WalletError::IdentityBuildFailed)?)?;
            encode_length_prefixed_vdxf_value(key, &bytes)
        }
        // CDataDescriptor. Profile updates return these structured objects from
        // decoderawtransaction; normalize them to the exact identity bytes so
        // the transaction intent binds the provider-built template.
        "i4GC1YGEVD21afWudGoFJVdnfjJ5XWnCQv" => {
            let descriptor = value.as_object().ok_or(WalletError::IdentityBuildFailed)?;
            let version = descriptor
                .get("version")
                .and_then(Value::as_u64)
                .ok_or(WalletError::IdentityBuildFailed)?;
            let flags = descriptor
                .get("flags")
                .and_then(Value::as_u64)
                .ok_or(WalletError::IdentityBuildFailed)?;
            if version != 1 || flags & !0xff != 0 {
                return Err(WalletError::IdentityBuildFailed);
            }
            let mut inner = varint_bytes(version);
            inner.extend_from_slice(&varint_bytes(flags));
            if flags & 0x80 != 0 {
                let vdxf_key = descriptor
                    .get("vdxfkey")
                    .and_then(Value::as_str)
                    .ok_or(WalletError::IdentityBuildFailed)?;
                inner.extend_from_slice(&decode_base58_destination(vdxf_key)?.destination_bytes);
            }
            push_hex_vector(&mut inner, descriptor.get("objectdata"), 2 * 1024 * 1024)?;
            if flags & 0x20 != 0 {
                push_string(&mut inner, descriptor.get("label"), 64)?;
            }
            if flags & 0x40 != 0 {
                push_string(&mut inner, descriptor.get("mimetype"), 128)?;
            }
            if flags & 0x02 != 0 {
                push_hex_vector(&mut inner, descriptor.get("salt"), 64)?;
            }
            if flags & 0x04 != 0 {
                push_hex_vector(&mut inner, descriptor.get("epk"), 32)?;
            }
            if flags & 0x08 != 0 {
                push_hex_vector(&mut inner, descriptor.get("ivk"), 32)?;
            }
            if flags & 0x10 != 0 {
                push_hex_vector(&mut inner, descriptor.get("ssk"), 32)?;
            }
            encode_vdxf_wrapper(key, version, &inner)
        }
        // CContentMultiMapRemove with exact key/value-hash semantics.
        "i5Zkx5Z7tEfh42xtKfwbJ5LgEWE9rEgpFY" => {
            let removal = value.as_object().ok_or(WalletError::IdentityBuildFailed)?;
            let version = removal.get("version").and_then(Value::as_u64).unwrap_or(1);
            let action = removal
                .get("action")
                .and_then(Value::as_u64)
                .ok_or(WalletError::IdentityBuildFailed)?;
            if version != 1 || !(1..=4).contains(&action) {
                return Err(WalletError::IdentityBuildFailed);
            }
            let mut inner = varint_bytes(version);
            inner.extend_from_slice(&varint_bytes(action));
            if action != 4 {
                let entry_key = removal
                    .get("entrykey")
                    .and_then(Value::as_str)
                    .ok_or(WalletError::IdentityBuildFailed)?;
                inner.extend_from_slice(&decode_base58_destination(entry_key)?.destination_bytes);
                if action != 3 {
                    let mut value_hash = decode_identity_hex(
                        removal
                            .get("valuehash")
                            .and_then(Value::as_str)
                            .ok_or(WalletError::IdentityBuildFailed)?,
                    )?;
                    if value_hash.len() != 32 {
                        return Err(WalletError::IdentityBuildFailed);
                    }
                    value_hash.reverse();
                    inner.extend_from_slice(&value_hash);
                }
            }
            encode_vdxf_wrapper(key, version, &inner)
        }
        _ => Err(WalletError::IdentityBuildFailed),
    }
}

fn push_hex_vector(
    target: &mut Vec<u8>,
    value: Option<&Value>,
    maximum: usize,
) -> Result<(), WalletError> {
    let bytes = decode_identity_hex(
        value
            .and_then(Value::as_str)
            .ok_or(WalletError::IdentityBuildFailed)?,
    )?;
    if bytes.len() > maximum {
        return Err(WalletError::IdentityBuildFailed);
    }
    target.extend_from_slice(&compact_size_bytes(bytes.len()));
    target.extend_from_slice(&bytes);
    Ok(())
}

fn push_string(
    target: &mut Vec<u8>,
    value: Option<&Value>,
    maximum: usize,
) -> Result<(), WalletError> {
    let bytes = value
        .and_then(Value::as_str)
        .ok_or(WalletError::IdentityBuildFailed)?
        .as_bytes();
    if bytes.len() > maximum {
        return Err(WalletError::IdentityBuildFailed);
    }
    target.extend_from_slice(&compact_size_bytes(bytes.len()));
    target.extend_from_slice(bytes);
    Ok(())
}

fn encode_vdxf_wrapper(key: &str, version: u64, inner: &[u8]) -> Result<Vec<u8>, WalletError> {
    let mut encoded = decode_base58_destination(key)?.destination_bytes;
    encoded.extend_from_slice(&varint_bytes(version));
    encoded.extend_from_slice(&compact_size_bytes(inner.len()));
    encoded.extend_from_slice(inner);
    Ok(encoded)
}

fn varint_bytes(mut value: u64) -> Vec<u8> {
    let mut encoded = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        if !encoded.is_empty() {
            byte |= 0x80;
        }
        encoded.push(byte);
        if value <= 0x7f {
            break;
        }
        value = (value >> 7).saturating_sub(1);
    }
    encoded.reverse();
    encoded
}

fn encode_length_prefixed_vdxf_value(key: &str, value: &[u8]) -> Result<Vec<u8>, WalletError> {
    if value.is_empty() {
        return Err(WalletError::IdentityBuildFailed);
    }
    let mut encoded = decode_base58_destination(key)?.destination_bytes;
    encoded.push(1); // VDXF_UNI_VALUE_VERSION_CURRENT, encoded as Verus varint.
    let mut inner = compact_size_bytes(value.len());
    inner.extend_from_slice(value);
    encoded.extend_from_slice(&compact_size_bytes(inner.len()));
    encoded.extend_from_slice(&inner);
    Ok(encoded)
}

fn compact_size_bytes(value: usize) -> Vec<u8> {
    if value < 253 {
        vec![value as u8]
    } else if value <= u16::MAX as usize {
        let mut encoded = vec![253];
        encoded.extend_from_slice(&(value as u16).to_le_bytes());
        encoded
    } else if value <= u32::MAX as usize {
        let mut encoded = vec![254];
        encoded.extend_from_slice(&(value as u32).to_le_bytes());
        encoded
    } else {
        let mut encoded = vec![255];
        encoded.extend_from_slice(&(value as u64).to_le_bytes());
        encoded
    }
}

fn json_content_map(identity: &Value) -> Result<Vec<(Vec<u8>, Vec<u8>)>, WalletError> {
    let Some(value) = identity
        .get("contentmap")
        .or_else(|| identity.get("contentMap"))
    else {
        return Ok(Vec::new());
    };
    let object = value.as_object().ok_or(WalletError::IdentityBuildFailed)?;
    let mut entries = Vec::with_capacity(object.len());
    for (key, value) in object {
        let mut key = hex::decode(key).map_err(|_| WalletError::IdentityBuildFailed)?;
        let mut value =
            decode_identity_hex(value.as_str().ok_or(WalletError::IdentityBuildFailed)?)?;
        if key.len() != 20 || value.len() != 32 {
            return Err(WalletError::IdentityBuildFailed);
        }
        // verusd renders uint160/uint256 map entries in display byte order.
        key.reverse();
        value.reverse();
        entries.push((key, value));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

fn decode_identity_hex(value: &str) -> Result<Vec<u8>, WalletError> {
    hex::decode(value.trim()).map_err(|_| WalletError::IdentityBuildFailed)
}

pub(crate) fn validate_identity_control_intent(
    tx_hex: &str,
    expected: &IdentityControlIntent,
) -> Result<(), WalletError> {
    let outputs = decode_outputs(tx_hex)?;
    let mut matched = 0usize;
    for output in outputs {
        let Ok(params) = parse_cc_params(&output.script) else {
            continue;
        };
        if params.condition.eval_code != EVAL_IDENTITY_PRIMARY || params.condition.vdata.is_empty()
        {
            continue;
        }
        let actual = parse_identity_control(&params.condition.vdata[0])?;
        if &actual != expected {
            return Err(WalletError::IdentityBuildFailed);
        }
        validate_identity_master(&params, expected)?;
        matched += 1;
    }
    if matched != 1 {
        return Err(WalletError::IdentityBuildFailed);
    }
    Ok(())
}

pub(crate) fn validate_identity_transaction_intent(
    tx_hex: &str,
    expected: &IdentityControlIntent,
    change_address: &str,
    input_total_sats: i64,
    expected_fee_sats: i64,
) -> Result<(), WalletError> {
    let outputs = decode_outputs(tx_hex)?;
    let change = decode_base58_destination(change_address)?;
    if change.destination_type != 2 {
        return Err(WalletError::IdentityBuildFailed);
    }
    let change_script = p2pkh_script(&change.destination_bytes)?;
    let mut identity_outputs = 0usize;
    let mut output_total = 0u64;

    for output in outputs {
        output_total = output_total
            .checked_add(output.value)
            .ok_or(WalletError::IdentityBuildFailed)?;
        if let Ok(params) = parse_cc_params(&output.script) {
            if params.condition.eval_code == EVAL_IDENTITY_PRIMARY
                && !params.condition.vdata.is_empty()
            {
                if parse_identity_control(&params.condition.vdata[0])? != *expected {
                    return Err(WalletError::IdentityBuildFailed);
                }
                validate_identity_master(&params, expected)?;
                identity_outputs += 1;
                continue;
            }
        }
        if output.script != change_script {
            return Err(WalletError::IdentityBuildFailed);
        }
    }

    let output_total = i64::try_from(output_total).map_err(|_| WalletError::IdentityBuildFailed)?;
    let actual_fee = input_total_sats
        .checked_sub(output_total)
        .ok_or(WalletError::IdentityBuildFailed)?;
    if identity_outputs != 1 || actual_fee != expected_fee_sats || actual_fee <= 0 {
        return Err(WalletError::IdentityBuildFailed);
    }
    Ok(())
}

fn json_u32(identity: &Value, names: &[&str]) -> Result<u32, WalletError> {
    let value = names
        .iter()
        .find_map(|name| identity.get(*name))
        .ok_or(WalletError::IdentityBuildFailed)?;
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(WalletError::IdentityBuildFailed)
}

fn json_array<'a>(identity: &'a Value, names: &[&str]) -> Result<&'a [Value], WalletError> {
    names
        .iter()
        .find_map(|name| identity.get(*name))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(WalletError::IdentityBuildFailed)
}

fn json_address_hash(identity: &Value, names: &[&str]) -> Result<Vec<u8>, WalletError> {
    let address = names
        .iter()
        .find_map(|name| identity.get(*name))
        .and_then(Value::as_str)
        .ok_or(WalletError::IdentityBuildFailed)?;
    Ok(decode_base58_destination(address)?.destination_bytes)
}

fn decode_private_address(address: Option<&str>) -> Result<Vec<u8>, WalletError> {
    let address = address.ok_or(WalletError::IdentityBuildFailed)?;
    let payment =
        decode_sapling_payment_address(address).map_err(|_| WalletError::IdentityBuildFailed)?;
    Ok(payment.to_bytes().to_vec())
}

fn decode_sapling_payment_address(address: &str) -> Result<sapling::PaymentAddress, WalletError> {
    let normalized = address.trim().to_ascii_lowercase();
    decode_payment_address("zs", &normalized)
        .or_else(|_| decode_payment_address("ztestsapling", &normalized))
        .map_err(|_| WalletError::InvalidAddress)
}

fn output_matches_intent(
    output: &DecodedOutput,
    intent: &VrpcOutputIntent,
) -> Result<bool, WalletError> {
    match intent {
        VrpcOutputIntent::NativePayment {
            destination,
            amount_sats,
        } => {
            if output.value != *amount_sats {
                return Ok(false);
            }
            if destination.destination_type == 2 {
                return Ok(output.script == p2pkh_script(&destination.destination_bytes)?);
            }
            let params = parse_cc_params(&output.script)?;
            validate_single_condition_master(&params)?;
            Ok(params.condition.eval_code == EVAL_NONE
                && params.condition.vdata.is_empty()
                && params.condition.destinations.as_slice() == [destination.clone()])
        }
        VrpcOutputIntent::TokenPayment {
            destination,
            currency_id,
            amount_sats,
            output_value_sats,
        } => {
            if output.value != *output_value_sats {
                return Ok(false);
            }
            let params = parse_cc_params(&output.script)?;
            validate_single_condition_master(&params)?;
            if params.condition.eval_code != EVAL_RESERVE_OUTPUT
                || params.condition.destinations.as_slice() != [destination.clone()]
                || params.condition.vdata.len() != 1
            {
                return Ok(false);
            }
            let (decoded_values, consumed) = parse_token_output(&params.condition.vdata[0], 0)?;
            Ok(consumed == params.condition.vdata[0].len()
                && decoded_values.len() == 1
                && decoded_values.get(currency_id) == Some(amount_sats))
        }
        VrpcOutputIntent::ReserveTransfer {
            system_currency_id: _,
            source_currency_id,
            amount_sats,
            flags,
            fee_currency_id,
            fee_sats,
            destination,
            dest_currency_id,
            second_reserve_id,
            dest_system_id,
            output_value_sats,
        } => {
            if output.value != *output_value_sats {
                return Ok(false);
            }
            let params = parse_cc_params(&output.script)?;
            validate_single_condition_master(&params)?;
            if params.condition.eval_code != EVAL_RESERVE_TRANSFER
                || params.condition.vdata.len() != 1
            {
                return Ok(false);
            }
            let decoded = parse_reserve_transfer(&params.condition.vdata[0])?;
            Ok(decoded.source_currency_id == *source_currency_id
                && decoded.amount_sats == *amount_sats
                && decoded.flags == *flags
                && decoded.fee_currency_id == *fee_currency_id
                && decoded.fee_sats == *fee_sats
                && decoded.destination == *destination
                && dest_currency_id
                    .as_ref()
                    .map(|expected| decoded.dest_currency_id == *expected)
                    .unwrap_or(true)
                && decoded.second_reserve_id == *second_reserve_id
                && decoded.dest_system_id == *dest_system_id)
        }
    }
}

fn checked_add_reserve(
    values: &mut BTreeMap<Vec<u8>, u64>,
    currency: Vec<u8>,
    amount: u64,
) -> Result<(), WalletError> {
    let next = values
        .get(&currency)
        .copied()
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or(WalletError::OperationFailed)?;
    values.insert(currency, next);
    Ok(())
}

fn reserve_values_from_script(
    script: &[u8],
) -> Result<Option<BTreeMap<Vec<u8>, u64>>, WalletError> {
    let Ok(params) = parse_cc_params(script) else {
        return Ok(None);
    };
    if params.condition.eval_code != EVAL_RESERVE_OUTPUT {
        return Ok(None);
    }
    validate_single_condition_master(&params)?;
    if params.condition.vdata.len() != 1 {
        return Err(WalletError::OperationFailed);
    }
    let (values, consumed) = parse_token_output(&params.condition.vdata[0], 0)?;
    if consumed != params.condition.vdata[0].len() || values.is_empty() {
        return Err(WalletError::OperationFailed);
    }
    Ok(Some(values))
}

fn reserve_values_from_inputs(
    inputs: &[VrpcInputRef],
) -> Result<BTreeMap<Vec<u8>, u64>, WalletError> {
    let mut totals = BTreeMap::new();
    for input in inputs {
        let Some(script) = input.script_pub_key.as_deref() else {
            continue;
        };
        let script = hex::decode(script).map_err(|_| WalletError::OperationFailed)?;
        if let Some(values) = reserve_values_from_script(&script)? {
            for (currency, amount) in values {
                checked_add_reserve(&mut totals, currency, amount)?;
            }
        }
    }
    Ok(totals)
}

fn add_output_reserve_values(
    output: &DecodedOutput,
    totals: &mut BTreeMap<Vec<u8>, u64>,
) -> Result<(), WalletError> {
    if let Some(values) = reserve_values_from_script(&output.script)? {
        for (currency, amount) in values {
            checked_add_reserve(totals, currency, amount)?;
        }
    }
    Ok(())
}

fn add_change_reserve_values(
    output: &DecodedOutput,
    change: &SemanticDestination,
    totals: &mut BTreeMap<Vec<u8>, u64>,
) -> Result<bool, WalletError> {
    let Ok(params) = parse_cc_params(&output.script) else {
        return Ok(false);
    };
    validate_single_condition_master(&params)?;
    if params.condition.eval_code != EVAL_RESERVE_OUTPUT
        || params.condition.destinations.as_slice() != [change.clone()]
    {
        return Ok(false);
    }
    add_output_reserve_values(output, totals)?;
    Ok(true)
}

fn validate_reserve_conservation(
    input: &BTreeMap<Vec<u8>, u64>,
    output: &BTreeMap<Vec<u8>, u64>,
    intent: &VrpcOutputIntent,
) -> Result<(), WalletError> {
    let mut expected = input.clone();
    if let VrpcOutputIntent::ReserveTransfer {
        system_currency_id,
        source_currency_id,
        amount_sats,
        fee_currency_id,
        fee_sats,
        ..
    } = intent
    {
        for (currency, amount) in [
            (source_currency_id, amount_sats),
            (fee_currency_id, fee_sats),
        ] {
            if currency == system_currency_id {
                continue;
            }
            let remaining = expected
                .get(currency)
                .copied()
                .ok_or(WalletError::OperationFailed)?
                .checked_sub(*amount)
                .ok_or(WalletError::OperationFailed)?;
            if remaining == 0 {
                expected.remove(currency);
            } else {
                expected.insert(currency.clone(), remaining);
            }
        }
    }
    if &expected != output {
        return Err(WalletError::OperationFailed);
    }
    Ok(())
}

fn decode_outputs(tx_hex: &str) -> Result<Vec<DecodedOutput>, WalletError> {
    if let Ok(tx) = decode_verus_tx(tx_hex) {
        return Ok(tx
            .outputs
            .into_iter()
            .map(|output| DecodedOutput {
                value: output.value,
                script: output.script_pub_key,
            })
            .collect());
    }
    let raw = hex::decode(tx_hex.trim().trim_start_matches("0x"))
        .map_err(|_| WalletError::OperationFailed)?;
    let tx: bitcoin::Transaction = bitcoin::Transaction::consensus_decode(&mut Cursor::new(raw))
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(tx
        .output
        .into_iter()
        .map(|output| DecodedOutput {
            value: output.value.to_sat(),
            script: output.script_pubkey.into_bytes(),
        })
        .collect())
}

fn p2pkh_script(hash: &[u8]) -> Result<Vec<u8>, WalletError> {
    if hash.len() != 20 {
        return Err(WalletError::OperationFailed);
    }
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(hash);
    script.extend_from_slice(&[0x88, 0xac]);
    Ok(script)
}

fn read_push(script: &[u8], offset: &mut usize) -> Result<Vec<u8>, WalletError> {
    let opcode = *script.get(*offset).ok_or(WalletError::OperationFailed)?;
    *offset += 1;
    let length = match opcode {
        0..=75 => opcode as usize,
        76 => {
            let length = *script.get(*offset).ok_or(WalletError::OperationFailed)? as usize;
            *offset += 1;
            length
        }
        77 => {
            let bytes = script
                .get(*offset..*offset + 2)
                .ok_or(WalletError::OperationFailed)?;
            *offset += 2;
            u16::from_le_bytes([bytes[0], bytes[1]]) as usize
        }
        78 => {
            let bytes = script
                .get(*offset..*offset + 4)
                .ok_or(WalletError::OperationFailed)?;
            *offset += 4;
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize
        }
        _ => return Err(WalletError::OperationFailed),
    };
    let value = script
        .get(*offset..*offset + length)
        .ok_or(WalletError::OperationFailed)?
        .to_vec();
    *offset += length;
    Ok(value)
}

fn parse_cc_params(script: &[u8]) -> Result<CcScriptParams, WalletError> {
    let mut offset = 0usize;
    let master = read_push(script, &mut offset)?;
    let master = parse_optcc_chunk(&master)?;
    if master.version != 3 || master.eval_code != EVAL_NONE || !master.vdata.is_empty() {
        return Err(WalletError::OperationFailed);
    }
    if script.get(offset) != Some(&OP_CHECKCRYPTOCONDITION) {
        return Err(WalletError::OperationFailed);
    }
    offset += 1;
    let params = read_push(script, &mut offset)?;
    if script.get(offset) != Some(&OP_DROP) || offset + 1 != script.len() {
        return Err(WalletError::OperationFailed);
    }
    Ok(CcScriptParams {
        master,
        condition: parse_optcc_chunk(&params)?,
    })
}

fn validate_single_condition_master(params: &CcScriptParams) -> Result<(), WalletError> {
    if params.condition.version != 3
        || params.condition.required_signatures != 1
        || params.condition.destinations.len() != 1
    {
        return Err(WalletError::OperationFailed);
    }
    // Verus MakeMofNCCScript indexes no destination for EVAL_NONE and the
    // condition's first destination for every other single-condition output.
    let expected_master_destinations = if params.condition.eval_code == EVAL_NONE {
        &[][..]
    } else {
        &params.condition.destinations[..1]
    };
    if params.master.required_signatures as usize != expected_master_destinations.len()
        || params.master.destinations.as_slice() != expected_master_destinations
    {
        return Err(WalletError::OperationFailed);
    }
    Ok(())
}

fn validate_identity_master(
    params: &CcScriptParams,
    identity: &IdentityControlIntent,
) -> Result<(), WalletError> {
    if params.condition.version != 3
        || params.condition.required_signatures != 1
        || params.condition.destinations.len() != 1
        || params.condition.destinations[0].destination_type != 4
        || params.master.required_signatures != 1
    {
        return Err(WalletError::IdentityBuildFailed);
    }

    // Identity outputs explicitly index the identity plus its live revoke and
    // recovery authorities, and repeat those authorities in nested conditions.
    let is_revoked = identity.flags & FLAG_IDENTITY_REVOKED != 0;
    let mut expected = vec![params.condition.destinations[0].clone()];
    if !is_revoked {
        expected.push(SemanticDestination {
            destination_type: 4,
            destination_bytes: identity.revocation_authority.clone(),
        });
    }
    expected.push(SemanticDestination {
        destination_type: 4,
        destination_bytes: identity.recovery_authority.clone(),
    });
    if params.master.destinations != expected {
        return Err(WalletError::IdentityBuildFailed);
    }

    let expected_condition_count = if is_revoked { 2 } else { 3 };
    if params.condition.vdata.len() != expected_condition_count {
        return Err(WalletError::IdentityBuildFailed);
    }
    let mut condition_index = 1;
    if !is_revoked {
        validate_identity_authority_condition(
            &params.condition.vdata[condition_index],
            EVAL_IDENTITY_REVOKE,
            &identity.revocation_authority,
            false,
        )?;
        condition_index += 1;
    }
    validate_identity_authority_condition(
        &params.condition.vdata[condition_index],
        EVAL_IDENTITY_RECOVER,
        &identity.recovery_authority,
        identity.flags & FLAG_IDENTITY_TOKENIZED_CONTROL != 0,
    )?;
    Ok(())
}

fn validate_identity_authority_condition(
    chunk: &[u8],
    expected_eval: u8,
    expected_authority: &[u8],
    tokenized_recovery: bool,
) -> Result<(), WalletError> {
    let condition = parse_optcc_chunk(chunk).map_err(|_| WalletError::IdentityBuildFailed)?;
    if condition.version != 3
        || condition.eval_code != expected_eval
        || condition.required_signatures != 1
        || !condition.vdata.is_empty()
    {
        return Err(WalletError::IdentityBuildFailed);
    }
    let authority = SemanticDestination {
        destination_type: 4,
        destination_bytes: expected_authority.to_vec(),
    };
    if tokenized_recovery {
        let authority_count = condition
            .destinations
            .iter()
            .filter(|destination| **destination == authority)
            .count();
        let public_key_hash_count = condition
            .destinations
            .iter()
            .filter(|destination| destination.destination_type == 2)
            .count();
        let canonical_contract = decode_base58_destination(IDENTITY_RECOVER_CONTRACT_ADDRESS)
            .map_err(|_| WalletError::IdentityBuildFailed)?;
        if condition.destinations.len() != 2
            || authority_count != 1
            || public_key_hash_count != 1
            || !condition.destinations.contains(&canonical_contract)
        {
            return Err(WalletError::IdentityBuildFailed);
        }
    } else if condition.destinations.as_slice() != [authority] {
        return Err(WalletError::IdentityBuildFailed);
    }
    Ok(())
}

fn parse_optcc_chunk(chunk: &[u8]) -> Result<OptCcParams, WalletError> {
    let mut offset = 0usize;
    let header = read_push(chunk, &mut offset)?;
    if header.len() != 4
        || !(1..=3).contains(&header[0])
        || header[1] > 0x1a
        || (header[0] < 3 && header[1] >= 2)
        || header[2] > header[3]
        || header[3] > 4
        || (header[0] < 3 && header[3] == 0)
    {
        return Err(WalletError::OperationFailed);
    }
    let mut chunks = Vec::new();
    while offset < chunk.len() {
        chunks.push(read_push(chunk, &mut offset)?);
    }
    let destination_count = usize::from(header[3]);
    if chunks.len() < destination_count {
        return Err(WalletError::OperationFailed);
    }
    let destinations = chunks[..destination_count]
        .iter()
        .map(|chunk| parse_tx_destination_chunk(chunk))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OptCcParams {
        version: header[0],
        eval_code: header[1],
        required_signatures: header[2],
        destinations,
        vdata: chunks[destination_count..].to_vec(),
    })
}

fn parse_tx_destination_chunk(chunk: &[u8]) -> Result<SemanticDestination, WalletError> {
    if chunk.len() == 20 {
        return Ok(SemanticDestination {
            destination_type: 2,
            destination_bytes: chunk.to_vec(),
        });
    }
    if chunk.len() == 33 {
        return Ok(SemanticDestination {
            destination_type: 1,
            destination_bytes: chunk.to_vec(),
        });
    }
    if chunk.len() < 2 {
        return Err(WalletError::OperationFailed);
    }
    let expected_length = match chunk[0] {
        1 => 33,
        2..=6 | 9 => 20,
        7 => 43,
        _ => return Err(WalletError::OperationFailed),
    };
    if chunk.len() != expected_length + 1 {
        return Err(WalletError::OperationFailed);
    }
    Ok(SemanticDestination {
        destination_type: chunk[0],
        destination_bytes: chunk[1..].to_vec(),
    })
}

fn read_compact_size(data: &[u8], offset: &mut usize) -> Result<u64, WalletError> {
    let marker = *data.get(*offset).ok_or(WalletError::OperationFailed)?;
    *offset += 1;
    let (value, width) = match marker {
        0..=252 => (u64::from(marker), 0),
        253 => (0, 2),
        254 => (0, 4),
        255 => (0, 8),
    };
    if width == 0 {
        return Ok(value);
    }
    let bytes = data
        .get(*offset..*offset + width)
        .ok_or(WalletError::OperationFailed)?;
    *offset += width;
    let mut padded = [0u8; 8];
    padded[..width].copy_from_slice(bytes);
    Ok(u64::from_le_bytes(padded))
}

fn read_varint(data: &[u8], offset: &mut usize) -> Result<u64, WalletError> {
    let mut value = 0u64;
    loop {
        let byte = *data.get(*offset).ok_or(WalletError::OperationFailed)?;
        *offset += 1;
        value = value
            .checked_shl(7)
            .and_then(|value| value.checked_add(u64::from(byte & 0x7f)))
            .ok_or(WalletError::OperationFailed)?;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        value = value.checked_add(1).ok_or(WalletError::OperationFailed)?;
    }
}

fn read_slice(data: &[u8], offset: &mut usize, length: usize) -> Result<Vec<u8>, WalletError> {
    let value = data
        .get(*offset..*offset + length)
        .ok_or(WalletError::OperationFailed)?
        .to_vec();
    *offset += length;
    Ok(value)
}

fn read_var_slice(data: &[u8], offset: &mut usize) -> Result<Vec<u8>, WalletError> {
    let length = usize::try_from(read_compact_size(data, offset)?)
        .map_err(|_| WalletError::OperationFailed)?;
    read_slice(data, offset, length)
}

fn parse_token_output(
    data: &[u8],
    mut offset: usize,
) -> Result<(BTreeMap<Vec<u8>, u64>, usize), WalletError> {
    let version = read_varint(data, &mut offset)?;
    let multivalue = version & 0x8000_0000 != 0;
    if version & !0x8000_0000 != 1 {
        return Err(WalletError::OperationFailed);
    }
    let count = if multivalue {
        read_compact_size(data, &mut offset)?
    } else {
        1
    };
    if count == 0 {
        return Err(WalletError::OperationFailed);
    }
    let mut values = BTreeMap::new();
    for _ in 0..count {
        let currency = read_slice(data, &mut offset, 20)?;
        let amount = if multivalue {
            let bytes = read_slice(data, &mut offset, 8)?;
            let amount =
                i64::from_le_bytes(bytes.try_into().map_err(|_| WalletError::OperationFailed)?);
            u64::try_from(amount).map_err(|_| WalletError::OperationFailed)?
        } else {
            read_varint(data, &mut offset)?
        };
        if amount == 0 || values.insert(currency, amount).is_some() {
            return Err(WalletError::OperationFailed);
        }
    }
    Ok((values, offset))
}

fn parse_transfer_destination(
    data: &[u8],
    offset: &mut usize,
) -> Result<SemanticDestination, WalletError> {
    let destination_type = *data.get(*offset).ok_or(WalletError::OperationFailed)?;
    *offset += 1;
    let destination_bytes = read_var_slice(data, offset)?;
    if destination_type & 0x80 != 0 {
        let _gateway_id = read_slice(data, offset, 20)?;
        let _gateway_code = read_slice(data, offset, 20)?;
        let _fees = read_slice(data, offset, 8)?;
    }
    if destination_type & 0x40 != 0 {
        let count = read_compact_size(data, offset)?;
        for _ in 0..count {
            let nested = read_var_slice(data, offset)?;
            let mut nested_offset = 0;
            parse_transfer_destination(&nested, &mut nested_offset)?;
            if nested_offset != nested.len() {
                return Err(WalletError::OperationFailed);
            }
        }
    }
    Ok(SemanticDestination {
        destination_type,
        destination_bytes,
    })
}

fn parse_reserve_transfer(data: &[u8]) -> Result<DecodedReserveTransfer, WalletError> {
    let (source_values, mut offset) = parse_token_output(data, 0)?;
    if source_values.len() != 1 {
        return Err(WalletError::OperationFailed);
    }
    let (source_currency_id, amount_sats) = source_values
        .into_iter()
        .next()
        .ok_or(WalletError::OperationFailed)?;
    let flags = read_varint(data, &mut offset)?;
    let fee_currency_id = read_slice(data, &mut offset, 20)?;
    let fee_sats = read_varint(data, &mut offset)?;
    let destination = parse_transfer_destination(data, &mut offset)?;
    let dest_currency_id = read_slice(data, &mut offset, 20)?;
    let second_reserve_id = if flags & 0x400 != 0 {
        Some(read_slice(data, &mut offset, 20)?)
    } else {
        None
    };
    let dest_system_id = if flags & 0x40 != 0 {
        Some(read_slice(data, &mut offset, 20)?)
    } else {
        None
    };
    if offset != data.len() {
        return Err(WalletError::OperationFailed);
    }
    Ok(DecodedReserveTransfer {
        source_currency_id,
        amount_sats,
        flags,
        fee_currency_id,
        fee_sats,
        destination,
        dest_currency_id,
        second_reserve_id,
        dest_system_id,
    })
}

fn read_u32_le(data: &[u8], offset: &mut usize) -> Result<u32, WalletError> {
    let bytes = read_slice(data, offset, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_content_map(
    data: &[u8],
    offset: &mut usize,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>, WalletError> {
    let count = read_compact_size(data, offset)?;
    let mut entries =
        Vec::with_capacity(usize::try_from(count).map_err(|_| WalletError::IdentityBuildFailed)?);
    for _ in 0..count {
        entries.push((read_slice(data, offset, 20)?, read_slice(data, offset, 32)?));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(entries)
}

fn parse_identity_control(data: &[u8]) -> Result<IdentityControlIntent, WalletError> {
    let mut offset = 0usize;
    let version = read_u32_le(data, &mut offset)?;
    if !(1..=3).contains(&version) {
        return Err(WalletError::IdentityBuildFailed);
    }
    let flags = read_u32_le(data, &mut offset)?;
    let primary_count = read_compact_size(data, &mut offset)?;
    let mut primary_addresses = Vec::new();
    for _ in 0..primary_count {
        let address = read_var_slice(data, &mut offset)?;
        if address.len() != 20 && address.len() != 33 {
            return Err(WalletError::IdentityBuildFailed);
        }
        primary_addresses.push(address);
    }
    let minimum_signatures = read_u32_le(data, &mut offset)?;
    if minimum_signatures == 0 || u64::from(minimum_signatures) > primary_count {
        return Err(WalletError::IdentityBuildFailed);
    }
    let parent = read_slice(data, &mut offset, 20)?;
    let name = read_var_slice(data, &mut offset)?;

    let content_multimap = if version >= 3 {
        let content_count = read_compact_size(data, &mut offset)?;
        let mut entries = Vec::with_capacity(
            usize::try_from(content_count).map_err(|_| WalletError::IdentityBuildFailed)?,
        );
        for _ in 0..content_count {
            let key = read_slice(data, &mut offset, 20)?;
            let value_count = read_compact_size(data, &mut offset)?;
            let mut values = Vec::with_capacity(
                usize::try_from(value_count).map_err(|_| WalletError::IdentityBuildFailed)?,
            );
            for _ in 0..value_count {
                values.push(read_var_slice(data, &mut offset)?);
            }
            entries.push((key, values));
        }
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        entries
    } else {
        Vec::new()
    };
    let legacy_content_map = if version < 3 {
        read_content_map(data, &mut offset)?
    } else {
        Vec::new()
    };
    let content_map = read_content_map(data, &mut offset)?;

    let revocation_authority = read_slice(data, &mut offset, 20)?;
    let recovery_authority = read_slice(data, &mut offset, 20)?;
    let private_count = read_compact_size(data, &mut offset)?;
    let mut private_addresses = Vec::new();
    for _ in 0..private_count {
        private_addresses.push(read_slice(data, &mut offset, 43)?);
    }
    let (system_id, unlock_after) = if version >= 2 {
        (
            Some(read_slice(data, &mut offset, 20)?),
            Some(read_u32_le(data, &mut offset)?),
        )
    } else {
        (None, None)
    };
    if offset != data.len() {
        return Err(WalletError::IdentityBuildFailed);
    }

    Ok(IdentityControlIntent {
        version,
        flags,
        primary_addresses,
        minimum_signatures,
        parent,
        name,
        content_multimap,
        legacy_content_map,
        content_map,
        revocation_authority,
        recovery_authority,
        private_addresses,
        system_id,
        unlock_after,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::vrpc::identity::verus_tx::codec::encode_hex;
    use crate::core::channels::vrpc::identity::verus_tx::model::{VerusTx, VerusTxOut};

    fn push(data: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        if data.len() <= 75 {
            output.push(data.len() as u8);
        } else {
            output.extend_from_slice(&[76, data.len() as u8]);
        }
        output.extend_from_slice(data);
        output
    }

    fn cc_script(eval_code: u8, destination: &SemanticDestination, vdata: &[u8]) -> Vec<u8> {
        let mut destination_chunk = Vec::new();
        if destination.destination_type == 2 {
            destination_chunk.extend_from_slice(&destination.destination_bytes);
        } else {
            destination_chunk.push(destination.destination_type);
            destination_chunk.extend_from_slice(&destination.destination_bytes);
        }
        let mut params = push(&[3, eval_code, 1, 1]);
        params.extend_from_slice(&push(&destination_chunk));
        if !vdata.is_empty() {
            params.extend_from_slice(&push(vdata));
        }
        let master = if eval_code == EVAL_NONE {
            push(&[3, EVAL_NONE, 0, 0])
        } else {
            let mut master = push(&[3, EVAL_NONE, 1, 1]);
            master.extend_from_slice(&push(&destination_chunk));
            master
        };
        let mut script = push(&master);
        script.push(OP_CHECKCRYPTOCONDITION);
        script.extend_from_slice(&push(&params));
        script.push(OP_DROP);
        script
    }

    fn identity_cc_script(destination: &SemanticDestination, vdata: &[u8]) -> Vec<u8> {
        let mut identity_destination = vec![destination.destination_type];
        identity_destination.extend_from_slice(&destination.destination_bytes);
        let revocation_destination = [&[4u8][..], &[3u8; 20][..]].concat();
        let recovery_destination = [&[4u8][..], &[4u8; 20][..]].concat();
        let mut master = push(&[3, EVAL_NONE, 1, 3]);
        master.extend_from_slice(&push(&identity_destination));
        master.extend_from_slice(&push(&revocation_destination));
        master.extend_from_slice(&push(&recovery_destination));

        let mut condition = push(&[3, EVAL_IDENTITY_PRIMARY, 1, 1]);
        condition.extend_from_slice(&push(&identity_destination));
        condition.extend_from_slice(&push(vdata));
        let mut revoke = push(&[3, EVAL_IDENTITY_REVOKE, 1, 1]);
        revoke.extend_from_slice(&push(&revocation_destination));
        condition.extend_from_slice(&push(&revoke));
        let mut recover = push(&[3, EVAL_IDENTITY_RECOVER, 1, 1]);
        recover.extend_from_slice(&push(&recovery_destination));
        condition.extend_from_slice(&push(&recover));

        let mut script = push(&master);
        script.push(OP_CHECKCRYPTOCONDITION);
        script.extend_from_slice(&push(&condition));
        script.push(OP_DROP);
        script
    }

    fn identity_bytes(primary: [u8; 20]) -> Vec<u8> {
        let mut identity = Vec::new();
        identity.extend_from_slice(&3u32.to_le_bytes());
        identity.extend_from_slice(&0u32.to_le_bytes());
        identity.push(1);
        identity.push(20);
        identity.extend_from_slice(&primary);
        identity.extend_from_slice(&1u32.to_le_bytes());
        identity.extend_from_slice(&[2u8; 20]);
        identity.extend_from_slice(&[4, b't', b'e', b's', b't']);
        identity.push(0); // content multimap
        identity.push(0); // content map
        identity.extend_from_slice(&[3u8; 20]);
        identity.extend_from_slice(&[4u8; 20]);
        identity.push(0); // private addresses
        identity.extend_from_slice(&[5u8; 20]);
        identity.extend_from_slice(&0u32.to_le_bytes());
        identity
    }

    fn identity_tx(primary: [u8; 20]) -> String {
        let destination = SemanticDestination {
            destination_type: 4,
            destination_bytes: vec![9u8; 20],
        };
        encode_hex(&VerusTx {
            version: 4,
            overwintered: true,
            version_group_id: 0x892f2085,
            inputs: vec![],
            outputs: vec![VerusTxOut {
                value: 0,
                script_pub_key: identity_cc_script(&destination, &identity_bytes(primary)),
            }],
            lock_time: 0,
            expiry_height: 0,
            value_balance: 0,
        })
        .expect("identity tx")
    }

    fn funded_identity_tx(primary: [u8; 20], extra_output: Option<VerusTxOut>) -> String {
        let destination = SemanticDestination {
            destination_type: 4,
            destination_bytes: vec![9u8; 20],
        };
        let mut outputs = vec![VerusTxOut {
            value: 0,
            script_pub_key: identity_cc_script(&destination, &identity_bytes(primary)),
        }];
        if let Some(output) = extra_output {
            outputs.push(output);
        }
        outputs.push(VerusTxOut {
            value: 9_000,
            script_pub_key: p2pkh_script(&[9u8; 20]).expect("change script"),
        });
        transaction_hex(outputs)
    }

    fn transaction_hex(outputs: Vec<VerusTxOut>) -> String {
        encode_hex(&VerusTx {
            version: 4,
            overwintered: true,
            version_group_id: 0x892f2085,
            inputs: vec![],
            outputs,
            lock_time: 0,
            expiry_height: 0,
            value_balance: 0,
        })
        .expect("transaction hex")
    }

    fn native_payment_tx(recipient: [u8; 20], amount: u64, extra_recipient: bool) -> String {
        let mut outputs = vec![VerusTxOut {
            value: amount,
            script_pub_key: p2pkh_script(&recipient).expect("recipient script"),
        }];
        if extra_recipient {
            outputs.push(VerusTxOut {
                value: 1,
                script_pub_key: p2pkh_script(&recipient).expect("extra recipient script"),
            });
        }
        outputs.push(VerusTxOut {
            value: 4_000,
            script_pub_key: p2pkh_script(&[9u8; 20]).expect("change script"),
        });
        transaction_hex(outputs)
    }

    fn reserve_transfer_bytes(dest_currency: [u8; 20]) -> Vec<u8> {
        let mut data = vec![1]; // token-output version
        data.extend_from_slice(&[1u8; 20]); // source currency
        data.push(50); // amount
        data.push(1); // flags
        data.extend_from_slice(&[2u8; 20]); // fee currency
        data.push(3); // fee
        data.push(2); // destination type
        data.push(20); // compact-size destination length
        data.extend_from_slice(&[4u8; 20]);
        data.extend_from_slice(&dest_currency);
        data
    }

    fn reserve_transfer_tx(dest_currency: [u8; 20]) -> String {
        let cc_destination = SemanticDestination {
            destination_type: 2,
            destination_bytes: vec![7u8; 20],
        };
        transaction_hex(vec![
            VerusTxOut {
                value: 0,
                script_pub_key: cc_script(
                    EVAL_RESERVE_TRANSFER,
                    &cc_destination,
                    &reserve_transfer_bytes(dest_currency),
                ),
            },
            VerusTxOut {
                value: 9_000,
                script_pub_key: p2pkh_script(&[9u8; 20]).expect("change script"),
            },
        ])
    }

    fn token_payment_tx(destination: [u8; 20], output_value: u64) -> String {
        let destination = SemanticDestination {
            destination_type: 2,
            destination_bytes: destination.to_vec(),
        };
        let mut token = vec![1];
        token.extend_from_slice(&[6u8; 20]);
        token.push(50);
        transaction_hex(vec![
            VerusTxOut {
                value: output_value,
                script_pub_key: cc_script(EVAL_RESERVE_OUTPUT, &destination, &token),
            },
            VerusTxOut {
                value: 9_000,
                script_pub_key: p2pkh_script(&[9u8; 20]).expect("change script"),
            },
        ])
    }

    fn token_payment_with_change_tx(
        destination: [u8; 20],
        amount: u8,
        change_amount: u8,
    ) -> String {
        let recipient = SemanticDestination {
            destination_type: 2,
            destination_bytes: destination.to_vec(),
        };
        let change = SemanticDestination {
            destination_type: 2,
            destination_bytes: vec![9u8; 20],
        };
        let token = |amount| {
            let mut bytes = vec![1];
            bytes.extend_from_slice(&[6u8; 20]);
            bytes.push(amount);
            bytes
        };
        transaction_hex(vec![
            VerusTxOut {
                value: 0,
                script_pub_key: cc_script(EVAL_RESERVE_OUTPUT, &recipient, &token(amount)),
            },
            VerusTxOut {
                value: 0,
                script_pub_key: cc_script(EVAL_RESERVE_OUTPUT, &change, &token(change_amount)),
            },
            VerusTxOut {
                value: 9_000,
                script_pub_key: p2pkh_script(&[9u8; 20]).expect("change script"),
            },
        ])
    }

    fn reserve_input(currency: [u8; 20], amount: u8) -> VrpcInputRef {
        let destination = SemanticDestination {
            destination_type: 2,
            destination_bytes: vec![9u8; 20],
        };
        let mut token = vec![1];
        token.extend_from_slice(&currency);
        token.push(amount);
        VrpcInputRef {
            txid: "00".repeat(32),
            vout: 0,
            satoshis: 0,
            script_pub_key: Some(hex::encode(cc_script(
                EVAL_RESERVE_OUTPUT,
                &destination,
                &token,
            ))),
        }
    }

    #[test]
    fn base58_destinations_preserve_type_and_hash() {
        let destination =
            decode_base58_destination("RLcoqsCLBQJPvciM1EvFzXH9p42Y61AtiB").expect("R destination");
        assert_eq!(destination.destination_type, 2);
        assert_eq!(destination.destination_bytes.len(), 20);

        let destination =
            decode_base58_destination("i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV").expect("i destination");
        assert_eq!(destination.destination_type, 4);
        assert_eq!(destination.destination_bytes.len(), 20);
    }

    #[test]
    fn identity_json_normalizes_official_vdxf_forms_and_compressed_public_keys() {
        let public_key = "03a058410b33f893fe182f15336577f3941c28c8cadcfb0395b9c31dd5c07ccd11";
        let identity = serde_json::json!({
            "version": 3,
            "flags": 0,
            "minimumsignatures": 1,
            "parent": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "name": "structured",
            "contentmultimap": {
                "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": [
                    "deadbeef",
                    {"message": "hello"},
                    {"serializedhex": "cafe"},
                    {"serializedbase64": "AQID"},
                    {"iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": "typed"},
                    {"iKMhRLX1JHQihVZx2t2pAWW2uzmK6AzwW3": "010203"}
                ]
            },
            "contentmap": {},
            "primaryaddresses": [public_key],
            "revocationauthority": "i98Mnj1YugaRzoURXt4aRhdqQDu7rML9J5",
            "recoveryauthority": "i9ps1xDcr7eM66Fko6aTkeuvvBPZFLEXRN",
            "privateaddresses": [],
            "systemid": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "timelock": 0
        });

        let intent = identity_control_intent_from_json(&identity).expect("normalized identity");
        assert_eq!(
            intent.primary_addresses,
            vec![hex::decode(public_key).unwrap()]
        );
        assert_eq!(intent.content_multimap.len(), 1);
        assert_eq!(
            intent.content_multimap[0].1,
            vec![
                hex::decode("deadbeef").unwrap(),
                hex::decode("68656c6c6f").unwrap(),
                hex::decode("cafe").unwrap(),
                hex::decode("010203").unwrap(),
                hex::decode("ab8b7b8b4418de66e611921699a328126461c0e50106057479706564").unwrap(),
                hex::decode("ae377010192abb2513f705519f62066046e63acc010403010203").unwrap(),
            ]
        );

        let mut unsupported = identity;
        unsupported["contentmultimap"]["iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c"] =
            serde_json::json!({"unknown": "value"});
        assert!(identity_control_intent_from_json(&unsupported).is_err());
    }

    #[test]
    fn tokenized_recovery_requires_the_canonical_contract_destination() {
        let authority = SemanticDestination {
            destination_type: 4,
            destination_bytes: vec![4u8; 20],
        };
        let canonical_contract =
            decode_base58_destination(IDENTITY_RECOVER_CONTRACT_ADDRESS).unwrap();
        let mut recover = push(&[3, EVAL_IDENTITY_RECOVER, 1, 2]);
        recover.extend_from_slice(&push(
            &[&[4u8][..], authority.destination_bytes.as_slice()].concat(),
        ));
        recover.extend_from_slice(&push(&canonical_contract.destination_bytes));
        validate_identity_authority_condition(
            &recover,
            EVAL_IDENTITY_RECOVER,
            &authority.destination_bytes,
            true,
        )
        .expect("canonical tokenized recovery");

        let mut substituted = recover;
        let last = substituted.last_mut().expect("contract hash byte");
        *last ^= 1;
        assert!(validate_identity_authority_condition(
            &substituted,
            EVAL_IDENTITY_RECOVER,
            &authority.destination_bytes,
            true,
        )
        .is_err());
    }

    #[test]
    fn semantic_destination_accepts_testnet_sapling_addresses() {
        let destination = decode_semantic_destination(
            "ztestsapling1qqqqqqqqqqqqqqqqqqcguyvaw2vjk4sdyeg0lc970u659lvhqq7t0np6hlup5lusxle75ss7jnk",
        )
        .expect("testnet Sapling destination");
        assert_eq!(destination.destination_type, 7);
        assert_eq!(destination.destination_bytes.len(), 43);
    }

    #[test]
    fn signed_varint_matches_verus_encoding() {
        let mut offset = 0;
        assert_eq!(
            read_varint(&[0x80, 0x00], &mut offset).expect("varint"),
            128
        );
        assert_eq!(offset, 2);
    }

    #[test]
    fn optcc_thresholds_and_destination_lengths_are_enforced() {
        let invalid_threshold = push(&[3, EVAL_RESERVE_OUTPUT, 2, 1]);
        assert!(parse_optcc_chunk(&invalid_threshold).is_err());

        let mut invalid_pubkey = push(&[3, EVAL_NONE, 1, 1]);
        invalid_pubkey.extend_from_slice(&push(&[1u8; 21]));
        assert!(parse_optcc_chunk(&invalid_pubkey).is_err());

        let mut valid_pubkey = push(&[3, EVAL_NONE, 1, 1]);
        valid_pubkey.extend_from_slice(&push(&[1u8; 33]));
        let parsed = parse_optcc_chunk(&valid_pubkey).expect("33-byte public key destination");
        assert_eq!(parsed.destinations[0].destination_type, 1);
        assert_eq!(parsed.destinations[0].destination_bytes.len(), 33);
    }

    #[test]
    fn cc_master_binds_eval_threshold_and_index_destination() {
        let destination = SemanticDestination {
            destination_type: 2,
            destination_bytes: vec![7u8; 20],
        };
        let script = cc_script(EVAL_RESERVE_OUTPUT, &destination, &[1u8; 21]);
        let params = parse_cc_params(&script).expect("canonical CC script");
        validate_single_condition_master(&params).expect("canonical master relation");

        let mut tampered_eval = script.clone();
        tampered_eval[3] = EVAL_RESERVE_OUTPUT;
        assert!(parse_cc_params(&tampered_eval).is_err());

        let mut tampered_threshold = script.clone();
        tampered_threshold[4] = 0;
        let params = parse_cc_params(&tampered_threshold).expect("structured master");
        assert!(validate_single_condition_master(&params).is_err());

        let mut tampered_destination = script;
        tampered_destination[7] ^= 1;
        let params = parse_cc_params(&tampered_destination).expect("structured master");
        assert!(validate_single_condition_master(&params).is_err());
    }

    #[test]
    fn multivalue_token_output_uses_compact_size_and_fixed_i64_values() {
        let fixture = hex::decode("86fefeff010275939018c507ed9cf366d309d4614b2e43ca3c009008abfbd8080000848374dd2a47335f0252c8caa066b94de4bf800f804a5d0500000000")
            .expect("official primitive fixture");
        let (values, consumed) = parse_token_output(&fixture, 0).expect("multivalue token");
        assert_eq!(consumed, fixture.len());
        assert_eq!(
            values.get(&hex::decode("75939018c507ed9cf366d309d4614b2e43ca3c00").unwrap()),
            Some(&9_728_028_248_208)
        );
        assert_eq!(
            values.get(&hex::decode("848374dd2a47335f0252c8caa066b94de4bf800f").unwrap()),
            Some(&90_000_000)
        );
    }

    #[test]
    fn official_identity_script_binds_parent_name_and_content_map() {
        let script = hex::decode("470403000103150438411ff17100e15b6df8dd72fecbe4fa4964dfea15043e006293b9e3341262eed040048d3a2260367f47150445a96c0179cbb19221c0e8625a2ed691d762fd37cc4d360104030e0101150438411ff17100e15b6df8dd72fecbe4fa4964dfea4ce103000000000000000114c165bce63e47698278f859ee75c35c78eb23e8df01000000a6ef9ea235635e328124ff3429db9f9e91b64e2d085665727573506179000113a542e2075696772ee9861c9b2a4d55c86cf353c45c3e2987f09e8c48d9e2681288bd455a18ebc73ef977750724a7fc51bd32633e006293b9e3341262eed040048d3a2260367f4745a96c0179cbb19221c0e8625a2ed691d762fd370176041f9ab6ca1d155ce87a7c677e9e0d16c9846e6ee62db670751f8be3ead27cb740aa7595d0be24b741a9a6ef9ea235635e328124ff3429db9f9e91b64e2d000000001b04030f010115043e006293b9e3341262eed040048d3a2260367f471b0403100101150445a96c0179cbb19221c0e8625a2ed691d762fd3775")
            .expect("official identity script fixture");
        let tx = transaction_hex(vec![VerusTxOut {
            value: 0,
            script_pub_key: script.clone(),
        }]);
        let identity = serde_json::json!({
            "contentmap": {"53f36cc8554d2a9b1c86e92e77965607e242a513": "6332bd51fca724077577f93ec7eb185a45bd881268e2d9488c9ef087293e5cc4"},
            "contentmultimap": {},
            "flags": 0,
            "minimumsignatures": 1,
            "name": "VerusPay",
            "parent": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "primaryaddresses": ["RSunNQqKnSwpNBRd6kcenScuQEFUWgL3bZ"],
            "privateaddress": "zs1wczplx4kegw32h8g0f7xwl57p5tvnprwdmnzmdnsw50chcl26f7tws92wk2ap03ykaq6jyyztfa",
            "recoveryauthority": "i9ps1xDcr7eM66Fko6aTkeuvvBPZFLEXRN",
            "revocationauthority": "i98Mnj1YugaRzoURXt4aRhdqQDu7rML9J5",
            "systemid": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "timelock": 0,
            "version": 3
        });
        let expected = identity_control_intent_from_json(&identity).expect("identity intent");
        validate_identity_control_intent(&tx, &expected).expect("official identity matches");

        let mut tampered = expected.clone();
        tampered.name = b"VerusPay2".to_vec();
        assert!(validate_identity_control_intent(&tx, &tampered).is_err());
        let mut tampered = expected.clone();
        tampered.content_map[0].1[0] ^= 1;
        assert!(validate_identity_control_intent(&tx, &tampered).is_err());

        let mut tampered_master = script;
        tampered_master[8] ^= 1;
        let tx = transaction_hex(vec![VerusTxOut {
            value: 0,
            script_pub_key: tampered_master,
        }]);
        assert!(validate_identity_control_intent(&tx, &expected).is_err());
    }

    #[test]
    fn identity_control_validation_rejects_primary_authority_substitution() {
        let expected = IdentityControlIntent {
            version: 3,
            flags: 0,
            primary_addresses: vec![vec![1u8; 20]],
            minimum_signatures: 1,
            parent: vec![2u8; 20],
            name: b"test".to_vec(),
            content_multimap: vec![],
            legacy_content_map: vec![],
            content_map: vec![],
            revocation_authority: vec![3u8; 20],
            recovery_authority: vec![4u8; 20],
            private_addresses: vec![],
            system_id: Some(vec![5u8; 20]),
            unlock_after: Some(0),
        };

        validate_identity_control_intent(&identity_tx([1u8; 20]), &expected)
            .expect("matching identity controls");
        assert!(validate_identity_control_intent(&identity_tx([8u8; 20]), &expected).is_err());

        let change_address = bs58::encode([&[60u8][..], &[9u8; 20][..]].concat())
            .with_check()
            .into_string();
        validate_identity_transaction_intent(
            &funded_identity_tx([1u8; 20], None),
            &expected,
            &change_address,
            10_000,
            1_000,
        )
        .expect("matching funded identity transaction");
        assert!(validate_identity_transaction_intent(
            &funded_identity_tx(
                [1u8; 20],
                Some(VerusTxOut {
                    value: 1,
                    script_pub_key: p2pkh_script(&[8u8; 20]).expect("attacker output"),
                }),
            ),
            &expected,
            &change_address,
            10_001,
            1_000,
        )
        .is_err());
    }

    #[test]
    fn native_intent_rejects_recipient_amount_extra_output_and_fee_substitution() {
        let change_address = bs58::encode([&[60u8][..], &[9u8; 20][..]].concat())
            .with_check()
            .into_string();
        let intent = VrpcOutputIntent::NativePayment {
            destination: SemanticDestination {
                destination_type: 2,
                destination_bytes: vec![1u8; 20],
            },
            amount_sats: 5_000,
        };
        let valid = native_payment_tx([1u8; 20], 5_000, false);
        validate_transaction_intent(&valid, &intent, &change_address, 10_000, 1_000, &[])
            .expect("matching native intent");

        assert!(validate_transaction_intent(
            &native_payment_tx([2u8; 20], 5_000, false),
            &intent,
            &change_address,
            10_000,
            1_000,
            &[],
        )
        .is_err());
        assert!(validate_transaction_intent(
            &native_payment_tx([1u8; 20], 4_999, false),
            &intent,
            &change_address,
            9_999,
            1_000,
            &[],
        )
        .is_err());
        assert!(validate_transaction_intent(
            &native_payment_tx([1u8; 20], 5_000, true),
            &intent,
            &change_address,
            10_001,
            1_000,
            &[],
        )
        .is_err());
        assert!(
            validate_transaction_intent(&valid, &intent, &change_address, 10_001, 1_000, &[])
                .is_err()
        );
    }

    #[test]
    fn reserve_transfer_intent_binds_daemon_derived_currency_and_rejects_substitution() {
        let change_address = bs58::encode([&[60u8][..], &[9u8; 20][..]].concat())
            .with_check()
            .into_string();
        let mut intent = VrpcOutputIntent::ReserveTransfer {
            system_currency_id: vec![0u8; 20],
            source_currency_id: vec![1u8; 20],
            amount_sats: 50,
            flags: 1,
            fee_currency_id: vec![2u8; 20],
            fee_sats: 3,
            destination: SemanticDestination {
                destination_type: 2,
                destination_bytes: vec![4u8; 20],
            },
            dest_currency_id: None,
            second_reserve_id: None,
            dest_system_id: None,
            output_value_sats: 0,
        };
        let unfunded = reserve_transfer_tx([5u8; 20]);
        let decoded_outputs = decode_outputs(&unfunded).expect("decode reserve transfer tx");
        let params = parse_cc_params(&decoded_outputs[0].script).expect("reserve transfer params");
        assert_eq!(params.condition.eval_code, EVAL_RESERVE_TRANSFER);
        assert_eq!(params.condition.vdata.len(), 1);
        let decoded_transfer =
            parse_reserve_transfer(&params.condition.vdata[0]).expect("reserve transfer payload");
        assert_eq!(decoded_transfer.source_currency_id, vec![1u8; 20]);
        assert_eq!(decoded_transfer.amount_sats, 50);
        assert_eq!(decoded_transfer.flags, 1);
        assert_eq!(decoded_transfer.fee_currency_id, vec![2u8; 20]);
        assert_eq!(decoded_transfer.fee_sats, 3);
        assert_eq!(
            decoded_transfer.destination.destination_bytes,
            vec![4u8; 20]
        );
        assert!(output_matches_intent(&decoded_outputs[0], &intent).expect("match reserve intent"));
        bind_derived_reserve_fields(&unfunded, &mut intent).expect("bind derived currency");
        assert!(matches!(
            &intent,
            VrpcOutputIntent::ReserveTransfer {
                dest_currency_id: Some(value),
                ..
            } if value == &vec![5u8; 20]
        ));
        let reserve_inputs = [reserve_input([1u8; 20], 50), reserve_input([2u8; 20], 3)];
        validate_transaction_intent(
            &unfunded,
            &intent,
            &change_address,
            10_000,
            1_000,
            &reserve_inputs,
        )
        .expect("matching reserve transfer");

        assert!(validate_transaction_intent(
            &reserve_transfer_tx([6u8; 20]),
            &intent,
            &change_address,
            10_000,
            1_000,
            &reserve_inputs,
        )
        .is_err());
        let mut substituted = intent.clone();
        if let VrpcOutputIntent::ReserveTransfer { amount_sats, .. } = &mut substituted {
            *amount_sats = 51;
        }
        assert!(validate_transaction_intent(
            &unfunded,
            &substituted,
            &change_address,
            10_000,
            1_000,
            &reserve_inputs,
        )
        .is_err());
    }

    #[test]
    fn token_intent_rejects_destination_currency_amount_and_native_value_substitution() {
        let change_address = bs58::encode([&[60u8][..], &[9u8; 20][..]].concat())
            .with_check()
            .into_string();
        let intent = VrpcOutputIntent::TokenPayment {
            destination: SemanticDestination {
                destination_type: 2,
                destination_bytes: vec![4u8; 20],
            },
            currency_id: vec![6u8; 20],
            amount_sats: 50,
            output_value_sats: 0,
        };
        let valid = token_payment_tx([4u8; 20], 0);
        let token_inputs = [reserve_input([6u8; 20], 50)];
        validate_transaction_intent(
            &valid,
            &intent,
            &change_address,
            10_000,
            1_000,
            &token_inputs,
        )
        .expect("matching token payment");
        assert!(validate_transaction_intent(
            &token_payment_tx([5u8; 20], 0),
            &intent,
            &change_address,
            10_000,
            1_000,
            &token_inputs,
        )
        .is_err());
        assert!(validate_transaction_intent(
            &token_payment_tx([4u8; 20], 1),
            &intent,
            &change_address,
            10_001,
            1_000,
            &token_inputs,
        )
        .is_err());
        let mut wrong_amount = intent.clone();
        if let VrpcOutputIntent::TokenPayment { amount_sats, .. } = &mut wrong_amount {
            *amount_sats = 51;
        }
        assert!(validate_transaction_intent(
            &valid,
            &wrong_amount,
            &change_address,
            10_000,
            1_000,
            &token_inputs,
        )
        .is_err());

        let input_with_change = [reserve_input([6u8; 20], 75)];
        validate_transaction_intent(
            &token_payment_with_change_tx([4u8; 20], 50, 25),
            &intent,
            &change_address,
            10_000,
            1_000,
            &input_with_change,
        )
        .expect("token change is conserved to the reviewed change destination");
        assert!(validate_transaction_intent(
            &token_payment_with_change_tx([4u8; 20], 50, 26),
            &intent,
            &change_address,
            10_000,
            1_000,
            &input_with_change,
        )
        .is_err());
    }
}
