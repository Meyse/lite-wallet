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
    IdentitySessionContext,
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
    compute_identity_signature_hash, decode_compact_address_at, extract_chain_height,
    extract_primary_addresses, get_raw_envelope_sha256, parse_generic_envelope_hex,
    parse_identity_signature, sign_hash_compact, sign_identity_hash_with_private_bytes,
    validate_identity_control_for_active_wallet, verify_generic_request_signature_with_provider,
    verify_identity_hash_with_provider, verify_identity_signature_against_addresses,
};
use crate::core::crypto::Network;
use crate::core::generic_request::details::{self, DetailKind};
use crate::core::generic_request::request::{
    parse_request_hex, ParsedRequest, RequestDetail, ResponseUri, ResponseUriKind, FLAG_IS_TESTNET,
};
use crate::core::generic_request::store::{GenericRequestStore, OpenRequest};
use crate::core::generic_request::{acceptance, credentials, legacy, names, response};
use crate::core::{AccountStateStore, PreflightStore};
use crate::types::wallet::WalletNetwork;
use crate::types::{
    BuildAndSignGenericResponseResult, GenericCredentialReview, GenericIdentityAuthorities,
    GenericIdentityPrimaryAddressEntry, GenericIdentityPrimaryAddressInfo,
    GenericIdentityUpdatePreflightResult, GenericIdentityUpdateRequestMeta,
    GenericIdentityUpdateReviewResult, GenericInvoicePreflightResult, GenericInvoiceReview,
    GenericRequestTransactionResult, GenericSharedCredential, IdentityOperation, IdentityWarning,
    LinkReadyProvisioningJobResult, LinkedIdentity, OpenedGenericRequest, ProvisioningJobRecord,
    WalletError,
};
use uuid::Uuid;

const PROVISIONING_JOB_REQUEST_TYPE_GENERIC: &str = "generic";
const PROVISIONING_JOB_STATUS_PENDING: &str = "pending";
const PROVISIONING_JOB_STATUS_READY: &str = "ready";
const PROVISIONING_JOB_STATUS_LINKED: &str = "linked";
const PROVISIONING_JOB_STATUS_EXPIRED: &str = "expired";
const PROVISIONING_JOB_TTL_SECS: u64 = 7 * 24 * 60 * 60;
const VDXF_ORDINAL_AUTHENTICATION_REQUEST: u64 = 2;
const VDXF_ORDINAL_PROVISION_IDENTITY: u64 = 6;
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

pub(crate) fn wallet_network_to_crypto_network(network: WalletNetwork) -> Network {
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

/// POST delivery uses HTTPS. Debug builds also allow plain HTTP, for local
/// test receivers such as Deep Link Sim.
fn parse_generic_response_post_callback_uri(uri: &str) -> Result<reqwest::Url, WalletError> {
    let parsed = reqwest::Url::parse(uri.trim()).map_err(|_| WalletError::OperationFailed)?;
    match parsed.scheme() {
        "https" => Ok(parsed),
        "http" if cfg!(debug_assertions) => Ok(parsed),
        "http" => Err(WalletError::GenericRequestRejected(
            "insecureDelivery".to_string(),
        )),
        _ => Err(WalletError::OperationFailed),
    }
}

/// A response POST and a provisioning call give up after 30 seconds.
const DELIVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

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
    parameter: &str,
) -> Result<reqwest::Url, WalletError> {
    let mut redirect_url = parse_generic_response_redirect_uri(callback_uri)?;
    let response_bytes =
        hex::decode(response_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
    let encoded_response = URL_SAFE_NO_PAD.encode(response_bytes);
    redirect_url
        .query_pairs_mut()
        .append_pair(parameter, &encoded_response);
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
    /// The offered VerusID. Without one, the user chooses the name, as in
    /// Verus Mobile's provisioning form.
    identity_id: Option<String>,
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
    let identity_id = read_address(flags & PROVISION_FLAG_HAS_IDENTITY_ID != 0)?;
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

/// A name the user chose for a new VerusID: one label, without the
/// characters a VerusID name cannot hold, and at most 64 bytes.
fn is_valid_chosen_name(name: &str) -> bool {
    const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '@', '.'];
    !name.is_empty()
        && name.len() <= 64
        && name.trim() == name
        && !name
            .chars()
            .any(|character| character.is_control() || FORBIDDEN.contains(&character))
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

/// Signs an unsigned response for `signed_block_height`, then verifies the
/// result against the signer's primary addresses before it leaves Rust.
fn sign_unsigned_response(
    unsigned_response: &[u8],
    network: Network,
    active_wallet_primary_address: &str,
    allowed_primary_addresses: &[String],
    signed_block_height: u32,
    private_key: &[u8; 32],
) -> Result<String, WalletError> {
    ensure_active_wallet_controls_signer(allowed_primary_addresses, active_wallet_primary_address)?;

    let parsed_response = parse_generic_envelope_hex(&hex::encode(unsigned_response))?;
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

/// `LoginConsentResponse` for a legacy login (`LoginRequestIdentity.js`,
/// `signLoginConsentResponse`): the decision echoes the challenge and embeds
/// the request, and the responder signs its version 2 hash. The signature is
/// verified against the responder before it leaves Rust.
fn sign_legacy_login_response(
    login: &legacy::LoginConsentRequest,
    signer: &crate::types::GenericResponseSignerInput,
    network: Network,
    active_wallet_primary_address: &str,
    allowed_primary_addresses: &[String],
    signed_block_height: u32,
    private_key: &[u8; 32],
) -> Result<String, WalletError> {
    ensure_active_wallet_controls_signer(allowed_primary_addresses, active_wallet_primary_address)?;
    if !signer
        .system_id
        .eq_ignore_ascii_case(&names::i_address(&login.system_id))
    {
        return Err(WalletError::GenericRequestRejected("responder".to_string()));
    }
    let responder =
        names::decode_i_address(&signer.identity_id).ok_or(WalletError::InvalidAddress)?;
    let decision = login.decision(now_unix_seconds())?;
    let hash = legacy::decision_hash(login.system_id, responder, signed_block_height, &decision);
    let signature = sign_identity_hash_with_private_bytes(hash, signed_block_height, private_key)?;
    let parsed_signature = parse_identity_signature(&signature)?;
    if !verify_identity_signature_against_addresses(
        hash,
        &parsed_signature,
        allowed_primary_addresses,
        network,
    ) {
        return Err(WalletError::OperationFailed);
    }
    Ok(hex::encode(legacy::login_response(
        login.system_id,
        responder,
        &signature,
        &decision,
    )?))
}

/// The wallet's private account key, which encryption channels derive from.
pub(crate) async fn private_account_key(
    context: &crate::core::auth::session::ActiveWalletAccessContext,
) -> Result<sapling::zip32::ExtendedSpendingKey, WalletError> {
    let (encoded, _) = context
        .stronghold_store
        .load_dlight_runtime_material(
            &context.account_id,
            context.password_hash(),
            context.wallet_network,
        )
        .await?
        .ok_or_else(|| WalletError::from(acceptance::Rejection::PrivateNotSetUp))?;
    crate::core::channels::dlight_private::decode_runtime_spending_key(&encoded)
}

/// The share data detail of the open request.
fn open_user_data(open: &OpenRequest) -> Result<Option<details::UserDataRequest>, WalletError> {
    open.parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::UserData)
        .map(|(detail, _)| details::parse_user_data(&detail.data, open.parsed.is_testnet))
        .transpose()
}

/// The credentials the response shares: those the user reviewed for this
/// responder. The request's required identity, when it names one, must answer.
fn shared_credentials(
    open: &OpenRequest,
    responder: &str,
) -> Result<Vec<credentials::Credential>, WalletError> {
    let Some(user_data) = open_user_data(open)? else {
        return Ok(Vec::new());
    };
    if user_data
        .signer
        .as_deref()
        .is_some_and(|required| !required.eq_ignore_ascii_case(responder))
    {
        return Err(WalletError::GenericRequestRejected("responder".to_string()));
    }
    match &open.reviewed_credentials {
        Some((identity, found)) if identity.eq_ignore_ascii_case(responder) => Ok(found.clone()),
        _ => Err(WalletError::OperationFailed),
    }
}

/// `processEncryptedKeys`: the credentials an update writes are encrypted to
/// the identity's own credential address before the update leaves Rust, and
/// stored under the hashed credential key. The identity's private address
/// must come from this wallet's seed, or the wallet could not read them back.
async fn encrypt_update_credentials(
    update: &details::IdentityUpdateRequest,
    target_identity_address: &str,
    current_identity: &Value,
    requested_identity: &mut Value,
    context: &crate::core::auth::session::ActiveWalletAccessContext,
) -> Result<(), WalletError> {
    let values = update
        .identity
        .content_multimap
        .iter()
        .flatten()
        .filter(|(key, _)| {
            key == credentials::IDENTITY_CREDENTIAL_KEY
                || key.eq_ignore_ascii_case("vrsc::identity.credential")
        })
        .flat_map(|(_, values)| values.iter().cloned())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Ok(());
    }
    let content = requested_identity
        .get_mut("contentmultimap")
        .and_then(Value::as_object_mut)
        .ok_or(WalletError::IdentityBuildFailed)?;
    content
        .remove(credentials::IDENTITY_CREDENTIAL_KEY)
        .ok_or(WalletError::IdentityBuildFailed)?;

    let account = private_account_key(context).await?;
    let (_, wallet_address) = account
        .to_diversifiable_full_viewing_key()
        .default_address();
    let identity_address = current_identity
        .get("privateaddress")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if names::sapling_address(&wallet_address.to_bytes()).as_deref() != Some(identity_address) {
        return Err(acceptance::Rejection::CredentialAddress.into());
    }

    let identity =
        names::decode_i_address(target_identity_address).ok_or(WalletError::InvalidAddress)?;
    let keys = crate::core::generic_request::private_crypto::channel_keys(
        &account,
        Some(&identity),
        Some(&identity),
        0,
        false,
    )?;
    let encrypted = credentials::encrypt_for_storage(&values, &keys.address.to_bytes())?;
    content.insert(
        credentials::storage_key(&keys.incoming_viewing_key)?,
        Value::Array(encrypted),
    );
    Ok(())
}

/// The keys an encryption key request asks for (`appEncryptionRequestHandler`):
/// the channel from the responder to the derivation identity, else the app
/// identity, else the requester, at the request's derivation number, under the
/// wallet's private account key.
async fn app_encryption_keys(
    open: &OpenRequest,
    signer: &response::ResponseSigner<'_>,
    context: &crate::core::auth::session::ActiveWalletAccessContext,
) -> Result<Option<response::AppEncryptionKeys>, WalletError> {
    let Some(detail) = open
        .parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::AppEncryption)
        .map(|(detail, _)| detail)
    else {
        return Ok(None);
    };
    let app = details::parse_app_encryption(&detail.data, open.parsed.is_testnet)?;
    let counterparty = app
        .derivation_id
        .clone()
        .or_else(|| open.parsed.app_or_delegated_id.clone())
        .or_else(|| {
            open.parsed
                .signature
                .as_ref()
                .map(|signature| signature.signer_identity_id.clone())
        })
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let from = names::decode_i_address(signer.identity_id).ok_or(WalletError::InvalidAddress)?;
    let to = names::decode_i_address(&counterparty).ok_or(WalletError::InvalidAddress)?;
    let index = u32::try_from(app.derivation_number)
        .map_err(|_| WalletError::GenericRequestInvalidEnvelope)?;

    let account = private_account_key(context).await?;
    let keys = crate::core::generic_request::private_crypto::channel_keys(
        &account,
        Some(&from),
        Some(&to),
        index,
        app.return_spending_key,
    )?;
    Ok(Some(response::AppEncryptionKeys {
        incoming_viewing_key: keys.incoming_viewing_key,
        extended_viewing_key: keys.extended_viewing_key,
        address: keys.address.to_bytes(),
        extended_spending_key: keys.extended_spending_key,
    }))
}

