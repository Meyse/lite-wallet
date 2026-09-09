use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;
use zcash_client_backend::encoding::encode_payment_address;
use zcash_client_backend::keys::{ReceiverRequirement, UnifiedAddressRequest, UnifiedSpendingKey};
use zcash_keys::encoding::encode_extended_spending_key;
use zcash_protocol::consensus::{MainNetwork, Parameters};
use zcash_protocol::constants::mainnet;
#[cfg(test)]
use zcash_protocol::constants::testnet;
use zip32::AccountId;

use crate::types::transaction::{BalanceResult, Transaction};
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

mod cache;
mod consensus;
mod destination;
mod preflight;
mod reader;
mod recipient_resolution;
mod runtime;
mod send;
mod spend_db;
mod spend_engine;
mod spend_keys;
mod spend_params;
mod spend_sync;
mod state;
mod store;
mod synchronizer;

pub use consensus::{DlightBirthday, DlightSeedMetadata};
pub use preflight::{preflight, DlightPreflightPayload};
pub(super) use runtime::cached_request as cached_runtime_request;
pub(crate) use runtime::creation_birthday;
pub use runtime::stop_all_runtimes;
pub use send::send;
pub use spend_params::{DlightProverFileDiagnostics, DlightProverStatus};
pub use synchronizer::{DlightSynchronizerAdapter, DlightSynchronizerRuntimeAdapter};

const SAPLING_ADDRESS_REQUEST: UnifiedAddressRequest = UnifiedAddressRequest::unsafe_custom(
    ReceiverRequirement::Omit,
    ReceiverRequirement::Require,
    ReceiverRequirement::Omit,
);

