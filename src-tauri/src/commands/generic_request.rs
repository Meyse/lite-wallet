use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{
    engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

use crate::commands::identity::{
    build_identity_details_from_payload, identity_session_context, linked_identity_from_details,
    load_linked_for_context, map_identity_lookup_error as map_link_identity_lookup_error,
    parse_getidentity_payload, store_linked_for_context, upsert_linked_identity,
};
use crate::core::auth::SessionManager;
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
    compute_identity_signature_hash, encode_compact_i_address, extract_chain_height,
    extract_primary_addresses, get_raw_envelope_sha256, parse_generic_envelope_hex,
    parse_identity_signature, sign_identity_hash, validate_identity_control_for_active_wallet,
    verify_generic_request_signature_with_provider, verify_identity_signature_against_addresses,
    write_compact_size, write_var_slice, write_varint,
};
use crate::core::crypto::Network;
use crate::core::PreflightStore;
use crate::types::wallet::WalletNetwork;
use crate::types::{
    BuildAndSignGenericResponseRequest, BuildAndSignGenericResponseResult,
    GenericIdentityAuthorities, GenericIdentityPrimaryAddressEntry,
    GenericIdentityPrimaryAddressInfo, GenericIdentityUpdatePreflightResult,
    GenericIdentityUpdateRequestMeta, GenericRequestVerificationResult, IdentityOperation,
    IdentityWarning, LinkReadyProvisioningJobResult, ProvisioningJobRecord, WalletError,
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

fn parse_generic_response_callback_uri(uri: &str) -> Result<reqwest::Url, WalletError> {
    let parsed = reqwest::Url::parse(uri.trim()).map_err(|_| WalletError::OperationFailed)?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        _ => Err(WalletError::OperationFailed),
    }
}

