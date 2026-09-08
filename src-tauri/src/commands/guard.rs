//
// Signed-out guard session commands for revoke/recover flows.

use std::sync::Arc;

use bip39::{Language, Mnemonic};
use secp256k1::SecretKey;
use serde_json::Value;
use tauri::State;
use tokio::sync::Mutex;

use crate::core::auth::GuardSessionManager;
use crate::core::channels::vrpc::identity as vrpc_identity;
use crate::core::channels::vrpc::{self, VrpcProviderPool};
use crate::core::channels::PreflightStore;
use crate::core::coins::CoinRegistry;
use crate::core::crypto::wif_encoding::decode_wif_unchecked_network;
use crate::core::crypto::{derive_keys_from_material, Network};
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use crate::types::{
    BeginGuardSessionRequest, BeginGuardSessionResult, EndGuardSessionRequest,
    EndGuardSessionResult, GuardIdentityLookupRequest, GuardIdentityLookupResult,
    GuardIdentityPreflightRequest, GuardIdentitySendRequest, GuardImportMode, GuardPreflightResult,
    GuardSendResult, WalletError,
};

fn normalize_hex_private_key_candidate(input: &str) -> Option<String> {
    let stripped = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
        .unwrap_or(input);
    if stripped.len() != 64 || !stripped.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let decoded = hex::decode(stripped).ok()?;
    if decoded.len() != 32 {
        return None;
    }
    SecretKey::from_slice(&decoded).ok()?;
    Some(stripped.to_lowercase())
}

fn classify_import_text(import_text: &str) -> Result<(WalletSecretKind, String), WalletError> {
    let trimmed = import_text.trim();
    if trimmed.is_empty() {
        return Err(WalletError::InvalidImportText);
    }

    if decode_wif_unchecked_network(trimmed).is_ok() {
        return Ok((WalletSecretKind::Wif, trimmed.to_string()));
    }

    if let Some(private_key_hex) = normalize_hex_private_key_candidate(trimmed) {
        return Ok((WalletSecretKind::PrivateKeyHex, private_key_hex));
    }

    Ok((WalletSecretKind::SeedText, trimmed.to_string()))
}

fn classify_mnemonic24_import_text(
    import_text: &str,
) -> Result<(WalletSecretKind, String), WalletError> {
    let normalized_words = import_text
        .split_whitespace()
        .map(|word| word.trim().to_lowercase())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    if normalized_words.len() != 24 {
        return Err(WalletError::InvalidImportText);
    }

    let normalized = normalized_words.join(" ");
    if Mnemonic::parse_in(Language::English, &normalized).is_err() {
        return Err(WalletError::InvalidImportText);
    }

    Ok((WalletSecretKind::SeedText, normalized))
}

fn classify_import_text_by_mode(
    import_text: &str,
    import_mode: GuardImportMode,
) -> Result<(WalletSecretKind, String), WalletError> {
    match import_mode {
        GuardImportMode::Mnemonic24 => classify_mnemonic24_import_text(import_text),
        GuardImportMode::TextAuto => classify_import_text(import_text),
    }
}

fn to_derivation_network(network: WalletNetwork) -> Network {
    match network {
        WalletNetwork::Mainnet => Network::Mainnet,
        WalletNetwork::Testnet => Network::Testnet,
    }
}

fn map_identity_lookup_error(err: WalletError) -> WalletError {
    match err {
        WalletError::IdentityRpcUnsupported => WalletError::IdentityRpcUnsupported,
        WalletError::NetworkError => WalletError::NetworkError,
        WalletError::IdentityNotFound
        | WalletError::InvalidAddress
        | WalletError::OperationFailed => WalletError::IdentityNotFound,
        _ => WalletError::IdentityNotFound,
    }
}

fn parse_identity_lookup_exists(raw: &Value) -> bool {
    raw.get("status").and_then(Value::as_str).is_some() && raw.get("identity").is_some()
}

fn secret_kind_label(kind: WalletSecretKind) -> &'static str {
    match kind {
        WalletSecretKind::SeedText => "seed_text",
        WalletSecretKind::Wif => "wif",
        WalletSecretKind::PrivateKeyHex => "private_key_hex",
    }
}

