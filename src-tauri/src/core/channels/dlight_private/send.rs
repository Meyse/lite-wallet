use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::core::auth::{ensure_active_wallet_session, SessionManager};
use crate::core::channels::store::PreflightStore;
use crate::core::updates::{TxSendProgressPayload, EVENT_TX_SEND_PROGRESS};
use crate::types::transaction::SendResult;
use crate::types::WalletError;

use super::spend_engine::{
    execute_send, satoshis_to_decimal_string, DlightSendStage, ExecuteSendParams,
};
use super::{DlightPreflightPayload, DlightRuntimeRequest};

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    request: DlightRuntimeRequest,
    app_handle: &AppHandle,
) -> Result<SendResult, WalletError> {
    let session = session_manager.lock().await;
    let session_id = session
        .active_session_id()
        .ok_or(WalletError::WalletLocked)?
        .to_string();
    let record = preflight_store
        .take(preflight_id, &session_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let active_id = session
        .active_account_id()
        .ok_or(WalletError::WalletLocked)?;
    if active_id.as_str() != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }
    if request.session_id != session_id {
        return Err(WalletError::WalletLocked);
    }
    drop(session);

    let payload: DlightPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    super::runtime::ensure_runtime(&request).await?;
    let runtime_snapshot =
        super::runtime::get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    super::ensure_runtime_ready_for_spend(runtime_snapshot.status_kind)?;
    super::ensure_spend_cache_ready(&request, &runtime_snapshot)?;

    let progress_app = app_handle.clone();
    let progress_channel = record.channel_id.clone();
    let progress_coin_id = payload.coin_id.clone();
    let progress_cancellation = request.submission_guard.cancellation();
    let progress = move |stage: DlightSendStage| {
        if progress_cancellation.is_cancelled() {
            return;
        }
        let payload = TxSendProgressPayload {
            channel: progress_channel.clone(),
            coin_id: progress_coin_id.clone(),
            stage: send_stage_label(stage).to_string(),
        };
        if let Err(err) = progress_app.emit(EVENT_TX_SEND_PROGRESS, &payload) {
            eprintln!(
                "[dlight_private][spend_send] failed to emit tx-send-progress event: {}",
                err
            );
        }
    };

    ensure_active_wallet_session(session_manager, &session_id).await?;
    let executed = execute_send(
        &request,
        &ExecuteSendParams {
            destination_kind: payload.destination_kind,
            display_to_address: payload.display_to_address.clone(),
            delivery_to_address: payload.delivery_to_address.clone(),
            value_sats: payload.value_sats,
            fee_sats: payload.fee_sats,
            memo: payload.memo.clone(),
        },
        Some(Arc::new(progress)),
    )
    .await?;

    Ok(SendResult {
        txid: executed.txid,
        fee: payload.fee,
        value: satoshis_to_decimal_string(i128::from(payload.value_sats)),
        to_address: payload.display_to_address,
        from_address: request.scope_address,
    })
}

fn send_stage_label(stage: DlightSendStage) -> &'static str {
    match stage {
        DlightSendStage::SyncingSpendState => "syncing_spend_state",
        DlightSendStage::LoadingProver => "loading_prover",
        DlightSendStage::BuildingProof => "building_proof",
        DlightSendStage::Broadcasting => "broadcasting",
    }
}
