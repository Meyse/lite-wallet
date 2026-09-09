use super::{
    cache::CacheStore,
    consensus, normalize_grpc_endpoint,
    spend_keys::DlightSpendKeyMaterial,
    spend_sync::{SpendCacheStatus, SpendSyncSnapshot},
    state::{Checkpoint, WalletState},
    store::{resolve_paths, unix_timestamp_secs},
    DlightInfo, DlightRuntimeRequest,
};
use crate::types::{wallet::WalletNetwork, WalletError};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};
use zcash_client_backend::{
    keys::UnifiedFullViewingKey,
    proto::{
        compact_formats::CompactBlock,
        service::{self, compact_tx_streamer_client::CompactTxStreamerClient},
    },
};
use zcash_protocol::consensus::BranchId;

const SYNC_BATCH_SIZE: u64 = 600;
const DIAL_CONNECT_TIMEOUT_SECS: u64 = 8;
const DIAL_RPC_TIMEOUT_SECS: u64 = 12;
const DNS_LOOKUP_PASSES: usize = 3;
const STALLED_SYNC_THRESHOLD_SECS: u64 = 90;

fn dial_rotation_counter() -> &'static AtomicUsize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    &COUNTER
}

fn validate_server_chain_info(
    network: WalletNetwork,
    info: &service::LightdInfo,
) -> Result<(), String> {
    let names: &[&str] = match network {
        WalletNetwork::Mainnet => &["vrsc", "main"],
        WalletNetwork::Testnet => &["vrsctest", "test"],
    };
    if !names.contains(&info.chain_name.trim().to_ascii_lowercase().as_str())
        || info.sapling_activation_height != consensus::activation_height(network)
        || info.consensus_branch_id() != Some(BranchId::Sapling)
    {
        return Err("private wallet endpoint has incompatible chain metadata".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatusKind {
    Initializing,
    Syncing,
    Synced,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuntimeTransaction {
    pub txid: String,
    pub net_sats: i128,
    pub block_height: u64,
    pub block_time: u64,
    pub pending: bool,
    pub to_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub status_kind: RuntimeStatusKind,
    pub info: DlightInfo,
    pub scanned_height: u64,
    pub chain_tip_height: Option<u64>,
    pub estimated_tip_height: Option<u64>,
    pub last_error: Option<String>,
    pub last_updated: u64,
    pub last_progress_at: Option<u64>,
    pub last_tip_probe_at: Option<u64>,
    pub consecutive_failures: u32,
    pub scan_rate_blocks_per_sec: Option<f64>,
    pub stalled: bool,
    pub confirmed_sats: i128,
    pub pending_sats: i128,
    pub total_sats: i128,
    pub note_count: u64,
    pub transactions: Vec<RuntimeTransaction>,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            status_kind: RuntimeStatusKind::Initializing,
            info: DlightInfo {
                blocks: Some(0),
                longest_chain: Some(0),
                syncing: true,
                percent: Some(0.0),
                status_kind: Some("initializing".to_string()),
                last_updated: Some(unix_timestamp_secs()),
                last_progress_at: None,
                stalled: Some(false),
                scan_rate_blocks_per_sec: None,
            },
            scanned_height: 0,
            chain_tip_height: Some(0),
            estimated_tip_height: Some(0),
            last_error: None,
            last_updated: unix_timestamp_secs(),
            last_progress_at: None,
            last_tip_probe_at: None,
            consecutive_failures: 0,
            scan_rate_blocks_per_sec: None,
            stalled: false,
            confirmed_sats: 0,
            pending_sats: 0,
            total_sats: 0,
            note_count: 0,
            transactions: vec![],
        }
    }
}

async fn refresh_tip_info(
    endpoint: &str,
    network: WalletNetwork,
) -> Result<(CompactTxStreamerClient<Channel>, u64, u64), String> {
    let grpc_endpoint = normalize_grpc_endpoint(endpoint).map_err(|error| error.to_string())?;
    let parsed_uri: Uri = grpc_endpoint
        .parse()
        .map_err(|_| "invalid dlight endpoint URL".to_string())?;
    let host = parsed_uri
        .host()
        .ok_or_else(|| "dlight endpoint has no host".to_string())?
        .to_string();
    let port = parsed_uri.port_u16().unwrap_or(443);
    let is_https = parsed_uri.scheme_str() == Some("https");

    let mut resolved_addrs = Vec::<SocketAddr>::new();
    for pass in 0..DNS_LOOKUP_PASSES {
        match tokio::net::lookup_host((host.as_str(), port)).await {
            Ok(pass_addrs) => {
                for socket_addr in pass_addrs {
                    if !resolved_addrs
                        .iter()
                        .any(|existing| existing == &socket_addr)
                    {
                        resolved_addrs.push(socket_addr);
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "[dlight_private] endpoint lookup failed on pass {} for {}:{}: {}",
                    pass + 1,
                    host,
                    port,
                    error
                );
            }
        }

        if pass + 1 < DNS_LOOKUP_PASSES {
            tokio::task::yield_now().await;
        }
    }

    let mut ip_candidates = resolved_addrs
        .iter()
        .map(|socket_addr| format!("https://{socket_addr}"))
        .collect::<Vec<_>>();

    if !ip_candidates.is_empty() {
        let rotate_from = dial_rotation_counter().fetch_add(1, Ordering::Relaxed);
        let rotate_offset = rotate_from % ip_candidates.len();
        ip_candidates.rotate_left(rotate_offset);
    }

    let mut candidates = ip_candidates;
    if !candidates
        .iter()
        .any(|candidate| candidate == &grpc_endpoint)
    {
        candidates.push(grpc_endpoint.clone());
    }

    eprintln!(
        "[dlight_private] tip probe for {}:{} candidates={}",
        host,
        port,
        candidates.len()
    );

    let mut last_metadata_error: Option<String> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        eprintln!(
            "[dlight_private] tip probe attempt {}/{} via {}",
            index + 1,
            candidates.len(),
            candidate
        );

        let endpoint_builder = match Endpoint::from_shared(candidate.clone()) {
            Ok(builder) => builder
                .origin(parsed_uri.clone())
                .connect_timeout(Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS))
                .timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS))
                .tcp_nodelay(true),
            Err(error) => {
                eprintln!(
                    "[dlight_private] tip probe endpoint parse failed for {}: {}",
                    candidate, error
                );
                continue;
            }
        };

        let endpoint_builder = if is_https {
            match endpoint_builder.tls_config(
                ClientTlsConfig::new()
                    .with_webpki_roots()
                    .domain_name(host.clone()),
            ) {
                Ok(builder) => builder,
                Err(error) => {
                    eprintln!(
                        "[dlight_private] tip probe tls config failed for {}: {}",
                        candidate, error
                    );
                    continue;
                }
            }
        } else {
            endpoint_builder
        };

        let connect_result = tokio::time::timeout(
            Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS + 1),
            endpoint_builder.connect(),
        )
        .await;

        let channel = match connect_result {
            Ok(Ok(channel)) => channel,
            Ok(Err(error)) => {
                eprintln!(
                    "[dlight_private] tip probe connect failed for {}: {}",
                    candidate, error
                );
                continue;
            }
            Err(_) => {
                eprintln!(
                    "[dlight_private] tip probe connect timed out for {}",
                    candidate
                );
                continue;
            }
        };

        let mut client = CompactTxStreamerClient::new(channel);
        let info_result = tokio::time::timeout(
            Duration::from_secs(DIAL_RPC_TIMEOUT_SECS),
            client.get_lightd_info(service::Empty {}),
        )
        .await;

        let lightd_info = match info_result {
            Ok(Ok(info)) => info.into_inner(),
            Ok(Err(error)) => {
                eprintln!(
                    "[dlight_private] tip probe rpc failed for {}: {}",
                    candidate, error
                );
                continue;
            }
            Err(_) => {
                eprintln!("[dlight_private] tip probe rpc timed out for {}", candidate);
                continue;
            }
        };

        if let Err(error) = validate_server_chain_info(network, &lightd_info) {
            eprintln!(
                "[dlight_private] tip probe metadata validation failed for {}: {}",
                candidate, error
            );
            last_metadata_error = Some(error);
            continue;
        }

        let chain_tip_height = lightd_info.block_height;
        let estimated_tip_height = if lightd_info.estimated_height > 0 {
            lightd_info.estimated_height
        } else {
            chain_tip_height
        };

        eprintln!(
            "[dlight_private] tip probe success via {} chain_tip={} estimated_tip={}",
            candidate, chain_tip_height, estimated_tip_height
        );

        return Ok((client, chain_tip_height, estimated_tip_height));
    }

    eprintln!(
        "[dlight_private] tip probe exhausted all candidates for {}:{}",
        host, port
    );
    Err(last_metadata_error.unwrap_or_else(|| {
        format!(
            "failed to connect to dlight endpoint {}:{} with valid chain metadata",
            host, port
        )
    }))
}

