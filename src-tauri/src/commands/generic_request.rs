use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{
    engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use reqwest::header::CONTENT_TYPE;
use ripemd::Ripemd160;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::commands::identity::{
    build_identity_details_from_payload, identity_session_context, linked_identity_from_details,
    load_linked_for_context, map_identity_lookup_error as map_link_identity_lookup_error,
    parse_getidentity_payload, store_linked_for_context, upsert_linked_identity,
};
use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, ProvisioningSignatureChallenge,
    ProvisioningSignatureStore, SessionManager,
};
use crate::core::channels::vrpc;
use crate::core::channels::vrpc::identity::preflight::{
    build_unsigned_identity_tx, fetch_identity_prevout,
    map_identity_lookup_error as map_preflight_identity_lookup_error, parse_funding_utxos,
    parse_target_identity, parse_updateidentity_hex, sat_to_decimal_string, total_satoshis,
    IdentityPreflightPayload, DEFAULT_FEE_SAT, IDENTITY_PREFLIGHT_TTL,
};
use crate::core::channels::vrpc::identity::validate::{
    classify_high_risk_changes, validate_operation_authority, validate_target_state,
};
use crate::core::channels::vrpc::VrpcProviderPool;
use crate::core::channels::PreflightRecord;
use crate::core::crypto::verus_id_signature::{
    compute_identity_signature_hash, decode_compact_address_at, encode_compact_i_address,
    extract_chain_height, extract_primary_addresses, get_raw_envelope_sha256,
    parse_generic_envelope_hex, parse_identity_signature, sign_identity_hash_with_private_bytes,
    validate_identity_control_for_active_wallet, verify_generic_request_signature_with_provider,
    verify_identity_signature_against_addresses, write_compact_size, write_var_slice, write_varint,
};
use crate::core::crypto::Network;
use crate::core::{AccountStateStore, PreflightStore};
use crate::types::wallet::WalletNetwork;
use crate::types::{
    BuildAndSignGenericResponseRequest, BuildAndSignGenericResponseResult,
    GenericIdentityAuthorities, GenericIdentityPrimaryAddressEntry,
    GenericIdentityPrimaryAddressInfo, GenericIdentityUpdatePreflightResult,
    GenericIdentityUpdateRequestMeta, GenericIdentityUpdateReviewResult,
    GenericRequestVerificationResult, IdentityOperation, IdentityWarning,
    LinkReadyProvisioningJobResult, LinkedIdentity, ProvisioningJobRecord, WalletError,
};
use uuid::Uuid;

const PROVISIONING_JOB_REQUEST_TYPE_GENERIC: &str = "generic";
const PROVISIONING_JOB_STATUS_PENDING: &str = "pending";
const PROVISIONING_JOB_STATUS_READY: &str = "ready";
const PROVISIONING_JOB_STATUS_LINKED: &str = "linked";
const PROVISIONING_JOB_STATUS_EXPIRED: &str = "expired";
const PROVISIONING_JOB_TTL_SECS: u64 = 7 * 24 * 60 * 60;
const VDXF_ORDINAL_AUTHENTICATION_REQUEST: u64 = 2;
const VDXF_ORDINAL_AUTHENTICATION_RESPONSE: u64 = 3;
const VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST: u64 = 4;
const VDXF_ORDINAL_IDENTITY_UPDATE_RESPONSE: u64 = 5;
const VDXF_ORDINAL_PROVISION_IDENTITY: u64 = 6;
const VERIFIABLE_SIGNATURE_VERSION_V2: u64 = 2;
const HASH_TYPE_SHA256: u64 = 5;
const GENERIC_RESPONSE_FLAG_SIGNED: u64 = 1;
const GENERIC_RESPONSE_FLAG_HAS_REQUEST_ID: u64 = 2;
const GENERIC_RESPONSE_FLAG_HAS_CREATED_AT: u64 = 4;
const GENERIC_RESPONSE_FLAG_MULTI_DETAILS: u64 = 8;
const GENERIC_RESPONSE_FLAG_IS_TESTNET: u64 = 16;
const GENERIC_RESPONSE_FLAG_HAS_REQUEST_HASH: u64 = 128;
const AUTHENTICATION_RESPONSE_FLAG_HAS_REQUEST_ID: u64 = 1;
const IDENTITY_UPDATE_RESPONSE_FLAG_CONTAINS_TXID: u64 = 1;
const IDENTITY_UPDATE_RESPONSE_FLAG_CONTAINS_REQUEST_ID: u64 = 2;
const GENERIC_RESPONSE_DEEPLINK_VDXF_ID: &str = "i9JzVt59mAVHqjc8WAQJx7bEFAQ4ffuhrC";
const DATA_TYPE_DEFINEDKEY_VDXF_ID: &str = "iD3yzD6KnrSG75d8RzirMD6SyvrAS2HxjH";
const I_ADDRESS_VERSION: u8 = 102;
const MAINNET_VERUS_CHAIN_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const TESTNET_VERUS_CHAIN_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
const AUTH_REQUEST_FLAG_HAS_REQUEST_ID: u64 = 1;
const AUTH_REQUEST_FLAG_HAS_RECIPIENT_CONSTRAINTS: u64 = 2;
const AUTH_REQUEST_FLAG_HAS_EXPIRY_TIME: u64 = 4;
const PROVISION_FLAG_HAS_SYSTEM_ID: u64 = 1;
const PROVISION_FLAG_HAS_PARENT_ID: u64 = 2;
const PROVISION_FLAG_HAS_IDENTITY_ID: u64 = 4;
const PROVISION_FLAG_HAS_URI: u64 = 8;
const PROVISIONING_SIGNATURE_TTL_SECS: u64 = 5 * 60;
const PROVISIONING_CREATED_AT_CLOCK_SKEW_SECS: u64 = 60;
const PROVISIONING_CHALLENGE_KEY_HASH_HEX: &str = "bf5b8b3997fe0381a6383fddc8f055c1f0dd6774";
const LOGIN_CONSENT_CONTEXT_KEY_HASH_HEX: &str = "567b9a87b17131f6eeb2dd0bdd191ece4a5d603b";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreGenericProvisioningJobRequest {
    pub request_hex: String,
    pub requested_identity_address: Option<String>,
    pub requested_fqn: String,
    pub signing_id: String,
    pub has_response_uris: bool,
    pub info_uri: Option<String>,
    pub error: Option<String>,
    pub status: Option<String>,
}

fn wallet_network_to_crypto_network(network: WalletNetwork) -> Network {
    match network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn parse_generic_response_post_callback_uri(uri: &str) -> Result<reqwest::Url, WalletError> {
    let parsed = reqwest::Url::parse(uri.trim()).map_err(|_| WalletError::OperationFailed)?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        _ => Err(WalletError::OperationFailed),
    }
}

fn parse_generic_response_redirect_uri(uri: &str) -> Result<reqwest::Url, WalletError> {
    let parsed = reqwest::Url::parse(uri.trim()).map_err(|_| WalletError::OperationFailed)?;
    if parsed.scheme().trim().is_empty() {
        return Err(WalletError::OperationFailed);
    }
    Ok(parsed)
}

fn build_generic_response_redirect_url(
    callback_uri: &str,
    response_hex: &str,
) -> Result<reqwest::Url, WalletError> {
    let mut redirect_url = parse_generic_response_redirect_uri(callback_uri)?;
    let response_bytes =
        hex::decode(response_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
    let encoded_response = URL_SAFE_NO_PAD.encode(response_bytes);
    redirect_url
        .query_pairs_mut()
        .append_pair(GENERIC_RESPONSE_DEEPLINK_VDXF_ID, &encoded_response);
    Ok(redirect_url)
}

fn ensure_active_wallet_controls_signer(
    allowed_primary_addresses: &[String],
    active_wallet_primary_address: &str,
) -> Result<(), WalletError> {
    if allowed_primary_addresses
        .iter()
        .any(|address| address.eq_ignore_ascii_case(active_wallet_primary_address))
    {
        return Ok(());
    }

    Err(WalletError::IdentityUnsupportedAuthority)
}

fn double_sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut first = Sha256::new();
    for part in parts {
        first.update(part);
    }
    let first_digest = first.finalize();
    let second_digest = Sha256::digest(first_digest);
    second_digest.into()
}

fn hash160(data: &[u8]) -> [u8; 20] {
    let sha = Sha256::digest(data);
    let ripemd = Ripemd160::digest(sha);
    ripemd.into()
}

fn to_base58_check(hash: &[u8; 20], version: u8) -> String {
    let mut payload = Vec::with_capacity(21);
    payload.push(version);
    payload.extend_from_slice(hash);
    bs58::encode(payload).with_check().into_string()
}

fn from_base58_check_hash160(address: &str) -> Option<[u8; 20]> {
    let decoded = bs58::decode(address.trim())
        .with_check(None)
        .into_vec()
        .ok()?;
    if decoded.len() != 21 || decoded[0] != I_ADDRESS_VERSION {
        return None;
    }

    decoded[1..].try_into().ok()
}

fn capitalize_label(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    out.push_str(chars.as_str());
    out
}

fn normalize_identity_name(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn name_and_parent_addr_to_i_addr(name: &str, parent_iaddr: Option<&str>) -> Option<String> {
    let name_bytes = normalize_identity_name(name).into_bytes();
    let name_hash = double_sha256(&[&name_bytes]);
    let final_hash = if let Some(parent) = parent_iaddr {
        let parent_hash = from_base58_check_hash160(parent)?;
        double_sha256(&[&parent_hash, &name_hash])
    } else {
        name_hash
    };

    Some(to_base58_check(&hash160(&final_hash), I_ADDRESS_VERSION))
}

fn fqn_to_i_addr(name: &str) -> Option<String> {
    let at_parts = name
        .split('@')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if at_parts.len() != 1 {
        return None;
    }

    let mut name_parts = at_parts[0].split('.').collect::<Vec<_>>();
    if name_parts.last().is_some_and(|part| part.is_empty()) {
        let _ = name_parts.pop();
    }

    let first = name_parts.first()?.trim();
    if first.is_empty() {
        return None;
    }

    let mut parent: Option<String> = None;
    for part in name_parts.iter().skip(1).rev() {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return None;
        }
        parent = Some(name_and_parent_addr_to_i_addr(trimmed, parent.as_deref())?);
    }

    name_and_parent_addr_to_i_addr(first, parent.as_deref())
}

fn hierarchical_name_to_i_addr(name: &str, parent_iaddr: Option<&str>) -> Option<String> {
    if name == "::" {
        return name_and_parent_addr_to_i_addr(name, parent_iaddr);
    }

    let name_parts = name.split('.').collect::<Vec<_>>();
    let first = name_parts.first()?.trim();
    if first.is_empty() {
        return None;
    }

    let mut parent = parent_iaddr.map(ToString::to_string);
    for part in name_parts.iter().skip(1).rev() {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return None;
        }
        parent = Some(name_and_parent_addr_to_i_addr(trimmed, parent.as_deref())?);
    }

    name_and_parent_addr_to_i_addr(first, parent.as_deref())
}