#[derive(Debug, Clone)]
pub struct DlightChannelRef {
    pub address: String,
    pub system_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct DlightInfo {
    pub blocks: Option<u64>,
    pub longest_chain: Option<u64>,
    pub syncing: bool,
    pub percent: Option<f64>,
    pub status_kind: Option<String>,
    pub last_updated: Option<u64>,
    pub last_progress_at: Option<u64>,
    pub stalled: Option<bool>,
    pub scan_rate_blocks_per_sec: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct DlightRuntimeDiagnostics {
    pub runtime_key: String,
    pub status_kind: String,
    pub percent: Option<f64>,
    pub scanned_height: u64,
    pub tip_height: Option<u64>,
    pub estimated_tip_height: Option<u64>,
    pub syncing: bool,
    pub last_updated: u64,
    pub last_progress_at: Option<u64>,
    pub last_tip_probe_at: Option<u64>,
    pub consecutive_failures: u32,
    pub scan_rate_blocks_per_sec: Option<f64>,
    pub stalled: bool,
    pub last_error: Option<String>,
    pub spend_cache_ready: Option<bool>,
    pub spend_cache_status_kind: Option<String>,
    pub spend_cache_percent: Option<f64>,
    pub spend_cache_lag_blocks: Option<u64>,
    pub spend_cache_last_error: Option<String>,
    pub spend_cache_scanned_height: Option<u64>,
    pub spend_cache_tip_height: Option<u64>,
    pub spend_cache_last_updated: Option<u64>,
    pub spend_cache_note_count: Option<u64>,
}

#[derive(Clone)]
pub struct DlightRuntimeRequest {
    pub runtime_key: String,
    pub endpoint: String,
    pub scope_address: String,
    pub scope_system_id: String,
    pub coin_id: String,
    pub network: WalletNetwork,
    pub seed_material: zeroize::Zeroizing<String>,
    pub session_id: String,
    pub(crate) submission_guard: crate::core::auth::SessionSubmissionGuard,
    pub birthday: Option<DlightBirthday>,
    pub account_hash: String,
    pub app_data_dir: PathBuf,
}

type DlightAdapterHandle = Arc<dyn DlightSynchronizerAdapter>;

fn adapter_store() -> &'static Mutex<DlightAdapterHandle> {
    static ADAPTER: OnceLock<Mutex<DlightAdapterHandle>> = OnceLock::new();
    ADAPTER.get_or_init(|| Mutex::new(Arc::new(DlightSynchronizerRuntimeAdapter::default())))
}

fn active_adapter() -> DlightAdapterHandle {
    adapter_store()
        .lock()
        .map(|guard| Arc::clone(&*guard))
        .unwrap_or_else(|_| Arc::new(DlightSynchronizerRuntimeAdapter::default()))
}

pub fn install_synchronizer_adapter(adapter: DlightAdapterHandle) {
    if let Ok(mut guard) = adapter_store().lock() {
        *guard = adapter;
    }
}

pub fn canonical_dlight_channel_id(address: &str, system_id: &str) -> String {
    format!("dlight_private.{}.{}", address.trim(), system_id.trim())
}

pub fn parse_dlight_channel_id(channel_id: &str) -> Result<DlightChannelRef, WalletError> {
    let rest = channel_id
        .strip_prefix("dlight_private.")
        .ok_or(WalletError::UnsupportedChannel)?;
    let (address, system_id) = rest
        .split_once('.')
        .ok_or(WalletError::UnsupportedChannel)?;

    let address = address.trim();
    let system_id = system_id.trim();
    if address.is_empty() || system_id.is_empty() {
        return Err(WalletError::UnsupportedChannel);
    }

    Ok(DlightChannelRef {
        address: address.to_string(),
        system_id: system_id.to_string(),
    })
}

pub(super) fn ensure_runtime_ready_for_spend(
    status_kind: runtime::RuntimeStatusKind,
) -> Result<(), WalletError> {
    match status_kind {
        runtime::RuntimeStatusKind::Synced => Ok(()),
        runtime::RuntimeStatusKind::Initializing | runtime::RuntimeStatusKind::Syncing => {
            Err(WalletError::DlightSynchronizerNotReady)
        }
        runtime::RuntimeStatusKind::Error => Err(WalletError::NetworkError),
    }
}

pub(super) fn ensure_spend_cache_ready(
    request: &DlightRuntimeRequest,
    runtime_snapshot: &runtime::RuntimeSnapshot,
) -> Result<(), WalletError> {
    let runtime_tip_hint = runtime_snapshot
        .chain_tip_height
        .or(runtime_snapshot.estimated_tip_height)
        .filter(|tip| *tip > 0);
    let spend_status = spend_sync::get_spend_cache_status(request, runtime_tip_hint)
        .ok_or(WalletError::DlightSpendCacheNotReady)?;
    if spend_status.ready {
        Ok(())
    } else {
        Err(WalletError::DlightSpendCacheNotReady)
    }
}

fn is_dlight_spending_key(value: &str) -> bool {
    value.trim().starts_with("secret-extended-key-")
}

fn derive_scope_address_from_spending_key(
    spending_key: &str,
    network: WalletNetwork,
) -> Result<String, WalletError> {
    let payment_address_hrp = sapling_payment_address_hrp(network);

    let extsk = spend_keys::decode_spending_key(spending_key)?;
    let (_diversifier_index, payment_address) = extsk.default_address();
    Ok(encode_payment_address(
        payment_address_hrp,
        &payment_address,
    ))
}

fn derive_scope_address_for_network<P: Parameters>(
    seed_bytes: &[u8],
    network: &P,
    payment_address_hrp: &str,
) -> Result<String, WalletError> {
    let usk = UnifiedSpendingKey::from_seed(network, seed_bytes, AccountId::ZERO)
        .map_err(|_| WalletError::InvalidSeedPhrase)?;
    let ufvk = usk.to_unified_full_viewing_key();
    let (address, _diversifier_index) = ufvk
        .default_address(SAPLING_ADDRESS_REQUEST)
        .map_err(|_| WalletError::OperationFailed)?;

    address
        .sapling()
        .map(|sapling| encode_payment_address(payment_address_hrp, sapling))
        .ok_or(WalletError::OperationFailed)
}

pub fn derive_scope_address(
    seed_or_spending_key: &str,
    network: WalletNetwork,
) -> Result<String, WalletError> {
    let normalized = seed_or_spending_key.trim();
    if normalized.is_empty() {
        return Err(WalletError::InvalidSeedPhrase);
    }

    if is_dlight_spending_key(normalized) {
        return derive_scope_address_from_spending_key(normalized, network);
    }

    let mnemonic =
        bip39::Mnemonic::parse(normalized).map_err(|_| WalletError::InvalidSeedPhrase)?;
    let seed_bytes = zeroize::Zeroizing::new(mnemonic.to_seed_normalized(""));
    let payment_address_hrp = sapling_payment_address_hrp(network);

    // Verus Mobile derives m/32'/133'/0' for both chains. The selected
    // network controls synchronization and consensus, never this key path.
    derive_scope_address_for_network(seed_bytes.as_slice(), &MainNetwork, payment_address_hrp)
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn sapling_extsk_hrp(_network: WalletNetwork) -> &'static str {
    mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY
}

pub fn derive_spending_key(
    seed_or_spending_key: &str,
    network: WalletNetwork,
) -> Result<String, WalletError> {
    let normalized = seed_or_spending_key.trim();
    if normalized.is_empty() {
        return Err(WalletError::InvalidSeedPhrase);
    }

    let hrp = sapling_extsk_hrp(network);
    if is_dlight_spending_key(normalized) {
        let extsk = spend_keys::decode_spending_key(normalized)?;
        return Ok(encode_extended_spending_key(hrp, &extsk));
    }

    let mnemonic =
        bip39::Mnemonic::parse(normalized).map_err(|_| WalletError::InvalidSeedPhrase)?;
    let seed_bytes = zeroize::Zeroizing::new(mnemonic.to_seed_normalized(""));
    let extsk = UnifiedSpendingKey::from_seed(&MainNetwork, seed_bytes.as_slice(), AccountId::ZERO)
        .map_err(|_| WalletError::InvalidSeedPhrase)?
        .sapling()
        .clone();

    Ok(encode_extended_spending_key(hrp, &extsk))
}

pub fn runtime_spending_key(
    secret: &str,
    network: WalletNetwork,
) -> Result<zeroize::Zeroizing<String>, WalletError> {
    derive_spending_key(secret, network).map(zeroize::Zeroizing::new)
}

pub fn normalize_endpoint_url(endpoint: &str) -> Result<String, WalletError> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(WalletError::UnsupportedChannel);
    }

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Ok(if trimmed.ends_with('/') {
            trimmed.to_string()
        } else {
            format!("{trimmed}/")
        });
    }
    if trimmed.contains("://") {
        return Err(WalletError::UnsupportedChannel);
    }
    Ok(format!("https://{trimmed}/"))
}

