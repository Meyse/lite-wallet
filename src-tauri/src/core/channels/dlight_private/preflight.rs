use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::channels::dlight_private::destination::DlightDestinationKind;
use crate::core::channels::store::{PreflightRecord, PreflightStore};
use crate::core::channels::vrpc::VrpcProvider;
use crate::types::transaction::{PreflightParams, PreflightResult};
use crate::types::WalletError;

use super::spend_engine::compute_preflight;
use super::DlightRuntimeRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlightPreflightPayload {
    pub coin_id: String,
    pub destination_kind: DlightDestinationKind,
    pub display_to_address: String,
    pub delivery_to_address: String,
    pub from_address: String,
    pub value: String,
    pub fee: String,
    pub value_sats: u64,
    pub fee_sats: u64,
    pub memo: Option<String>,
}

pub async fn preflight(
    params: PreflightParams,
    preflight_store: &PreflightStore,
    account_id: &str,
    session_id: &str,
    channel_id: &str,
    request: DlightRuntimeRequest,
    vrpc_provider: &VrpcProvider,
) -> Result<PreflightResult, WalletError> {
    // Preflight should use runtime state only; avoid triggering full spend sync work here.
    let _ = super::runtime::ensure_runtime(&request);
    let runtime_snapshot =
        super::runtime::get_runtime_snapshot(&request.runtime_key).unwrap_or_default();
    super::ensure_runtime_ready_for_spend(runtime_snapshot.status_kind)?;
    super::ensure_spend_cache_ready(&request, &runtime_snapshot)?;
    let confirmed_balance_sats = if runtime_snapshot.confirmed_sats <= 0 {
        0
    } else {
        u64::try_from(runtime_snapshot.confirmed_sats).map_err(|_| WalletError::OperationFailed)?
    };

    println!(
        "[dlight_private][spend_preflight] channel={} coin={} to={}",
        channel_id,
        params.coin_id,
        params.to_address.trim()
    );

    super::spend_params::ensure_prover_ready()?;

    let preflight = compute_preflight(
        &request,
        &params.to_address,
        &params.amount,
        params.memo.clone(),
        confirmed_balance_sats,
        vrpc_provider,
    )
    .await?;

    let preflight_id = Uuid::new_v4().to_string();
    let payload = DlightPreflightPayload {
        coin_id: params.coin_id.clone(),
        destination_kind: preflight.resolved_recipient.destination_kind,
        display_to_address: preflight.resolved_recipient.display_to_address.clone(),
        delivery_to_address: preflight.resolved_recipient.delivery_to_address.clone(),
        from_address: request.scope_address.clone(),
        value: preflight.value.clone(),
        fee: preflight.fee.clone(),
        value_sats: preflight.value_sats,
        fee_sats: preflight.fee_sats,
        memo: preflight.memo.clone(),
    };
    let payload_value = serde_json::to_value(&payload).map_err(|_| WalletError::OperationFailed)?;

    if !preflight_store.put(
        preflight_id.clone(),
        PreflightRecord {
            session_id: session_id.to_string(),
            channel_id: channel_id.to_string(),
            account_id: account_id.to_string(),
            payload: payload_value,
        },
    ) {
        return Err(WalletError::WalletLocked);
    }

    Ok(PreflightResult {
        preflight_id,
        fee: payload.fee.clone(),
        fee_currency: params.coin_id,
        value: payload.value.clone(),
        amount_submitted: params.amount,
        to_address: payload.display_to_address.clone(),
        from_address: payload.from_address.clone(),
        fee_taken_from_amount: preflight.fee_taken_from_amount,
        fee_taken_message: preflight.fee_taken_message,
        warnings: vec![],
        memo: payload.memo,
    })
}
