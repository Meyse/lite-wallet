use rand::rngs::OsRng;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint, Uri};
use zcash_client_backend::proto::service::{
    self, compact_tx_streamer_client::CompactTxStreamerClient,
};
use zcash_primitives::transaction::builder::{BuildConfig, Builder, Error as TxBuildError};
use zcash_primitives::transaction::fees::fixed;
use zcash_protocol::consensus::{BlockHeight, Parameters};
use zcash_protocol::memo::MemoBytes;
use zcash_protocol::value::Zatoshis;
use zip32::Scope;

use crate::core::channels::dlight_private::destination::DlightDestinationKind;
use crate::types::wallet::WalletNetwork;
use crate::types::WalletError;

use super::recipient_resolution::{resolve_dlight_recipient, ResolvedDlightRecipient};
use super::spend_keys::DlightSpendKeyMaterial;
use super::spend_params::load_sapling_provers;
use super::spend_sync::SpendableNote;
use super::{normalize_grpc_endpoint, DlightRuntimeRequest};
use super::{runtime, state::PendingSend, store::unix_timestamp_secs};
use crate::core::channels::vrpc::VrpcProvider;

pub const SATOSHIS_PER_COIN: i128 = 100_000_000;
pub const FIXED_FEE_SATS: i128 = 10_000;

const DIAL_CONNECT_TIMEOUT_SECS: u64 = 10;
const DIAL_RPC_TIMEOUT_SECS: u64 = 20;
#[derive(Debug, Clone)]
pub struct DlightPreflightComputation {
    pub resolved_recipient: ResolvedDlightRecipient,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub value: String,
    pub fee: String,
    pub fee_taken_from_amount: bool,
    pub fee_taken_message: Option<String>,
    pub memo: Option<String>,
}

pub async fn compute_preflight(
    request: &DlightRuntimeRequest,
    to_address: &str,
    amount: &str,
    memo: Option<String>,
    confirmed_balance_sats: u64,
    vrpc_provider: &VrpcProvider,
) -> Result<DlightPreflightComputation, WalletError> {
    DlightSpendKeyMaterial::from_seed_material(
        &request.seed_material,
        request.network,
        &request.scope_address,
    )?;
    let resolved_recipient =
        resolve_dlight_recipient(to_address, request.network, vrpc_provider).await?;

    let submitted_sat = parse_positive_satoshis(amount)?;
    let confirmed_balance = i128::from(confirmed_balance_sats);
    if confirmed_balance <= 0 {
        return Err(WalletError::InsufficientFunds);
    }

    let (value_sat_i128, fee_taken_from_amount, fee_taken_message) =
        resolve_send_value(submitted_sat, confirmed_balance, FIXED_FEE_SATS)?;
    let value_sats = u64::try_from(value_sat_i128).map_err(|_| WalletError::OperationFailed)?;
    let fee_sats = u64::try_from(FIXED_FEE_SATS).map_err(|_| WalletError::OperationFailed)?;

    let normalized_memo = if resolved_recipient.is_shielded() {
        normalize_optional_memo(memo)
    } else {
        None
    };

    Ok(DlightPreflightComputation {
        resolved_recipient,
        value_sats,
        fee_sats,
        value: satoshis_to_decimal_string(value_sat_i128),
        fee: satoshis_to_decimal_string(FIXED_FEE_SATS),
        fee_taken_from_amount,
        fee_taken_message,
        memo: normalized_memo,
    })
}

#[derive(Debug, Clone)]
pub struct ExecuteSendParams {
    pub destination_kind: DlightDestinationKind,
    pub display_to_address: String,
    pub delivery_to_address: String,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlightSendStage {
    SyncingSpendState,
    LoadingProver,
    BuildingProof,
    Broadcasting,
}

#[derive(Debug, Clone)]
pub struct ExecutedSend {
    pub txid: String,
}

type SendProgress = Arc<dyn Fn(DlightSendStage) + Send + Sync>;

/// Dropping a cancelled build releases only a reservation that has no serialized
/// transaction. Prepared submissions remain reserved through ambiguous outcomes.
struct BuildReservation {
    runtime_key: String,
    id: String,
}
impl Drop for BuildReservation {
    fn drop(&mut self) {
        let key = self.runtime_key.clone();
        let id = self.id.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = runtime::update_pending(&key, false, move |state| {
                    if state.pending.get(&id).is_some_and(|p| p.txid.is_none()) {
                        state.pending.remove(&id);
                    }
                    Ok(())
                })
                .await;
            });
        }
    }
}