fn get_data_key_id(key_name: &str, verus_chain_id: &str) -> Option<String> {
    let address_parts = key_name.split(':').collect::<Vec<_>>();
    let mut namespace_id = verus_chain_id.to_string();
    let mut key_copy = key_name.to_string();

    if address_parts.len() > 2 && address_parts[1].is_empty() {
        namespace_id = fqn_to_i_addr(address_parts[0])?;
        key_copy = address_parts[2..].join(":");
    }

    let parent = name_and_parent_addr_to_i_addr("::", Some(&namespace_id))?;
    hierarchical_name_to_i_addr(&key_copy, Some(&parent))
}

fn read_compact_size_local(bytes: &[u8], offset: usize) -> Result<(u64, usize), WalletError> {
    if offset >= bytes.len() {
        return Err(WalletError::OperationFailed);
    }

    let first = bytes[offset];
    match first {
        0..=252 => Ok((first as u64, 1)),
        253 => {
            let end = offset.checked_add(3).ok_or(WalletError::OperationFailed)?;
            let slice = bytes
                .get(offset + 1..end)
                .ok_or(WalletError::OperationFailed)?;
            Ok((
                u16::from_le_bytes(slice.try_into().map_err(|_| WalletError::OperationFailed)?)
                    as u64,
                3,
            ))
        }
        254 => {
            let end = offset.checked_add(5).ok_or(WalletError::OperationFailed)?;
            let slice = bytes
                .get(offset + 1..end)
                .ok_or(WalletError::OperationFailed)?;
            Ok((
                u32::from_le_bytes(slice.try_into().map_err(|_| WalletError::OperationFailed)?)
                    as u64,
                5,
            ))
        }
        _ => {
            let end = offset.checked_add(9).ok_or(WalletError::OperationFailed)?;
            let slice = bytes
                .get(offset + 1..end)
                .ok_or(WalletError::OperationFailed)?;
            Ok((
                u64::from_le_bytes(slice.try_into().map_err(|_| WalletError::OperationFailed)?),
                9,
            ))
        }
    }
}

fn read_varint_local(bytes: &[u8], offset: usize) -> Result<(u64, usize), WalletError> {
    let mut value = 0u64;
    let mut shift = 0u32;
    let mut cursor = offset;

    loop {
        let byte = *bytes.get(cursor).ok_or(WalletError::OperationFailed)?;
        cursor += 1;
        value |= ((byte & 0x7f) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, cursor - offset));
        }
        value = value.checked_add(1).ok_or(WalletError::OperationFailed)?;
        shift = shift.checked_add(7).ok_or(WalletError::OperationFailed)?;
        if shift > 63 {
            return Err(WalletError::OperationFailed);
        }
    }
}