struct RuntimeCore {
    state: WalletState,
    store: CacheStore,
    viewing_key: UnifiedFullViewingKey,
    usable: bool,
}

impl RuntimeCore {
    fn commit(&mut self, state: WalletState) -> Result<(), WalletError> {
        if !self.usable {
            return Err(WalletError::OperationFailed);
        }
        if let Err(error) = self.store.save(&state) {
            // A failed durable write is never followed by another spend against
            // a possibly older in-memory generation. Reopen and verify on unlock.
            self.usable = false;
            return Err(error);
        }
        self.state = state;
        Ok(())
    }
}

pub struct RuntimeHandle {
    core: Mutex<RuntimeCore>,
    snapshot: Mutex<RuntimeSnapshot>,
    pub(super) cancel_token: CancellationToken,
    runtime_task: Mutex<Option<JoinHandle<()>>>,
    // Do not store spending keys or mnemonic text in the background scanner.
    public_request: DlightRuntimeRequest,
}

fn runtime_registry() -> &'static Mutex<HashMap<String, Arc<RuntimeHandle>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<RuntimeHandle>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn handle_for(key: &str) -> Result<Arc<RuntimeHandle>, WalletError> {
    runtime_registry()
        .lock()
        .map_err(|_| WalletError::OperationFailed)?
        .get(key)
        .cloned()
        .filter(|h| !h.cancel_token.is_cancelled())
        .ok_or(WalletError::DlightSynchronizerNotReady)
}

pub(crate) fn cached_request(key: &str) -> Option<DlightRuntimeRequest> {
    handle_for(key).ok().map(|h| h.public_request.clone())
}