pub async fn execute_send(
    request: &DlightRuntimeRequest,
    params: &ExecuteSendParams,
    progress: Option<SendProgress>,
) -> Result<ExecutedSend, WalletError> {
    let cancellation = request.submission_guard.cancellation();
    if cancellation.is_cancelled() {
        return Err(WalletError::WalletLocked);
    }
    let required = params
        .value_sats
        .checked_add(params.fee_sats)
        .ok_or(WalletError::OperationFailed)?;
    let id = uuid::Uuid::new_v4().to_string();
    let reservation_key = request.runtime_key.clone();
    let reserve_params = params.clone();
    let reserve_id = id.clone();
    let (reservation, selected, change, height, hash) =
        runtime::update_pending(&request.runtime_key, true, move |state| {
            let notes = state.available_notes()?;
            let (selected, total) =
                select_notes(&notes, required).ok_or(WalletError::InsufficientFunds)?;
            let change = total - required;
            state.reserve(PendingSend {
                id: reserve_id.clone(),
                nullifiers: selected.iter().map(|n| n.nullifier_hex.clone()).collect(),
                value_sats: reserve_params.value_sats,
                fee_sats: reserve_params.fee_sats,
                change_sats: change,
                to_address: reserve_params.display_to_address,
                created_at: unix_timestamp_secs(),
                txid: None,
                raw_tx_hex: None,
                expiry_height: None,
                mined_height: None,
                conflict_height: None,
            })?;
            Ok((
                BuildReservation {
                    runtime_key: reservation_key,
                    id: reserve_id,
                },
                selected,
                change,
                state.chain.height,
                state.chain.block_hash_hex.clone(),
            ))
        })
        .await?;
    let build_request = request.clone();
    let build_params = params.clone();
    let build_progress = progress.clone();
    let proof = tokio::task::spawn_blocking(move || {
        if cancellation.is_cancelled() {
            return Err(WalletError::WalletLocked);
        }
        let keys = DlightSpendKeyMaterial::from_seed_material(
            &build_request.seed_material,
            build_request.network,
            &build_request.scope_address,
        )?;
        build_transaction(
            build_request.network,
            &build_params,
            &keys,
            &selected,
            change,
            height,
            super::consensus::parameters(build_request.network),
            build_progress.as_deref(),
        )
    });
    // Cryptographic proving cannot be interrupted mid-call. Its result is
    // discarded after lock, and admission to the transport remains cancelled.
    let built = request
        .submission_guard
        .run(async { proof.await.map_err(|_| WalletError::OperationFailed)? })
        .await?;
    let txid = built.txid.clone();
    let persist_txid = txid.clone();
    let raw_hex = hex::encode(&built.raw);
    let expiry = built.expiry_height;
    runtime::update_pending(&request.runtime_key, true, move |state| {
        let anchor_present = std::iter::once(&state.chain)
            .chain(state.checkpoints.iter())
            .any(|cp| cp.height == height && cp.block_hash_hex == hash);
        if !anchor_present || state.chain.height > expiry {
            return Err(WalletError::DlightSpendCacheNotReady);
        }
        let pending = state
            .pending
            .get_mut(&id)
            .ok_or(WalletError::InvalidPreflight)?;
        if !pending.is_active(state.chain.height)
            || pending.nullifiers.iter().any(|nf| {
                !state
                    .chain
                    .notes
                    .iter()
                    .any(|note| note.nullifier_hex == *nf)
            })
        {
            return Err(WalletError::InvalidPreflight);
        }
        pending.txid = Some(persist_txid);
        pending.raw_tx_hex = Some(raw_hex);
        pending.expiry_height = Some(expiry);
        Ok(())
    })
    .await?;
    emit_send_stage(progress.as_deref(), DlightSendStage::Broadcasting);
    // Persist before the first admitted network poll. Any transport error or
    // cancellation retains the reservation until a mined/conflicting/expired
    // chain observation; a dropped RPC is not evidence that no broadcast occurred.
    request
        .submission_guard
        .run(broadcast_transaction(&request.endpoint, built.raw))
        .await
        .map_err(|error| match error {
            WalletError::WalletLocked => WalletError::WalletLocked,
            _ => WalletError::DlightBroadcastUncertain,
        })?;
    drop(reservation);
    Ok(ExecutedSend { txid })
}