fn read_var_slice_local(bytes: &[u8], offset: usize) -> Result<(&[u8], usize), WalletError> {
    let (len, prefix_len) = read_compact_size_local(bytes, offset)?;
    let start = offset
        .checked_add(prefix_len)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let end = start
        .checked_add(usize::try_from(len).map_err(|_| WalletError::GenericRequestInvalidEnvelope)?)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let value = bytes
        .get(start..end)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    Ok((value, end))
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedAuthenticationSemantics {
    request_id: Option<String>,
    expiry_time: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedProvisioningSemantics {
    system_id: Option<String>,
    parent_id: Option<String>,
    identity_id: String,
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedProvisioningChallenge {
    challenge_id: String,
    created_at: u64,
    name: String,
    system_id: Option<String>,
    parent_id: Option<String>,
    hash: [u8; 32],
}

fn parse_authentication_semantics(
    data: &[u8],
) -> Result<ParsedAuthenticationSemantics, WalletError> {
    let (flags, mut cursor) = read_compact_size_local(data, 0)?;
    if flags
        & !(AUTH_REQUEST_FLAG_HAS_REQUEST_ID
            | AUTH_REQUEST_FLAG_HAS_RECIPIENT_CONSTRAINTS
            | AUTH_REQUEST_FLAG_HAS_EXPIRY_TIME)
        != 0
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let request_id = if flags & AUTH_REQUEST_FLAG_HAS_REQUEST_ID != 0 {
        let (value, next) = decode_compact_address_at(data, cursor)?;
        cursor = next;
        Some(normalize_provisioning_address(&value)?)
    } else {
        None
    };

    if flags & AUTH_REQUEST_FLAG_HAS_RECIPIENT_CONSTRAINTS != 0 {
        let (count, count_len) = read_compact_size_local(data, cursor)?;
        cursor += count_len;
        for _ in 0..count {
            let (_, type_len) = read_compact_size_local(data, cursor)?;
            cursor += type_len;
            let (_, next) = decode_compact_address_at(data, cursor)?;
            cursor = next;
        }
    }

    let expiry_time = if flags & AUTH_REQUEST_FLAG_HAS_EXPIRY_TIME != 0 {
        let (value, len) = read_compact_size_local(data, cursor)?;
        cursor += len;
        Some(value)
    } else {
        None
    };
    if cursor != data.len() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    Ok(ParsedAuthenticationSemantics {
        request_id,
        expiry_time,
    })
}

fn normalize_provisioning_address(value: &str) -> Result<String, WalletError> {
    let trimmed = value.trim();
    if from_base58_check_hash160(trimmed).is_some() {
        return Ok(trimmed.to_string());
    }
    fqn_to_i_addr(trimmed).ok_or(WalletError::GenericRequestInvalidEnvelope)
}

fn parse_provisioning_semantics(data: &[u8]) -> Result<ParsedProvisioningSemantics, WalletError> {
    let (flags, mut cursor) = read_compact_size_local(data, 0)?;
    if flags
        & !(PROVISION_FLAG_HAS_SYSTEM_ID
            | PROVISION_FLAG_HAS_PARENT_ID
            | PROVISION_FLAG_HAS_IDENTITY_ID
            | PROVISION_FLAG_HAS_URI)
        != 0
        || flags & PROVISION_FLAG_HAS_IDENTITY_ID == 0
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    if flags & PROVISION_FLAG_HAS_URI != 0 {
        let (uri_type, type_len) = read_compact_size_local(data, cursor)?;
        cursor += type_len;
        if uri_type != 1 {
            return Err(WalletError::GenericRequestInvalidEnvelope);
        }
        let (uri, next) = read_var_slice_local(data, cursor)?;
        cursor = next;
        let uri =
            std::str::from_utf8(uri).map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
        parse_generic_response_post_callback_uri(uri)?;
    }

    let mut read_address = |present: bool| -> Result<Option<String>, WalletError> {
        if !present {
            return Ok(None);
        }
        let (value, next) = decode_compact_address_at(data, cursor)?;
        cursor = next;
        Ok(Some(normalize_provisioning_address(&value)?))
    };
    let system_id = read_address(flags & PROVISION_FLAG_HAS_SYSTEM_ID != 0)?;
    let parent_id = read_address(flags & PROVISION_FLAG_HAS_PARENT_ID != 0)?;
    let identity_id = read_address(true)?.ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if cursor != data.len() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    Ok(ParsedProvisioningSemantics {
        system_id,
        parent_id,
        identity_id,
    })
}

fn read_optional_hash160_address(
    data: &[u8],
    cursor: &mut usize,
) -> Result<Option<String>, WalletError> {
    let (hash, next) = read_var_slice_local(data, *cursor)?;
    *cursor = next;
    match hash.len() {
        0 => Ok(None),
        20 => {
            let hash: [u8; 20] = hash
                .try_into()
                .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
            Ok(Some(to_base58_check(&hash, I_ADDRESS_VERSION)))
        }
        _ => Err(WalletError::GenericRequestInvalidEnvelope),
    }
}

fn parse_provisioning_challenge_hex(
    challenge_hex: &str,
) -> Result<ParsedProvisioningChallenge, WalletError> {
    let bytes = hex::decode(challenge_hex.trim())
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;
    if bytes.len() > 2_048 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let challenge_key = bytes
        .get(..20)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if hex::encode(challenge_key) != PROVISIONING_CHALLENGE_KEY_HASH_HEX {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let (version, version_len) = read_varint_local(&bytes, 20)?;
    if version != 1 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let (data, outer_end) = read_var_slice_local(&bytes, 20 + version_len)?;
    if outer_end != bytes.len() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let mut cursor = 0usize;
    let challenge_id = read_optional_hash160_address(data, &mut cursor)?
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let created_at_bytes = data
        .get(cursor..cursor + 8)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let created_at = u64::from_le_bytes(
        created_at_bytes
            .try_into()
            .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?,
    );
    cursor += 8;
    if read_optional_hash160_address(data, &mut cursor)?.is_some() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let context_key = data
        .get(cursor..cursor + 20)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if hex::encode(context_key) != LOGIN_CONSENT_CONTEXT_KEY_HASH_HEX {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    cursor += 20;
    let (context_version, context_version_len) = read_varint_local(data, cursor)?;
    cursor += context_version_len;
    if context_version != 1 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let (context_data, next) = read_var_slice_local(data, cursor)?;
    cursor = next;
    if context_data != [0] {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let (name, next) = read_var_slice_local(data, cursor)?;
    cursor = next;
    let name = std::str::from_utf8(name)
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?
        .to_string();
    if name.trim().is_empty() || name.len() > 256 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let system_id = read_optional_hash160_address(data, &mut cursor)?;
    let parent_id = read_optional_hash160_address(data, &mut cursor)?;
    if cursor != data.len() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let challenge_digest = Sha256::digest(&bytes);
    let mut signature_hash = Sha256::new();
    signature_hash.update(b"\x13Verus signed data:\n");
    signature_hash.update(challenge_digest);

    Ok(ParsedProvisioningChallenge {
        challenge_id,
        created_at,
        name,
        system_id,
        parent_id,
        hash: signature_hash.finalize().into(),
    })
}

fn provisioning_challenge_name(identity_id: &str) -> String {
    identity_id
        .split(['.', '@'])
        .next()
        .unwrap_or(identity_id)
        .to_string()
}

fn decode_defined_key_label(
    value_hex: &str,
    verus_chain_id: &str,
) -> Option<(String, String, String)> {
    let bytes = hex::decode(value_hex.trim()).ok()?;
    let (_, version_len) = read_varint_local(&bytes, 0).ok()?;
    let (_, flags_len) = read_varint_local(&bytes, version_len).ok()?;
    let payload_offset = version_len.checked_add(flags_len)?;
    let (uri_len, uri_len_size) = read_compact_size_local(&bytes, payload_offset).ok()?;
    let uri_offset = payload_offset.checked_add(uri_len_size)?;
    let uri_end = uri_offset.checked_add(uri_len as usize)?;
    let uri_bytes = bytes.get(uri_offset..uri_end)?;
    let vdxf_uri = std::str::from_utf8(uri_bytes).ok()?.trim();
    if vdxf_uri.is_empty() {
        return None;
    }

    let key_id = get_data_key_id(vdxf_uri, verus_chain_id)?;
    let namespace_id = if let Some((namespace, suffix)) = vdxf_uri.split_once("::") {
        let suffix = suffix.trim();
        if suffix.is_empty() {
            verus_chain_id.to_string()
        } else {
            fqn_to_i_addr(namespace)?
        }
    } else {
        verus_chain_id.to_string()
    };
    let raw_label = vdxf_uri
        .split("::")
        .nth(1)
        .unwrap_or(vdxf_uri)
        .replace('.', " ");
    let label = capitalize_label(&raw_label);
    Some((key_id, namespace_id, label))
}

fn resolve_signer_cmm_key_labels_from_identity(
    signer_identity: &Value,
    network: WalletNetwork,
) -> HashMap<String, String> {
    let mut labels = HashMap::<String, String>::new();
    let identity = signer_identity
        .get("identity")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let content_map = identity
        .get("contentmultimap")
        .or_else(|| identity.get("contentMultiMap"))
        .and_then(Value::as_object);
    let Some(content_map) = content_map else {
        return labels;
    };
    let signer_identity_id = identity
        .get("identityaddress")
        .or_else(|| identity.get("identityAddress"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let defined_key_entries = content_map.get(DATA_TYPE_DEFINEDKEY_VDXF_ID);
    let Some(defined_key_entries) = defined_key_entries else {
        return labels;
    };

    let verus_chain_id = match network {
        WalletNetwork::Mainnet => MAINNET_VERUS_CHAIN_ID,
        WalletNetwork::Testnet => TESTNET_VERUS_CHAIN_ID,
    };

    let raw_entries = if let Some(entries) = defined_key_entries.as_array() {
        entries.iter().collect::<Vec<_>>()
    } else {
        vec![defined_key_entries]
    };

    for entry in raw_entries {
        let Some(value_hex) = entry.as_str() else {
            continue;
        };
        let Some((key_id, namespace_id, label)) =
            decode_defined_key_label(value_hex, verus_chain_id)
        else {
            continue;
        };
        if signer_identity_id.is_some_and(|expected| !namespace_id.eq_ignore_ascii_case(expected)) {
            continue;
        }
        labels.entry(key_id).or_insert(label);
    }

    labels
}

fn validate_supported_request_grouping(
    detail_ordinals: &[u64],
) -> Result<(bool, bool), WalletError> {
    let auth_count = detail_ordinals
        .iter()
        .filter(|ordinal| **ordinal == VDXF_ORDINAL_AUTHENTICATION_REQUEST)
        .count();
    let update_count = detail_ordinals
        .iter()
        .filter(|ordinal| **ordinal == VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST)
        .count();
    let provisioning_count = detail_ordinals
        .iter()
        .filter(|ordinal| **ordinal == VDXF_ORDINAL_PROVISION_IDENTITY)
        .count();

    if auth_count > 1 || update_count > 1 || provisioning_count > 1 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    if auth_count == 0 && update_count == 0 && provisioning_count == 0 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    if provisioning_count > 0 && auth_count == 0 {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let has_auth = auth_count == 1;
    let has_update = update_count == 1;

    if detail_ordinals.iter().any(|ordinal| {
        !matches!(
            *ordinal,
            VDXF_ORDINAL_AUTHENTICATION_REQUEST
                | VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST
                | VDXF_ORDINAL_PROVISION_IDENTITY
        )
    }) {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    Ok((has_auth, has_update))
}

fn validate_response_inputs_for_request(
    has_auth_request: bool,
    has_update_request: bool,
    request: &BuildAndSignGenericResponseRequest,
) -> Result<(), WalletError> {
    let has_auth_response = request.authentication.is_some();
    let has_update_response = request.identity_update.is_some();

    if has_auth_request != has_auth_response || has_update_request != has_update_response {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    if !has_auth_response && !has_update_response {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    Ok(())
}

fn build_authentication_response_detail(request_id: Option<&str>) -> Result<Vec<u8>, WalletError> {
    let mut detail_data = Vec::new();
    let flags = if request_id.is_some() {
        AUTHENTICATION_RESPONSE_FLAG_HAS_REQUEST_ID
    } else {
        0
    };
    write_varint(&mut detail_data, flags);
    if let Some(request_id) = request_id {
        detail_data.extend_from_slice(&encode_compact_i_address(request_id)?);
    }

    let mut detail = Vec::new();
    write_compact_size(&mut detail, VDXF_ORDINAL_AUTHENTICATION_RESPONSE as usize);
    write_varint(&mut detail, 1);
    write_var_slice(&mut detail, &detail_data);
    Ok(detail)
}

fn build_identity_update_response_detail(
    request_id: Option<&str>,
    txid_hex: &str,
) -> Result<Vec<u8>, WalletError> {
    let mut detail_data = Vec::new();
    let mut flags = IDENTITY_UPDATE_RESPONSE_FLAG_CONTAINS_TXID;
    if request_id.is_some() {
        flags |= IDENTITY_UPDATE_RESPONSE_FLAG_CONTAINS_REQUEST_ID;
    }

    write_varint(&mut detail_data, flags);
    if let Some(request_id) = request_id {
        detail_data.extend_from_slice(&encode_compact_i_address(request_id)?);
    }

    let mut txid = hex::decode(txid_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
    if txid.len() != 32 {
        return Err(WalletError::OperationFailed);
    }
    txid.reverse();
    detail_data.extend_from_slice(&txid);

    let mut detail = Vec::new();
    write_compact_size(&mut detail, VDXF_ORDINAL_IDENTITY_UPDATE_RESPONSE as usize);
    write_varint(&mut detail, 1);
    write_var_slice(&mut detail, &detail_data);
    Ok(detail)
}

fn build_unsigned_generic_response_hex(
    request: &BuildAndSignGenericResponseRequest,
    created_at: u64,
) -> Result<String, WalletError> {
    let parsed_request = parse_generic_envelope_hex(&request.request_hex)?;
    let detail_ordinals = parsed_request
        .details
        .iter()
        .map(|detail| detail.ordinal)
        .collect::<Vec<_>>();
    let (has_auth_request, has_update_request) =
        validate_supported_request_grouping(&detail_ordinals)?;
    validate_response_inputs_for_request(has_auth_request, has_update_request, request)?;

    let mut details = Vec::new();
    if let Some(authentication) = request.authentication.as_ref() {
        details.push(build_authentication_response_detail(
            authentication.request_id.as_deref(),
        )?);
    }
    if let Some(identity_update) = request.identity_update.as_ref() {
        details.push(build_identity_update_response_detail(
            identity_update.request_id.as_deref(),
            &identity_update.txid,
        )?);
    }

    if details.is_empty() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let mut flags = GENERIC_RESPONSE_FLAG_SIGNED
        | GENERIC_RESPONSE_FLAG_HAS_CREATED_AT
        | GENERIC_RESPONSE_FLAG_HAS_REQUEST_HASH;
    if parsed_request.is_testnet {
        flags |= GENERIC_RESPONSE_FLAG_IS_TESTNET;
    }
    if parsed_request.request_id.is_some() {
        flags |= GENERIC_RESPONSE_FLAG_HAS_REQUEST_ID;
    }
    if details.len() > 1 {
        flags |= GENERIC_RESPONSE_FLAG_MULTI_DETAILS;
    }

    let mut response = Vec::new();
    write_compact_size(&mut response, 1);
    write_compact_size(&mut response, flags as usize);

    write_varint(&mut response, 0);
    write_compact_size(&mut response, 0);
    write_compact_size(&mut response, VERIFIABLE_SIGNATURE_VERSION_V2 as usize);
    write_compact_size(&mut response, HASH_TYPE_SHA256 as usize);
    response.extend_from_slice(&encode_compact_i_address(&request.signer.system_id)?);
    response.extend_from_slice(&encode_compact_i_address(&request.signer.identity_id)?);
    write_var_slice(&mut response, &[]);

    if let Some(request_id) = parsed_request.request_id.as_deref() {
        response.extend_from_slice(&encode_compact_i_address(request_id)?);
    }
    write_compact_size(&mut response, created_at as usize);

    if details.len() > 1 {
        write_compact_size(&mut response, details.len());
        for detail in details {
            response.extend_from_slice(&detail);
        }
    } else {
        response.extend_from_slice(&details[0]);
    }

    write_compact_size(&mut response, HASH_TYPE_SHA256 as usize);
    let request_hash = get_raw_envelope_sha256(&parsed_request);
    write_var_slice(&mut response, &request_hash);

    Ok(hex::encode(response))
}

fn build_and_sign_generic_response_internal(
    request: &BuildAndSignGenericResponseRequest,
    network: Network,
    active_wallet_primary_address: &str,
    allowed_primary_addresses: &[String],
    signed_block_height: u32,
    private_key: &[u8; 32],
    created_at: u64,
) -> Result<String, WalletError> {
    let parsed_request = parse_generic_envelope_hex(&request.request_hex)?;
    let request_is_testnet = parsed_request.is_testnet;
    let active_is_testnet = matches!(network, Network::Testnet);
    if request_is_testnet != active_is_testnet {
        return Err(WalletError::UnsupportedNetwork);
    }

    ensure_active_wallet_controls_signer(allowed_primary_addresses, active_wallet_primary_address)?;

    let unsigned_response_hex = build_unsigned_generic_response_hex(request, created_at)?;
    let parsed_response = parse_generic_envelope_hex(&unsigned_response_hex)?;
    if parsed_response.created_at.is_none() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    if !parsed_response.signature_data.extra_hash_data.is_empty() {
        return Err(WalletError::GenericRequestUnsupportedSignature);
    }

    let raw_envelope_sha256 = get_raw_envelope_sha256(&parsed_response);
    let identity_hash = compute_identity_signature_hash(
        &parsed_response.signature_data,
        signed_block_height,
        raw_envelope_sha256,
    )?;
    let signature_as_vch =
        sign_identity_hash_with_private_bytes(identity_hash, signed_block_height, private_key)?;
    let signed_signature_data = parsed_response
        .signature_data
        .to_buffer_with_signature(&signature_as_vch);

    let mut signed_envelope = Vec::with_capacity(
        parsed_response.prefix_without_signature.len()
            + signed_signature_data.len()
            + parsed_response.tail_after_signature.len(),
    );
    signed_envelope.extend_from_slice(&parsed_response.prefix_without_signature);
    signed_envelope.extend_from_slice(&signed_signature_data);
    signed_envelope.extend_from_slice(&parsed_response.tail_after_signature);

    let signed_response_hex = hex::encode(&signed_envelope);
    let parsed_signed = parse_generic_envelope_hex(&signed_response_hex)?;
    let parsed_identity_signature =
        parse_identity_signature(&parsed_signed.signature_data.signature_as_vch)?;
    let verify_hash = compute_identity_signature_hash(
        &parsed_signed.signature_data,
        parsed_identity_signature.block_height,
        get_raw_envelope_sha256(&parsed_signed),
    )?;
    let valid = verify_identity_signature_against_addresses(
        verify_hash,
        &parsed_identity_signature,
        allowed_primary_addresses,
        network,
    );
    if !valid {
        return Err(WalletError::OperationFailed);
    }

    Ok(signed_response_hex)
}

fn expire_provisioning_jobs(jobs: &mut [ProvisioningJobRecord], now: u64) -> bool {
    let mut changed = false;

    for job in jobs {
        if matches!(
            job.status.as_str(),
            PROVISIONING_JOB_STATUS_LINKED | PROVISIONING_JOB_STATUS_EXPIRED
        ) {
            continue;
        }

        if now.saturating_sub(job.created_at) > PROVISIONING_JOB_TTL_SECS {
            job.status = PROVISIONING_JOB_STATUS_EXPIRED.to_string();
            changed = true;
        }
    }

    changed
}

async fn store_provisioning_jobs_for_context(
    context: &crate::commands::identity::IdentitySessionContext,
    jobs: &[ProvisioningJobRecord],
) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
    context.account_state_store.store_provisioning_jobs(
        &context.account_id,
        context.network,
        jobs,
    )?;
    context
        .account_state_store
        .load_provisioning_jobs(&context.account_id, context.network)
}

async fn refresh_provisioning_jobs_for_context(
    context: &crate::commands::identity::IdentitySessionContext,
    vrpc_provider_pool: &VrpcProviderPool,
    jobs: &mut [ProvisioningJobRecord],
) -> Result<bool, WalletError> {
    let mut changed = false;
    let provider = vrpc_provider_pool.for_network(context.network);

    for job in jobs {
        if job.status != PROVISIONING_JOB_STATUS_PENDING
            && job.status != PROVISIONING_JOB_STATUS_READY
        {
            continue;
        }

        let Some(lookup_target) = job
            .requested_identity_address
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| Some(job.requested_fqn.as_str()).filter(|value| !value.trim().is_empty()))
        else {
            continue;
        };

        let raw_identity = match provider.getidentity(lookup_target).await {
            Ok(value) => value,
            Err(error) => match map_link_identity_lookup_error(error) {
                WalletError::IdentityNotFound => {
                    if job.status != PROVISIONING_JOB_STATUS_PENDING {
                        job.status = PROVISIONING_JOB_STATUS_PENDING.to_string();
                        changed = true;
                    }
                    continue;
                }
                other => return Err(other),
            },
        };

        let parsed = match parse_getidentity_payload(raw_identity) {
            Ok(value) => value,
            Err(WalletError::IdentityNotFound) => {
                if job.status != PROVISIONING_JOB_STATUS_PENDING {
                    job.status = PROVISIONING_JOB_STATUS_PENDING.to_string();
                    changed = true;
                }
                continue;
            }
            Err(other) => return Err(other),
        };

        let details = build_identity_details_from_payload(
            &parsed.identity,
            parsed.status,
            &context.primary_address,
            job.requested_identity_address.as_deref(),
            parsed.fully_qualified_name.as_deref(),
            parsed.friendly_name.as_deref(),
        )?;

        let next_status = if details.owned_by_primary_address {
            PROVISIONING_JOB_STATUS_READY
        } else {
            PROVISIONING_JOB_STATUS_PENDING
        };

        if job.status != next_status {
            job.status = next_status.to_string();
            changed = true;
        }
    }

    Ok(changed)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn verify_generic_request_signature(
    request_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GenericRequestVerificationResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let parsed = parse_generic_envelope_hex(&request_hex)?;
    let identity_signature = parse_identity_signature(&parsed.signature_data.signature_as_vch)?;
    let provider = vrpc_provider_pool.for_system(network, &parsed.signature_data.signer_system_id);
    let valid = verify_generic_request_signature_with_provider(
        provider,
        &parsed,
        wallet_network_to_crypto_network(network),
    )
    .await?;

    Ok(GenericRequestVerificationResult {
        valid,
        signer_system_id: parsed.signature_data.signer_system_id,
        signer_identity_id: parsed.signature_data.signer_identity_id,
        signature_block_height: identity_signature.block_height,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sign_generic_response(
    response_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let network = context.wallet_network;
    let active_wallet_primary_address = context.vrsc_address.clone();
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let parsed = parse_generic_envelope_hex(&response_hex)?;
    if parsed.created_at.is_none() {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    if !parsed.signature_data.extra_hash_data.is_empty() {
        return Err(WalletError::GenericRequestUnsupportedSignature);
    }

    let provider = vrpc_provider_pool.for_system(network, &parsed.signature_data.signer_system_id);
    validate_identity_control_for_active_wallet(
        provider,
        &parsed.signature_data.signer_identity_id,
        &active_wallet_primary_address,
    )
    .await?;

    let info = provider.getinfo().await?;
    let signed_block_height = extract_chain_height(&info)?;
    let raw_envelope_sha256 = get_raw_envelope_sha256(&parsed);
    let identity_hash = compute_identity_signature_hash(
        &parsed.signature_data,
        signed_block_height,
        raw_envelope_sha256,
    )?;
    let signature_as_vch =
        sign_identity_hash_with_private_bytes(identity_hash, signed_block_height, &private_key)?;

    let signed_signature_data = parsed
        .signature_data
        .to_buffer_with_signature(&signature_as_vch);

    let mut signed_envelope = Vec::with_capacity(
        parsed.prefix_without_signature.len()
            + signed_signature_data.len()
            + parsed.tail_after_signature.len(),
    );
    signed_envelope.extend_from_slice(&parsed.prefix_without_signature);
    signed_envelope.extend_from_slice(&signed_signature_data);
    signed_envelope.extend_from_slice(&parsed.tail_after_signature);

    let parsed_signed = parse_generic_envelope_hex(&hex::encode(&signed_envelope))?;
    let parsed_identity_signature =
        parse_identity_signature(&parsed_signed.signature_data.signature_as_vch)?;
    let signer_identity = provider
        .getidentity(&parsed_signed.signature_data.signer_identity_id)
        .await?;
    let allowed_addresses = signer_identity
        .get("identity")
        .and_then(|identity| {
            identity
                .get("primaryaddresses")
                .or_else(|| identity.get("primaryAddresses"))
        })
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let verify_hash = compute_identity_signature_hash(
        &parsed_signed.signature_data,
        parsed_identity_signature.block_height,
        get_raw_envelope_sha256(&parsed_signed),
    )?;
    let valid = verify_identity_signature_against_addresses(
        verify_hash,
        &parsed_identity_signature,
        &allowed_addresses,
        wallet_network_to_crypto_network(network),
    );
    if !valid {
        return Err(WalletError::OperationFailed);
    }

    Ok(hex::encode(signed_envelope))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn build_and_sign_generic_response(
    request_hex: String,
    signer: crate::types::GenericResponseSignerInput,
    authentication: Option<crate::types::GenericAuthenticationResponseInput>,
    identity_update: Option<crate::types::GenericIdentityUpdateResponseInput>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<BuildAndSignGenericResponseResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let network = context.wallet_network;
    let active_wallet_primary_address = context.vrsc_address.clone();
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let request = BuildAndSignGenericResponseRequest {
        request_hex,
        signer,
        authentication,
        identity_update,
    };

    let parsed_request = parse_generic_envelope_hex(&request.request_hex)?;
    if parsed_request.is_testnet != matches!(network, WalletNetwork::Testnet) {
        return Err(WalletError::UnsupportedNetwork);
    }

    let provider = vrpc_provider_pool.for_system(network, &request.signer.system_id);
    validate_identity_control_for_active_wallet(
        provider,
        &request.signer.identity_id,
        &active_wallet_primary_address,
    )
    .await?;

    let signer_identity = provider.getidentity(&request.signer.identity_id).await?;
    let allowed_primary_addresses = extract_primary_addresses(&signer_identity);
    let info = provider.getinfo().await?;
    let signed_block_height = extract_chain_height(&info)?;
    let signed_response_hex = build_and_sign_generic_response_internal(
        &request,
        wallet_network_to_crypto_network(network),
        &active_wallet_primary_address,
        &allowed_primary_addresses,
        signed_block_height,
        &private_key,
        now_unix_seconds(),
    )?;

    Ok(BuildAndSignGenericResponseResult {
        signed_response_hex,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn post_generic_response_callback(
    callback_uri: String,
    response_hex: String,
) -> Result<(), WalletError> {
    let callback_uri = parse_generic_response_post_callback_uri(&callback_uri)?;
    let response_bytes =
        hex::decode(response_hex.trim()).map_err(|_| WalletError::OperationFailed)?;

    let response = reqwest::Client::new()
        .post(callback_uri)
        .header(CONTENT_TYPE, "application/octet-stream")
        .body(response_bytes)
        .send()
        .await
        .map_err(|_| WalletError::NetworkError)?;

    if !response.status().is_success() {
        return Err(WalletError::OperationFailed);
    }

    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_generic_request_callback(
    callback_uri: String,
    response_hex: String,
    app: AppHandle,
) -> Result<(), WalletError> {
    let redirect_url = build_generic_response_redirect_url(&callback_uri, &response_hex)?;
    app.opener()
        .open_url(redirect_url.to_string(), None::<&str>)
        .map_err(|_| WalletError::OperationFailed)?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn prepare_provisioning_signature(
    request_hex: String,
    challenge_hex: String,
    signing_address: String,
    system_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    provisioning_signature_store: State<'_, ProvisioningSignatureStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let network = context.wallet_network;
    if !signing_address
        .trim()
        .eq_ignore_ascii_case(&context.vrsc_address)
    {
        return Err(WalletError::InvalidAddress);
    }

    let parsed_request = parse_generic_envelope_hex(request_hex.trim())?;
    if parsed_request.is_testnet != matches!(network, WalletNetwork::Testnet) {
        return Err(WalletError::UnsupportedNetwork);
    }
    validate_supported_request_grouping(
        &parsed_request
            .details
            .iter()
            .map(|detail| detail.ordinal)
            .collect::<Vec<_>>(),
    )?;

    let request_provider =
        vrpc_provider_pool.for_system(network, &parsed_request.signature_data.signer_system_id);
    if !verify_generic_request_signature_with_provider(
        request_provider,
        &parsed_request,
        wallet_network_to_crypto_network(network),
    )
    .await?
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let authentication = parsed_request
        .details
        .iter()
        .find(|detail| detail.ordinal == VDXF_ORDINAL_AUTHENTICATION_REQUEST)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)
        .and_then(|detail| parse_authentication_semantics(&detail.data))?;
    let provisioning = parsed_request
        .details
        .iter()
        .find(|detail| detail.ordinal == VDXF_ORDINAL_PROVISION_IDENTITY)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)
        .and_then(|detail| parse_provisioning_semantics(&detail.data))?;
    let challenge = parse_provisioning_challenge_hex(&challenge_hex)?;
    let request_id = authentication
        .request_id
        .or(parsed_request.request_id)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if !challenge.challenge_id.eq_ignore_ascii_case(&request_id)
        || challenge.name != provisioning_challenge_name(&provisioning.identity_id)
        || challenge.system_id != provisioning.system_id
        || challenge.parent_id != provisioning.parent_id
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let expected_system_id = provisioning
        .system_id
        .as_deref()
        .unwrap_or(&parsed_request.signature_data.signer_system_id);
    if !system_id.trim().eq_ignore_ascii_case(expected_system_id) {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let now = now_unix_seconds();
    if challenge.created_at > now.saturating_add(PROVISIONING_CREATED_AT_CLOCK_SKEW_SECS)
        || now.saturating_sub(challenge.created_at) > PROVISIONING_SIGNATURE_TTL_SECS
        || authentication
            .expiry_time
            .is_some_and(|expiry| expiry < now || challenge.created_at > expiry)
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let expires_at = authentication
        .expiry_time
        .unwrap_or_else(|| now.saturating_add(PROVISIONING_SIGNATURE_TTL_SECS))
        .min(now.saturating_add(PROVISIONING_SIGNATURE_TTL_SECS));
    let signing_challenge_id = Uuid::new_v4().to_string();
    if !provisioning_signature_store.put(
        signing_challenge_id.clone(),
        ProvisioningSignatureChallenge {
            session_id: context.session_id,
            account_id: context.account_id,
            network,
            system_id: expected_system_id.to_string(),
            challenge_hash: challenge.hash,
            expires_at,
        },
    ) {
        return Err(WalletError::WalletLocked);
    }

    Ok(signing_challenge_id)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn confirm_provisioning_signature(
    signing_challenge_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    provisioning_signature_store: State<'_, ProvisioningSignatureStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let challenge = provisioning_signature_store
        .take(signing_challenge_id.trim(), &context.session_id)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if challenge.account_id != context.account_id || challenge.network != context.wallet_network {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let private_key = load_primary_private_scalar_for_context(&context).await?;
    let provider = vrpc_provider_pool.for_system(challenge.network, &challenge.system_id);
    let info = provider.getinfo().await?;
    let signed_block_height = extract_chain_height(&info)?;
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;
    let signature_as_vch = sign_identity_hash_with_private_bytes(
        challenge.challenge_hash,
        signed_block_height,
        &private_key,
    )?;

    Ok(BASE64_STANDARD.encode(signature_as_vch))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn verify_identity_signature_hash(
    hash_hex: String,
    signature_base64: String,
    signer_identity_id: String,
    signer_system_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<bool, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    let decoded_hash = hex::decode(hash_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
    let hash_bytes: [u8; 32] = decoded_hash
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::OperationFailed)?;
    let signature_as_vch = BASE64_STANDARD
        .decode(signature_base64.trim())
        .map_err(|_| WalletError::OperationFailed)?;
    let parsed_signature = parse_identity_signature(&signature_as_vch)?;
    let provider = vrpc_provider_pool.for_system(network, signer_system_id.trim());
    let signer_identity = provider.getidentity(signer_identity_id.trim()).await?;
    let allowed_addresses = signer_identity
        .get("identity")
        .and_then(|identity| {
            identity
                .get("primaryaddresses")
                .or_else(|| identity.get("primaryAddresses"))
        })
        .and_then(|value| value.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let minimum_signatures = signer_identity
        .get("identity")
        .and_then(|identity| {
            identity
                .get("minimumsignatures")
                .or_else(|| identity.get("minimumSignatures"))
        })
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| {
                    value
                        .as_i64()
                        .and_then(|number| (number >= 0).then_some(number as u64))
                })
                .or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<u64>().ok())
                })
        })
        .unwrap_or(1);

    if minimum_signatures != 1 {
        return Ok(false);
    }

    Ok(verify_identity_signature_against_addresses(
        hash_bytes,
        &parsed_signature,
        &allowed_addresses,
        wallet_network_to_crypto_network(network),
    ))
}

struct GenericIdentityUpdateInspection {
    target: crate::core::channels::vrpc::identity::preflight::TargetIdentityState,
    effective_requested_identity: Value,
    review: GenericIdentityUpdateReviewResult,
}

fn validate_generic_request_funding_source(
    selected_address: &str,
    selected_system_id: &str,
    request_system_id: Option<&str>,
    primary_address: &str,
    linked_identities: &[LinkedIdentity],
    watched_addresses: &[String],
) -> Result<(), WalletError> {
    // Generic request funding mirrors mobile: allow wallet-owned transparent scopes
    // (primary address or linked identities) and reject watched/read-only addresses.
    if let Some(request_system_id) = request_system_id {
        let trimmed_request_system_id = request_system_id.trim();
        if !trimmed_request_system_id.is_empty()
            && !selected_system_id.eq_ignore_ascii_case(trimmed_request_system_id)
        {
            return Err(WalletError::InvalidAddress);
        }
    }

    if selected_address.eq_ignore_ascii_case(primary_address) {
        return Ok(());
    }

    if linked_identities.iter().any(|identity| {
        identity
            .identity_address
            .eq_ignore_ascii_case(selected_address)
    }) {
        return Ok(());
    }

    if watched_addresses
        .iter()
        .any(|address| address.eq_ignore_ascii_case(selected_address))
    {
        return Err(WalletError::InvalidAddress);
    }

    Err(WalletError::InvalidAddress)
}

async fn ensure_identity_update_request_not_expired(
    provider: &crate::core::channels::vrpc::VrpcProvider,
    request_meta: Option<&GenericIdentityUpdateRequestMeta>,
) -> Result<(), WalletError> {
    let info = provider.getinfo().await?;
    let current_height = extract_chain_height(&info)?;
    if request_meta
        .and_then(|meta| meta.expiry_height)
        .is_some_and(|expiry_height| current_height > expiry_height)
    {
        return Err(WalletError::IdentityRequestExpired);
    }

    Ok(())
}

async fn inspect_generic_identity_update(
    provider: &crate::core::channels::vrpc::VrpcProvider,
    network: WalletNetwork,
    requested_identity_json: &Value,
    target_identity_address: &str,
    active_wallet_primary_address: &str,
    request_meta: Option<&GenericIdentityUpdateRequestMeta>,
) -> Result<GenericIdentityUpdateInspection, WalletError> {
    let raw_target = provider
        .getidentity(target_identity_address.trim())
        .await
        .map_err(map_preflight_identity_lookup_error)?;
    let target = parse_target_identity(raw_target.clone())?;
    let effective_requested_identity =
        merge_identity_update_patch(&target.identity, requested_identity_json);

    validate_target_state(&target.status, &IdentityOperation::Update)?;
    validate_unsupported_identity_flags(&target.identity, &effective_requested_identity)?;

    let mut warnings = Vec::<IdentityWarning>::new();
    if target.identity == effective_requested_identity {
        warnings.push(IdentityWarning {
            warning_type: "no_effect".to_string(),
            message: "Request does not change identity fields.".to_string(),
        });
    }

    let fully_qualified_name = first_non_empty_field(
        &raw_target,
        &[
            "fullyqualifiedname",
            "fullyQualifiedName",
            "friendlyname",
            "friendlyName",
        ],
    );
    let high_risk_changes =
        classify_high_risk_changes(&target.identity, &effective_requested_identity);
    let signer_cmm_key_labels = if let Some(signer_identity_id) =
        request_meta.and_then(|meta| meta.signer_identity_id.as_deref())
    {
        let signer_identity = match provider.getidentitycontent(signer_identity_id).await {
            Ok(identity_content) => Ok(identity_content),
            Err(_) => provider.getidentity(signer_identity_id).await,
        };

        signer_identity
            .ok()
            .map(|signer_identity| {
                resolve_signer_cmm_key_labels_from_identity(&signer_identity, network)
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let friendly_names = resolve_identity_friendly_names(
        provider,
        &target.identity,
        &effective_requested_identity,
        request_meta.and_then(|meta| meta.signer_identity_id.as_deref()),
    )
    .await;
    let primary_address_after_update_info = build_primary_address_update_info(
        &effective_requested_identity,
        active_wallet_primary_address,
    );

    Ok(GenericIdentityUpdateInspection {
        target: target.clone(),
        effective_requested_identity: effective_requested_identity.clone(),
        review: GenericIdentityUpdateReviewResult {
            target_identity: target_identity_address.trim().to_string(),
            warnings,
            high_risk_changes,
            current_identity: target.identity.clone(),
            requested_identity: effective_requested_identity,
            fully_qualified_name,
            friendly_names,
            signer_cmm_key_labels,
            primary_address_after_update_info,
            current_authorities: GenericIdentityAuthorities {
                revocation: first_non_empty_field(
                    &target.identity,
                    &["revocationauthority", "revocationAuthority"],
                ),
                recovery: first_non_empty_field(
                    &target.identity,
                    &["recoveryauthority", "recoveryAuthority"],
                ),
            },
        },
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn review_generic_identity_update(
    requested_identity_json: Value,
    target_identity_address: String,
    system_id: String,
    request_meta: Option<GenericIdentityUpdateRequestMeta>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GenericIdentityUpdateReviewResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    if !requested_identity_json.is_object() {
        return Err(WalletError::IdentityBuildFailed);
    }

    let provider = vrpc_provider_pool.for_system(network, system_id.trim());
    ensure_identity_update_request_not_expired(&provider, request_meta.as_ref()).await?;

    let inspection = inspect_generic_identity_update(
        &provider,
        network,
        &requested_identity_json,
        &target_identity_address,
        &session_vrpc_address,
        request_meta.as_ref(),
    )
    .await?;

    Ok(inspection.review)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_generic_identity_update(
    requested_identity_json: Value,
    target_identity_address: String,
    source_channel_id: String,
    request_meta: Option<GenericIdentityUpdateRequestMeta>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GenericIdentityUpdatePreflightResult, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let account_id = context.account_id.clone();
    let session_vrpc_address = context.primary_address.clone();
    let network = context.network;

    if !requested_identity_json.is_object() {
        return Err(WalletError::IdentityBuildFailed);
    }

    let resolved = vrpc::parse_vrpc_channel_id(&source_channel_id, Some(&session_vrpc_address))?;
    let linked_identities = load_linked_for_context(&context).await?;
    let watched_addresses = context
        .account_state_store
        .load_watched_vrpc_addresses(&context.account_id, context.network)?;
    validate_generic_request_funding_source(
        &resolved.address,
        &resolved.system_id,
        request_meta
            .as_ref()
            .and_then(|meta| meta.request_system_id.as_deref()),
        &context.primary_address,
        &linked_identities,
        &watched_addresses,
    )?;

    let canonical_channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
    let provider = vrpc_provider_pool.for_system(network, &resolved.system_id);
    ensure_identity_update_request_not_expired(&provider, request_meta.as_ref()).await?;

    let inspection = inspect_generic_identity_update(
        &provider,
        network,
        &requested_identity_json,
        &target_identity_address,
        &session_vrpc_address,
        request_meta.as_ref(),
    )
    .await?;
    let GenericIdentityUpdateInspection {
        target,
        effective_requested_identity: _effective_requested_identity,
        mut review,
    } = inspection;

    validate_operation_authority(
        &provider,
        &IdentityOperation::Update,
        &target.identity,
        &target.status,
        &resolved.address,
    )
    .await?;

    let funding_candidates = parse_funding_utxos(
        &provider
            .getaddressutxos(&[resolved.address.clone()])
            .await
            .map_err(|_| WalletError::IdentityBuildFailed)?,
    );
    if total_satoshis(&funding_candidates) < DEFAULT_FEE_SAT {
        return Err(WalletError::InsufficientFunds);
    }

    let update_tx_raw = provider
        .updateidentity(&requested_identity_json, true)
        .await
        .map_err(|err| match err {
            WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
            _ => WalletError::IdentityBuildFailed,
        })?;
    let update_tx_hex = parse_updateidentity_hex(update_tx_raw)?;
    let mut template_tx =
        crate::core::channels::vrpc::identity::verus_tx::codec::decode_hex(&update_tx_hex)
            .map_err(|_| WalletError::IdentityBuildFailed)?;

    let (identity_input_script, identity_input_satoshis) =
        fetch_identity_prevout(provider, &target.txid, target.vout).await?;
    let (unsigned_hex, signable_inputs, dropped_dust_change) = build_unsigned_identity_tx(
        &mut template_tx,
        &target.txid,
        target.vout,
        &identity_input_script,
        identity_input_satoshis,
        &funding_candidates,
        DEFAULT_FEE_SAT,
    )?;

    let fee = sat_to_decimal_string(DEFAULT_FEE_SAT);
    let preflight_id = Uuid::new_v4().to_string();
    let payload = IdentityPreflightPayload {
        unsigned_hex,
        signable_inputs,
        operation: IdentityOperation::Update,
        target_identity: target_identity_address.trim().to_string(),
        from_address: resolved.address.clone(),
        fee: fee.clone(),
        memo: None,
    };
    let payload_value =
        serde_json::to_value(payload).map_err(|_| WalletError::IdentityBuildFailed)?;

    if !preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            session_id: context.session_id,
            channel_id: canonical_channel_id,
            account_id,
            payload: payload_value,
        },
        Some(IDENTITY_PREFLIGHT_TTL),
    ) {
        return Err(WalletError::WalletLocked);
    }

    if dropped_dust_change {
        review.warnings.push(IdentityWarning {
            warning_type: "dust_change".to_string(),
            message: "Change below dust threshold is added to fee.".to_string(),
        });
    }

    Ok(GenericIdentityUpdatePreflightResult {
        preflight_id,
        target_identity: target_identity_address.trim().to_string(),
        from_address: resolved.address.clone(),
        fee,
        fee_currency: resolved.system_id,
        warnings: review.warnings,
        high_risk_changes: review.high_risk_changes,
        current_identity: review.current_identity,
        requested_identity: review.requested_identity,
        fully_qualified_name: review.fully_qualified_name,
        friendly_names: review.friendly_names,
        signer_cmm_key_labels: review.signer_cmm_key_labels,
        primary_address_after_update_info: review.primary_address_after_update_info,
        current_authorities: review.current_authorities,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_identity_provisioning_jobs(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut jobs = context
        .account_state_store
        .load_provisioning_jobs(&context.account_id, context.network)?;

    if expire_provisioning_jobs(&mut jobs, now_unix_seconds()) {
        jobs = store_provisioning_jobs_for_context(&context, &jobs).await?;
    }

    Ok(jobs)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn store_generic_provisioning_job(
    request: StoreGenericProvisioningJobRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
) -> Result<ProvisioningJobRecord, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut jobs = context
        .account_state_store
        .load_provisioning_jobs(&context.account_id, context.network)?;

    expire_provisioning_jobs(&mut jobs, now_unix_seconds());

    let requested_fqn = request.requested_fqn.trim();
    let signing_id = request.signing_id.trim();
    if request.request_hex.trim().is_empty() || requested_fqn.is_empty() || signing_id.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let record = ProvisioningJobRecord {
        job_id: Uuid::new_v4().to_string(),
        request_type: PROVISIONING_JOB_REQUEST_TYPE_GENERIC.to_string(),
        request_hex: request.request_hex.trim().to_string(),
        requested_identity_address: request.requested_identity_address.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
        requested_fqn: requested_fqn.to_string(),
        signing_id: signing_id.to_string(),
        has_response_uris: request.has_response_uris,
        info_uri: request.info_uri.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
        status: request
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(PROVISIONING_JOB_STATUS_PENDING)
            .to_string(),
        created_at: now_unix_seconds(),
        error: request.error.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
    };

    jobs.retain(|existing| {
        !(existing.request_type == PROVISIONING_JOB_REQUEST_TYPE_GENERIC
            && existing
                .request_hex
                .eq_ignore_ascii_case(&record.request_hex)
            && existing
                .requested_fqn
                .eq_ignore_ascii_case(&record.requested_fqn)
            && existing.status != PROVISIONING_JOB_STATUS_LINKED)
    });
    jobs.push(record.clone());

    let stored_jobs = store_provisioning_jobs_for_context(&context, &jobs).await?;
    stored_jobs
        .into_iter()
        .find(|job| job.job_id.eq_ignore_ascii_case(&record.job_id))
        .ok_or(WalletError::OperationFailed)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn refresh_identity_provisioning_jobs(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut jobs = context
        .account_state_store
        .load_provisioning_jobs(&context.account_id, context.network)?;

    let mut changed = expire_provisioning_jobs(&mut jobs, now_unix_seconds());
    changed |= refresh_provisioning_jobs_for_context(
        &context,
        vrpc_provider_pool.inner().as_ref(),
        &mut jobs,
    )
    .await?;

    if changed {
        jobs = store_provisioning_jobs_for_context(&context, &jobs).await?;
    }

    Ok(jobs)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn link_ready_identity_provisioning(
    job_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<LinkReadyProvisioningJobResult, WalletError> {
    let requested_job_id = job_id.trim();
    if requested_job_id.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let mut jobs = context
        .account_state_store
        .load_provisioning_jobs(&context.account_id, context.network)?;

    let mut changed = expire_provisioning_jobs(&mut jobs, now_unix_seconds());
    changed |= refresh_provisioning_jobs_for_context(
        &context,
        vrpc_provider_pool.inner().as_ref(),
        &mut jobs,
    )
    .await?;

    let job_index = jobs
        .iter()
        .position(|job| job.job_id.eq_ignore_ascii_case(requested_job_id))
        .ok_or(WalletError::OperationFailed)?;

    let lookup_target = jobs[job_index]
        .requested_identity_address
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| jobs[job_index].requested_fqn.clone());

    if jobs[job_index].request_type != PROVISIONING_JOB_REQUEST_TYPE_GENERIC {
        return Err(WalletError::OperationFailed);
    }

    if jobs[job_index].status != PROVISIONING_JOB_STATUS_READY {
        if changed {
            let _ = store_provisioning_jobs_for_context(&context, &jobs).await?;
        }
        return Err(WalletError::OperationFailed);
    }

    let raw_identity = vrpc_provider_pool
        .for_network(context.network)
        .getidentity(&lookup_target)
        .await
        .map_err(map_link_identity_lookup_error)?;
    let parsed = parse_getidentity_payload(raw_identity)?;
    let details = build_identity_details_from_payload(
        &parsed.identity,
        parsed.status,
        &context.primary_address,
        jobs[job_index].requested_identity_address.as_deref(),
        parsed.fully_qualified_name.as_deref(),
        parsed.friendly_name.as_deref(),
    )?;

    if !details.owned_by_primary_address {
        return Err(WalletError::IdentityOwnershipMismatch);
    }

    let current = load_linked_for_context(&context).await?;
    let updated = upsert_linked_identity(current, linked_identity_from_details(&details));
    let linked_identities = store_linked_for_context(&context, &updated).await?;

    jobs[job_index].status = PROVISIONING_JOB_STATUS_LINKED.to_string();
    let stored_jobs = store_provisioning_jobs_for_context(&context, &jobs).await?;
    let linked_job = stored_jobs
        .into_iter()
        .find(|job| job.job_id.eq_ignore_ascii_case(requested_job_id))
        .ok_or(WalletError::OperationFailed)?;

    Ok(LinkReadyProvisioningJobResult {
        job: linked_job,
        linked_identities,
    })
}

fn first_non_empty_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        let field = value.get(*key)?;
        let Some(string_value) = field.as_str() else {
            continue;
        };
        let trimmed = string_value.trim();
        if trimmed.is_empty() {
            continue;
        }
        return Some(trimmed.to_string());
    }

    None
}

fn extract_flags(value: &Value) -> u64 {
    let Some(flags_value) = value.get("flags") else {
        return 0;
    };

    if let Some(number) = flags_value.as_u64() {
        return number;
    }
    if let Some(number) = flags_value.as_i64() {
        return if number >= 0 { number as u64 } else { 0 };
    }
    flags_value
        .as_str()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn merge_identity_update_patch(current: &Value, patch: &Value) -> Value {
    match (current, patch) {
        (Value::Object(current_obj), Value::Object(patch_obj)) => {
            let mut merged = current_obj.clone();
            for (key, patch_value) in patch_obj {
                let next_value = merged
                    .get(key)
                    .map(|current_value| merge_identity_update_patch(current_value, patch_value))
                    .unwrap_or_else(|| patch_value.clone());
                merged.insert(key.clone(), next_value);
            }
            Value::Object(merged)
        }
        _ => patch.clone(),
    }
}

fn validate_unsupported_identity_flags(before: &Value, after: &Value) -> Result<(), WalletError> {
    const IDENTITY_FLAG_ACTIVE_CURRENCY: u64 = 0x1;
    const IDENTITY_FLAG_TOKENIZED_CONTROL: u64 = 0x4;

    let before_flags = extract_flags(before);
    let after_flags = extract_flags(after);
    let before_active = (before_flags & IDENTITY_FLAG_ACTIVE_CURRENCY) != 0;
    let after_active = (after_flags & IDENTITY_FLAG_ACTIVE_CURRENCY) != 0;
    if before_active != after_active {
        return Err(WalletError::IdentityUnsupportedActiveCurrencyChange);
    }

    let before_tokenized = (before_flags & IDENTITY_FLAG_TOKENIZED_CONTROL) != 0;
    let after_tokenized = (after_flags & IDENTITY_FLAG_TOKENIZED_CONTROL) != 0;
    if before_tokenized != after_tokenized {
        return Err(WalletError::IdentityUnsupportedTokenizedControlChange);
    }

    Ok(())
}

fn build_primary_address_update_info(
    requested_identity_json: &Value,
    active_wallet_primary_address: &str,
) -> GenericIdentityPrimaryAddressInfo {
    let addresses = requested_identity_json
        .get("primaryaddresses")
        .or_else(|| requested_identity_json.get("primaryAddresses"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|address| GenericIdentityPrimaryAddressEntry {
                    address: address.to_string(),
                    in_wallet: address.eq_ignore_ascii_case(active_wallet_primary_address),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let wallet_count = addresses.iter().filter(|entry| entry.in_wallet).count();
    let external_count = addresses.len().saturating_sub(wallet_count);

    GenericIdentityPrimaryAddressInfo {
        addresses,
        wallet_count,
        external_count,
    }
}

async fn resolve_identity_friendly_names(
    provider: &crate::core::channels::vrpc::VrpcProvider,
    current_identity: &Value,
    requested_identity: &Value,
    signer_identity_id: Option<&str>,
) -> HashMap<String, String> {
    let mut names = HashMap::<String, String>::new();

    let mut candidate_ids = [
        first_non_empty_field(
            current_identity,
            &["identityaddress", "identityAddress", "iaddress"],
        ),
        first_non_empty_field(
            current_identity,
            &["revocationauthority", "revocationAuthority"],
        ),
        first_non_empty_field(
            current_identity,
            &["recoveryauthority", "recoveryAuthority"],
        ),
        first_non_empty_field(
            requested_identity,
            &["revocationauthority", "revocationAuthority"],
        ),
        first_non_empty_field(
            requested_identity,
            &["recoveryauthority", "recoveryAuthority"],
        ),
        signer_identity_id.map(|value| value.trim().to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    candidate_ids.extend(
        current_identity
            .get("primaryaddresses")
            .or_else(|| current_identity.get("primaryAddresses"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    );

    candidate_ids.extend(
        requested_identity
            .get("primaryaddresses")
            .or_else(|| requested_identity.get("primaryAddresses"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    );

    for identity_id in candidate_ids {
        if names.contains_key(&identity_id) || identity_id.trim().is_empty() {
            continue;
        }

        let resolved = provider.getidentity(&identity_id).await;
        let Ok(raw_identity) = resolved else {
            continue;
        };

        if let Some(friendly_name) = first_non_empty_field(
            &raw_identity,
            &[
                "fullyqualifiedname",
                "fullyQualifiedName",
                "friendlyname",
                "friendlyName",
            ],
        ) {
            names.insert(identity_id, friendly_name);
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::{
        build_and_sign_generic_response_internal, build_generic_response_redirect_url,
        build_unsigned_generic_response_hex, decode_defined_key_label, merge_identity_update_patch,
        parse_generic_response_post_callback_uri, parse_provisioning_challenge_hex,
        post_generic_response_callback, resolve_signer_cmm_key_labels_from_identity,
        validate_generic_request_funding_source, wallet_network_to_crypto_network,
        BuildAndSignGenericResponseRequest, WalletNetwork, DATA_TYPE_DEFINEDKEY_VDXF_ID,
        GENERIC_RESPONSE_DEEPLINK_VDXF_ID, GENERIC_RESPONSE_FLAG_HAS_CREATED_AT,
        GENERIC_RESPONSE_FLAG_IS_TESTNET, GENERIC_RESPONSE_FLAG_MULTI_DETAILS,
        GENERIC_RESPONSE_FLAG_SIGNED, HASH_TYPE_SHA256, MAINNET_VERUS_CHAIN_ID,
        VDXF_ORDINAL_AUTHENTICATION_REQUEST, VDXF_ORDINAL_AUTHENTICATION_RESPONSE,
        VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST, VDXF_ORDINAL_IDENTITY_UPDATE_RESPONSE,
        VDXF_ORDINAL_PROVISION_IDENTITY, VERIFIABLE_SIGNATURE_VERSION_V2,
    };
    use crate::core::crypto::verus_id_signature::{
        compute_identity_signature_hash, encode_compact_i_address, get_raw_envelope_sha256,
        parse_generic_envelope_hex, parse_identity_signature,
        verify_identity_signature_against_addresses, write_compact_size, write_var_slice,
        write_varint,
    };
    use crate::core::crypto::wif_encoding::generate_p2pkh_address;
    use crate::types::{
        GenericAuthenticationResponseInput, GenericIdentityUpdateResponseInput,
        GenericResponseSignerInput, LinkedIdentity, WalletError,
    };
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const TEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
    const TEST_SIGNER_ID: &str = "i89UVSuN6vfWg1mWpXMuc6dsJBdeYTi7bX";
    const TEST_AUTH_REQUEST_ID: &str = "iBxsUePVWyf4QuM1b8wP7Np1SUJhszwyin";
    const FIXED_CREATED_AT: u64 = 1_700_000_000;
    const FIXED_SIGNED_BLOCK_HEIGHT: u32 = 965_771;

    #[test]
    fn provisioning_challenge_hash_matches_typescript_primitives() {
        let challenge = parse_provisioning_challenge_hex(
            "bf5b8b3997fe0381a6383fddc8f055c1f0dd67740165141af5b8015c64d39ab44c60ead8317f9f5a9b6c4c00f153650000000000567b9a87b17131f6eeb2dd0bdd191ece4a5d603b01010005616c696365141af5b8015c64d39ab44c60ead8317f9f5a9b6c4c14a6ef9ea235635e328124ff3429db9f9e91b64e2d",
        )
        .expect("challenge parses");

        assert_eq!(challenge.challenge_id, MAINNET_VERUS_CHAIN_ID);
        assert_eq!(challenge.created_at, FIXED_CREATED_AT);
        assert_eq!(challenge.name, "alice");
        assert_eq!(challenge.system_id.as_deref(), Some(MAINNET_VERUS_CHAIN_ID));
        assert_eq!(challenge.parent_id.as_deref(), Some(TEST_SYSTEM_ID));
        assert_eq!(
            hex::encode(challenge.hash),
            "35d3bef3d19528f98c4edeebba920d50438ca346d38257aa1c91b53805bd5c31"
        );
    }

    fn build_test_request_detail(ordinal: u64, payload: &[u8]) -> Vec<u8> {
        let mut detail = Vec::new();
        write_compact_size(&mut detail, ordinal as usize);
        write_varint(&mut detail, 1);
        write_var_slice(&mut detail, payload);
        detail
    }

    fn build_test_request_hex(detail_ordinals: &[u64], testnet: bool) -> String {
        let mut flags = GENERIC_RESPONSE_FLAG_SIGNED | GENERIC_RESPONSE_FLAG_HAS_CREATED_AT;
        if testnet {
            flags |= GENERIC_RESPONSE_FLAG_IS_TESTNET;
        }
        if detail_ordinals.len() > 1 {
            flags |= GENERIC_RESPONSE_FLAG_MULTI_DETAILS;
        }

        let mut request = Vec::new();
        write_compact_size(&mut request, 1);
        write_compact_size(&mut request, flags as usize);
        write_varint(&mut request, 0);
        write_compact_size(&mut request, 0);
        write_compact_size(&mut request, VERIFIABLE_SIGNATURE_VERSION_V2 as usize);
        write_compact_size(&mut request, HASH_TYPE_SHA256 as usize);
        request.extend_from_slice(&encode_compact_i_address(TEST_SYSTEM_ID).expect("system id"));
        request.extend_from_slice(&encode_compact_i_address(TEST_SIGNER_ID).expect("signer id"));
        write_var_slice(&mut request, &[]);
        write_compact_size(&mut request, FIXED_CREATED_AT as usize);

        if detail_ordinals.len() > 1 {
            write_compact_size(&mut request, detail_ordinals.len());
        }
        for ordinal in detail_ordinals {
            request.extend_from_slice(&build_test_request_detail(*ordinal, &[*ordinal as u8]));
        }

        hex::encode(request)
    }

    fn test_signing_context(network: crate::core::crypto::Network, seed: u8) -> ([u8; 32], String) {
        let private_key = [seed; 32];
        let secret_key = SecretKey::from_slice(&private_key).expect("secret key");
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let address = generate_p2pkh_address(&public_key, network).expect("address");
        (private_key, address)
    }

    fn test_request(
        request_hex: String,
        authentication: Option<GenericAuthenticationResponseInput>,
        identity_update: Option<GenericIdentityUpdateResponseInput>,
    ) -> BuildAndSignGenericResponseRequest {
        BuildAndSignGenericResponseRequest {
            request_hex,
            signer: GenericResponseSignerInput {
                system_id: TEST_SYSTEM_ID.to_string(),
                identity_id: TEST_SIGNER_ID.to_string(),
            },
            authentication,
            identity_update,
        }
    }

    fn linked_identity(identity_address: &str) -> LinkedIdentity {
        LinkedIdentity {
            identity_address: identity_address.to_string(),
            name: None,
            fully_qualified_name: None,
            status: None,
            system_id: None,
            favorite: false,
        }
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut headers_complete = false;
        let mut expected_len: Option<usize> = None;

        loop {
            let mut chunk = [0u8; 1024];
            let read = stream.read(&mut chunk).await.expect("read request");
            if read == 0 {
                break;
            }

            request.extend_from_slice(&chunk[..read]);

            if !headers_complete {
                if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    headers_complete = true;
                    let header_text = String::from_utf8_lossy(&request[..index + 4]).to_lowercase();
                    let content_length = header_text
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .expect("content-length header");
                    expected_len = Some(index + 4 + content_length);
                }
            }

            if headers_complete && request.len() >= expected_len.expect("expected request length") {
                break;
            }
        }

        request
    }

    #[test]
    fn auth_only_request_builds_and_signs_a_valid_response() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, active_wallet_primary_address) = test_signing_context(network, 7);
        let request = test_request(
            build_test_request_hex(&[VDXF_ORDINAL_AUTHENTICATION_REQUEST], true),
            Some(GenericAuthenticationResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
            }),
            None,
        );

        let signed_response_hex = build_and_sign_generic_response_internal(
            &request,
            network,
            &active_wallet_primary_address,
            std::slice::from_ref(&active_wallet_primary_address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
            FIXED_CREATED_AT,
        )
        .expect("auth-only response");

        let parsed = parse_generic_envelope_hex(&signed_response_hex).expect("parsed response");
        assert_eq!(parsed.created_at, Some(FIXED_CREATED_AT));
        assert!(parsed.is_testnet);
        assert_eq!(
            parsed
                .details
                .iter()
                .map(|detail| detail.ordinal)
                .collect::<Vec<_>>(),
            vec![VDXF_ORDINAL_AUTHENTICATION_RESPONSE]
        );

        let parsed_identity_signature =
            parse_identity_signature(&parsed.signature_data.signature_as_vch)
                .expect("identity signature");
        let verify_hash = compute_identity_signature_hash(
            &parsed.signature_data,
            parsed_identity_signature.block_height,
            get_raw_envelope_sha256(&parsed),
        )
        .expect("verify hash");
        assert!(verify_identity_signature_against_addresses(
            verify_hash,
            &parsed_identity_signature,
            std::slice::from_ref(&active_wallet_primary_address),
            network,
        ));
    }

    #[test]
    fn auth_and_update_request_builds_response_details_in_stable_order() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, active_wallet_primary_address) = test_signing_context(network, 7);
        let request = test_request(
            build_test_request_hex(
                &[
                    VDXF_ORDINAL_AUTHENTICATION_REQUEST,
                    VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST,
                ],
                true,
            ),
            Some(GenericAuthenticationResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
            }),
            Some(GenericIdentityUpdateResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
                txid: "11".repeat(32),
            }),
        );

        let signed_response_hex = build_and_sign_generic_response_internal(
            &request,
            network,
            &active_wallet_primary_address,
            std::slice::from_ref(&active_wallet_primary_address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
            FIXED_CREATED_AT,
        )
        .expect("auth+update response");

        let parsed = parse_generic_envelope_hex(&signed_response_hex).expect("parsed response");
        assert_eq!(
            parsed
                .details
                .iter()
                .map(|detail| detail.ordinal)
                .collect::<Vec<_>>(),
            vec![
                VDXF_ORDINAL_AUTHENTICATION_RESPONSE,
                VDXF_ORDINAL_IDENTITY_UPDATE_RESPONSE,
            ]
        );
    }

    #[test]
    fn update_only_request_signs_with_the_explicit_signer() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, active_wallet_primary_address) = test_signing_context(network, 7);
        let request = test_request(
            build_test_request_hex(&[VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST], true),
            None,
            Some(GenericIdentityUpdateResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
                txid: "22".repeat(32),
            }),
        );

        let signed_response_hex = build_and_sign_generic_response_internal(
            &request,
            network,
            &active_wallet_primary_address,
            std::slice::from_ref(&active_wallet_primary_address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
            FIXED_CREATED_AT,
        )
        .expect("update-only response");

        let parsed = parse_generic_envelope_hex(&signed_response_hex).expect("parsed response");
        assert_eq!(parsed.signature_data.signer_system_id, TEST_SYSTEM_ID);
        assert_eq!(parsed.signature_data.signer_identity_id, TEST_SIGNER_ID);
        assert_eq!(
            parsed
                .details
                .iter()
                .map(|detail| detail.ordinal)
                .collect::<Vec<_>>(),
            vec![VDXF_ORDINAL_IDENTITY_UPDATE_RESPONSE]
        );
    }

    #[test]
    fn malformed_txid_is_rejected() {
        let request = test_request(
            build_test_request_hex(&[VDXF_ORDINAL_IDENTITY_UPDATE_REQUEST], true),
            None,
            Some(GenericIdentityUpdateResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
                txid: "1234".to_string(),
            }),
        );

        let error = build_unsigned_generic_response_hex(&request, FIXED_CREATED_AT)
            .expect_err("txid should fail");
        assert!(matches!(error, WalletError::OperationFailed));
    }

    #[test]
    fn unsupported_grouping_is_rejected() {
        let request = test_request(
            build_test_request_hex(
                &[
                    VDXF_ORDINAL_AUTHENTICATION_REQUEST,
                    VDXF_ORDINAL_AUTHENTICATION_REQUEST,
                ],
                true,
            ),
            Some(GenericAuthenticationResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
            }),
            None,
        );

        let error = build_unsigned_generic_response_hex(&request, FIXED_CREATED_AT)
            .expect_err("grouping should fail");
        assert!(matches!(error, WalletError::GenericRequestInvalidEnvelope));
    }

    #[test]
    fn signer_not_controlled_by_the_active_wallet_is_rejected() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, active_wallet_primary_address) = test_signing_context(network, 7);
        let (_, foreign_address) = test_signing_context(network, 8);
        let request = test_request(
            build_test_request_hex(&[VDXF_ORDINAL_AUTHENTICATION_REQUEST], true),
            Some(GenericAuthenticationResponseInput {
                request_id: Some(TEST_AUTH_REQUEST_ID.to_string()),
            }),
            None,
        );

        let error = build_and_sign_generic_response_internal(
            &request,
            network,
            &active_wallet_primary_address,
            std::slice::from_ref(&foreign_address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
            FIXED_CREATED_AT,
        )
        .expect_err("control check should fail");
        assert!(matches!(error, WalletError::IdentityUnsupportedAuthority));
    }

    #[test]
    fn provisioning_without_auth_request_is_rejected() {
        let request = test_request(
            build_test_request_hex(&[VDXF_ORDINAL_PROVISION_IDENTITY], true),
            None,
            None,
        );

        let error = build_unsigned_generic_response_hex(&request, FIXED_CREATED_AT)
            .expect_err("grouping should fail");
        assert!(matches!(error, WalletError::GenericRequestInvalidEnvelope));
    }

    #[test]
    fn funding_source_validation_allows_primary_address() {
        let result = validate_generic_request_funding_source(
            "RPrimary",
            TEST_SYSTEM_ID,
            Some(TEST_SYSTEM_ID),
            "RPrimary",
            &[],
            &[],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn funding_source_validation_allows_linked_identity_scope() {
        let result = validate_generic_request_funding_source(
            "iLinkedIdentity",
            TEST_SYSTEM_ID,
            Some(TEST_SYSTEM_ID),
            "RPrimary",
            &[linked_identity("iLinkedIdentity")],
            &[],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn funding_source_validation_rejects_watched_scope() {
        let result = validate_generic_request_funding_source(
            "RWatchedOnly",
            TEST_SYSTEM_ID,
            Some(TEST_SYSTEM_ID),
            "RPrimary",
            &[],
            &["RWatchedOnly".to_string()],
        );

        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn funding_source_validation_rejects_wrong_request_system() {
        let result = validate_generic_request_funding_source(
            "RPrimary",
            "iDifferentSystem",
            Some(TEST_SYSTEM_ID),
            "RPrimary",
            &[],
            &[],
        );

        assert!(matches!(result, Err(WalletError::InvalidAddress)));
    }

    #[test]
    fn callback_uri_rejects_non_http_schemes() {
        let error = parse_generic_response_post_callback_uri("ftp://example.com/callback")
            .expect_err("non-http scheme should fail");
        assert!(matches!(error, WalletError::OperationFailed));
    }

    #[test]
    fn generic_response_redirect_url_allows_custom_schemes() {
        let redirect_url =
            build_generic_response_redirect_url("valu://callback/finish", "deadbeef")
                .expect("redirect url");

        assert_eq!(redirect_url.scheme(), "valu");
        assert_eq!(redirect_url.host_str(), Some("callback"));
        assert_eq!(redirect_url.path(), "/finish");
        assert_eq!(
            redirect_url
                .query_pairs()
                .find(|(key, _)| key == GENERIC_RESPONSE_DEEPLINK_VDXF_ID)
                .map(|(_, value)| value.into_owned()),
            Some("3q2-7w".to_string())
        );
    }

    #[test]
    fn decode_defined_key_label_extracts_key_id_and_label() {
        let (key_id, namespace_id, label) = decode_defined_key_label(
            "0100216578616d706c652e6e616d6573706163653a3a70726f66696c652e617661746172",
            MAINNET_VERUS_CHAIN_ID,
        )
        .expect("defined key label");

        assert_eq!(key_id, "i5ZsXhytTtBe6f6DeV1eQRVLUgBBXwVi6h");
        assert_eq!(namespace_id, "iKdh7UtrXGLXedeA2Dub8AsPZcSZchx47T");
        assert_eq!(label, "Profile avatar");
    }

    #[test]
    fn resolve_signer_cmm_key_labels_reads_defined_keys_from_identity_content() {
        let signer_identity = serde_json::json!({
            "identity": {
                "contentmultimap": {
                    DATA_TYPE_DEFINEDKEY_VDXF_ID: [
                        "0100216578616d706c652e6e616d6573706163653a3a70726f66696c652e617661746172"
                    ]
                }
            }
        });

        let labels =
            resolve_signer_cmm_key_labels_from_identity(&signer_identity, WalletNetwork::Mainnet);

        assert_eq!(
            labels.get("i5ZsXhytTtBe6f6DeV1eQRVLUgBBXwVi6h"),
            Some(&"Profile avatar".to_string())
        );
    }

    #[test]
    fn merge_identity_update_patch_preserves_omitted_primary_addresses() {
        let current = serde_json::json!({
            "name": "before",
            "primaryaddresses": ["RFWcHcpnFh57ovrV1hmzN8o8wmMZjvqCSh"],
            "contentmultimap": {
                "existing": {
                    "data": {
                        "message": "before"
                    }
                }
            }
        });
        let patch = serde_json::json!({
            "name": "maxs",
            "contentmultimap": {
                "new_key": {
                    "data": {
                        "message": "after"
                    }
                }
            }
        });

        let merged = merge_identity_update_patch(&current, &patch);

        assert_eq!(merged["name"], serde_json::json!("maxs"));
        assert_eq!(
            merged["primaryaddresses"],
            serde_json::json!(["RFWcHcpnFh57ovrV1hmzN8o8wmMZjvqCSh"])
        );
        assert!(merged["contentmultimap"].get("existing").is_some());
        assert!(merged["contentmultimap"].get("new_key").is_some());
    }

    #[tokio::test]
    async fn post_generic_response_callback_sends_binary_post_body() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");
        let expected_hex = "deadbeef".to_string();
        let expected_body = hex::decode(&expected_hex).expect("expected body");

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let request = read_http_request(&mut stream).await;
            let request_text = String::from_utf8_lossy(&request).to_lowercase();

            assert!(request_text.starts_with("post /callback http/1.1"));
            assert!(request_text.contains("content-type: application/octet-stream"));
            assert!(request.ends_with(&expected_body));

            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await
                .expect("write response");
        });

        post_generic_response_callback(format!("http://{}/callback", address), expected_hex)
            .await
            .expect("callback post succeeds");

        server.await.expect("server completes");
    }

    #[tokio::test]
    async fn post_generic_response_callback_rejects_non_success_status() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let _request = read_http_request(&mut stream).await;
            stream
                .write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n")
                .await
                .expect("write response");
        });

        let error = post_generic_response_callback(
            format!("http://{}/callback", address),
            "deadbeef".to_string(),
        )
        .await
        .expect_err("callback status should fail");
        assert!(matches!(error, WalletError::OperationFailed));

        server.await.expect("server completes");
    }

    #[test]
    fn generic_response_redirect_url_appends_base64url_response_param() {
        let redirect_url = build_generic_response_redirect_url("https://www.verus.io", "deadbeef")
            .expect("redirect url");

        assert_eq!(redirect_url.scheme(), "https");
        assert_eq!(redirect_url.host_str(), Some("www.verus.io"));
        assert_eq!(
            redirect_url
                .query_pairs()
                .find(|(key, _)| key == "i9JzVt59mAVHqjc8WAQJx7bEFAQ4ffuhrC")
                .map(|(_, value)| value.into_owned()),
            Some("3q2-7w".to_string())
        );
    }
}