pub fn normalize_grpc_endpoint(endpoint: &str) -> Result<String, WalletError> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(WalletError::UnsupportedChannel);
    }

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Ok(trimmed.to_string());
    }
    if trimmed.contains("://") {
        return Err(WalletError::UnsupportedChannel);
    }
    Ok(format!("https://{trimmed}"))
}

fn as_u64(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    if let Some(number) = value.as_u64() {
        return Some(number);
    }
    if let Some(number) = value.as_i64() {
        return u64::try_from(number).ok();
    }
    value
        .as_str()
        .and_then(|text| text.trim().parse::<u64>().ok())
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    if let Some(number) = value.as_f64() {
        return Some(number);
    }
    if let Some(number) = value.as_i64() {
        return Some(number as f64);
    }
    value
        .as_str()
        .and_then(|text| text.trim().parse::<f64>().ok())
}

pub fn info_from_getinfo(value: &Value) -> DlightInfo {
    let blocks = as_u64(value.get("blocks"));
    let longest_chain = as_u64(
        value
            .get("longestchain")
            .or_else(|| value.get("longest_chain"))
            .or_else(|| value.get("headers")),
    );
    let percent = as_f64(value.get("percent")).or_else(|| {
        as_f64(value.get("verificationprogress")).map(|progress| {
            if progress <= 1.0 {
                progress * 100.0
            } else {
                progress
            }
        })
    });

    let syncing = if let Some(percent) = percent {
        (percent - 100.0).abs() > f64::EPSILON && (percent + 1.0).abs() > f64::EPSILON
    } else if let (Some(blocks), Some(longest_chain)) = (blocks, longest_chain) {
        longest_chain > 0 && blocks < longest_chain
    } else {
        false
    };

    DlightInfo {
        blocks,
        longest_chain,
        syncing,
        percent,
        status_kind: None,
        last_updated: None,
        last_progress_at: None,
        stalled: None,
        scan_rate_blocks_per_sec: None,
    }
}