struct BuiltTransaction {
    txid: String,
    raw: Vec<u8>,
    expiry_height: u64,
}

fn build_transaction<P: Parameters + Send + Clone>(
    wallet_network: WalletNetwork,
    params: &ExecuteSendParams,
    key_material: &DlightSpendKeyMaterial,
    selected_notes: &[SpendableNote],
    change_sats: u64,
    chain_tip_height: u64,
    network: P,
    progress: Option<&(dyn Fn(DlightSendStage) + Send + Sync)>,
) -> Result<BuiltTransaction, WalletError> {
    let anchor = selected_notes
        .first()
        .map(|note| {
            let node = sapling::Node::from_cmu(&note.note.cmu());
            sapling::Anchor::from(note.merkle_path.root(node))
        })
        .ok_or(WalletError::InsufficientFunds)?;

    let target_height = chain_tip_height.saturating_add(1);
    let target_height = BlockHeight::from_u32(u32::try_from(target_height).map_err(|_| {
        dlight_send_failure(
            "target height selection",
            "next block height could not be represented as u32",
        )
    })?);

    let build_config = BuildConfig::Standard {
        sapling_anchor: Some(anchor),
        orchard_anchor: None,
    };
    let mut builder = Builder::new(network, target_height, build_config);

    for note in selected_notes {
        let fvk = key_material.sapling_fvk_for_scope(note.scope);
        builder
            .add_sapling_spend::<Infallible>(fvk, note.note.clone(), note.merkle_path.clone())
            .map_err(map_build_error)?;
    }

    let send_value = Zatoshis::from_u64(params.value_sats).map_err(|_| {
        dlight_send_failure(
            "amount encoding",
            "send amount could not be represented as zatoshis",
        )
    })?;
    match params.destination_kind {
        DlightDestinationKind::Shielded => {
            let recipient = decode_shielded_delivery(&params.delivery_to_address, wallet_network)?;
            let memo_bytes = parse_memo_bytes(params.memo.as_deref())?;
            builder
                .add_sapling_output::<Infallible>(
                    Some(key_material.sapling_ovk_for_scope(Scope::External)),
                    recipient,
                    send_value,
                    memo_bytes,
                )
                .map_err(map_build_error)?;
        }
        DlightDestinationKind::Transparent => {
            let recipient = decode_transparent_delivery(&params.delivery_to_address)?;
            builder
                .add_transparent_output(&recipient, send_value)
                .map_err(|error| {
                    dlight_send_failure(
                        "transparent output creation",
                        format!("transparent output builder rejected destination: {error}"),
                    )
                })?;
        }
    }

    if change_sats > 0 {
        let change_value = Zatoshis::from_u64(change_sats).map_err(|_| {
            dlight_send_failure(
                "change encoding",
                "change amount could not be represented as zatoshis",
            )
        })?;
        builder
            .add_sapling_output::<Infallible>(
                Some(key_material.sapling_ovk_for_scope(Scope::Internal)),
                key_material.change_payment_address(),
                change_value,
                MemoBytes::empty(),
            )
            .map_err(map_build_error)?;
    }

    // A spend set may include both external and internal (change) notes.
    // Provide both keys so Sapling builder can authorize either scope.
    let sapling_extsks = key_material.sapling_extsks_for_builder();
    let fee_rule =
        fixed::FeeRule::non_standard(Zatoshis::from_u64(params.fee_sats).map_err(|_| {
            dlight_send_failure("fee encoding", "fee could not be represented as zatoshis")
        })?);
    emit_send_stage(progress, DlightSendStage::LoadingProver);
    let proving = load_sapling_provers()?;

    emit_send_stage(progress, DlightSendStage::BuildingProof);
    let build_result = builder
        .build(
            &zcash_transparent::builder::TransparentSigningSet::new(),
            &sapling_extsks,
            &[],
            OsRng,
            &proving.spend,
            &proving.output,
            &fee_rule,
        )
        .map_err(map_build_error)?;

    let tx = build_result.transaction();
    let txid = tx.txid().to_string();

    let mut raw = Vec::<u8>::new();
    tx.write(&mut raw).map_err(|error| {
        dlight_send_failure(
            "transaction serialization",
            format!("failed to serialize transaction bytes: {error}"),
        )
    })?;
    Ok(BuiltTransaction {
        txid,
        raw,
        expiry_height: u64::from(tx.expiry_height()),
    })
}