/// Signs every object and statement of the open request's data packets, each
/// over its own height-bound identity hash (`signDataPacket.js`). Rust builds
/// the hashes from the verified request; it never signs a hash from the UI.
fn sign_data_packets(
    open: &OpenRequest,
    signer: &response::ResponseSigner<'_>,
    signed_block_height: u32,
    private_key: &[u8; 32],
) -> Result<Vec<Vec<response::ItemSignature>>, WalletError> {
    let system = names::decode_i_address(signer.system_id).ok_or(WalletError::InvalidAddress)?;
    let identity =
        names::decode_i_address(signer.identity_id).ok_or(WalletError::InvalidAddress)?;
    response::data_packet_hashes(&open.parsed, &open.kinds)?
        .into_iter()
        .map(|hashes| {
            hashes
                .into_iter()
                .map(|hash| {
                    let identity_hash =
                        crate::core::crypto::verus_id_signature::compute_verus_data_identity_hash(
                            system,
                            identity,
                            signed_block_height,
                            hash,
                        );
                    Ok(response::ItemSignature {
                        hash,
                        signature: sign_identity_hash_with_private_bytes(
                            identity_hash,
                            signed_block_height,
                            private_key,
                        )?,
                    })
                })
                .collect()
        })
        .collect()
}

/// The responder must meet the sign-in rules: a required VerusID, system,
/// or parent. Verus Mobile applies them in its identity picker.
fn check_responder_constraints(
    open: &OpenRequest,
    signer: &crate::types::GenericResponseSignerInput,
    signer_identity: &Value,
) -> Result<(), WalletError> {
    for (detail, kind) in open.parsed.details.iter().zip(&open.kinds) {
        if *kind != DetailKind::Authentication {
            continue;
        }
        let auth = details::parse_authentication(&detail.data, open.parsed.is_testnet)?;
        if auth
            .expiry_time
            .is_some_and(|expiry| expiry <= now_unix_seconds())
        {
            return Err(acceptance::Rejection::Expired.into());
        }
        let parent = signer_identity
            .get("identity")
            .and_then(|identity| identity.get("parent"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        for constraint in &auth.constraints {
            let meets = match constraint.kind {
                1 => constraint
                    .identity
                    .eq_ignore_ascii_case(&signer.identity_id),
                2 => constraint.identity.eq_ignore_ascii_case(&signer.system_id),
                3 => constraint.identity.eq_ignore_ascii_case(parent),
                _ => false,
            };
            if !meets {
                return Err(WalletError::GenericRequestRejected("responder".to_string()));
            }
        }
    }
    Ok(())
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

/// Opens a request: Rust checks the detail order, the unsigned rule, the
/// network, the signature, expiry, and the app identity rule, then holds the
/// request as the one open request for this session.
#[tauri::command(rename_all = "snake_case")]
pub async fn open_generic_request(
    request_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<OpenedGenericRequest, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let network = context.wallet_network;
    let wallet_is_testnet = matches!(network, WalletNetwork::Testnet);

    if legacy::is_legacy_link(&request_hex) {
        return open_legacy_request(
            request_hex.trim(),
            &context,
            session_manager.inner(),
            vrpc_provider_pool.inner(),
            request_store.inner(),
        )
        .await;
    }

    let parsed = parse_request_hex(&request_hex)?;
    let kinds = acceptance::check_request(&parsed, wallet_is_testnet)?;
    let guard = context.session_submission_guard();
    let root_provider =
        vrpc_provider_pool.for_system(network, names::root_system_id(parsed.is_testnet));

    check_request_expiry(&parsed, &kinds, root_provider, &guard).await?;

    let mut signature_block_height = None;
    if let Some(signature) = parsed.signature.as_ref() {
        let envelope = parse_generic_envelope_hex(&request_hex)?;
        let identity_signature = parse_identity_signature(&signature.signature_as_vch)?;
        let provider = vrpc_provider_pool.for_system(network, &signature.signer_system_id);
        let valid = guard
            .run(verify_generic_request_signature_with_provider(
                provider,
                &envelope,
                wallet_network_to_crypto_network(network),
            ))
            .await
            .map_err(map_generic_session_change)?;
        if !valid {
            return Err(acceptance::Rejection::Signature.into());
        }
        if acceptance::names_delegated_app(&parsed) {
            guard
                .run(validate_identity_control_for_active_wallet(
                    provider,
                    &signature.signer_identity_id,
                    &context.vrsc_address,
                ))
                .await
                .map_err(|error| match error {
                    WalletError::IdentityUnsupportedAuthority => {
                        acceptance::Rejection::DelegatedSigner.into()
                    }
                    other => map_generic_session_change(other),
                })?;
        }
        signature_block_height = Some(identity_signature.block_height);
    }

    ensure_active_wallet_session(session_manager.inner(), &context.session_id)
        .await
        .map_err(map_generic_session_change)?;
    let opened = OpenedGenericRequest {
        request_hash: hex::encode(parsed.request_hash),
        signed: parsed.is_signed(),
        signer_system_id: parsed
            .signature
            .as_ref()
            .map(|signature| signature.signer_system_id.clone()),
        signer_identity_id: parsed
            .signature
            .as_ref()
            .map(|signature| signature.signer_identity_id.clone()),
        signature_block_height,
    };
    request_store.open(OpenRequest {
        session_id: context.session_id,
        request_hex: request_hex.trim().to_ascii_lowercase(),
        parsed,
        kinds,
        broadcast_txid: None,
        reviewed_credentials: None,
        legacy_login: None,
        claim: None,
    });
    Ok(opened)
}

/// A request with no envelope: a legacy link is held as one, without a
/// signature, request ID, or creation time.
fn legacy_envelope(
    is_testnet: bool,
    details: Vec<RequestDetail>,
    response_uris: Vec<ResponseUri>,
    request_hash: [u8; 32],
) -> ParsedRequest {
    ParsedRequest {
        flags: if is_testnet { FLAG_IS_TESTNET } else { 0 },
        is_testnet,
        signature: None,
        request_id: None,
        created_at: None,
        app_or_delegated_id: None,
        details,
        response_uris,
        encrypt_response_to: None,
        preferred_handler: None,
        request_hash,
    }
}

/// Opens a legacy link as Verus Mobile does (`src/sagas/deeplink.js`,
/// `DeepLink.js`). A legacy invoice is held as a request with one invoice
/// detail (version 4), so the invoice commands pay it. A login consent request
/// keeps its own fields; its one response address is the one mobile uses.
async fn open_legacy_request(
    link: &str,
    context: &crate::core::auth::session::ActiveWalletAccessContext,
    session_manager: &Arc<Mutex<SessionManager>>,
    vrpc_provider_pool: &VrpcProviderPool,
    request_store: &GenericRequestStore,
) -> Result<OpenedGenericRequest, WalletError> {
    let network = context.wallet_network;
    let wallet_is_testnet = matches!(network, WalletNetwork::Testnet);
    let guard = context.session_submission_guard();
    let root_provider =
        vrpc_provider_pool.for_system(network, names::root_system_id(wallet_is_testnet));
    let crypto_network = wallet_network_to_crypto_network(network);

    let (parsed, kinds, legacy_login, signer, signature_block_height) =
        match legacy::parse_legacy_link(link)? {
            legacy::LegacyLink::Invoice(payload) => {
                let invoice = legacy::parse_legacy_invoice(&payload)?;
                let is_testnet = details::parse_invoice(&invoice.details_v4)?.flags
                    & details::INVOICE_IS_TESTNET
                    != 0;
                if is_testnet != wallet_is_testnet {
                    return Err(acceptance::Rejection::Network.into());
                }
                let parsed = legacy_envelope(
                    is_testnet,
                    vec![RequestDetail {
                        ordinal: details::ORDINAL_INVOICE,
                        data: invoice.details_v4.clone(),
                    }],
                    Vec::new(),
                    names::sha256(&payload),
                );
                let kinds = acceptance::check_request(&parsed, wallet_is_testnet)?;
                check_request_expiry(&parsed, &kinds, root_provider, &guard).await?;
                let mut signer = None;
                let mut height = None;
                if let Some(legacy_signer) = &invoice.signer {
                    let signer_id = names::i_address(&legacy_signer.signing_id);
                    let valid = guard
                        .run(verify_identity_hash_with_provider(
                            root_provider,
                            &signer_id,
                            &legacy_signer.signature,
                            None,
                            crypto_network,
                            |height, version| invoice.details_hash(height, version),
                        ))
                        .await
                        .map_err(map_generic_session_change)?;
                    if !valid {
                        return Err(acceptance::Rejection::Signature.into());
                    }
                    height = Some(parse_identity_signature(&legacy_signer.signature)?.block_height);
                    signer = Some((names::i_address(&legacy_signer.system_id), signer_id));
                }
                (parsed, kinds, None, signer, height)
            }
            legacy::LegacyLink::Login(bytes) => {
                let login = legacy::parse_login_consent(&bytes)?;
                legacy::check_login_consent(&login, wallet_is_testnet)?;
                let signer_id = names::i_address(&login.signing_id);
                let valid = guard
                    .run(verify_identity_hash_with_provider(
                        root_provider,
                        &signer_id,
                        &login.signature,
                        Some(login.created_at),
                        crypto_network,
                        |height, _| login.challenge_hash(height),
                    ))
                    .await
                    .map_err(map_generic_session_change)?;
                if !valid {
                    return Err(acceptance::Rejection::Signature.into());
                }
                let height = parse_identity_signature(&login.signature)?.block_height;
                let response_uris = login
                    .delivery()
                    .map(|(kind, uri)| ResponseUri {
                        kind: if kind == legacy::LOGIN_CONSENT_REDIRECT_KEY {
                            ResponseUriKind::Redirect
                        } else {
                            ResponseUriKind::Post
                        },
                        uri: uri.to_string(),
                    })
                    .into_iter()
                    .collect();
                let parsed = legacy_envelope(
                    wallet_is_testnet,
                    Vec::new(),
                    response_uris,
                    names::sha256(&bytes),
                );
                let signer = Some((names::i_address(&login.system_id), signer_id));
                (parsed, Vec::new(), Some(login), signer, Some(height))
            }
        };

    ensure_active_wallet_session(session_manager, &context.session_id)
        .await
        .map_err(map_generic_session_change)?;
    let opened = OpenedGenericRequest {
        request_hash: hex::encode(parsed.request_hash),
        signed: signer.is_some(),
        signer_system_id: signer.as_ref().map(|(system, _)| system.clone()),
        signer_identity_id: signer.map(|(_, identity)| identity),
        signature_block_height,
    };
    request_store.open(OpenRequest {
        session_id: context.session_id.clone(),
        request_hex: link.to_string(),
        parsed,
        kinds,
        broadcast_txid: None,
        reviewed_credentials: None,
        legacy_login,
        claim: None,
    });
    Ok(opened)
}

/// Closes the open request after Done, Decline, or a replacement link.
#[tauri::command(rename_all = "snake_case")]
pub fn close_generic_request(request_store: State<'_, GenericRequestStore>) {
    request_store.close();
}

/// Rejects a request whose sign in has passed its expiry time, whose invoice
/// or identity update has passed its expiry height, or whose invoice desktop
/// cannot pay.
async fn check_request_expiry(
    parsed: &ParsedRequest,
    kinds: &[DetailKind],
    provider: &crate::core::channels::vrpc::VrpcProvider,
    guard: &crate::core::auth::SessionSubmissionGuard,
) -> Result<(), WalletError> {
    let now = now_unix_seconds();
    let mut expiry_height = None;
    for (detail, kind) in parsed.details.iter().zip(kinds) {
        match kind {
            DetailKind::Authentication => {
                let auth = details::parse_authentication(&detail.data, parsed.is_testnet)?;
                if auth.expiry_time.is_some_and(|expiry| expiry <= now) {
                    return Err(acceptance::Rejection::Expired.into());
                }
            }
            DetailKind::IdentityUpdate => {
                let update = details::parse_identity_update(&detail.data, parsed.is_testnet)?;
                expiry_height = update.expiry_height.or(expiry_height);
            }
            DetailKind::Invoice => {
                let invoice = details::parse_invoice(&detail.data)?;
                acceptance::check_invoice(&invoice, parsed.is_testnet)?;
                expiry_height = invoice.expiry_height.or(expiry_height);
            }
            DetailKind::AppEncryption => {
                // `appEncryptionRequestValidator.js`: the derivation identity
                // must resolve on the chain.
                let app = details::parse_app_encryption(&detail.data, parsed.is_testnet)?;
                if let Some(identity) = app.derivation_id.as_deref() {
                    guard.run(provider.getidentity(identity)).await.map_err(
                        |error| match error {
                            WalletError::WalletLocked | WalletError::WalletSessionChanged => {
                                WalletError::WalletSessionChanged
                            }
                            _ => acceptance::Rejection::EncryptionKeyIdentity.into(),
                        },
                    )?;
                }
            }
            _ => {}
        }
    }
    if let Some(expiry_height) = expiry_height {
        let info = guard
            .run(provider.getinfo())
            .await
            .map_err(map_generic_session_change)?;
        if u64::from(extract_chain_height(&info)?) > expiry_height {
            return Err(acceptance::Rejection::Expired.into());
        }
    }
    Ok(())
}

/// Builds and signs the response to the open request. Rust takes every
/// request ID, the transaction ID, and the request hash from the verified
/// request and from what it did after Approval; the UI names only the
/// responder.
#[tauri::command(rename_all = "snake_case")]
pub async fn build_and_sign_generic_response(
    request_hex: String,
    signer: crate::types::GenericResponseSignerInput,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<BuildAndSignGenericResponseResult, WalletError> {
    build_and_sign_generic_response_for_session(
        &request_hex,
        &signer,
        session_manager.inner(),
        vrpc_provider_pool.inner(),
        request_store.inner(),
    )
    .await
}

async fn build_and_sign_generic_response_for_session(
    request_hex: &str,
    signer: &crate::types::GenericResponseSignerInput,
    session_manager: &Arc<Mutex<SessionManager>>,
    vrpc_provider_pool: &VrpcProviderPool,
    request_store: &GenericRequestStore,
) -> Result<BuildAndSignGenericResponseResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let open = request_store.get(&context.session_id, request_hex)?;
    let submission_guard = context.session_submission_guard();
    let network = context.wallet_network;
    let active_wallet_primary_address = context.vrsc_address.clone();
    let private_key = load_primary_private_scalar_for_context(&context)
        .await
        .map_err(map_generic_session_change)?;

    let provider = vrpc_provider_pool.for_system(network, &signer.system_id);
    submission_guard
        .run(validate_identity_control_for_active_wallet(
            provider,
            &signer.identity_id,
            &active_wallet_primary_address,
        ))
        .await
        .map_err(map_generic_session_change)?;

    let signer_identity = submission_guard
        .run(provider.getidentity(&signer.identity_id))
        .await
        .map_err(map_generic_session_change)?;
    check_responder_constraints(&open, signer, &signer_identity)?;
    let allowed_primary_addresses = extract_primary_addresses(&signer_identity);
    let info = submission_guard
        .run(provider.getinfo())
        .await
        .map_err(map_generic_session_change)?;
    let signed_block_height = extract_chain_height(&info)?;

    if let Some(login) = &open.legacy_login {
        let signed_response_hex = sign_legacy_login_response(
            login,
            signer,
            wallet_network_to_crypto_network(network),
            &active_wallet_primary_address,
            &allowed_primary_addresses,
            signed_block_height,
            &private_key,
        )?;
        ensure_active_wallet_session(session_manager, &context.session_id)
            .await
            .map_err(map_generic_session_change)?;
        return Ok(BuildAndSignGenericResponseResult {
            signed_response_hex,
            session_id: context.session_id,
        });
    }

    let response_signer = response::ResponseSigner {
        system_id: &signer.system_id,
        identity_id: &signer.identity_id,
    };
    let data_signatures =
        sign_data_packets(&open, &response_signer, signed_block_height, &private_key)?;
    let app_encryption_keys = app_encryption_keys(&open, &response_signer, &context).await?;
    let outcome = response::ResponseOutcome {
        txid: open.broadcast_txid.clone(),
        data_signatures,
        app_encryption_keys,
        shared_credentials: shared_credentials(&open, &signer.identity_id)?,
    };
    let response_details =
        response::response_details(&open.parsed, &open.kinds, &outcome, &response_signer)?;
    let unsigned = response::build_unsigned_response(
        &open.parsed,
        response_details,
        &response_signer,
        now_unix_seconds(),
    )?;
    let signed_response_hex = match submission_guard.poll_admitted(|| {
        std::task::Poll::Ready(sign_unsigned_response(
            &unsigned,
            wallet_network_to_crypto_network(network),
            &active_wallet_primary_address,
            &allowed_primary_addresses,
            signed_block_height,
            &private_key,
        ))
    }) {
        std::task::Poll::Ready(result) => result.map_err(map_generic_session_change)?,
        std::task::Poll::Pending => return Err(WalletError::OperationFailed),
    };

    ensure_active_wallet_session(session_manager, &context.session_id)
        .await
        .map_err(map_generic_session_change)?;

    match submission_guard.poll_admitted(|| {
        std::task::Poll::Ready(Ok(BuildAndSignGenericResponseResult {
            signed_response_hex,
            session_id: context.session_id,
        }))
    }) {
        std::task::Poll::Ready(result) => result.map_err(map_generic_session_change),
        std::task::Poll::Pending => Err(WalletError::OperationFailed),
    }
}

/// Delivers the signed response by POST. The address must be one of the open
/// request's POST response addresses.
#[tauri::command(rename_all = "snake_case")]
pub async fn post_generic_response_callback(
    request_hex: String,
    callback_uri: String,
    response_hex: String,
    session_id: String,
    body_json: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<(), WalletError> {
    let open = request_store.get(&session_id, &request_hex)?;
    ensure_request_response_uri(&open, ResponseUriKind::Post, &callback_uri)?;
    let body = match open.legacy_login {
        // A legacy login's webhook takes the response object as JSON.
        Some(_) => {
            let json = body_json.ok_or(WalletError::OperationFailed)?;
            let value: Value =
                serde_json::from_str(&json).map_err(|_| WalletError::OperationFailed)?;
            let response_bytes =
                hex::decode(response_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
            legacy::check_webhook_body(&legacy::parse_login_response(&response_bytes)?, &value)?;
            CallbackBody::Json(json)
        }
        None => CallbackBody::Bytes(
            hex::decode(response_hex.trim()).map_err(|_| WalletError::OperationFailed)?,
        ),
    };
    post_generic_response_callback_for_session(
        callback_uri,
        body,
        session_id,
        session_manager.inner(),
    )
    .await
}

/// What a POST delivery sends: the response bytes, or a legacy login's JSON.
enum CallbackBody {
    Bytes(Vec<u8>),
    Json(String),
}

fn ensure_request_response_uri(
    open: &OpenRequest,
    kind: ResponseUriKind,
    uri: &str,
) -> Result<(), WalletError> {
    if open
        .parsed
        .response_uris
        .iter()
        .any(|candidate| candidate.kind == kind && candidate.uri == uri)
    {
        Ok(())
    } else {
        Err(WalletError::GenericRequestRejected(
            "responseAddress".to_string(),
        ))
    }
}

async fn post_generic_response_callback_for_session(
    callback_uri: String,
    body: CallbackBody,
    session_id: String,
    session_manager: &Arc<Mutex<SessionManager>>,
) -> Result<(), WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    if context.session_id != session_id {
        return Err(WalletError::WalletSessionChanged);
    }
    context
        .session_submission_guard()
        .run(post_generic_response_callback_request(callback_uri, body))
        .await
        .map_err(map_generic_session_change)?;

    ensure_active_wallet_session(session_manager, &session_id)
        .await
        .map_err(map_generic_session_change)?;

    Ok(())
}

async fn post_generic_response_callback_request(
    callback_uri: String,
    body: CallbackBody,
) -> Result<(), WalletError> {
    let callback_uri = parse_generic_response_post_callback_uri(&callback_uri)?;
    let (content_type, body) = match body {
        CallbackBody::Bytes(bytes) => ("application/octet-stream", bytes),
        CallbackBody::Json(json) => ("application/json", json.into_bytes()),
    };
    let response = reqwest::Client::builder()
        .timeout(DELIVERY_TIMEOUT)
        .build()
        .map_err(|_| WalletError::NetworkError)?
        .post(callback_uri)
        .header(CONTENT_TYPE, content_type)
        .body(body)
        .send()
        .await
        .map_err(|_| WalletError::NetworkError)?;

    if !response.status().is_success() {
        return Err(WalletError::OperationFailed);
    }

    Ok(())
}

/// The largest provisioning service answer the wallet reads.
const PROVISIONING_RESPONSE_MAX_BYTES: usize = 64 * 1024;

/// Sends a signed provisioning request to the provisioning address of the
/// open request and returns the service's JSON answer. Rust sends it, as it
/// sends every response: only to the address the verified request names,
/// over HTTPS in release builds, and with the 30-second limit. The UI checks
/// the answer's signature.
#[tauri::command(rename_all = "snake_case")]
pub async fn post_provisioning_request(
    request_hex: String,
    webhook_url: String,
    body_json: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let provisioning_uri = match open.legacy_login.as_ref() {
        Some(login) => login
            .provisioning
            .as_ref()
            .map(|offer| offer.webhook.clone()),
        None => open
            .parsed
            .details
            .iter()
            .zip(&open.kinds)
            .find(|(_, kind)| **kind == DetailKind::ProvisionIdentity)
            .map(|(detail, _)| {
                details::parse_provision_identity(&detail.data, open.parsed.is_testnet)
            })
            .transpose()?
            .and_then(|provisioning| provisioning.uri),
    }
    .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if provisioning_uri.trim() != webhook_url.trim() {
        return Err(WalletError::GenericRequestRejected(
            "responseAddress".to_string(),
        ));
    }
    serde_json::from_str::<Value>(&body_json).map_err(|_| WalletError::OperationFailed)?;
    let url = parse_generic_response_post_callback_uri(&webhook_url)?;
    let post = async move {
        let response = reqwest::Client::builder()
            .timeout(DELIVERY_TIMEOUT)
            .build()
            .map_err(|_| WalletError::NetworkError)?
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(body_json)
            .send()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if !response.status().is_success() {
            return Err(WalletError::OperationFailed);
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| WalletError::NetworkError)?;
        if bytes.len() > PROVISIONING_RESPONSE_MAX_BYTES {
            return Err(WalletError::OperationFailed);
        }
        String::from_utf8(bytes.to_vec()).map_err(|_| WalletError::OperationFailed)
    };
    context
        .session_submission_guard()
        .run(post)
        .await
        .map_err(map_generic_session_change)
}

/// The longest one request's approved stages hold auto-lock.
const REQUEST_STAGES_AUTO_LOCK_LIMIT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// Pauses auto-lock while the approved stages of the open request run
/// (ADR 0001). Only an open request can pause it, and only for five minutes.
#[tauri::command(rename_all = "snake_case")]
pub async fn pause_auto_lock_for_request(
    request_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<(), WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    request_store.get(&context.session_id, &request_hex)?;
    let mut session = session_manager.lock().await;
    if session.active_session_id() != Some(context.session_id.as_str()) {
        return Err(WalletError::WalletLocked);
    }
    session.hold_expiry(REQUEST_STAGES_AUTO_LOCK_LIMIT)
}

/// Ends the pause after the stages stop, whether they passed or failed.
#[tauri::command(rename_all = "snake_case")]
pub async fn resume_auto_lock(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<(), WalletError> {
    session_manager.lock().await.release_expiry_hold();
    Ok(())
}

/// Delivers the signed response by opening the request's redirect address.
#[tauri::command(rename_all = "snake_case")]
pub async fn open_generic_request_callback(
    request_hex: String,
    callback_uri: String,
    response_hex: String,
    session_id: String,
    app: AppHandle,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<(), WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    if context.session_id != session_id {
        return Err(WalletError::WalletSessionChanged);
    }
    let open = request_store.get(&session_id, &request_hex)?;
    ensure_request_response_uri(&open, ResponseUriKind::Redirect, &callback_uri)?;
    // A legacy login's response travels under its own key (`handleRedirect.js`).
    let parameter = if open.legacy_login.is_some() {
        legacy::LOGIN_CONSENT_RESPONSE_KEY
    } else {
        GENERIC_RESPONSE_DEEPLINK_VDXF_ID
    };
    let redirect_url =
        build_generic_response_redirect_url(&callback_uri, &response_hex, parameter)?;
    match context.session_submission_guard().poll_admitted(|| {
        std::task::Poll::Ready(
            app.opener()
                .open_url(redirect_url.to_string(), None::<&str>)
                .map_err(|_| WalletError::OperationFailed),
        )
    }) {
        std::task::Poll::Ready(result) => result.map_err(map_generic_session_change),
        std::task::Poll::Pending => Err(WalletError::OperationFailed),
    }
}

fn map_generic_session_change(error: WalletError) -> WalletError {
    match error {
        WalletError::WalletLocked => WalletError::WalletSessionChanged,
        other => other,
    }
}

/// The new VerusID a legacy login offers, from the open request, which Rust
/// verified when it opened: its provisioning entries, the login's challenge ID
/// (`ProvisionIdentityConfirm.js`), and the login's system. It has no expiry.
fn legacy_provisioning_terms(
    open: &OpenRequest,
) -> Result<(ParsedProvisioningSemantics, String, String, Option<u64>), WalletError> {
    let login = open
        .legacy_login
        .as_ref()
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let offer = login
        .provisioning
        .as_ref()
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    Ok((
        ParsedProvisioningSemantics {
            system_id: offer.system_id.clone(),
            parent_id: offer.parent_id.clone(),
            identity_id: offer
                .fully_qualified_name
                .clone()
                .or_else(|| offer.identity_id.clone()),
        },
        names::i_address(&login.challenge_id),
        names::i_address(&login.system_id),
        None,
    ))
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
    request_store: State<'_, GenericRequestStore>,
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let network = context.wallet_network;
    if !signing_address
        .trim()
        .eq_ignore_ascii_case(&context.vrsc_address)
    {
        return Err(WalletError::InvalidAddress);
    }

    let (provisioning, request_id, signer_system_id, expiry_time) =
        if legacy::is_legacy_link(&request_hex) {
            legacy_provisioning_terms(&request_store.get(&context.session_id, &request_hex)?)?
        } else {
            let parsed_request = parse_generic_envelope_hex(request_hex.trim())?;
            if parsed_request.is_testnet != matches!(network, WalletNetwork::Testnet) {
                return Err(WalletError::UnsupportedNetwork);
            }
            acceptance::check_request(
                &parse_request_hex(request_hex.trim())?,
                matches!(network, WalletNetwork::Testnet),
            )?;

            let request_provider = vrpc_provider_pool
                .for_system(network, &parsed_request.signature_data.signer_system_id);
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
            // Verus Mobile's challenge ID: the request's own ID, else the
            // sign-in detail's (`ProvisionIdentityConfirm.js`).
            let request_id = parsed_request
                .request_id
                .clone()
                .or(authentication.request_id)
                .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
            (
                provisioning,
                request_id,
                parsed_request.signature_data.signer_system_id.clone(),
                authentication.expiry_time,
            )
        };
    let challenge = parse_provisioning_challenge_hex(&challenge_hex)?;
    if !challenge.challenge_id.eq_ignore_ascii_case(&request_id)
        || !match provisioning.identity_id.as_deref() {
            Some(identity_id) => challenge.name == provisioning_challenge_name(identity_id),
            None => is_valid_chosen_name(&challenge.name),
        }
        || challenge.system_id != provisioning.system_id
        || challenge.parent_id != provisioning.parent_id
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let expected_system_id = provisioning
        .system_id
        .as_deref()
        .unwrap_or(&signer_system_id);
    if !system_id.trim().eq_ignore_ascii_case(expected_system_id) {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let now = now_unix_seconds();
    if challenge.created_at > now.saturating_add(PROVISIONING_CREATED_AT_CLOCK_SKEW_SECS)
        || now.saturating_sub(challenge.created_at) > PROVISIONING_SIGNATURE_TTL_SECS
        || expiry_time.is_some_and(|expiry| expiry < now || challenge.created_at > expiry)
    {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }
    let expires_at = expiry_time
        .unwrap_or_else(|| now.saturating_add(PROVISIONING_SIGNATURE_TTL_SECS))
        .min(now.saturating_add(PROVISIONING_SIGNATURE_TTL_SECS));
    let signing_challenge_id = Uuid::new_v4().to_string();
    if !provisioning_signature_store.put(
        signing_challenge_id.clone(),
        ProvisioningSignatureChallenge {
            session_id: context.session_id,
            account_id: context.account_id,
            network,
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
) -> Result<String, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let challenge = provisioning_signature_store
        .take(signing_challenge_id.trim(), &context.session_id)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if challenge.account_id != context.account_id || challenge.network != context.wallet_network {
        return Err(WalletError::GenericRequestInvalidEnvelope);
    }

    let private_key = load_primary_private_scalar_for_context(&context).await?;
    ensure_active_wallet_session(session_manager.inner(), &context.session_id).await?;
    provisioning_signature(challenge.challenge_hash, &private_key)
}

/// `VerusIdInterface.signVerusIdProvisioningRequest`: the bare compact
/// signature over the challenge hash, which the provisioning service checks
/// with `verifyVerusIdProvisioningRequest`. It carries no identity signature
/// header or block height.
fn provisioning_signature(
    challenge_hash: [u8; 32],
    private_key: &[u8; 32],
) -> Result<String, WalletError> {
    Ok(BASE64_STANDARD.encode(sign_hash_compact(challenge_hash, private_key)?))
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

/// The update detail of the open request, the identity JSON Rust built from
/// it, and the metadata the review needs, all read from the verified bytes.
struct OpenIdentityUpdate {
    update: details::IdentityUpdateRequest,
    target_identity: String,
    system_id: String,
    meta: GenericIdentityUpdateRequestMeta,
    requested_identity_json: Value,
    /// Labels for content keys the request writes as names, keyed by the
    /// resolved key.
    content_key_labels: HashMap<String, String>,
}

async fn open_identity_update(
    open: &OpenRequest,
    ui_json: &Value,
    network: WalletNetwork,
    vrpc_provider_pool: &VrpcProviderPool,
) -> Result<OpenIdentityUpdate, WalletError> {
    let detail = open
        .parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::IdentityUpdate)
        .map(|(detail, _)| detail)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let update = details::parse_identity_update(&detail.data, open.parsed.is_testnet)?;
    if update.contains_sign_data {
        // Acceptance refuses sign data in an update (see `check_request`).
        return Err(acceptance::Rejection::UnsupportedDetail.into());
    }
    let target_identity = update
        .target_identity(open.parsed.is_testnet)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let system_id = update
        .system_id
        .clone()
        .unwrap_or_else(|| names::root_system_id(open.parsed.is_testnet).to_string());
    let provider = vrpc_provider_pool.for_system(network, &system_id);
    let (requested_identity_json, content_key_labels) = resolve_content_keys(
        provider,
        details::requested_identity_json(&update, ui_json)?,
    )
    .await?;
    let meta = GenericIdentityUpdateRequestMeta {
        request_id: update.request_id.clone(),
        signer_system_id: open
            .parsed
            .signature
            .as_ref()
            .map(|signature| signature.signer_system_id.clone()),
        signer_identity_id: open
            .parsed
            .signature
            .as_ref()
            .map(|signature| signature.signer_identity_id.clone()),
        expiry_height: update
            .expiry_height
            .map(|height| u32::try_from(height).unwrap_or(u32::MAX)),
        request_system_id: update.system_id.clone(),
    };
    Ok(OpenIdentityUpdate {
        update,
        target_identity,
        system_id,
        meta,
        requested_identity_json,
        content_key_labels,
    })
}

/// Content keys written as names (`ns@::key`) resolve to the VDXF key the
/// daemon stores them under. The library's own hashing of such names differs
/// from the daemon's, so the daemon decides (`getvdxfid`).
async fn resolve_content_keys(
    provider: &crate::core::channels::vrpc::VrpcProvider,
    mut identity_json: Value,
) -> Result<(Value, HashMap<String, String>), WalletError> {
    let mut labels = HashMap::new();
    let Some(content) = identity_json
        .get_mut("contentmultimap")
        .and_then(Value::as_object_mut)
    else {
        return Ok((identity_json, labels));
    };
    let mut resolved = serde_json::Map::new();
    for (key, values) in std::mem::take(content) {
        let key = if names::decode_i_address(&key).is_some() {
            key
        } else {
            let vdxfid = provider
                .getvdxfid(&key)
                .await
                .map_err(|_| WalletError::IdentityBuildFailed)?
                .get("vdxfid")
                .and_then(Value::as_str)
                .filter(|vdxfid| names::decode_i_address(vdxfid).is_some())
                .ok_or(WalletError::IdentityBuildFailed)?
                .to_string();
            if let Some(label) = content_key_label(&key) {
                labels.insert(vdxfid.clone(), label);
            }
            vdxfid
        };
        if resolved.insert(key, values).is_some() {
            return Err(WalletError::IdentityBuildFailed);
        }
    }
    *content = resolved;
    Ok((identity_json, labels))
}

/// Verus Mobile labels a key written as `ns::key.name` "Key name".
fn content_key_label(key: &str) -> Option<String> {
    let name = key.split("::").nth(1)?.trim();
    let words = name
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }
    Some(capitalize_label(&words.join(" ")))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn review_generic_identity_update(
    request_hex: String,
    requested_identity_json: Value,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericIdentityUpdateReviewResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let network = context.wallet_network;
    let update = open_identity_update(
        &open,
        &requested_identity_json,
        network,
        vrpc_provider_pool.inner(),
    )
    .await?;

    let provider = vrpc_provider_pool.for_system(network, &update.system_id);
    ensure_identity_update_request_not_expired(provider, Some(&update.meta)).await?;

    let mut inspection = inspect_generic_identity_update(
        provider,
        network,
        &update.requested_identity_json,
        &update.target_identity,
        &context.vrsc_address,
        Some(&update.meta),
    )
    .await?;
    for (key, label) in update.content_key_labels {
        inspection
            .review
            .signer_cmm_key_labels
            .entry(key)
            .or_insert(label);
    }

    Ok(inspection.review)
}

async fn load_watched_funding_addresses(
    context: &IdentitySessionContext,
) -> Result<Vec<String>, WalletError> {
    Ok(context
        .access
        .stronghold_store
        .load_watchlist_entries(
            &context.account_id,
            context.access.password_hash(),
            &context.account_state_store,
            context.network,
            Some(context.access.session_submission_guard()),
        )
        .await?
        .into_iter()
        .map(|entry| entry.address)
        .collect())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_generic_identity_update(
    request_hex: String,
    requested_identity_json: Value,
    source_channel_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    account_state_store: State<'_, AccountStateStore>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericIdentityUpdatePreflightResult, WalletError> {
    let context =
        identity_session_context(session_manager.inner(), account_state_store.inner()).await?;
    let account_id = context.account_id.clone();
    let session_vrpc_address = context.primary_address.clone();
    let network = context.network;

    let open = request_store.get(&context.session_id, &request_hex)?;
    let OpenIdentityUpdate {
        update,
        target_identity: target_identity_address,
        meta,
        requested_identity_json,
        content_key_labels,
        ..
    } = open_identity_update(
        &open,
        &requested_identity_json,
        network,
        vrpc_provider_pool.inner(),
    )
    .await?;
    let request_meta = Some(meta);

    let resolved = vrpc::parse_vrpc_channel_id(&source_channel_id, Some(&session_vrpc_address))?;
    let linked_identities = load_linked_for_context(&context).await?;
    let watched_addresses = load_watched_funding_addresses(&context).await?;
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

    let canonical_channel_id = resolved.canonical_channel_id();
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
        mut effective_requested_identity,
        mut review,
    } = inspection;
    // The review keeps the plain credentials; the daemon sees them encrypted.
    encrypt_update_credentials(
        &update,
        &target_identity_address,
        &target.identity,
        &mut effective_requested_identity,
        &context.access,
    )
    .await?;

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
            .funding_utxos(std::slice::from_ref(&resolved.address))
            .await
            .map_err(|_| WalletError::IdentityBuildFailed)?,
    );
    if total_satoshis(&funding_candidates) < DEFAULT_FEE_SAT {
        return Err(WalletError::InsufficientFunds);
    }

    let update_tx_raw = provider
        .updateidentity(&effective_requested_identity, true)
        .await
        .map_err(|err| match err {
            WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
            _ => WalletError::IdentityBuildFailed,
        })?;
    let update_tx_hex = parse_updateidentity_hex(update_tx_raw)?;
    let control_intent = crate::core::channels::vrpc::intent::identity_control_intent_from_json(
        &effective_requested_identity,
    )?;
    crate::core::channels::vrpc::intent::validate_identity_control_intent(
        &update_tx_hex,
        &control_intent,
    )?;
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
        None,
    )?;
    let authenticated_inputs = signable_inputs
        .iter()
        .map(|input| crate::core::channels::vrpc::common::VrpcInputRef {
            txid: input.txid.clone(),
            vout: input.vout,
            satoshis: input.satoshis,
            script_pub_key: Some(input.script_pub_key.clone()),
        })
        .collect::<Vec<_>>();
    crate::core::channels::vrpc::common::authenticate_payload_inputs(
        provider,
        &authenticated_inputs,
    )
    .await
    .map_err(|_| WalletError::IdentityBuildFailed)?;
    let input_total = authenticated_inputs
        .iter()
        .try_fold(0i64, |total, input| total.checked_add(input.satoshis))
        .ok_or(WalletError::IdentityBuildFailed)?;
    crate::core::channels::vrpc::intent::validate_identity_transaction_intent(
        &unsigned_hex,
        &control_intent,
        &resolved.address,
        input_total,
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
        control_intent,
        profile_intent: None,
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
        signer_cmm_key_labels: {
            let mut labels = review.signer_cmm_key_labels;
            for (key, label) in content_key_labels {
                labels.entry(key).or_insert(label);
            }
            labels
        },
        primary_address_after_update_info: review.primary_address_after_update_info,
        current_authorities: review.current_authorities,
    })
}

/// Broadcasts the open request's identity update after Approval. When a later
/// stage failed and the user tries again, it returns the first transaction
/// instead of sending a second one (ADR 0001).
#[tauri::command(rename_all = "snake_case")]
pub async fn send_generic_identity_update(
    request_hex: String,
    preflight_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericRequestTransactionResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    if let Some(txid) = open.broadcast_txid.clone() {
        return Ok(GenericRequestTransactionResult { txid });
    }

    let target_identity = open
        .parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::IdentityUpdate)
        .map(|(detail, _)| details::parse_identity_update(&detail.data, open.parsed.is_testnet))
        .transpose()?
        .and_then(|update| update.target_identity(open.parsed.is_testnet))
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let record = preflight_store
        .get(&preflight_id, &context.session_id)
        .ok_or(WalletError::InvalidPreflight)?;
    let payload: IdentityPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;
    if payload.operation != IdentityOperation::Update
        || !payload
            .target_identity
            .eq_ignore_ascii_case(&target_identity)
    {
        return Err(WalletError::InvalidPreflight);
    }

    let result = crate::core::channels::vrpc::identity::send(
        &preflight_id,
        preflight_store.inner(),
        session_manager.inner(),
        vrpc_provider_pool.inner(),
    )
    .await?;
    request_store.record_broadcast(&context.session_id, &open.parsed.request_hash, &result.txid);
    Ok(GenericRequestTransactionResult { txid: result.txid })
}

// --- Share data -----------------------------------------------------------------

/// Finds the credentials a share data request would send from `identity_id`
/// (`getScopedCredentials`): Rust reads the identity's content under its
/// credential key, decrypts it, and keeps the valid credentials whose key was
/// requested and whose scopes include the app identity, or the requester
/// when the request names no app. The response shares exactly these.
#[tauri::command(rename_all = "snake_case")]
pub async fn review_generic_credentials(
    request_hex: String,
    identity_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericCredentialReview, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let user_data = open_user_data(&open)?.ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    if user_data
        .signer
        .as_deref()
        .is_some_and(|required| !required.eq_ignore_ascii_case(&identity_id))
    {
        return Err(WalletError::GenericRequestRejected("responder".to_string()));
    }
    let identity = names::decode_i_address(&identity_id).ok_or(WalletError::InvalidAddress)?;
    let scope = open
        .parsed
        .app_or_delegated_id
        .clone()
        .or_else(|| {
            open.parsed
                .signature
                .as_ref()
                .map(|signature| signature.signer_identity_id.clone())
        })
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;

    let account = private_account_key(&context).await?;
    let keys = crate::core::generic_request::private_crypto::channel_keys(
        &account,
        Some(&identity),
        Some(&identity),
        0,
        false,
    )?;
    let storage_key = credentials::storage_key(&keys.incoming_viewing_key)?;
    let provider = vrpc_provider_pool.for_system(
        context.wallet_network,
        names::root_system_id(open.parsed.is_testnet),
    );
    let content = provider
        .getidentitycontent_for_key(&identity_id, 0, 0, false, &storage_key)
        .await?;
    let stored = match content
        .get("identity")
        .and_then(|identity| identity.get("contentmultimap"))
        .and_then(|map| map.get(&storage_key))
    {
        Some(Value::Array(values)) => values.clone(),
        Some(value) => vec![value.clone()],
        None => Vec::new(),
    };

    let requested_keys = user_data
        .search_keys
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    let found =
        credentials::read_credentials(&stored, &keys.incoming_viewing_key, &requested_keys, &scope);
    let missing_keys = requested_keys
        .into_iter()
        .filter(|key| !found.iter().any(|credential| &credential.key == key))
        .collect();
    let review = GenericCredentialReview {
        credentials: found
            .iter()
            .map(|credential| GenericSharedCredential {
                key: credential.key.clone(),
                label: credential.label.clone(),
                credential: credential.credential.clone(),
                scopes: credential.scopes.clone(),
            })
            .collect(),
        missing_keys,
    };
    request_store.record_credentials(
        &context.session_id,
        &open.parsed.request_hash,
        &identity_id,
        found,
    );
    Ok(review)
}

// --- Pay invoice ----------------------------------------------------------------

fn open_invoice(open: &OpenRequest) -> Result<details::InvoiceRequest, WalletError> {
    let detail = open
        .parsed
        .details
        .iter()
        .zip(&open.kinds)
        .find(|(_, kind)| **kind == DetailKind::Invoice)
        .map(|(detail, _)| detail)
        .ok_or(WalletError::GenericRequestInvalidEnvelope)?;
    let invoice = details::parse_invoice(&detail.data)?;
    acceptance::check_invoice(&invoice, open.parsed.is_testnet)?;
    Ok(invoice)
}

/// The invoice rules that need the chain: expiry, a known currency, and the
/// preconvert window. Returns the currency's `getcurrency` result and the
/// chain height.
async fn check_invoice_on_chain(
    invoice: &details::InvoiceRequest,
    provider: &crate::core::channels::vrpc::VrpcProvider,
    guard: &crate::core::auth::SessionSubmissionGuard,
) -> Result<(Value, u64), WalletError> {
    let info = guard
        .run(provider.getinfo())
        .await
        .map_err(map_generic_session_change)?;
    let height = u64::from(extract_chain_height(&info)?);
    if invoice.expiry_height.is_some_and(|expiry| expiry < height) {
        return Err(acceptance::Rejection::Expired.into());
    }
    let currency = guard
        .run(provider.getcurrency(&invoice.requested_currency_id))
        .await
        .map_err(|error| match error {
            WalletError::WalletLocked | WalletError::WalletSessionChanged => {
                WalletError::WalletSessionChanged
            }
            _ => acceptance::Rejection::InvoiceCurrency.into(),
        })?;
    if invoice.has(details::INVOICE_IS_PRECONVERT) {
        let start_block = currency
            .get("startblock")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        return Err(if start_block <= height {
            acceptance::Rejection::InvoicePreconvertLaunched.into()
        } else {
            acceptance::Rejection::UnsupportedDetail.into()
        });
    }
    Ok((currency, height))
}

fn invoice_destination(invoice: &details::InvoiceRequest) -> Result<&str, WalletError> {
    match &invoice.destination {
        Some(details::InvoiceDestination::Transparent(address)) => Ok(address),
        _ => Err(acceptance::Rejection::InvoiceNoDestination.into()),
    }
}

/// What the invoice asks for, as Rust will pay it.
#[tauri::command(rename_all = "snake_case")]
pub async fn review_generic_invoice(
    request_hex: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericInvoiceReview, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    let invoice = open_invoice(&open)?;
    let provider = vrpc_provider_pool.for_system(
        context.wallet_network,
        names::root_system_id(open.parsed.is_testnet),
    );
    let guard = context.session_submission_guard();
    let (currency, height) = check_invoice_on_chain(&invoice, provider, &guard).await?;
    let destination = invoice_destination(&invoice)?.to_string();
    let destination_name = if names::decode_i_address(&destination).is_some() {
        guard
            .run(provider.getidentity(&destination))
            .await
            .ok()
            .and_then(|identity| {
                first_non_empty_field(&identity, &["fullyqualifiedname", "friendlyname"])
            })
    } else {
        None
    };
    Ok(GenericInvoiceReview {
        amount: invoice
            .amount
            .map(|amount| vrpc::sat_to_decimal_string(amount as i64)),
        currency_id: invoice.requested_currency_id.clone(),
        currency_name: first_non_empty_field(&currency, &["fullyqualifiedname", "name"]),
        destination,
        destination_name,
        expiry_height: invoice.expiry_height,
        blocks_left: invoice
            .expiry_height
            .map(|expiry| expiry.saturating_sub(height)),
        already_paid: request_store.is_paid(&context.session_id, &open.parsed.request_hash),
    })
}

/// Prepares the payment from the chosen source. The amount is exactly the
/// invoice's; the fee is added on top. An invoice that accepts any amount
/// takes `amount` from the user.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_generic_invoice(
    request_hex: String,
    source_channel_id: String,
    amount: Option<String>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<crate::core::CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericInvoicePreflightResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    if request_store.is_paid(&context.session_id, &open.parsed.request_hash) {
        return Err(acceptance::Rejection::AlreadyPaid.into());
    }
    let invoice = open_invoice(&open)?;
    let network = context.wallet_network;
    let root_system = names::root_system_id(open.parsed.is_testnet);
    let provider = vrpc_provider_pool.for_system(network, root_system);
    let guard = context.session_submission_guard();
    check_invoice_on_chain(&invoice, provider, &guard).await?;
    let destination = invoice_destination(&invoice)?.to_string();

    let amount_sat = match invoice.amount {
        Some(amount) => i64::try_from(amount).map_err(|_| WalletError::InvalidAmount)?,
        None => vrpc::parse_positive_amount_sat(amount.as_deref().unwrap_or_default())?,
    };

    // Payments leave from the wallet's own address on the Verus chain, never
    // from a watched address.
    let resolved = vrpc::parse_vrpc_channel_id(&source_channel_id, Some(&context.vrsc_address))?;
    if !resolved.address.eq_ignore_ascii_case(&context.vrsc_address)
        || !resolved.system_id.eq_ignore_ascii_case(root_system)
    {
        return Err(WalletError::InvalidAddress);
    }
    let coin = coin_registry
        .get_all()
        .into_iter()
        .find(|coin| {
            coin.is_testnet == open.parsed.is_testnet
                && coin
                    .currency_id
                    .eq_ignore_ascii_case(&invoice.requested_currency_id)
        })
        .ok_or(WalletError::GenericRequestRejected(
            "invoiceCurrencyMissing".to_string(),
        ))?;

    let params = crate::types::VrpcTransferPreflightParams {
        coin_id: coin.id.clone(),
        channel_id: resolved.canonical_channel_id(),
        source_address: None,
        destination: destination.clone(),
        amount: vrpc::sat_to_decimal_string(amount_sat),
        convert_to: None,
        export_to: None,
        via: None,
        fee_currency: None,
        fee_satoshis: None,
        preconvert: None,
        map_to: None,
        vdxf_tag: None,
        memo: None,
    };
    let result = guard
        .run(vrpc::preflight_exact_transfer(
            params,
            preflight_store.inner(),
            &context.account_id,
            &context.session_id,
            &context.vrsc_address,
            &resolved.canonical_channel_id(),
            &resolved.system_id,
            &coin.currency_id,
            provider,
        ))
        .await
        .map_err(map_generic_session_change)?;
    Ok(GenericInvoicePreflightResult {
        preflight_id: result.preflight_id,
        amount: result.value,
        fee: result.fee,
        currency_id: invoice.requested_currency_id,
        fee_currency_id: root_system.to_string(),
        from_address: result.from_address,
        to_address: result.to_address,
    })
}

/// Pays the open request's invoice after Approval, once per request in this
/// session.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_generic_invoice_payment(
    request_hex: String,
    preflight_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
    request_store: State<'_, GenericRequestStore>,
) -> Result<GenericRequestTransactionResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager.inner()).await?;
    let open = request_store.get(&context.session_id, &request_hex)?;
    if let Some(txid) = open.broadcast_txid.clone() {
        return Ok(GenericRequestTransactionResult { txid });
    }
    if request_store.is_paid(&context.session_id, &open.parsed.request_hash) {
        return Err(acceptance::Rejection::AlreadyPaid.into());
    }
    let invoice = open_invoice(&open)?;
    let record = preflight_store
        .get(&preflight_id, &context.session_id)
        .ok_or(WalletError::InvalidPreflight)?;
    let payload: vrpc::VrpcPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;
    ensure_payment_matches_invoice(
        &payload,
        &invoice,
        names::root_system_id(open.parsed.is_testnet),
    )?;

    let result = vrpc::send(
        &preflight_id,
        preflight_store.inner(),
        session_manager.inner(),
        vrpc_provider_pool.inner(),
    )
    .await?;
    request_store.record_payment(&context.session_id, &open.parsed.request_hash);
    request_store.record_broadcast(&context.session_id, &open.parsed.request_hash, &result.txid);
    Ok(GenericRequestTransactionResult { txid: result.txid })
}