pub async fn ensure_runtime(
    request: &DlightRuntimeRequest,
) -> Result<Arc<RuntimeHandle>, WalletError> {
    if request.submission_guard.cancellation().is_cancelled() {
        return Err(WalletError::WalletLocked);
    }
    if let Ok(handle) = handle_for(&request.runtime_key) {
        return Ok(handle);
    }
    let request = request.clone();
    tokio::task::spawn_blocking(move || {
        let mut registry = runtime_registry()
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;
        if request.submission_guard.cancellation().is_cancelled() {
            return Err(WalletError::WalletLocked);
        }
        if let Some(handle) = registry.get(&request.runtime_key) {
            return Ok(handle.clone());
        }
        let keys = DlightSpendKeyMaterial::from_seed_material(
            &request.seed_material,
            request.network,
            &request.scope_address,
        )?;
        let legacy_paths = resolve_paths(
            &request.app_data_dir,
            request.network,
            &request.account_hash,
            &request.coin_id,
        );
        let binding = format!(
            "{}:{}:{}",
            request.account_hash, request.coin_id, request.scope_address
        );
        let scope_hash = blake2b_simd::Params::new()
            .hash_length(16)
            .hash(request.scope_address.as_bytes())
            .to_hex()
            .to_string();
        let store = CacheStore::open(
            legacy_paths.join(format!("scope-{scope_hash}")),
            keys.cache_key(binding.as_bytes()),
        )?;
        let loaded = store.load::<WalletState>()?;
        let mut state = match loaded {
            Some(state) => {
                state.validate()?;
                state
            }
            None => {
                let state = migrate_legacy(&request, &legacy_paths);
                store.save(&state)?;
                state
            }
        };
        if state.recover_interrupted_builds() {
            store.save(&state)?;
        }
        store.retire_legacy_files(&legacy_paths)?;
        let viewing_key = keys.viewing_key()?;
        let cancel_token = request.submission_guard.cancellation().child_token();
        if cancel_token.is_cancelled() {
            return Err(WalletError::WalletLocked);
        }
        let mut public_request = request.clone();
        public_request.seed_material = zeroize::Zeroizing::new(String::new());
        let cached_height = state.chain.height;
        let handle = Arc::new(RuntimeHandle {
            core: Mutex::new(RuntimeCore {
                state,
                store,
                viewing_key,
                usable: true,
            }),
            snapshot: Mutex::new(RuntimeSnapshot::default()),
            cancel_token,
            runtime_task: Mutex::new(None),
            public_request,
        });
        // Show the authenticated cached generation while the first network
        // check runs. It remains unavailable for spending until synchronized.
        publish(
            &handle,
            cached_height,
            RuntimeStatusKind::Initializing,
            None,
            None,
        );
        let task_handle = handle.clone();
        let runtime_request = handle.public_request.clone();
        let task = tokio::spawn(async move {
            run_sync_loop(runtime_request, task_handle).await;
        });
        *handle
            .runtime_task
            .lock()
            .map_err(|_| WalletError::OperationFailed)? = Some(task);
        registry.insert(request.runtime_key.clone(), handle.clone());
        Ok(handle)
    })
    .await
    .map_err(|_| WalletError::OperationFailed)?
}

fn migrate_legacy(request: &DlightRuntimeRequest, root: &std::path::Path) -> WalletState {
    let mut state = WalletState::new(request.network);
    // This is a pre-release cache format. Do not attempt to reconcile the old
    // independently written scanners or trust plaintext note state. Rescan it.
    // Birthdays are used only for a fresh cache without any legacy generation.
    if !root.join("data.db").exists() && !root.join("spend_wallet.db").exists() {
        if let Some(birthday) = &request.birthday {
            let tree = service::TreeState {
                sapling_tree: birthday.sapling_tree.clone(),
                ..Default::default()
            }
            .sapling_tree();
            if let Ok(tree) = tree {
                let checkpoint = Checkpoint {
                    height: birthday.height,
                    block_hash_hex: Some(birthday.block_hash_hex.clone()),
                    tree: super::spend_sync::encode_tree(&tree),
                    notes: vec![],
                };
                if checkpoint.metadata().ok().flatten().is_some()
                    && birthday.height >= consensus::birthday_floor(request.network)
                {
                    state.birthday = checkpoint.clone();
                    state.chain = checkpoint;
                }
            }
        }
    }
    state
}

pub fn get_runtime_snapshot(key: &str) -> Option<RuntimeSnapshot> {
    handle_for(key)
        .ok()?
        .snapshot
        .lock()
        .ok()
        .map(|s| s.clone())
}

fn publish(
    handle: &RuntimeHandle,
    tip: u64,
    status: RuntimeStatusKind,
    error: Option<String>,
    rate: Option<f64>,
) {
    publish_inner(handle, tip, status, error, rate, false);
}