fn emit_send_stage(
    progress: Option<&(dyn Fn(DlightSendStage) + Send + Sync)>,
    stage: DlightSendStage,
) {
    if let Some(report) = progress {
        report(stage);
    }
}

pub fn normalize_optional_memo(memo: Option<String>) -> Option<String> {
    memo.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn resolve_send_value(
    submitted_sat: i128,
    confirmed_balance: i128,
    fee_sat: i128,
) -> Result<(i128, bool, Option<String>), WalletError> {
    if submitted_sat <= 0 || confirmed_balance <= 0 || fee_sat <= 0 {
        return Err(WalletError::OperationFailed);
    }

    let deducted_amount = submitted_sat.saturating_add(fee_sat);

    if deducted_amount == confirmed_balance.saturating_add(fee_sat) {
        let adjusted = submitted_sat.saturating_sub(fee_sat);
        if adjusted <= 0 {
            return Err(WalletError::InsufficientFunds);
        }
        return Ok((
            adjusted,
            true,
            Some(
                "Fee was deducted from the submitted amount due to available balance.".to_string(),
            ),
        ));
    }

    if deducted_amount > confirmed_balance {
        return Err(WalletError::InsufficientFunds);
    }

    Ok((submitted_sat, false, None))
}

pub fn parse_positive_satoshis(value: &str) -> Result<i128, WalletError> {
    let parsed = parse_decimal_to_satoshis(value)?;
    if parsed <= 0 {
        return Err(WalletError::OperationFailed);
    }
    Ok(parsed)
}

pub fn parse_decimal_to_satoshis(value: &str) -> Result<i128, WalletError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let (is_negative, numeric) = if let Some(rest) = trimmed.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (false, rest)
    } else {
        (false, trimmed)
    };
    if numeric.is_empty() {
        return Err(WalletError::OperationFailed);
    }

    let mut parts = numeric.split('.');
    let whole_part = parts.next().unwrap_or_default();
    let frac_part = parts.next();
    if parts.next().is_some() {
        return Err(WalletError::OperationFailed);
    }

    if !whole_part.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(WalletError::OperationFailed);
    }

    let whole_sat = if whole_part.is_empty() {
        0i128
    } else {
        whole_part
            .parse::<i128>()
            .map_err(|_| WalletError::OperationFailed)?
            .checked_mul(SATOSHIS_PER_COIN)
            .ok_or(WalletError::OperationFailed)?
    };

    let mut frac_sat = 0i128;
    if let Some(frac) = frac_part {
        if !frac.chars().all(|ch| ch.is_ascii_digit()) || frac.len() > 8 {
            return Err(WalletError::OperationFailed);
        }
        if !frac.is_empty() {
            let padded = format!("{frac:0<8}");
            frac_sat = padded
                .parse::<i128>()
                .map_err(|_| WalletError::OperationFailed)?;
        }
    }

    let combined = whole_sat
        .checked_add(frac_sat)
        .ok_or(WalletError::OperationFailed)?;
    Ok(if is_negative { -combined } else { combined })
}

pub fn satoshis_to_decimal_string(value: i128) -> String {
    let is_negative = value.is_negative();
    let absolute = value.unsigned_abs();
    let whole = absolute / SATOSHIS_PER_COIN as u128;
    let frac = absolute % SATOSHIS_PER_COIN as u128;
    if is_negative {
        format!("-{whole}.{frac:08}")
    } else {
        format!("{whole}.{frac:08}")
    }
}

fn map_build_error(error: TxBuildError<Infallible>) -> WalletError {
    match error {
        TxBuildError::InsufficientFunds(_) | TxBuildError::ChangeRequired(_) => {
            WalletError::InsufficientFunds
        }
        _ => dlight_send_failure("transaction build", error.to_string()),
    }
}

fn parse_memo_bytes(memo: Option<&str>) -> Result<MemoBytes, WalletError> {
    let Some(value) = memo.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(MemoBytes::empty());
    };
    MemoBytes::from_bytes(value.as_bytes()).map_err(|error| {
        dlight_send_failure(
            "memo parsing",
            format!("memo is not valid for Sapling encoding: {error}"),
        )
    })
}