/// The prepared transaction pays the invoice's currency, amount, and
/// destination, and nothing else.
fn ensure_payment_matches_invoice(
    payload: &vrpc::VrpcPreflightPayload,
    invoice: &details::InvoiceRequest,
    root_system_id: &str,
) -> Result<(), WalletError> {
    use crate::core::channels::vrpc::intent::VrpcOutputIntent;
    let wanted_destination = invoice_destination(invoice)?;
    let (destination, amount, currency) = match &payload.intent {
        VrpcOutputIntent::NativePayment {
            destination,
            amount_sats,
        } => (destination, *amount_sats, root_system_id.to_string()),
        VrpcOutputIntent::TokenPayment {
            destination,
            currency_id,
            amount_sats,
            ..
        } => {
            let hash: [u8; 20] = currency_id
                .as_slice()
                .try_into()
                .map_err(|_| WalletError::InvalidPreflight)?;
            (destination, *amount_sats, names::i_address(&hash))
        }
        VrpcOutputIntent::ReserveTransfer { .. } => return Err(WalletError::InvalidPreflight),
    };
    let (version, hash) =
        names::decode_base58_check(wanted_destination).ok_or(WalletError::InvalidPreflight)?;
    let wanted_type = if version == names::I_ADDRESS_VERSION {
        4
    } else {
        2
    };
    if destination.destination_type != wanted_type
        || destination.destination_bytes != hash
        || !currency.eq_ignore_ascii_case(&invoice.requested_currency_id)
        || invoice.amount.is_some_and(|wanted| wanted != amount)
    {
        return Err(WalletError::InvalidPreflight);
    }
    Ok(())
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
        build_and_sign_generic_response_for_session, build_generic_response_redirect_url,
        decode_defined_key_label, map_generic_session_change, merge_identity_update_patch,
        parse_generic_response_post_callback_uri, parse_provisioning_challenge_hex,
        post_generic_response_callback_for_session, post_generic_response_callback_request,
        provisioning_signature, resolve_signer_cmm_key_labels_from_identity,
        sign_unsigned_response, validate_generic_request_funding_source,
        wallet_network_to_crypto_network, CallbackBody, WalletNetwork,
        DATA_TYPE_DEFINEDKEY_VDXF_ID, GENERIC_RESPONSE_DEEPLINK_VDXF_ID, MAINNET_VERUS_CHAIN_ID,
    };
    use crate::core::auth::{capture_active_wallet_access_context, SessionManager};
    use crate::core::crypto::verus_id_signature::{
        compute_identity_signature_hash, get_raw_envelope_sha256, parse_generic_envelope_hex,
        parse_identity_signature, verify_identity_signature_against_addresses,
    };
    use crate::core::crypto::wif_encoding::generate_p2pkh_address;
    use crate::core::crypto::{derive_keys_v1, derive_public_profile_from_material, Network};
    use crate::core::generic_request::acceptance::check_request;
    use crate::core::generic_request::request::parse_request_hex;
    use crate::core::generic_request::response::{
        build_unsigned_response, data_packet_hashes, response_details, ItemSignature,
        ResponseOutcome, ResponseSigner,
    };
    use crate::core::generic_request::store::{GenericRequestStore, OpenRequest};
    use crate::core::wallet::AccountStateStore;
    use crate::core::StrongholdStore;
    use crate::types::{
        GenericResponseSignerInput, LinkedIdentity, WalletError, WatchlistEntry,
        WatchlistTargetKind,
    };
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, Notify};
    use zeroize::Zeroizing;

    const TEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
    const FIXED_CREATED_AT: u64 = 1_700_000_000;
    const FIXED_SIGNED_BLOCK_HEIGHT: u32 = 965_771;

    #[test]
    fn a_chosen_verusid_name_is_one_clean_label() {
        // VerusID names may hold spaces ("Mike T@" exists on testnet).
        for good in ["alice", "Mike T", "player-7"] {
            assert!(super::is_valid_chosen_name(good), "{good}");
        }
        let long = "x".repeat(65);
        for bad in [
            "",
            " alice",
            "alice ",
            "alice.parent",
            "alice@",
            "a/b",
            "a:b",
            &long,
        ] {
            assert!(!super::is_valid_chosen_name(bad), "{bad}");
        }
    }

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

    #[test]
    fn provisioning_signature_matches_verusid_ts_client() {
        // `VerusIdInterface.signHashWithAddress` from verusid-ts-client 933e8f8
        // over the challenge hash above, with the key of the public test
        // phrase "verus id signature private byte parity"
        // (RWctcHsxKcYgtCwjTgatANSd96JJa2p9Cj).
        let keys = derive_keys_v1("verus id signature private byte parity", Network::Mainnet)
            .expect("derive keys");
        let private_key: [u8; 32] = hex::decode(&keys.eth_private_key)
            .expect("private key hex")
            .try_into()
            .expect("private key length");
        let hash: [u8; 32] =
            hex::decode("35d3bef3d19528f98c4edeebba920d50438ca346d38257aa1c91b53805bd5c31")
                .unwrap()
                .try_into()
                .unwrap();
        assert_eq!(
            provisioning_signature(hash, &private_key).unwrap(),
            "HxtGq48PEspGqbLll6cJZIIpVXJg18TUNWonsqZQg+DyFmTwd7P+GFB9hCVdnPmyAlrIUBpVODgFcWVUuRe3VKU="
        );
    }

    fn vectors() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../src/lib/genericRequest/fixtures/request-vectors.json"
        ))
        .expect("request vectors")
    }

    fn vector_request_hex(name: &str) -> String {
        vectors()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == name)
            .unwrap()["hex"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn open_vector_request(
        store: &GenericRequestStore,
        session_id: &str,
        request_hex: &str,
        testnet: bool,
    ) -> OpenRequest {
        let parsed = parse_request_hex(request_hex).expect("vector parses");
        let kinds = check_request(&parsed, testnet).expect("vector is accepted");
        let open = OpenRequest {
            session_id: session_id.to_string(),
            request_hex: request_hex.to_string(),
            parsed,
            kinds,
            broadcast_txid: None,
            reviewed_credentials: None,
            legacy_login: None,
            claim: None,
        };
        store.open(open.clone());
        open
    }

    /// The encryption keys of a `verus-zfunc-vectors.json` channel.
    fn vector_app_keys(case: usize) -> crate::core::generic_request::response::AppEncryptionKeys {
        let zfunc: serde_json::Value = serde_json::from_str(include_str!(
            "../core/generic_request/fixtures/verus-zfunc-vectors.json"
        ))
        .unwrap();
        let keys = &zfunc["channels"][case];
        crate::core::generic_request::response::AppEncryptionKeys {
            incoming_viewing_key: hex::decode(keys["ivkRaw"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap(),
            extended_viewing_key: hex::decode(keys["extfvk"].as_str().unwrap())
                .unwrap()
                .try_into()
                .unwrap(),
            address: crate::core::generic_request::names::decode_sapling_address(
                keys["address"].as_str().unwrap(),
            )
            .unwrap(),
            extended_spending_key: None,
        }
    }

    fn unsigned_vector_response(
        request: &str,
        txid: Option<&str>,
        signer: &str,
        created_at: u64,
        item_signature: &[u8],
        app_keys_case: Option<usize>,
    ) -> Vec<u8> {
        let open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex(request),
            true,
        );
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: signer,
        };
        // Data items carry the vector's placeholder signature; the bytes
        // around it must match mobile's.
        let data_signatures = data_packet_hashes(&open.parsed, &open.kinds)
            .unwrap()
            .into_iter()
            .map(|hashes| {
                hashes
                    .into_iter()
                    .map(|hash| ItemSignature {
                        hash,
                        signature: item_signature.to_vec(),
                    })
                    .collect()
            })
            .collect();
        let details = response_details(
            &open.parsed,
            &open.kinds,
            &ResponseOutcome {
                txid: txid.map(ToString::to_string),
                data_signatures,
                app_encryption_keys: app_keys_case.map(vector_app_keys),
                shared_credentials: Vec::new(),
            },
            &signer,
        )
        .expect("response details");
        build_unsigned_response(&open.parsed, details, &signer, created_at)
            .expect("unsigned response")
    }

    #[test]
    fn name_form_content_keys_get_mobile_labels() {
        assert_eq!(
            super::content_key_label("TestBob.antafri::deeplinksim.qa.note").as_deref(),
            Some("Deeplinksim qa note")
        );
        assert_eq!(
            super::content_key_label("iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c"),
            None
        );
    }

    /// The vector credentials, as Rust reads them from an identity.
    fn vector_credentials() -> Vec<super::credentials::Credential> {
        vectors()["credentials"]["credentialHex"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                super::credentials::parse_credential(&hex::decode(value.as_str().unwrap()).unwrap())
                    .unwrap()
            })
            .collect()
    }

    /// The response to a share data request before mobile encrypts it.
    fn unencrypted_credential_response(request: &str, signer: &str, created_at: u64) -> Vec<u8> {
        let mut open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex(request),
            true,
        );
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: signer,
        };
        let details = response_details(
            &open.parsed,
            &open.kinds,
            &ResponseOutcome {
                shared_credentials: vector_credentials(),
                ..ResponseOutcome::default()
            },
            &signer,
        )
        .expect("response details");
        open.parsed.encrypt_response_to = None;
        build_unsigned_response(&open.parsed, details, &signer, created_at)
            .expect("unsigned response")
    }

    #[test]
    fn responses_match_the_bytes_verus_mobile_builds() {
        for expected in vectors()["responses"].as_array().unwrap() {
            if expected["sharesCredentials"].as_bool() == Some(true) {
                assert_eq!(
                    hex::encode(unencrypted_credential_response(
                        expected["request"].as_str().unwrap(),
                        expected["signer"].as_str().unwrap(),
                        expected["createdAt"].as_u64().unwrap(),
                    )),
                    expected["hex"].as_str().unwrap(),
                    "{} response differs from Verus Mobile",
                    expected["request"]
                );
                continue;
            }
            let item_signature =
                hex::decode(expected["itemSignature"].as_str().unwrap_or_default()).unwrap();
            let unsigned = unsigned_vector_response(
                expected["request"].as_str().unwrap(),
                expected["txid"].as_str(),
                expected["signer"].as_str().unwrap(),
                expected["createdAt"].as_u64().unwrap(),
                &item_signature,
                expected["appKeysCase"].as_u64().map(|case| case as usize),
            );
            assert_eq!(
                hex::encode(unsigned),
                expected["hex"].as_str().unwrap(),
                "{} response differs from Verus Mobile",
                expected["request"]
            );
        }
    }

    #[test]
    fn a_share_data_answer_without_credentials_leaves_the_detail_out() {
        let open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex("auth-credential"),
            true,
        );
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk",
        };
        let details = response_details(
            &open.parsed,
            &open.kinds,
            &ResponseOutcome::default(),
            &signer,
        )
        .unwrap();
        // Only the sign-in response remains.
        assert_eq!(details.len(), 1);
    }

    #[test]
    fn credentials_are_shared_only_after_review_by_the_required_identity() {
        let store = GenericRequestStore::new();
        let mut open = open_vector_request(
            &store,
            "session",
            &vector_request_hex("auth-credential"),
            true,
        );
        let required = "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk";
        assert!(matches!(
            super::shared_credentials(&open, required),
            Err(WalletError::OperationFailed)
        ));
        open.reviewed_credentials = Some((required.to_string(), vector_credentials()));
        assert_eq!(super::shared_credentials(&open, required).unwrap().len(), 2);
        assert!(matches!(
            super::shared_credentials(&open, "iGHXn8mvh9kutBNotJ2LCNz3zDxvemEb2i"),
            Err(WalletError::GenericRequestRejected(code)) if code == "responder"
        ));
    }

    #[test]
    fn a_legacy_login_response_verifies_for_the_responder() {
        let link = vectors()["legacy"]["login"]["link"]
            .as_str()
            .unwrap()
            .to_string();
        use crate::core::generic_request::{legacy, names};
        let login = match legacy::parse_legacy_link(&link).unwrap() {
            legacy::LegacyLink::Login(bytes) => legacy::parse_login_consent(&bytes).unwrap(),
            other => panic!("expected a login link, got {other:?}"),
        };
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, address) = test_signing_context(network, 7);
        let responder = "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk";
        let signer = crate::types::GenericResponseSignerInput {
            system_id: TEST_SYSTEM_ID.to_string(),
            identity_id: responder.to_string(),
        };
        let response_hex = super::sign_legacy_login_response(
            &login,
            &signer,
            network,
            &address,
            std::slice::from_ref(&address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
        )
        .unwrap();

        let fields = legacy::parse_login_response(&hex::decode(&response_hex).unwrap()).unwrap();
        assert_eq!(fields.decision_id, login.challenge_id);
        let decision = login.decision(fields.created_at).unwrap();
        let hash = legacy::decision_hash(
            login.system_id,
            names::decode_i_address(responder).unwrap(),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &decision,
        );
        assert!(verify_identity_signature_against_addresses(
            hash,
            &parse_identity_signature(&fields.signature).unwrap(),
            std::slice::from_ref(&address),
            network,
        ));

        // A responder on another system cannot answer.
        let other_system = crate::types::GenericResponseSignerInput {
            system_id: "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV".to_string(),
            identity_id: responder.to_string(),
        };
        assert!(super::sign_legacy_login_response(
            &login,
            &other_system,
            network,
            &address,
            std::slice::from_ref(&address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
        )
        .is_err());
    }

    #[test]
    fn an_update_response_needs_the_broadcast_txid() {
        let open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex("auth-update"),
            true,
        );
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk",
        };
        assert!(response_details(
            &open.parsed,
            &open.kinds,
            &ResponseOutcome::default(),
            &signer
        )
        .is_err());
        let malformed = ResponseOutcome {
            txid: Some("1234".to_string()),
            ..ResponseOutcome::default()
        };
        assert!(matches!(
            response_details(&open.parsed, &open.kinds, &malformed, &signer),
            Err(WalletError::OperationFailed)
        ));
    }

    #[test]
    fn a_signed_response_verifies_against_the_responder() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, address) = test_signing_context(network, 7);
        let unsigned = unsigned_vector_response(
            "auth",
            None,
            "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk",
            FIXED_CREATED_AT,
            &[],
            None,
        );
        let signed = sign_unsigned_response(
            &unsigned,
            network,
            &address,
            std::slice::from_ref(&address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
        )
        .expect("signed response");

        let parsed = parse_generic_envelope_hex(&signed).expect("parsed response");
        let identity_signature =
            parse_identity_signature(&parsed.signature_data.signature_as_vch).unwrap();
        let hash = compute_identity_signature_hash(
            &parsed.signature_data,
            identity_signature.block_height,
            get_raw_envelope_sha256(&parsed),
        )
        .unwrap();
        assert!(verify_identity_signature_against_addresses(
            hash,
            &identity_signature,
            std::slice::from_ref(&address),
            network,
        ));
    }

    #[test]
    fn each_data_item_signature_verifies_for_the_responder() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, address) = test_signing_context(network, 7);
        let open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex("auth-sign-data"),
            true,
        );
        let identity = "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk";
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: identity,
        };
        let signatures =
            super::sign_data_packets(&open, &signer, FIXED_SIGNED_BLOCK_HEIGHT, &private_key)
                .unwrap();
        let expected = vectors()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == "auth-sign-data")
            .unwrap()["expect"]["dataPacket"]["hashes"]
            .clone();
        assert_eq!(signatures.len(), 1);
        assert_eq!(
            signatures[0]
                .iter()
                .map(|item| hex::encode(item.hash))
                .collect::<Vec<_>>(),
            expected
                .as_array()
                .unwrap()
                .iter()
                .map(|hash| hash.as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        );
        let system = crate::core::generic_request::names::decode_i_address(TEST_SYSTEM_ID).unwrap();
        let identity_hash =
            crate::core::generic_request::names::decode_i_address(identity).unwrap();
        for item in &signatures[0] {
            let parsed = parse_identity_signature(&item.signature).unwrap();
            assert_eq!(parsed.block_height, FIXED_SIGNED_BLOCK_HEIGHT);
            let hash = crate::core::crypto::verus_id_signature::compute_verus_data_identity_hash(
                system,
                identity_hash,
                parsed.block_height,
                item.hash,
            );
            assert!(verify_identity_signature_against_addresses(
                hash,
                &parsed,
                std::slice::from_ref(&address),
                network,
            ));
        }
    }

    #[test]
    fn an_encrypted_response_decrypts_to_the_plain_details() {
        let mut open = open_vector_request(
            &GenericRequestStore::new(),
            "session",
            &vector_request_hex("auth-sign-data"),
            true,
        );
        let signer = ResponseSigner {
            system_id: TEST_SYSTEM_ID,
            identity_id: "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk",
        };
        let outcome = ResponseOutcome {
            data_signatures: vec![(0..4)
                .map(|_| ItemSignature {
                    hash: [5; 32],
                    signature: vec![9; 73],
                })
                .collect()],
            ..ResponseOutcome::default()
        };
        let plain_details = response_details(&open.parsed, &open.kinds, &outcome, &signer).unwrap();
        let recipient = vector_app_keys(1);
        open.parsed.encrypt_response_to = Some(recipient.address);
        let encrypted = build_unsigned_response(&open.parsed, plain_details.clone(), &signer, 1)
            .expect("encrypted response");

        let parsed = parse_generic_envelope_hex(&hex::encode(&encrypted)).unwrap();
        assert_eq!(parsed.details.len(), 1);
        assert_eq!(parsed.details[0].ordinal, 0);
        // The descriptor: version, flags (encrypted, epk), ciphertext, epk.
        let mut reader = crate::core::generic_request::wire::Reader::new(&parsed.details[0].data);
        assert_eq!(reader.varint().unwrap(), 1);
        assert_eq!(reader.varint().unwrap(), 5);
        let ciphertext = reader.var_slice().unwrap().to_vec();
        let epk: [u8; 32] = reader.var_slice().unwrap().try_into().unwrap();
        let plaintext = crate::core::generic_request::private_crypto::decrypt_with_viewing_key(
            &recipient.incoming_viewing_key,
            &epk,
            &ciphertext,
        )
        .unwrap();
        // VdxfUniValue: the data descriptor key, version, length, descriptor.
        let mut uni = crate::core::generic_request::wire::Reader::new(&plaintext);
        assert_eq!(
            crate::core::generic_request::names::i_address(&uni.hash160().unwrap()),
            "i4GC1YGEVD21afWudGoFJVdnfjJ5XWnCQv"
        );
        assert_eq!(uni.varint().unwrap(), 1);
        let _length = uni.compact_size().unwrap();
        assert_eq!(uni.varint().unwrap(), 1);
        assert_eq!(uni.varint().unwrap(), 0);
        let inner = uni.var_slice().unwrap();
        let mut expected = Vec::new();
        crate::core::crypto::verus_id_signature::write_compact_size(&mut expected, 2);
        for detail in &plain_details {
            expected.extend_from_slice(detail);
        }
        assert_eq!(inner, expected.as_slice());
    }

    #[test]
    fn a_responder_this_wallet_does_not_control_is_rejected() {
        let network = wallet_network_to_crypto_network(WalletNetwork::Testnet);
        let (private_key, address) = test_signing_context(network, 7);
        let (_, foreign_address) = test_signing_context(network, 8);
        let unsigned = unsigned_vector_response(
            "auth",
            None,
            "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk",
            FIXED_CREATED_AT,
            &[],
            None,
        );
        let error = sign_unsigned_response(
            &unsigned,
            network,
            &address,
            std::slice::from_ref(&foreign_address),
            FIXED_SIGNED_BLOCK_HEIGHT,
            &private_key,
        )
        .expect_err("control check");
        assert!(matches!(error, WalletError::IdentityUnsupportedAuthority));
    }

    #[test]
    fn a_request_the_store_does_not_hold_is_refused() {
        let store = GenericRequestStore::new();
        let request_hex = vector_request_hex("auth");
        assert!(store.get("session", &request_hex).is_err());
        open_vector_request(&store, "session", &request_hex, true);
        assert!(store.get("session", &request_hex).is_ok());
        assert!(store.get("other-session", &request_hex).is_err());
        assert!(store
            .get("session", &vector_request_hex("auth-update"))
            .is_err());
    }

    fn test_signing_context(network: crate::core::crypto::Network, seed: u8) -> ([u8; 32], String) {
        let private_key = [seed; 32];
        let secret_key = SecretKey::from_slice(&private_key).expect("secret key");
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let address = generate_p2pkh_address(&public_key, network).expect("address");
        (private_key, address)
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

    async fn unlock_generic_test_session(session: &Arc<Mutex<SessionManager>>, material: &str) {
        let profile = derive_public_profile_from_material(
            material,
            crate::types::wallet::WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        session.lock().await.unlock_with_profile(
            "generic-account".to_string(),
            WalletNetwork::Mainnet,
            crate::types::wallet::WalletSecretKind::SeedText,
            profile,
            Zeroizing::new(vec![7u8; 32]),
        );
    }

    #[tokio::test]
    async fn watched_funding_lookup_fails_closed_when_encrypted_watchlist_is_corrupt() {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);
        let store_path = std::env::temp_dir().join(format!(
            "lite_wallet_generic_watchlist_{}",
            uuid::Uuid::new_v4()
        ));
        let state = AccountStateStore::new(store_path.clone());
        state
            .store_watchlist_entries(
                "generic-account",
                WalletNetwork::Mainnet,
                &[WatchlistEntry {
                    id: "watched".into(),
                    target_kind: WatchlistTargetKind::Address,
                    display_name: "Watched only".into(),
                    address: "RWatchedFunding".into(),
                    system_id: None,
                    created_at: 1,
                    updated_at: 1,
                }],
            )
            .expect("legacy watchlist");
        let session = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(store_path.clone()),
        )));
        unlock_generic_test_session(&session, "wallet A").await;
        let context = super::identity_session_context(&session, &state)
            .await
            .expect("identity context");
        assert_eq!(
            super::load_watched_funding_addresses(&context)
                .await
                .unwrap(),
            vec!["RWatchedFunding".to_string()]
        );
        let encrypted = store_path.join("accounts/generic-account/watchlist.snapshot.stronghold");
        std::fs::write(&encrypted, b"corrupt").unwrap();
        assert!(super::load_watched_funding_addresses(&context)
            .await
            .is_err());
        let _ = std::fs::remove_dir_all(store_path);
    }

    #[tokio::test]
    async fn build_and_sign_cancels_during_delayed_identity_lookup_after_session_change() {
        // The test Stronghold uses the cheap work factor; without it, this
        // test depends on another test having set it first.
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);
        let store_path = std::env::temp_dir().join(format!(
            "lite_wallet_generic_build_session_{}",
            uuid::Uuid::new_v4()
        ));
        let store = StrongholdStore::new_for_tests(store_path);
        store
            .store_seed("generic-account", "wallet A", &[7u8; 32])
            .await
            .expect("store test seed");
        let session = Arc::new(Mutex::new(SessionManager::new(store)));
        unlock_generic_test_session(&session, "wallet A").await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("wallet A context");

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");
        let request_started = Arc::new(Notify::new());
        let allow_response = Arc::new(Notify::new());
        let active_address = context.vrsc_address.clone();
        let server = tokio::spawn({
            let request_started = Arc::clone(&request_started);
            let allow_response = Arc::clone(&allow_response);
            async move {
                let (mut stream, _) = listener.accept().await.expect("accept request");
                let _request = read_http_request(&mut stream).await;
                request_started.notify_one();
                allow_response.notified().await;
                let body = serde_json::json!({
                    "result": {
                        "identity": {
                            "primaryaddresses": [active_address],
                            "minimumsignatures": 1
                        }
                    },
                    "error": null
                })
                .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });
        let pool = Arc::new(crate::core::channels::vrpc::VrpcProviderPool::for_tests(
            &format!("http://{address}"),
        ));
        let request_hex = vector_request_hex("mainnet-auth");
        let request_store = Arc::new(GenericRequestStore::new());
        open_vector_request(&request_store, &context.session_id, &request_hex, false);
        let operation = tokio::spawn({
            let session = Arc::clone(&session);
            let pool = Arc::clone(&pool);
            let request_store = Arc::clone(&request_store);
            async move {
                build_and_sign_generic_response_for_session(
                    &request_hex,
                    &GenericResponseSignerInput {
                        system_id: MAINNET_VERUS_CHAIN_ID.to_string(),
                        identity_id: "i8KEYF3QeR1xPVVCL5BSpDXkA1CAfFDvmk".to_string(),
                    },
                    &session,
                    &pool,
                    &request_store,
                )
                .await
            }
        });

        request_started.notified().await;
        session.lock().await.lock();
        unlock_generic_test_session(&session, "wallet B").await;
        allow_response.notify_one();

        let error = operation
            .await
            .expect("build task")
            .expect_err("stale build must fail");
        assert!(matches!(error, WalletError::WalletSessionChanged));
        let replacement = capture_active_wallet_access_context(&session)
            .await
            .expect("replacement wallet remains active");
        assert_ne!(replacement.session_id, context.session_id);
        server.await.expect("server completes");
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
        let redirect_url = build_generic_response_redirect_url(
            "valu://callback/finish",
            "deadbeef",
            GENERIC_RESPONSE_DEEPLINK_VDXF_ID,
        )
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
            "version": 3,
            "flags": 0,
            "minimumsignatures": 1,
            "name": "before",
            "parent": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "primaryaddresses": ["03a058410b33f893fe182f15336577f3941c28c8cadcfb0395b9c31dd5c07ccd11"],
            "contentmultimap": {
                "iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c": {"message": "before"}
            },
            "contentmap": {},
            "revocationauthority": "i98Mnj1YugaRzoURXt4aRhdqQDu7rML9J5",
            "recoveryauthority": "i9ps1xDcr7eM66Fko6aTkeuvvBPZFLEXRN",
            "privateaddresses": [],
            "systemid": "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq",
            "timelock": 0
        });
        let patch = serde_json::json!({
            "name": "maxs",
            "contentmultimap": {
                "iKMhRLX1JHQihVZx2t2pAWW2uzmK6AzwW3": {"serializedhex": "010203"}
            }
        });

        let merged = merge_identity_update_patch(&current, &patch);

        assert_eq!(merged["name"], serde_json::json!("maxs"));
        assert_eq!(
            merged["primaryaddresses"],
            serde_json::json!([
                "03a058410b33f893fe182f15336577f3941c28c8cadcfb0395b9c31dd5c07ccd11"
            ])
        );
        assert!(merged["contentmultimap"]
            .get("iK7a5JNJnbeuYWVHCDRpJosj3irGJ5Qa8c")
            .is_some());
        assert!(merged["contentmultimap"]
            .get("iKMhRLX1JHQihVZx2t2pAWW2uzmK6AzwW3")
            .is_some());
        crate::core::channels::vrpc::intent::identity_control_intent_from_json(&merged)
            .expect("GenericRequest merged structured identity remains serializable");
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

        post_generic_response_callback_request(
            format!("http://{}/callback", address),
            CallbackBody::Bytes(hex::decode(&expected_hex).unwrap()),
        )
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

        let error = post_generic_response_callback_request(
            format!("http://{}/callback", address),
            CallbackBody::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        )
        .await
        .expect_err("non-success callback status must fail");
        assert!(matches!(error, WalletError::OperationFailed));

        server.await.expect("server completes");
    }

    #[tokio::test]
    async fn callback_command_maps_http_failure_without_collapsing_it_to_network_error() {
        let store_path = std::env::temp_dir().join(format!(
            "lite_wallet_generic_callback_status_{}",
            uuid::Uuid::new_v4()
        ));
        let session = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(store_path),
        )));
        unlock_generic_test_session(&session, "wallet A").await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("wallet context");
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let _request = read_http_request(&mut stream).await;
            stream
                .write_all(b"HTTP/1.1 422 Unprocessable Entity\r\nContent-Length: 0\r\n\r\n")
                .await
                .expect("write response");
        });

        let error = post_generic_response_callback_for_session(
            format!("http://{address}/callback"),
            CallbackBody::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
            context.session_id,
            &session,
        )
        .await
        .expect_err("non-success callback status must fail");
        assert!(matches!(error, WalletError::OperationFailed));
        server.await.expect("server completes");
    }

    #[tokio::test]
    async fn callback_command_cancels_when_wallet_session_changes_during_http_wait() {
        let store_path = std::env::temp_dir().join(format!(
            "lite_wallet_generic_callback_session_{}",
            uuid::Uuid::new_v4()
        ));
        let session = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(store_path),
        )));
        unlock_generic_test_session(&session, "wallet A").await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("wallet A context");
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("listener address");
        let request_started = Arc::new(Notify::new());
        let allow_response = Arc::new(Notify::new());
        let server = tokio::spawn({
            let request_started = Arc::clone(&request_started);
            let allow_response = Arc::clone(&allow_response);
            async move {
                let (mut stream, _) = listener.accept().await.expect("accept request");
                let _request = read_http_request(&mut stream).await;
                request_started.notify_one();
                allow_response.notified().await;
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                    .await;
            }
        });
        let callback = tokio::spawn({
            let session = Arc::clone(&session);
            let session_id = context.session_id.clone();
            async move {
                post_generic_response_callback_for_session(
                    format!("http://{address}/callback"),
                    CallbackBody::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
                    session_id,
                    &session,
                )
                .await
            }
        });

        request_started.notified().await;
        session.lock().await.lock();
        unlock_generic_test_session(&session, "wallet B").await;
        allow_response.notify_one();

        let error = callback
            .await
            .expect("callback task")
            .expect_err("stale callback must fail");
        assert!(matches!(error, WalletError::WalletSessionChanged));
        let replacement = capture_active_wallet_access_context(&session)
            .await
            .expect("replacement wallet remains active");
        assert_ne!(replacement.session_id, context.session_id);
        server.await.expect("server completes");
    }

    #[tokio::test]
    async fn stale_generic_operation_maps_to_session_changed_without_affecting_replacement_wallet()
    {
        let store_path = std::env::temp_dir().join(format!(
            "lite_wallet_generic_session_{}",
            uuid::Uuid::new_v4()
        ));
        let session = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(store_path),
        )));
        unlock_generic_test_session(&session, "wallet A").await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("wallet A context");
        let guard = context.session_submission_guard();
        let started = Arc::new(Notify::new());
        let task = tokio::spawn({
            let started = Arc::clone(&started);
            async move {
                guard
                    .run(async move {
                        started.notify_one();
                        std::future::pending::<Result<(), WalletError>>().await
                    })
                    .await
                    .map_err(map_generic_session_change)
            }
        });

        started.notified().await;
        session.lock().await.lock();
        unlock_generic_test_session(&session, "wallet B").await;

        let error = task
            .await
            .expect("stale operation task")
            .expect_err("stale operation");
        assert!(matches!(error, WalletError::WalletSessionChanged));
        let replacement = capture_active_wallet_access_context(&session)
            .await
            .expect("replacement wallet remains active");
        assert_ne!(replacement.session_id, context.session_id);
        assert_eq!(replacement.account_id, "generic-account");
    }

    #[test]
    fn generic_response_redirect_url_appends_base64url_response_param() {
        let redirect_url = build_generic_response_redirect_url(
            "https://www.verus.io",
            "deadbeef",
            GENERIC_RESPONSE_DEEPLINK_VDXF_ID,
        )
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