fn publish_inner(
    handle: &RuntimeHandle,
    mut tip: u64,
    mut status: RuntimeStatusKind,
    mut error: Option<String>,
    mut rate: Option<f64>,
    preserve_sync: bool,
) {
    let Ok(core) = handle.core.lock() else {
        return;
    };
    let Ok(mut snapshot) = handle.snapshot.lock() else {
        return;
    };
    if preserve_sync {
        tip = snapshot.chain_tip_height.unwrap_or(0);
        status = snapshot.status_kind;
        error = snapshot.last_error.clone();
        rate = snapshot.scan_rate_blocks_per_sec;
    }
    let now = unix_timestamp_secs();
    let height = core.state.chain.height;
    let status = if core.usable {
        status
    } else {
        RuntimeStatusKind::Error
    };
    let previous_height = snapshot.scanned_height;
    let last_progress = if height > previous_height {
        Some(now)
    } else {
        snapshot.last_progress_at
    };
    let failures = if error.is_some() {
        snapshot.consecutive_failures.saturating_add(1)
    } else {
        0
    };
    let percent = if tip == 0 {
        0.0
    } else {
        let floor = core.state.birthday.height;
        let count = tip.saturating_sub(floor);
        if count == 0 {
            100.0
        } else {
            (height.saturating_sub(floor) as f64 / count as f64 * 100.0).clamp(0.0, 100.0)
        }
    };
    let syncing = status != RuntimeStatusKind::Synced;
    let kind = match status {
        RuntimeStatusKind::Initializing => "initializing",
        RuntimeStatusKind::Syncing => "syncing",
        RuntimeStatusKind::Synced => "synced",
        RuntimeStatusKind::Error => "error",
    }
    .to_string();
    let stalled = syncing
        && last_progress.is_some_and(|t| now.saturating_sub(t) > STALLED_SYNC_THRESHOLD_SECS);
    let (available, pending_change) = core.state.balances();
    let mut transactions: Vec<_> = core
        .state
        .transactions
        .values()
        .map(|tx| RuntimeTransaction {
            txid: tx.txid.clone(),
            net_sats: tx.net_sats,
            block_height: tx.block_height,
            block_time: tx.block_time,
            pending: false,
            to_address: tx.to_address.clone(),
        })
        .collect();
    transactions.extend(
        core.state
            .pending
            .values()
            .filter(|p| p.is_active(height))
            .filter_map(|p| {
                p.txid.as_ref().map(|txid| RuntimeTransaction {
                    txid: txid.clone(),
                    net_sats: -(i128::from(p.value_sats) + i128::from(p.fee_sats)),
                    block_height: 0,
                    block_time: p.created_at,
                    pending: true,
                    to_address: Some(p.to_address.clone()),
                })
            }),
    );
    transactions.sort_by(|a, b| b.block_time.cmp(&a.block_time).then(a.txid.cmp(&b.txid)));
    *snapshot = RuntimeSnapshot {
        status_kind: status,
        info: DlightInfo {
            blocks: Some(height),
            longest_chain: Some(tip),
            syncing,
            percent: Some(percent),
            status_kind: Some(kind),
            last_updated: Some(now),
            last_progress_at: last_progress,
            stalled: Some(stalled),
            scan_rate_blocks_per_sec: rate,
        },
        scanned_height: height,
        chain_tip_height: Some(tip),
        estimated_tip_height: Some(tip),
        last_error: error,
        last_updated: now,
        last_progress_at: last_progress,
        last_tip_probe_at: Some(now),
        consecutive_failures: failures,
        scan_rate_blocks_per_sec: rate,
        stalled,
        confirmed_sats: i128::from(available),
        pending_sats: i128::from(pending_change),
        total_sats: i128::from(available) + i128::from(pending_change),
        note_count: core.state.chain.notes.len() as u64,
        transactions,
    };
}

/// Only callers that just generated an unexposed random secret may use this.
/// Imported/reused phrases must scan from activation because they may hold funds.
pub(crate) async fn creation_birthday(
    endpoint: &str,
    network: WalletNetwork,
) -> Result<super::DlightBirthday, WalletError> {
    let (mut client, tip, _) = refresh_tip_info(endpoint, network)
        .await
        .map_err(|_| WalletError::NetworkError)?;
    let height = tip
        .saturating_sub(10)
        .max(consensus::activation_height(network));
    let block = fetch_block(&mut client, height).await?;
    let tree_state = client
        .get_tree_state(service::BlockId {
            height,
            hash: vec![],
        })
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();
    validate_birthday(&block, tree_state)
}

fn validate_birthday(
    block: &CompactBlock,
    tree_state: service::TreeState,
) -> Result<super::DlightBirthday, WalletError> {
    let mut hash = hex::decode(&tree_state.hash).map_err(|_| WalletError::NetworkError)?;
    hash.reverse(); // TreeState uses display order; CompactBlock uses internal order.
    let tree = tree_state
        .sapling_tree()
        .map_err(|_| WalletError::NetworkError)?;
    if tree_state.height != block.height
        || hash.len() != 32
        || hash != block.hash
        || block.chain_metadata.as_ref().is_some_and(|meta| {
            meta.sapling_commitment_tree_size > 0
                && u64::from(meta.sapling_commitment_tree_size) != tree.size() as u64
        })
    {
        return Err(WalletError::NetworkError);
    }
    Ok(super::DlightBirthday {
        height: block.height,
        block_hash_hex: hex::encode(hash),
        sapling_tree: tree_state.sapling_tree,
    })
}

async fn fetch_block(
    client: &mut CompactTxStreamerClient<Channel>,
    height: u64,
) -> Result<CompactBlock, WalletError> {
    let block = client
        .get_block(service::BlockId {
            height,
            hash: vec![],
        })
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();
    if block.height != height || block.hash.len() != 32 {
        return Err(WalletError::NetworkError);
    }
    Ok(block)
}