pub async fn get_balances(request: DlightRuntimeRequest) -> Result<BalanceResult, WalletError> {
    active_adapter().get_balances(&request).await
}

pub async fn get_transactions(
    request: DlightRuntimeRequest,
) -> Result<Vec<Transaction>, WalletError> {
    active_adapter().get_transactions(&request).await
}

pub async fn get_info(request: DlightRuntimeRequest) -> Result<DlightInfo, WalletError> {
    active_adapter().get_info(&request).await
}

pub async fn get_runtime_diagnostics(
    request: DlightRuntimeRequest,
) -> Result<DlightRuntimeDiagnostics, WalletError> {
    active_adapter().get_runtime_diagnostics(&request).await
}

pub fn get_prover_status() -> DlightProverStatus {
    spend_params::get_prover_status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_from_getinfo_supports_blockchain_info_shape() {
        let payload = serde_json::json!({
            "blocks": 42,
            "headers": 84,
            "verificationprogress": 0.75
        });

        let info = info_from_getinfo(&payload);
        assert_eq!(info.blocks, Some(42));
        assert_eq!(info.longest_chain, Some(84));
        assert_eq!(info.percent, Some(75.0));
        assert!(info.syncing);
    }

    #[test]
    fn canonical_dlight_channel_id_has_expected_shape() {
        let channel = canonical_dlight_channel_id("zsTest", "iSystem");
        assert_eq!(channel, "dlight_private.zsTest.iSystem");
    }

    #[test]
    fn runtime_ready_gate_requires_synced_status() {
        assert!(ensure_runtime_ready_for_spend(runtime::RuntimeStatusKind::Synced).is_ok());
        assert!(matches!(
            ensure_runtime_ready_for_spend(runtime::RuntimeStatusKind::Initializing),
            Err(WalletError::DlightSynchronizerNotReady)
        ));
        assert!(matches!(
            ensure_runtime_ready_for_spend(runtime::RuntimeStatusKind::Syncing),
            Err(WalletError::DlightSynchronizerNotReady)
        ));
        assert!(matches!(
            ensure_runtime_ready_for_spend(runtime::RuntimeStatusKind::Error),
            Err(WalletError::NetworkError)
        ));
    }
    #[test]
    fn both_networks_match_verus_mobile_coin_133() {
        let mnemonic = bip39::Mnemonic::from_entropy(&[0; 32]).unwrap().to_string();
        let testnet_key = runtime_spending_key(&mnemonic, WalletNetwork::Testnet).unwrap();
        let mainnet_key = runtime_spending_key(&mnemonic, WalletNetwork::Mainnet).unwrap();
        assert_eq!(*mainnet_key, *testnet_key);
        let address = derive_scope_address(&testnet_key, WalletNetwork::Testnet).unwrap();
        assert_eq!(
            address,
            derive_scope_address(&mnemonic, WalletNetwork::Testnet).unwrap()
        );
        assert_eq!(
            address,
            derive_scope_address(&mnemonic, WalletNetwork::Mainnet).unwrap()
        );
        let seed = bip39::Mnemonic::parse(&mnemonic)
            .unwrap()
            .to_seed_normalized("");
        let independently_derived = sapling::zip32::ExtendedSpendingKey::from_path(
            &sapling::zip32::ExtendedSpendingKey::master(&seed),
            &[
                zip32::ChildIndex::hardened(32),
                zip32::ChildIndex::hardened(133),
                zip32::ChildIndex::hardened(0),
            ],
        );
        assert_eq!(
            address,
            encode_payment_address(
                mainnet::HRP_SAPLING_PAYMENT_ADDRESS,
                &independently_derived.default_address().1
            )
        );
        // A spending key already identifies its derivation; import it as-is.
        let test_prefix = encode_extended_spending_key(
            testnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
            &independently_derived,
        );
        assert_eq!(
            address,
            derive_scope_address(&test_prefix, WalletNetwork::Testnet).unwrap()
        );
        let keys = spend_keys::DlightSpendKeyMaterial::from_seed_material(
            &testnet_key,
            WalletNetwork::Testnet,
            &address,
        )
        .unwrap();
        assert_ne!(*keys.cache_key(b"one"), *keys.cache_key(b"two"));
    }
}