/// Begin a temporary in-memory guard session from imported secret material.
#[tauri::command(rename_all = "snake_case")]
pub async fn begin_guard_session(
    request: BeginGuardSessionRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
) -> Result<BeginGuardSessionResult, WalletError> {
    let (secret_kind, normalized_secret) =
        classify_import_text_by_mode(&request.import_text, request.import_mode)?;
    let keys = derive_keys_from_material(
        &normalized_secret,
        secret_kind,
        to_derivation_network(request.network),
    )?;

    let mut guard = guard_session_manager.lock().await;
    let session = guard.begin_session(keys, request.network);

    Ok(BeginGuardSessionResult {
        guard_session_id: session.id,
        secret_kind: secret_kind_label(secret_kind).to_string(),
        vrsc_address: session.keys.address.clone(),
        eth_address: session.keys.eth_address.clone(),
        btc_address: session.keys.btc_address.clone(),
        network: session.network,
    })
}

/// End a guard session and zeroize its keys.
#[tauri::command(rename_all = "snake_case")]
pub async fn end_guard_session(
    request: EndGuardSessionRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
) -> Result<EndGuardSessionResult, WalletError> {
    let mut guard = guard_session_manager.lock().await;
    Ok(EndGuardSessionResult {
        ended: guard.end_session(&request.guard_session_id),
    })
}