async fn recover_chain(
    request: &DlightRuntimeRequest,
    handle: &Arc<RuntimeHandle>,
    client: &mut CompactTxStreamerClient<Channel>,
    tip: u64,
) -> Result<(), WalletError> {
    let state = handle
        .core
        .lock()
        .map_err(|_| WalletError::OperationFailed)?
        .state
        .clone();
    if state.chain.block_hash_hex.is_none()
        && state.chain.height == consensus::birthday_floor(request.network)
    {
        return Ok(());
    }
    if state.chain.height <= tip {
        let block = fetch_block(client, state.chain.height).await?;
        if state.chain.block_hash_hex.as_deref() == Some(hex::encode(&block.hash).as_str()) {
            return Ok(());
        }
    }
    // This includes same-height replacements and a lower server tip. Never just
    // lower a height while retaining the old chain's notes or witness tree.
    let mut ancestor = None;
    for checkpoint in state
        .checkpoints
        .iter()
        .rev()
        .chain(std::iter::once(&state.birthday))
    {
        if checkpoint.height > tip || checkpoint.block_hash_hex.is_none() {
            continue;
        }
        let block = fetch_block(client, checkpoint.height).await?;
        if checkpoint.block_hash_hex.as_deref() == Some(hex::encode(&block.hash).as_str()) {
            ancestor = Some(checkpoint.clone());
            break;
        }
    }
    let handle = handle.clone();
    let network = request.network;
    tokio::task::spawn_blocking(move || {
        if handle.cancel_token.is_cancelled() {
            return Err(WalletError::WalletLocked);
        }
        let mut core = handle
            .core
            .lock()
            .map_err(|_| WalletError::OperationFailed)?;
        let mut next = core.state.clone();
        let checkpoint = ancestor.unwrap_or_else(|| Checkpoint::empty(network));
        if checkpoint.height < next.birthday.height {
            next.birthday = checkpoint.clone();
        }
        next.rewind(checkpoint);
        core.commit(next)
    })
    .await
    .map_err(|_| WalletError::OperationFailed)?
}

async fn fetch_range(
    client: &mut CompactTxStreamerClient<Channel>,
    start: u64,
    end: u64,
) -> Result<Vec<CompactBlock>, WalletError> {
    let mut stream = client
        .get_block_range(service::BlockRange {
            start: Some(service::BlockId {
                height: start,
                hash: vec![],
            }),
            end: Some(service::BlockId {
                height: end,
                hash: vec![],
            }),
        })
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();
    let mut blocks = Vec::new();
    while let Some(block) =
        tokio::time::timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS), stream.message())
            .await
            .map_err(|_| WalletError::NetworkError)?
            .map_err(|_| WalletError::NetworkError)?
    {
        if block.height != start + blocks.len() as u64 || block.height > end {
            return Err(WalletError::NetworkError);
        }
        blocks.push(block);
    }
    if blocks.len() as u64 != end - start + 1 {
        return Err(WalletError::NetworkError);
    }
    Ok(blocks)
}

async fn sync_cycle(
    request: &DlightRuntimeRequest,
    handle: &Arc<RuntimeHandle>,
    client: &mut CompactTxStreamerClient<Channel>,
) -> Result<(), WalletError> {
    let info = client
        .get_lightd_info(service::Empty {})
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();
    validate_server_chain_info(request.network, &info).map_err(|_| WalletError::NetworkError)?;
    let tip = info.block_height;
    if tip < consensus::birthday_floor(request.network) {
        return Err(WalletError::NetworkError);
    }
    publish(handle, tip, RuntimeStatusKind::Syncing, None, None);
    recover_chain(request, handle, client, tip).await?;
    loop {
        let height = handle
            .core
            .lock()
            .map_err(|_| WalletError::OperationFailed)?
            .state
            .chain
            .height;
        if height >= tip {
            break;
        }
        let end = (height + SYNC_BATCH_SIZE).min(tip);
        let started = Instant::now();
        let blocks = fetch_range(client, height + 1, end).await?;
        let tree_state = client
            .get_tree_state(service::BlockId {
                height: end,
                hash: vec![],
            })
            .await
            .map_err(|_| WalletError::NetworkError)?
            .into_inner();
        validate_birthday(
            blocks.last().ok_or(WalletError::NetworkError)?,
            tree_state.clone(),
        )?;
        let expected_tree = tree_state
            .sapling_tree()
            .map_err(|_| WalletError::NetworkError)?;
        let batch_handle = handle.clone();
        let network = request.network;
        tokio::task::spawn_blocking(move || {
            let mut core = batch_handle
                .core
                .lock()
                .map_err(|_| WalletError::OperationFailed)?;
            if batch_handle.cancel_token.is_cancelled() {
                return Err(WalletError::WalletLocked);
            }
            let mut next = core.state.clone();
            next.scan_batch(
                blocks,
                network,
                core.viewing_key.clone(),
                &batch_handle.cancel_token,
            )?;
            let scanned_tree = super::spend_sync::decode_tree(&next.chain.tree)
                .ok_or(WalletError::OperationFailed)?;
            if scanned_tree.size() != expected_tree.size()
                || scanned_tree.root() != expected_tree.root()
            {
                return Err(WalletError::DlightSpendCacheNotReady);
            }
            core.commit(next)
        })
        .await
        .map_err(|_| WalletError::OperationFailed)??;
        publish(
            handle,
            tip,
            RuntimeStatusKind::Syncing,
            None,
            Some((end - height) as f64 / started.elapsed().as_secs_f64()),
        );
    }
    publish(handle, tip, RuntimeStatusKind::Synced, None, None);
    Ok(())
}