fn decode_shielded_delivery(
    address: &str,
    network: WalletNetwork,
) -> Result<sapling::PaymentAddress, WalletError> {
    let hrp = sapling_payment_address_hrp(network);
    zcash_client_backend::encoding::decode_payment_address(hrp, address.trim())
        .map_err(|_| WalletError::InvalidAddress)
}

fn sapling_payment_address_hrp(_network: WalletNetwork) -> &'static str {
    // Parity policy: use zs-addresses on both mainnet and testnet.
    zcash_protocol::constants::mainnet::HRP_SAPLING_PAYMENT_ADDRESS
}

fn decode_transparent_delivery(
    address: &str,
) -> Result<zcash_transparent::address::TransparentAddress, WalletError> {
    super::recipient_resolution::decode_r_address(address)
}

fn select_notes(notes: &[SpendableNote], required_sats: u64) -> Option<(Vec<SpendableNote>, u64)> {
    if required_sats == 0 {
        return Some((vec![], 0));
    }

    let mut selected = Vec::<SpendableNote>::new();
    let mut total = 0u64;
    for note in notes {
        selected.push(note.clone());
        total = total.saturating_add(note.value_sats);
        if total >= required_sats {
            return Some((selected, total));
        }
    }
    None
}

async fn broadcast_transaction(endpoint: &str, raw_tx: Vec<u8>) -> Result<(), WalletError> {
    let grpc_endpoint = normalize_grpc_endpoint(endpoint)?;
    let parsed_uri: Uri = grpc_endpoint
        .parse()
        .map_err(|_| WalletError::UnsupportedChannel)?;
    let host = parsed_uri.host().ok_or(WalletError::UnsupportedChannel)?;
    let is_https = parsed_uri.scheme_str() == Some("https");

    let endpoint_builder = Endpoint::from_shared(grpc_endpoint)
        .map_err(|_| WalletError::UnsupportedChannel)?
        .connect_timeout(Duration::from_secs(DIAL_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(DIAL_RPC_TIMEOUT_SECS))
        .tcp_nodelay(true);

    let endpoint_builder = if is_https {
        endpoint_builder
            .tls_config(
                ClientTlsConfig::new()
                    .with_webpki_roots()
                    .domain_name(host.to_string()),
            )
            .map_err(|error| {
                dlight_send_failure(
                    "lightwalletd TLS setup",
                    format!("failed to configure TLS for broadcast endpoint: {error}"),
                )
            })?
    } else {
        endpoint_builder
    };

    let channel: Channel = endpoint_builder.connect().await.map_err(|error| {
        dlight_send_failure(
            "lightwalletd connection",
            format!("failed to connect to broadcast endpoint: {error}"),
        )
    })?;
    let mut client = CompactTxStreamerClient::new(channel);

    let response = client
        .send_transaction(service::RawTransaction {
            data: raw_tx,
            height: 0,
        })
        .await
        .map_err(|_| WalletError::NetworkError)?
        .into_inner();

    if response.error_code != 0 {
        let error_message = response.error_message.trim().to_string();
        let message_lower = error_message.to_ascii_lowercase();
        if message_lower.contains("insufficient") {
            return Err(WalletError::InsufficientFunds);
        }
        let detail = format!(
            "Broadcast was rejected by lightwalletd (code {}).",
            response.error_code
        );
        return Err(WalletError::DlightBroadcastRejected(detail));
    }

    Ok(())
}

fn dlight_send_failure(stage: &str, detail: impl Into<String>) -> WalletError {
    let detail = detail.into();
    let normalized = detail.split_whitespace().collect::<Vec<_>>().join(" ");
    let suffix = if normalized.is_empty() {
        "unknown error".to_string()
    } else {
        normalized
    };
    WalletError::DlightSendFailed(format!("dlight send failed during {stage}: {suffix}"))
}

#[cfg(test)]
mod tests {
    use crate::types::WalletError;

    use super::{parse_decimal_to_satoshis, resolve_send_value};

    #[test]
    fn parse_decimal_to_satoshis_supports_eight_decimals() {
        assert_eq!(
            parse_decimal_to_satoshis("12.34567890").expect("valid amount"),
            1_234_567_890
        );
        assert_eq!(parse_decimal_to_satoshis("0.00000001").expect("1 sat"), 1);
        assert_eq!(parse_decimal_to_satoshis("1").expect("whole"), 100_000_000);
    }

    #[test]
    fn parse_decimal_to_satoshis_rejects_precision_overflow() {
        assert!(matches!(
            parse_decimal_to_satoshis("1.123456789"),
            Err(WalletError::OperationFailed)
        ));
    }

    #[test]
    fn resolve_send_value_deducts_fee_for_max_send() {
        let (value_sats, fee_taken, message) =
            resolve_send_value(1_000_000, 1_000_000, 10_000).expect("max send should be adjusted");
        assert_eq!(value_sats, 990_000);
        assert!(fee_taken);
        assert!(message.is_some());
    }

    #[test]
    fn resolve_send_value_rejects_insufficient_balance() {
        assert!(matches!(
            resolve_send_value(100_000_000, 50_000_000, 10_000),
            Err(WalletError::InsufficientFunds)
        ));
    }
    #[test]
    #[ignore = "requires canonical Sapling proving files; performs real CPU proving"]
    fn real_proof_roundtrip_preserves_verus_v4_and_pre_zip212_notes() {
        use super::super::state::testing::{self, block, funded_state};
        use super::*;
        use zcash_client_backend::proto::compact_formats::{
            CompactSaplingOutput, CompactSaplingSpend, CompactTx,
        };
        use zcash_primitives::transaction::{
            sighash::SignableInput, sighash_v4::v4_signature_hash, Transaction, TxVersion,
        };
        use zcash_protocol::consensus::BranchId;
        let (mut state, keys, _) = funded_state();
        let notes = state.available_notes().unwrap();
        let address = keys.scope_address(WalletNetwork::Testnet).unwrap();
        let params = ExecuteSendParams {
            destination_kind: DlightDestinationKind::Shielded,
            display_to_address: address.clone(),
            delivery_to_address: address,
            value_sats: 80_000,
            fee_sats: 10_000,
            memo: Some("fixture".into()),
        };
        let built = build_transaction(
            WalletNetwork::Testnet,
            &params,
            &keys,
            &notes,
            10_000,
            1,
            super::super::consensus::parameters(WalletNetwork::Testnet),
            None,
        )
        .unwrap();
        let tx = Transaction::read(built.raw.as_slice(), BranchId::Sapling).unwrap();
        assert_eq!(tx.version(), TxVersion::V4);
        assert_eq!(tx.txid().to_string(), built.txid);
        assert_eq!(u64::from(tx.expiry_height()), built.expiry_height);
        assert!(built.expiry_height > 2);
        let bundle = tx.sapling_bundle().unwrap();
        let sighash = v4_signature_hash(&tx, &SignableInput::Shielded);
        let mut verifier = sapling::BatchValidator::new();
        assert!(verifier.check_bundle(bundle.clone(), sighash.as_bytes().try_into().unwrap()));
        let provers = load_sapling_provers().unwrap();
        assert!(verifier.validate(
            &provers.spend.verifying_key(),
            &provers.output.verifying_key(),
            OsRng
        ));
        assert!(Arc::ptr_eq(&provers, &load_sapling_provers().unwrap()));
        let mut tx_hash = hex::decode(&built.txid).unwrap();
        tx_hash.reverse();
        let compact = CompactTx {
            hash: tx_hash,
            spends: bundle
                .shielded_spends()
                .iter()
                .map(|spend| CompactSaplingSpend {
                    nf: spend.nullifier().0.to_vec(),
                })
                .collect(),
            outputs: bundle
                .shielded_outputs()
                .iter()
                .map(|output| CompactSaplingOutput {
                    cmu: output.cmu().to_bytes().to_vec(),
                    ephemeral_key: output.ephemeral_key().0.to_vec(),
                    ciphertext: output.enc_ciphertext()[..52].to_vec(),
                })
                .collect(),
            ..Default::default()
        };
        let tree_size = 1 + compact.outputs.len() as u32;
        testing::scan(
            &mut state,
            &keys,
            vec![block(2, 2, 1, vec![compact], tree_size)],
        )
        .unwrap();
        assert_eq!(state.balances(), (90_000, 0));
        assert_eq!(state.transactions[&built.txid].net_sats, -10_000);
        assert!(state
            .available_notes()
            .unwrap()
            .iter()
            .all(|note| matches!(note.note.rseed(), sapling::Rseed::BeforeZip212(_))));
    }
}