/// Resolve a target identity by i-address or handle in signed-out guard mode.
#[tauri::command(rename_all = "snake_case")]
pub async fn lookup_guard_target_identity(
    request: GuardIdentityLookupRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GuardIdentityLookupResult, WalletError> {
    let guard_session = {
        let mut guard = guard_session_manager.lock().await;
        guard.get_session(&request.guard_session_id)?
    };

    let target_identity = request.target_identity.trim();
    if target_identity.is_empty() {
        return Ok(GuardIdentityLookupResult { exists: false });
    }

    let raw = vrpc_provider_pool
        .for_network(guard_session.network)
        .getidentity(target_identity)
        .await
        .map_err(map_identity_lookup_error)?;

    Ok(GuardIdentityLookupResult {
        exists: parse_identity_lookup_exists(&raw),
    })
}

/// Preflight identity update/revoke/recover in signed-out guard mode.
#[tauri::command(rename_all = "snake_case")]
pub async fn preflight_guard_identity_update(
    request: GuardIdentityPreflightRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    coin_registry: State<'_, Arc<CoinRegistry>>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GuardPreflightResult, WalletError> {
    let guard_session = {
        let mut guard = guard_session_manager.lock().await;
        guard.get_session(&request.guard_session_id)?
    };

    let resolved = vrpc::parse_vrpc_channel_id(
        &request.params.channel_id,
        Some(&guard_session.keys.address),
    )?;
    if resolved.address != guard_session.keys.address {
        return Err(WalletError::InvalidAddress);
    }

    let is_testnet = matches!(guard_session.network, WalletNetwork::Testnet);
    if coin_registry
        .find_by_system_id(&resolved.system_id, is_testnet)
        .is_none()
    {
        return Err(WalletError::UnsupportedChannel);
    }

    let canonical_channel_id =
        vrpc::canonical_vrpc_channel_id(&resolved.address, &resolved.system_id);
    let guard_account_id = format!("guard:{}", request.guard_session_id);

    vrpc_identity::preflight(
        request.params,
        &preflight_store,
        &guard_account_id,
        &request.guard_session_id,
        &resolved.address,
        &canonical_channel_id,
        vrpc_provider_pool.for_network(guard_session.network),
    )
    .await
}

/// Broadcast identity tx in signed-out guard mode.
#[tauri::command(rename_all = "snake_case")]
pub async fn send_guard_identity_update(
    request: GuardIdentitySendRequest,
    guard_session_manager: State<'_, Arc<Mutex<GuardSessionManager>>>,
    preflight_store: State<'_, PreflightStore>,
    vrpc_provider_pool: State<'_, Arc<VrpcProviderPool>>,
) -> Result<GuardSendResult, WalletError> {
    let guard_session = {
        let mut guard = guard_session_manager.lock().await;
        guard.get_session(&request.guard_session_id)?
    };

    let guard_account_id = format!("guard:{}", request.guard_session_id);
    vrpc_identity::send_with_signing_material(
        &request.preflight_id,
        &preflight_store,
        &guard_account_id,
        &request.guard_session_id,
        &guard_session.keys.wif,
        guard_session.network,
        vrpc_provider_pool.inner().as_ref(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crypto::{derive_keys_v1, Network};
    use serde_json::json;

    const VALID_MNEMONIC_24: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    #[test]
    fn classify_mnemonic24_accepts_valid_24_word_phrase() {
        let (secret_kind, normalized) =
            classify_import_text_by_mode(VALID_MNEMONIC_24, GuardImportMode::Mnemonic24)
                .expect("valid 24-word mnemonic should be accepted");

        assert_eq!(secret_kind, WalletSecretKind::SeedText);
        assert_eq!(normalized, VALID_MNEMONIC_24);
    }

    #[test]
    fn classify_mnemonic24_rejects_invalid_word_count() {
        let invalid_word_count =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
        let result = classify_import_text_by_mode(invalid_word_count, GuardImportMode::Mnemonic24);

        assert!(matches!(result, Err(WalletError::InvalidImportText)));
    }

    #[test]
    fn classify_mnemonic24_rejects_malformed_phrase() {
        let malformed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon notaword";
        let result = classify_import_text_by_mode(malformed, GuardImportMode::Mnemonic24);

        assert!(matches!(result, Err(WalletError::InvalidImportText)));
    }

    #[test]
    fn classify_text_auto_resolves_wif() {
        let keys =
            derive_keys_v1("guard-wif-classification", Network::Mainnet).expect("derive keys");
        let (secret_kind, normalized) =
            classify_import_text_by_mode(&keys.wif, GuardImportMode::TextAuto)
                .expect("wif should be classified correctly");

        assert_eq!(secret_kind, WalletSecretKind::Wif);
        assert_eq!(normalized, keys.wif);
    }

    #[test]
    fn classify_text_auto_resolves_private_key_hex() {
        let keys =
            derive_keys_v1("guard-hex-classification", Network::Mainnet).expect("derive keys");
        let hex_with_prefix = format!("0x{}", keys.eth_private_key.to_uppercase());
        let (secret_kind, normalized) =
            classify_import_text_by_mode(&hex_with_prefix, GuardImportMode::TextAuto)
                .expect("hex key should be classified correctly");

        assert_eq!(secret_kind, WalletSecretKind::PrivateKeyHex);
        assert_eq!(normalized, keys.eth_private_key);
    }

    #[test]
    fn classify_text_auto_falls_back_to_seed_text() {
        let seed_text = "custom free form seed text";
        let (secret_kind, normalized) =
            classify_import_text_by_mode(seed_text, GuardImportMode::TextAuto)
                .expect("seed text fallback should be accepted");

        assert_eq!(secret_kind, WalletSecretKind::SeedText);
        assert_eq!(normalized, seed_text);
    }

    #[test]
    fn parse_identity_lookup_exists_returns_true_for_status_and_identity_payload() {
        let raw = json!({
            "status": "active",
            "identity": {
                "name": "alice"
            }
        });
        assert!(parse_identity_lookup_exists(&raw));
    }

    #[test]
    fn parse_identity_lookup_exists_returns_false_for_missing_identity_payload() {
        let raw = json!({
            "status": "active"
        });
        assert!(!parse_identity_lookup_exists(&raw));
    }

    #[test]
    fn identity_lookup_error_mapping_preserves_network_error() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::NetworkError),
            WalletError::NetworkError
        ));
    }

    #[test]
    fn identity_lookup_error_mapping_maps_lookup_failures_to_not_found() {
        assert!(matches!(
            map_identity_lookup_error(WalletError::OperationFailed),
            WalletError::IdentityNotFound
        ));
        assert!(matches!(
            map_identity_lookup_error(WalletError::InvalidAddress),
            WalletError::IdentityNotFound
        ));
    }
}