async fn run_sync_loop(request: DlightRuntimeRequest, handle: Arc<RuntimeHandle>) {
    let mut client = None;
    loop {
        if handle.cancel_token.is_cancelled() {
            break;
        }
        let work = async {
            if client.is_none() {
                client = Some(
                    refresh_tip_info(&request.endpoint, request.network)
                        .await
                        .map_err(|_| WalletError::NetworkError)?
                        .0,
                );
            }
            sync_cycle(
                &request,
                &handle,
                client.as_mut().ok_or(WalletError::NetworkError)?,
            )
            .await
        };
        let result = tokio::select! {
            biased;
            _ = handle.cancel_token.cancelled() => break,
            result = work => result,
        };
        let sleep = if let Err(error) = result {
            let tip = get_runtime_snapshot(&request.runtime_key)
                .and_then(|s| s.chain_tip_height)
                .unwrap_or(0);
            publish(
                &handle,
                tip,
                RuntimeStatusKind::Error,
                Some(error.to_string()),
                None,
            );
            client = None;
            8
        } else {
            20
        };
        tokio::select! {
            _ = handle.cancel_token.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_secs(sleep)) => {}
        }
    }
}

pub fn spend_snapshot(key: &str) -> Result<SpendSyncSnapshot, WalletError> {
    let handle = handle_for(key)?;
    let core = handle
        .core
        .lock()
        .map_err(|_| WalletError::OperationFailed)?;
    let snapshot = handle
        .snapshot
        .lock()
        .map_err(|_| WalletError::OperationFailed)?
        .clone();
    if snapshot.status_kind != RuntimeStatusKind::Synced
        || snapshot.chain_tip_height != Some(core.state.chain.height)
    {
        return Err(WalletError::DlightSpendCacheNotReady);
    }
    if !core.usable {
        return Err(WalletError::OperationFailed);
    }
    let notes = core.state.available_notes()?;
    Ok(SpendSyncSnapshot {
        scanned_height: core.state.chain.height,
        chain_tip_height: snapshot.chain_tip_height.unwrap_or(0),
        confirmed_balance_sats: notes.iter().map(|n| n.value_sats).sum(),
        spendable_notes: notes,
    })
}

pub fn spend_cache_status(key: &str) -> Option<SpendCacheStatus> {
    let handle = handle_for(key).ok()?;
    // Diagnostics must not wait for a batch's decryption/witness work. The
    // public snapshot already contains the last fully committed generation.
    let snapshot = handle.snapshot.lock().ok()?.clone();
    let tip = snapshot.chain_tip_height.unwrap_or(0);
    Some(SpendCacheStatus {
        ready: snapshot.status_kind == RuntimeStatusKind::Synced,
        scanned_height: snapshot.scanned_height,
        chain_tip_height: tip,
        effective_tip_height: tip,
        lag_blocks: tip.saturating_sub(snapshot.scanned_height),
        status_kind: snapshot
            .info
            .status_kind
            .clone()
            .unwrap_or_else(|| "initializing".into()),
        percent: snapshot.info.percent,
        last_updated: snapshot.last_updated,
        note_count: snapshot.note_count,
        last_error: snapshot.last_error,
    })
}

/// Every pending-state update uses the same lock and durable generation as scans.
/// The closure must never perform network I/O or proving while holding the lock.
pub(super) async fn update_pending<T: Send + 'static>(
    key: &str,
    require_synced: bool,
    change: impl FnOnce(&mut WalletState) -> Result<T, WalletError> + Send + 'static,
) -> Result<T, WalletError> {
    let handle = handle_for(key)?;
    tokio::task::spawn_blocking(move || {
        let result = {
            let mut core = handle
                .core
                .lock()
                .map_err(|_| WalletError::OperationFailed)?;
            if handle.cancel_token.is_cancelled() {
                return Err(WalletError::WalletLocked);
            }
            if require_synced {
                let snapshot = handle
                    .snapshot
                    .lock()
                    .map_err(|_| WalletError::OperationFailed)?;
                if snapshot.status_kind != RuntimeStatusKind::Synced
                    || snapshot.chain_tip_height != Some(core.state.chain.height)
                {
                    return Err(WalletError::DlightSpendCacheNotReady);
                }
            }
            let mut next = core.state.clone();
            let result = change(&mut next)?;
            core.commit(next)?;
            result
        };
        publish_inner(
            &handle,
            0,
            RuntimeStatusKind::Initializing,
            None,
            None,
            true,
        );
        Ok(result)
    })
    .await
    .map_err(|_| WalletError::OperationFailed)?
}

pub async fn stop_all_runtimes() {
    let handles = runtime_registry()
        .lock()
        .map(|mut r| r.drain().map(|(_, h)| h).collect::<Vec<_>>())
        .unwrap_or_default();
    for handle in handles {
        handle.cancel_token.cancel();
        if let Ok(mut task) = handle.runtime_task.lock() {
            if let Some(task) = task.take() {
                task.abort();
            }
        }
    }
}