fn build_generic_response_redirect_url(
    callback_uri: &str,
    response_hex: &str,
) -> Result<reqwest::Url, WalletError> {
    let mut redirect_url = parse_generic_response_callback_uri(callback_uri)?;
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
    wif: &str,
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
    let signature_as_vch = sign_identity_hash(identity_hash, signed_block_height, wif, network)?;
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
    context
        .stronghold_store
        .store_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
            jobs,
        )
        .await?;
    context
        .stronghold_store
        .load_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await
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
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let (active_wallet_primary_address, _, _) = session.get_addresses()?;
    let wif = session.get_wif_for_signing()?;
    drop(session);

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
    let signature_as_vch = sign_identity_hash(
        identity_hash,
        signed_block_height,
        &wif,
        wallet_network_to_crypto_network(network),
    )?;

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
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let (active_wallet_primary_address, _, _) = session.get_addresses()?;
    let wif = session.get_wif_for_signing()?;
    drop(session);

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
        &wif,
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
    let callback_uri = parse_generic_response_callback_uri(&callback_uri)?;
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
pub async fn sign_identity_signature_hash(
    hash_hex: String,
    system_id: String,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<String, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    let wif = session.get_wif_for_signing()?;
    drop(session);

    let trimmed_system_id = system_id.trim();
    if trimmed_system_id.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let decoded_hash = hex::decode(hash_hex.trim()).map_err(|_| WalletError::OperationFailed)?;
    let hash_bytes: [u8; 32] = decoded_hash
        .as_slice()
        .try_into()
        .map_err(|_| WalletError::OperationFailed)?;

    let provider = vrpc_provider_pool.for_system(network, trimmed_system_id);
    let info = provider.getinfo().await?;
    let signed_block_height = extract_chain_height(&info)?;
    let signature_as_vch = sign_identity_hash(
        hash_bytes,
        signed_block_height,
        &wif,
        wallet_network_to_crypto_network(network),
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

#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_generic_identity_update(
    requested_identity_json: Value,
    target_identity_address: String,
    source_channel_id: String,
    request_meta: Option<GenericIdentityUpdateRequestMeta>,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GenericIdentityUpdatePreflightResult, WalletError> {
    let session = session_manager.lock().await;
    if !session.is_unlocked() {
        return Err(WalletError::WalletLocked);
    }

    let account_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let (session_vrpc_address, _, _) = session.get_addresses()?;
    let network = session.active_network().unwrap_or(WalletNetwork::Mainnet);
    drop(session);

    if !requested_identity_json.is_object() {
        return Err(WalletError::IdentityBuildFailed);
    }

    let resolved = vrpc::parse_vrpc_channel_id(&source_channel_id, Some(&session_vrpc_address))?;
    if resolved.address != session_vrpc_address {
        return Err(WalletError::InvalidAddress);
    }

    let canonical_channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
    let provider = vrpc_provider_pool.for_system(network, &resolved.system_id);

    let info = provider.getinfo().await?;
    let current_height = extract_chain_height(&info)?;
    if let Some(expiry_height) = request_meta
        .as_ref()
        .and_then(|meta| meta.expiry_height)
        .filter(|height| current_height > *height)
    {
        let _ = expiry_height;
        return Err(WalletError::IdentityRequestExpired);
    }

    let raw_target = provider
        .getidentity(target_identity_address.trim())
        .await
        .map_err(map_preflight_identity_lookup_error)?;
    let target = parse_target_identity(raw_target.clone())?;

    validate_target_state(&target.status, &IdentityOperation::Update)?;
    validate_operation_authority(
        provider,
        &IdentityOperation::Update,
        &target.identity,
        &target.status,
        &resolved.address,
    )
    .await?;

    validate_unsupported_identity_flags(&target.identity, &requested_identity_json)?;

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

    preflight_store.put_with_ttl(
        preflight_id.clone(),
        PreflightRecord {
            channel_id: canonical_channel_id,
            account_id,
            payload: payload_value,
        },
        Some(IDENTITY_PREFLIGHT_TTL),
    );

    let mut warnings = Vec::<IdentityWarning>::new();
    if target.identity == requested_identity_json {
        warnings.push(IdentityWarning {
            warning_type: "no_effect".to_string(),
            message: "Request does not change identity fields.".to_string(),
        });
    }
    if dropped_dust_change {
        warnings.push(IdentityWarning {
            warning_type: "dust_change".to_string(),
            message: "Change below dust threshold is added to fee.".to_string(),
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
    let high_risk_changes = classify_high_risk_changes(&target.identity, &requested_identity_json);
    let friendly_names = resolve_identity_friendly_names(
        provider,
        &target.identity,
        request_meta
            .as_ref()
            .and_then(|meta| meta.signer_identity_id.as_deref()),
    )
    .await;

    Ok(GenericIdentityUpdatePreflightResult {
        preflight_id,
        target_identity: target_identity_address.trim().to_string(),
        from_address: resolved.address.clone(),
        fee,
        fee_currency: resolved.system_id,
        warnings,
        high_risk_changes,
        current_identity: target.identity.clone(),
        requested_identity: requested_identity_json.clone(),
        fully_qualified_name,
        friendly_names,
        signer_cmm_key_labels: HashMap::new(),
        primary_address_after_update_info: build_primary_address_update_info(
            &requested_identity_json,
            &session_vrpc_address,
        ),
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
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_identity_provisioning_jobs(
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
    let context = identity_session_context(session_manager.inner()).await?;
    let mut jobs = context
        .stronghold_store
        .load_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await?;

    if expire_provisioning_jobs(&mut jobs, now_unix_seconds()) {
        jobs = store_provisioning_jobs_for_context(&context, &jobs).await?;
    }

    Ok(jobs)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn store_generic_provisioning_job(
    request: StoreGenericProvisioningJobRequest,
    session_manager: State<'_, Arc<Mutex<SessionManager>>>,
) -> Result<ProvisioningJobRecord, WalletError> {
    let context = identity_session_context(session_manager.inner()).await?;
    let mut jobs = context
        .stronghold_store
        .load_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await?;

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
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<Vec<ProvisioningJobRecord>, WalletError> {
    let context = identity_session_context(session_manager.inner()).await?;
    let mut jobs = context
        .stronghold_store
        .load_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await?;

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
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<LinkReadyProvisioningJobResult, WalletError> {
    let requested_job_id = job_id.trim();
    if requested_job_id.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let context = identity_session_context(session_manager.inner()).await?;
    let mut jobs = context
        .stronghold_store
        .load_provisioning_jobs(
            &context.account_id,
            context.password_hash.as_ref(),
            context.network,
        )
        .await?;

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
    signer_identity_id: Option<&str>,
) -> HashMap<String, String> {
    let mut names = HashMap::<String, String>::new();

    for identity_id in [
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
        signer_identity_id.map(|value| value.trim().to_string()),
    ]
    .into_iter()
    .flatten()
    {
        if names.contains_key(&identity_id) {
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
        build_unsigned_generic_response_hex, parse_generic_response_callback_uri,
        post_generic_response_callback, wallet_network_to_crypto_network,
        BuildAndSignGenericResponseRequest, WalletNetwork,
        GENERIC_RESPONSE_FLAG_HAS_CREATED_AT, GENERIC_RESPONSE_FLAG_IS_TESTNET,
        GENERIC_RESPONSE_FLAG_MULTI_DETAILS, GENERIC_RESPONSE_FLAG_SIGNED, HASH_TYPE_SHA256,
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
    use crate::core::crypto::wif_encoding::{encode_wif, generate_p2pkh_address};
    use crate::types::{
        GenericAuthenticationResponseInput, GenericIdentityUpdateResponseInput,
        GenericResponseSignerInput, WalletError,
    };
    use secp256k1::{PublicKey, Secp256k1, SecretKey};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const TEST_SYSTEM_ID: &str = "iJhCezBExJHvtyH3fGhNnt2NhU4Ztkf2yq";
    const TEST_SIGNER_ID: &str = "i89UVSuN6vfWg1mWpXMuc6dsJBdeYTi7bX";
    const TEST_AUTH_REQUEST_ID: &str = "iBxsUePVWyf4QuM1b8wP7Np1SUJhszwyin";
    const FIXED_CREATED_AT: u64 = 1_700_000_000;
    const FIXED_SIGNED_BLOCK_HEIGHT: u32 = 965_771;

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

    fn test_signing_context(network: crate::core::crypto::Network, seed: u8) -> (String, String) {
        let private_key = [seed; 32];
        let wif = encode_wif(&private_key, network).expect("wif");
        let secret_key = SecretKey::from_slice(&private_key).expect("secret key");
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let address = generate_p2pkh_address(&public_key, network).expect("address");
        (wif, address)
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
        let (wif, active_wallet_primary_address) = test_signing_context(network, 7);
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
            &wif,
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
        let (wif, active_wallet_primary_address) = test_signing_context(network, 7);
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
            &wif,
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
        let (wif, active_wallet_primary_address) = test_signing_context(network, 7);
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
            &wif,
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
        let (wif, active_wallet_primary_address) = test_signing_context(network, 7);
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
            &wif,
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
    fn callback_uri_rejects_non_http_schemes() {
        let error = parse_generic_response_callback_uri("ftp://example.com/callback")
            .expect_err("non-http scheme should fail");
        assert!(matches!(error, WalletError::OperationFailed));
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
        let redirect_url =
            build_generic_response_redirect_url("https://www.verus.io", "deadbeef")
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