pub async fn stop_runtime(key: &str) {
    let handle = runtime_registry()
        .lock()
        .ok()
        .and_then(|mut r| r.remove(key));
    if let Some(handle) = handle {
        handle.cancel_token.cancel();
        if let Ok(mut task) = handle.runtime_task.lock() {
            if let Some(task) = task.take() {
                task.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::state::testing::{self, block, funded_state, pending};
    use super::*;

    fn fixture() -> (Arc<RuntimeHandle>, String, std::path::PathBuf, String) {
        let root =
            std::env::temp_dir().join(format!("private-runtime-test-{}", uuid::Uuid::new_v4()));
        let (state, keys, nf) = funded_state();
        let mut session = crate::core::auth::SessionManager::new(
            crate::core::auth::StrongholdStore::new_for_tests(root.join("auth")),
        );
        let profile = crate::core::crypto::derive_public_profile_from_material(
            "runtime fixture only",
            crate::types::wallet::WalletSecretKind::SeedText,
            crate::core::crypto::Network::Testnet,
        )
        .unwrap();
        session.unlock_with_profile(
            "fixture".into(),
            WalletNetwork::Testnet,
            crate::types::wallet::WalletSecretKind::SeedText,
            profile,
            zeroize::Zeroizing::new(vec![7; 32]),
        );
        let context = session.active_wallet_access_context().unwrap();
        let runtime_key = uuid::Uuid::new_v4().to_string();
        let store = CacheStore::open(root.join("cache"), zeroize::Zeroizing::new([7; 32])).unwrap();
        store.save(&state).unwrap();
        let handle = Arc::new(RuntimeHandle {
            core: Mutex::new(RuntimeCore {
                state,
                store,
                viewing_key: keys.viewing_key().unwrap(),
                usable: true,
            }),
            snapshot: Mutex::new(RuntimeSnapshot::default()),
            cancel_token: context
                .session_submission_guard()
                .cancellation()
                .child_token(),
            runtime_task: Mutex::new(None),
            public_request: DlightRuntimeRequest {
                runtime_key: runtime_key.clone(),
                endpoint: "https://unused.invalid".into(),
                scope_address: keys.scope_address(WalletNetwork::Testnet).unwrap(),
                scope_system_id: "test".into(),
                coin_id: "VRSCTEST".into(),
                network: WalletNetwork::Testnet,
                seed_material: zeroize::Zeroizing::new(String::new()),
                session_id: context.session_id.clone(),
                submission_guard: context.session_submission_guard(),
                birthday: None,
                account_hash: "fixture".into(),
                app_data_dir: root.clone(),
            },
        });
        publish(&handle, 1, RuntimeStatusKind::Synced, None, None);
        runtime_registry()
            .lock()
            .unwrap()
            .insert(runtime_key.clone(), handle.clone());
        (handle, runtime_key, root, nf)
    }

    #[tokio::test]
    async fn diagnostics_do_not_wait_for_the_scan_writer() {
        let (handle, key, root, _) = fixture();
        let writer = handle.core.lock().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader_key = key.clone();
        let reader = std::thread::spawn(move || {
            sender.send(spend_cache_status(&reader_key)).unwrap();
        });
        let result = receiver.recv_timeout(Duration::from_secs(2));
        drop(writer);
        reader.join().unwrap();
        assert_eq!(result.unwrap().unwrap().note_count, 1);
        stop_runtime(&key).await;
        drop(handle);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn startup_publishes_encrypted_balance_before_a_network_reply() {
        let (previous, old_key, root, _) = fixture();
        let mut request = previous.public_request.clone();
        let state = previous.core.lock().unwrap().state.clone();
        stop_runtime(&old_key).await;
        drop(previous);
        request.runtime_key = uuid::Uuid::new_v4().to_string();
        request.endpoint = "https://[".into(); // No valid transport or network reply.
        let keys = testing::keys();
        request.seed_material =
            zeroize::Zeroizing::new(zcash_keys::encoding::encode_extended_spending_key(
                zcash_protocol::constants::mainnet::HRP_SAPLING_EXTENDED_SPENDING_KEY,
                &sapling::zip32::ExtendedSpendingKey::master(&[7; 32]),
            ));
        let scope_hash = blake2b_simd::Params::new()
            .hash_length(16)
            .hash(request.scope_address.as_bytes())
            .to_hex()
            .to_string();
        let binding = format!(
            "{}:{}:{}",
            request.account_hash, request.coin_id, request.scope_address
        );
        let cache = CacheStore::open(
            resolve_paths(
                &root,
                request.network,
                &request.account_hash,
                &request.coin_id,
            )
            .join(format!("scope-{scope_hash}")),
            keys.cache_key(binding.as_bytes()),
        )
        .unwrap();
        cache.save(&state).unwrap();
        drop(cache);
        let reopened = ensure_runtime(&request).await.unwrap();
        let snapshot = get_runtime_snapshot(&request.runtime_key).unwrap();
        assert_eq!(snapshot.confirmed_sats, 100_000);
        assert_eq!(snapshot.scanned_height, 1);
        assert_eq!(snapshot.transactions.len(), 1);
        assert_ne!(snapshot.status_kind, RuntimeStatusKind::Synced);
        assert!(spend_snapshot(&request.runtime_key).is_err());
        stop_runtime(&request.runtime_key).await;
        drop(reopened);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn concurrent_reservations_allow_only_one_spend_and_persist_it() {
        let (handle, key, root, nf) = fixture();
        let mut tasks = Vec::new();
        for i in 0..8 {
            let key = key.clone();
            let nf = nf.clone();
            tasks.push(tokio::spawn(async move {
                update_pending(&key, true, move |state| {
                    state.reserve(pending(&i.to_string(), &nf))
                })
                .await
            }));
        }
        let mut successes = 0;
        for task in tasks {
            if task.await.unwrap().is_ok() {
                successes += 1;
            }
        }
        assert_eq!(successes, 1);
        let core = handle.core.lock().unwrap();
        assert_eq!(core.state.pending.len(), 1);
        assert_eq!(
            core.store
                .load::<WalletState>()
                .unwrap()
                .unwrap()
                .pending
                .len(),
            1
        );
        drop(core);
        assert_eq!(get_runtime_snapshot(&key).unwrap().confirmed_sats, 0);
        stop_runtime(&key).await;
        drop(handle);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn scan_and_pending_write_share_one_generation() {
        let (handle, key, root, nf) = fixture();
        let scan_handle = handle.clone();
        let scan = tokio::task::spawn_blocking(move || {
            let mut core = scan_handle.core.lock().unwrap();
            let mut next = core.state.clone();
            testing::scan(&mut next, &testing::keys(), vec![block(2, 2, 1, vec![], 1)]).unwrap();
            core.commit(next).unwrap();
        });
        update_pending(&key, false, move |state| {
            state.reserve(pending("send", &nf))
        })
        .await
        .unwrap();
        scan.await.unwrap();
        let core = handle.core.lock().unwrap();
        let saved = core.store.load::<WalletState>().unwrap().unwrap();
        assert_eq!(saved.chain.height, 2);
        assert_eq!(saved.pending.len(), 1);
        assert!(saved.available_notes().unwrap().is_empty());
        drop(core);
        stop_runtime(&key).await;
        drop(handle);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn pending_refresh_cannot_restore_stale_synced_status() {
        let (handle, key, root, nf) = fixture();
        publish(&handle, 2, RuntimeStatusKind::Syncing, None, None);
        update_pending(&key, false, move |state| {
            state.reserve(pending("send", &nf))
        })
        .await
        .unwrap();
        let snapshot = get_runtime_snapshot(&key).unwrap();
        assert_eq!(snapshot.status_kind, RuntimeStatusKind::Syncing);
        assert_eq!(snapshot.chain_tip_height, Some(2));
        assert!(spend_snapshot(&key).is_err());
        handle.cancel_token.cancel();
        assert!(cached_request(&key).is_none());
        stop_runtime(&key).await;
        drop(handle);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn birthday_checks_hash_order_height_and_tree_size() {
        let mut compact = block(12, 3, 2, vec![], 0);
        compact.hash = (0u8..32).collect();
        let mut display_hash = compact.hash.clone();
        display_hash.reverse();
        let tree = service::TreeState {
            height: 12,
            hash: hex::encode(display_hash),
            sapling_tree: "000000".into(),
            ..Default::default()
        };
        let birthday = validate_birthday(&compact, tree.clone()).unwrap();
        assert_eq!(birthday.block_hash_hex, hex::encode(&compact.hash));
        assert!(validate_birthday(
            &compact,
            service::TreeState {
                height: 13,
                ..tree.clone()
            }
        )
        .is_err());
        assert!(validate_birthday(
            &compact,
            service::TreeState {
                hash: "01".repeat(32),
                ..tree.clone()
            }
        )
        .is_err());
        compact
            .chain_metadata
            .as_mut()
            .unwrap()
            .sapling_commitment_tree_size = 1;
        assert!(validate_birthday(&compact, tree).is_err());
    }

    #[tokio::test]
    async fn pre_release_legacy_cache_is_rescanned_without_trusting_note_data() {
        let (handle, key, root, _) = fixture();
        std::fs::write(root.join("data.db"), b"malformed legacy plaintext").unwrap();
        let state = migrate_legacy(&handle.public_request, &root);
        assert_eq!(state.chain.height, 0);
        assert!(state.chain.notes.is_empty());
        state.validate().unwrap();
        stop_runtime(&key).await;
        drop(handle);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    #[ignore = "read-only live testnet check; requires LITE_WALLET_TEST_DLIGHT_ENDPOINT"]
    async fn live_testnet_checkpoint_and_single_scan_match_server_tree() {
        let endpoint =
            std::env::var("LITE_WALLET_TEST_DLIGHT_ENDPOINT").expect("public testnet endpoint");
        let birthday = creation_birthday(&endpoint, WalletNetwork::Testnet)
            .await
            .unwrap();
        let (mut client, tip, _) = refresh_tip_info(&endpoint, WalletNetwork::Testnet)
            .await
            .unwrap();
        let end = (birthday.height + 5).min(tip);
        assert!(end > birthday.height);
        let tree = service::TreeState {
            sapling_tree: birthday.sapling_tree.clone(),
            ..Default::default()
        }
        .sapling_tree()
        .unwrap();
        let checkpoint = Checkpoint {
            height: birthday.height,
            block_hash_hex: Some(birthday.block_hash_hex),
            tree: super::super::spend_sync::encode_tree(&tree),
            notes: vec![],
        };
        let mut state = WalletState::new(WalletNetwork::Testnet);
        state.birthday = checkpoint.clone();
        state.chain = checkpoint;
        let blocks = fetch_range(&mut client, birthday.height + 1, end)
            .await
            .unwrap();
        testing::scan(&mut state, &testing::keys(), blocks).unwrap();
        state.validate().unwrap();
        let block = fetch_block(&mut client, end).await.unwrap();
        let server_tree = client
            .get_tree_state(service::BlockId {
                height: end,
                hash: vec![],
            })
            .await
            .unwrap()
            .into_inner();
        let verified = validate_birthday(&block, server_tree.clone()).unwrap();
        assert_eq!(
            state.chain.block_hash_hex.as_deref(),
            Some(verified.block_hash_hex.as_str())
        );
        let scanned_tree = super::super::spend_sync::decode_tree(&state.chain.tree).unwrap();
        let expected_tree = server_tree.sapling_tree().unwrap();
        assert_eq!(scanned_tree.size(), expected_tree.size());
        assert_eq!(scanned_tree.root(), expected_tree.root());
    }
}
