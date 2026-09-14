use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::task::{Context, Poll};
use std::time::Duration;

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{Address, Bytes, Eip1559TransactionRequest, H256, U256};
use ethers::utils::keccak256;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::core::auth::session::ActiveWalletAccessContext;
use crate::core::auth::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, SessionManager, SessionSubmissionGuard,
};
use crate::core::channels::eth::bridge::delegator::{
    CcurrencyValueMap, CreserveTransfer, CtransferDestination, VerusBridgeDelegatorContract,
};
use crate::core::channels::eth::preflight::EthPreflightPayload;
use crate::core::channels::eth::provider::EthProviderPool;
use crate::core::channels::store::PreflightStore;
use crate::types::transaction::SendResult;
use crate::types::WalletError;

const ERC20_TRANSFER_ABI: &str = r#"[
  {
    "constant": false,
    "inputs": [
      {"name": "_to", "type": "address"},
      {"name": "_value", "type": "uint256"}
    ],
    "name": "transfer",
    "outputs": [{"name": "", "type": "bool"}],
    "type": "function"
  }
]"#;

const ERC20_APPROVE_ABI: &str = r#"[
  {
    "constant": false,
    "inputs": [
      {"name": "_spender", "type": "address"},
      {"name": "_value", "type": "uint256"}
    ],
    "name": "approve",
    "outputs": [{"name": "", "type": "bool"}],
    "type": "function"
  }
]"#;

static ETH_SEND_LOCKS: LazyLock<StdMutex<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EthSubmissionStage {
    Eth,
    Erc20,
    BridgeZeroApproval,
    BridgeApproval,
    BridgeTransfer,
}

impl EthSubmissionStage {
    fn requires_receipt(self) -> bool {
        matches!(self, Self::BridgeZeroApproval | Self::BridgeApproval)
    }

    fn is_final(self) -> bool {
        matches!(self, Self::Eth | Self::Erc20 | Self::BridgeTransfer)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EthSubmissionStatus {
    Prepared,
    BroadcastKnown,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EthPendingSubmission {
    schema_version: u8,
    recovery_id: String,
    preflight_id: String,
    chain_id: u64,
    from_address: String,
    nonce: String,
    stage: EthSubmissionStage,
    status: EthSubmissionStatus,
    raw_signed_transaction: String,
    tx_hash: String,
    payload: EthPreflightPayload,
    result: SendResult,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EthPendingSubmissionReview {
    pub recovery_id: String,
    pub stage: String,
    pub status: String,
    pub txid: String,
    pub fee: String,
    pub value: String,
    pub to_address: String,
    pub from_address: String,
    pub requires_resume: bool,
    pub can_acknowledge: bool,
}

impl EthPendingSubmission {
    fn review(&self) -> EthPendingSubmissionReview {
        EthPendingSubmissionReview {
            recovery_id: self.recovery_id.clone(),
            stage: serde_json::to_value(self.stage)
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
            status: serde_json::to_value(self.status)
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_else(|| "unknown".to_string()),
            txid: self.tx_hash.clone(),
            fee: self.result.fee.clone(),
            value: self.result.value.clone(),
            to_address: self.result.to_address.clone(),
            from_address: self.result.from_address.clone(),
            requires_resume: !(self.stage.is_final()
                && self.status == EthSubmissionStatus::BroadcastKnown),
            can_acknowledge: self.stage.is_final()
                && self.status == EthSubmissionStatus::BroadcastKnown,
        }
    }
}

struct SessionBoundSubmission<F> {
    future: Pin<Box<F>>,
    guard: SessionSubmissionGuard,
}

impl<F> SessionBoundSubmission<F> {
    fn new(future: F, guard: SessionSubmissionGuard) -> Self {
        Self {
            future: Box::pin(future),
            guard,
        }
    }
}

impl<T, F> Future for SessionBoundSubmission<F>
where
    F: Future<Output = Result<T, WalletError>>,
{
    type Output = Result<T, WalletError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        let guard = this.guard.clone();
        guard.poll_admitted(|| this.future.as_mut().poll(cx))
    }
}

#[derive(Clone)]
struct SessionBoundEthOperation {
    session_manager: Arc<Mutex<SessionManager>>,
    session_id: String,
    submission_guard: SessionSubmissionGuard,
}

impl SessionBoundEthOperation {
    fn new(
        session_manager: &Arc<Mutex<SessionManager>>,
        context: &ActiveWalletAccessContext,
    ) -> Self {
        Self {
            session_manager: Arc::clone(session_manager),
            session_id: context.session_id.clone(),
            submission_guard: context.session_submission_guard(),
        }
    }

    /// Await reversible network work and verify the same unlock instance both
    /// before and after the await. The main session mutex is never held while
    /// the network future is pending.
    async fn wait<T, F>(&self, future: F) -> Result<T, WalletError>
    where
        F: Future<Output = Result<T, WalletError>>,
    {
        ensure_active_wallet_session(&self.session_manager, &self.session_id).await?;
        let cancellation = self.submission_guard.cancellation();
        let value = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err(WalletError::WalletLocked),
            result = future => result?,
        };
        ensure_active_wallet_session(&self.session_manager, &self.session_id).await?;
        Ok(value)
    }

    async fn submit<T, F>(&self, future: F) -> Result<T, WalletError>
    where
        F: Future<Output = Result<T, WalletError>>,
    {
        ensure_active_wallet_session(&self.session_manager, &self.session_id).await?;
        self.submit_after_validation(future).await
    }

    /// Atomically orders invalidation against every poll of the submission
    /// future. Invalidation first means the provider future cannot advance.
    /// A poll admitted first may already transmit the RPC request before it
    /// returns; cancellation cannot undo that submitted request, so its broadcast
    /// outcome must be reconciled before any retry.
    async fn submit_after_validation<T, F>(&self, future: F) -> Result<T, WalletError>
    where
        F: Future<Output = Result<T, WalletError>>,
    {
        let cancellation = self.submission_guard.cancellation();
        let submission = SessionBoundSubmission::new(future, self.submission_guard.clone());
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(WalletError::WalletLocked),
            result = submission => result,
        }
    }
}

pub async fn send(
    preflight_id: &str,
    preflight_store: &PreflightStore,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let wallet_network = context.wallet_network;
    let network_provider = provider_pool.for_network(wallet_network)?;
    let send_lock = eth_send_lock(&context.account_id, wallet_network);
    let _send_guard = send_lock.lock().await;

    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let active_eth_address: Address = context
        .eth_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    let record = preflight_store
        .take(preflight_id, &context.session_id)
        .ok_or(WalletError::InvalidPreflight)?;
    if context.account_id != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }
    let payload: EthPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;
    validate_payload_binding(&payload, network_provider.chain_id, active_eth_address)?;

    // A newly reviewed request must never authorize progress on an older
    // durable operation. Recovery is a separate, explicit command keyed by an
    // opaque recovery id and displays the old operation before any effect.
    if let Some(pending) = load_pending_submission(&context).await? {
        validate_pending_binding(&pending, &context, network_provider.chain_id)?;
        return Err(WalletError::EthRecoveryRequired(pending.recovery_id));
    }

    let private_key = load_primary_private_scalar_for_context(&context).await?;
    let wallet = LocalWallet::from_bytes(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?
        .with_chain_id(network_provider.chain_id);
    if wallet.address() != active_eth_address {
        return Err(WalletError::InvalidPreflight);
    }
    let signer = Arc::new(SignerMiddleware::new(
        network_provider.rpc_provider.clone(),
        wallet,
    ));
    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let session_operation = SessionBoundEthOperation::new(session_manager, &context);

    execute_payload(
        payload,
        None,
        preflight_id,
        &context,
        &session_operation,
        network_provider,
        signer,
    )
    .await
}

pub async fn get_pending_submission_review(
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<Option<EthPendingSubmissionReview>, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let network_provider = provider_pool.for_network(context.wallet_network)?;
    let send_lock = eth_send_lock(&context.account_id, context.wallet_network);
    let _send_guard = send_lock.lock().await;
    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let Some(pending) = load_pending_submission(&context).await? else {
        return Ok(None);
    };
    validate_pending_binding(&pending, &context, network_provider.chain_id)?;
    Ok(Some(pending.review()))
}

pub async fn resume_pending_submission(
    recovery_id: &str,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<SendResult, WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let network_provider = provider_pool.for_network(context.wallet_network)?;
    let send_lock = eth_send_lock(&context.account_id, context.wallet_network);
    let _send_guard = send_lock.lock().await;
    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let pending = load_pending_submission(&context)
        .await?
        .ok_or(WalletError::InvalidPreflight)?;
    if pending.recovery_id != recovery_id {
        return Err(WalletError::InvalidPreflight);
    }
    validate_pending_binding(&pending, &context, network_provider.chain_id)?;
    if pending.stage.is_final() && pending.status == EthSubmissionStatus::BroadcastKnown {
        return Ok(pending.result);
    }

    let private_key = load_primary_private_scalar_for_context(&context).await?;
    let wallet = LocalWallet::from_bytes(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?
        .with_chain_id(network_provider.chain_id);
    let active_eth_address: Address = context
        .eth_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    if wallet.address() != active_eth_address {
        return Err(WalletError::InvalidPreflight);
    }
    let signer = Arc::new(SignerMiddleware::new(
        network_provider.rpc_provider.clone(),
        wallet,
    ));
    let operation = SessionBoundEthOperation::new(session_manager, &context);
    resume_pending_submission_explicit(pending, &context, &operation, network_provider, signer)
        .await
}

pub async fn acknowledge_pending_submission(
    recovery_id: &str,
    session_manager: &Arc<Mutex<SessionManager>>,
    provider_pool: &EthProviderPool,
) -> Result<(), WalletError> {
    let context = capture_active_wallet_access_context(session_manager).await?;
    let network_provider = provider_pool.for_network(context.wallet_network)?;
    let send_lock = eth_send_lock(&context.account_id, context.wallet_network);
    let _send_guard = send_lock.lock().await;
    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let pending = load_pending_submission(&context)
        .await?
        .ok_or(WalletError::InvalidPreflight)?;
    if pending.recovery_id != recovery_id {
        return Err(WalletError::InvalidPreflight);
    }
    validate_pending_binding(&pending, &context, network_provider.chain_id)?;
    if !pending.stage.is_final() || pending.status != EthSubmissionStatus::BroadcastKnown {
        return Err(WalletError::InvalidPreflight);
    }
    let operation = SessionBoundEthOperation::new(session_manager, &context);
    clear_pending_submission(&context, &operation).await
}

fn eth_send_lock(account_id: &str, network: crate::types::wallet::WalletNetwork) -> Arc<Mutex<()>> {
    let key = format!("{}:{:?}", account_id, network);
    let mut locks = ETH_SEND_LOCKS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    locks
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn payload_chain_and_from(payload: &EthPreflightPayload) -> (u64, &str) {
    match payload {
        EthPreflightPayload::Eth {
            chain_id,
            from_address,
            ..
        }
        | EthPreflightPayload::Erc20 {
            chain_id,
            from_address,
            ..
        }
        | EthPreflightPayload::Bridge {
            chain_id,
            from_address,
            ..
        } => (*chain_id, from_address),
    }
}

fn validate_payload_binding(
    payload: &EthPreflightPayload,
    expected_chain_id: u64,
    expected_from: Address,
) -> Result<(), WalletError> {
    let (chain_id, from_address) = payload_chain_and_from(payload);
    let parsed_from: Address = from_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    if chain_id != expected_chain_id || parsed_from != expected_from {
        return Err(WalletError::InvalidPreflight);
    }
    Ok(())
}

fn payload_result(payload: &EthPreflightPayload) -> SendResult {
    match payload {
        EthPreflightPayload::Eth {
            from_address,
            to_address,
            fee,
            value,
            ..
        }
        | EthPreflightPayload::Erc20 {
            from_address,
            to_address,
            fee,
            value,
            ..
        }
        | EthPreflightPayload::Bridge {
            from_address,
            to_address,
            fee,
            value,
            ..
        } => SendResult {
            txid: String::new(),
            fee: fee.clone(),
            value: value.clone(),
            to_address: to_address.clone(),
            from_address: from_address.clone(),
            recovery_id: None,
        },
    }
}

async fn execute_payload(
    payload: EthPreflightPayload,
    resume_after: Option<EthSubmissionStage>,
    preflight_id: &str,
    context: &ActiveWalletAccessContext,
    session_operation: &SessionBoundEthOperation,
    network_provider: &crate::core::channels::eth::provider::EthNetworkProvider,
    signer: Arc<
        SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>,
    >,
) -> Result<SendResult, WalletError> {
    match payload.clone() {
        EthPreflightPayload::Eth {
            from_address,
            to_address,
            value_wei,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let value_wei = parse_u256(&value_wei)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;

            let tx: TypedTransaction = Eip1559TransactionRequest::new()
                .from(parsed_from)
                .to(parsed_to)
                .value(value_wei)
                .gas(gas_limit)
                .max_fee_per_gas(max_fee_per_gas)
                .max_priority_fee_per_gas(max_priority_fee_per_gas)
                .chain_id(network_provider.chain_id)
                .into();

            submit_final_transaction(
                tx,
                EthSubmissionStage::Eth,
                payload,
                preflight_id,
                context,
                session_operation,
                network_provider,
                signer,
            )
            .await
        }
        EthPreflightPayload::Erc20 {
            from_address,
            to_address,
            token_address,
            token_value_raw,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            max_fee_cap,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let parsed_to: Address = to_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let token_address: Address = token_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let amount_raw = parse_u256(&token_value_raw)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;
            let max_fee_cap = parse_u256(&max_fee_cap)?;

            let fee_data = session_operation
                .wait(async {
                    network_provider
                        .rpc_provider
                        .estimate_eip1559_fees(None)
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await?;
            let current_max_fee = fee_data.0;

            if fee_drift_exceeds_cap(gas_limit, current_max_fee, max_fee_cap) {
                return Err(WalletError::OperationFailed);
            }

            let abi: Abi = serde_json::from_str(ERC20_TRANSFER_ABI)
                .map_err(|_| WalletError::OperationFailed)?;
            let contract = Contract::new(token_address, abi, signer.clone());

            let transfer_call = contract
                .method::<_, bool>("transfer", (parsed_to, amount_raw))
                .map_err(|_| WalletError::OperationFailed)?;

            let configured_call = transfer_call
                .from(parsed_from)
                .gas(gas_limit)
                .gas_price(max_fee_per_gas.max(max_priority_fee_per_gas));

            submit_final_transaction(
                configured_call.tx,
                EthSubmissionStage::Erc20,
                payload,
                preflight_id,
                context,
                session_operation,
                network_provider,
                signer,
            )
            .await
        }
        EthPreflightPayload::Bridge {
            from_address,
            to_address: _,
            source_contract,
            source_amount_token_raw,
            reserve_transfer_version,
            reserve_transfer_currency,
            reserve_transfer_amount,
            reserve_transfer_flags,
            reserve_transfer_fee_currency_id,
            reserve_transfer_fees,
            reserve_transfer_destination_type,
            reserve_transfer_destination_address,
            reserve_transfer_dest_currency_id,
            reserve_transfer_dest_system_id,
            reserve_transfer_second_reserve_id,
            bridge_contract,
            transfer_value_wei,
            gas_limit,
            transfer_gas_limit,
            approval_gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            max_fee_cap,
            approval_zero_out,
            ..
        } => {
            let parsed_from: Address = from_address
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let source_contract: Address = source_contract
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let bridge_contract: Address = bridge_contract
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_currency: Address = reserve_transfer_currency
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_fee_currency: Address = reserve_transfer_fee_currency_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_dest_currency: Address = reserve_transfer_dest_currency_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_dest_system: Address = reserve_transfer_dest_system_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;
            let reserve_second_reserve: Address = reserve_transfer_second_reserve_id
                .parse()
                .map_err(|_| WalletError::InvalidAddress)?;

            let reserve_amount = parse_u64(&reserve_transfer_amount)?;
            let reserve_destination_address =
                parse_hex_bytes(&reserve_transfer_destination_address)?;
            let transfer_value_wei = parse_u256(&transfer_value_wei)?;
            let gas_limit = parse_u256(&gas_limit)?;
            let transfer_gas_limit = parse_u256(&transfer_gas_limit)?;
            let approval_gas_limit = parse_u256(&approval_gas_limit)?;
            let max_fee_per_gas = parse_u256(&max_fee_per_gas)?;
            let max_priority_fee_per_gas = parse_u256(&max_priority_fee_per_gas)?;
            let max_fee_cap = parse_u256(&max_fee_cap)?;

            let fee_data = session_operation
                .wait(async {
                    network_provider
                        .rpc_provider
                        .estimate_eip1559_fees(None)
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await?;
            let current_max_fee = fee_data.0;
            if fee_drift_exceeds_cap(gas_limit, current_max_fee, max_fee_cap) {
                return Err(WalletError::BridgeGasDriftExceeded);
            }

            if source_contract != Address::zero() {
                let approval_amount_raw = source_amount_token_raw
                    .as_deref()
                    .ok_or(WalletError::InvalidPreflight)
                    .and_then(parse_u256)?;

                let abi: Abi = serde_json::from_str(ERC20_APPROVE_ABI)
                    .map_err(|_| WalletError::OperationFailed)?;
                let token_contract = Contract::new(source_contract, abi, signer.clone());

                let approval_gas_price = max_fee_per_gas.max(max_priority_fee_per_gas);

                if approval_zero_out && resume_after.is_none() {
                    let zero_call = token_contract
                        .method::<_, bool>("approve", (bridge_contract, U256::zero()))
                        .map_err(|_| WalletError::OperationFailed)?;
                    let zero_call = zero_call
                        .from(parsed_from)
                        .gas(approval_gas_limit)
                        .gas_price(approval_gas_price);
                    submit_approval_transaction(
                        zero_call.tx,
                        EthSubmissionStage::BridgeZeroApproval,
                        payload.clone(),
                        preflight_id,
                        context,
                        session_operation,
                        network_provider,
                        signer.clone(),
                    )
                    .await?;
                }

                if !matches!(resume_after, Some(EthSubmissionStage::BridgeApproval)) {
                    let approval_call = token_contract
                        .method::<_, bool>("approve", (bridge_contract, approval_amount_raw))
                        .map_err(|_| WalletError::OperationFailed)?;
                    let approval_call = approval_call
                        .from(parsed_from)
                        .gas(approval_gas_limit)
                        .gas_price(approval_gas_price);
                    submit_approval_transaction(
                        approval_call.tx,
                        EthSubmissionStage::BridgeApproval,
                        payload.clone(),
                        preflight_id,
                        context,
                        session_operation,
                        network_provider,
                        signer.clone(),
                    )
                    .await?;
                }
            }

            let reserve_transfer = CreserveTransfer {
                version: reserve_transfer_version,
                currencyvalue: CcurrencyValueMap {
                    currency: reserve_currency,
                    amount: reserve_amount,
                },
                flags: reserve_transfer_flags,
                feecurrencyid: reserve_fee_currency,
                fees: reserve_transfer_fees,
                destination: CtransferDestination {
                    destinationtype: reserve_transfer_destination_type,
                    destinationaddress: reserve_destination_address,
                },
                destcurrencyid: reserve_dest_currency,
                destsystemid: reserve_dest_system,
                secondreserveid: reserve_second_reserve,
            };

            let delegator = VerusBridgeDelegatorContract::new(bridge_contract, signer.clone());
            let send_transfer_call = delegator
                .send_transfer(reserve_transfer)
                .from(parsed_from)
                .gas(transfer_gas_limit)
                .gas_price(max_fee_per_gas.max(max_priority_fee_per_gas))
                .value(transfer_value_wei);
            submit_final_transaction(
                send_transfer_call.tx,
                EthSubmissionStage::BridgeTransfer,
                payload,
                preflight_id,
                context,
                session_operation,
                network_provider,
                signer,
            )
            .await
        }
    }
}

async fn load_pending_submission(
    context: &ActiveWalletAccessContext,
) -> Result<Option<EthPendingSubmission>, WalletError> {
    let Some(bytes) = context
        .stronghold_store
        .load_eth_pending_submission(
            &context.account_id,
            context.password_hash(),
            context.wallet_network,
        )
        .await?
    else {
        return Ok(None);
    };
    let pending = serde_json::from_slice::<EthPendingSubmission>(&bytes)
        .map_err(|_| WalletError::SecureStorageUnavailable)?;
    if pending.schema_version != 2 || pending.recovery_id.trim().is_empty() {
        return Err(WalletError::SecureStorageUnavailable);
    }
    Ok(Some(pending))
}

fn validate_pending_binding(
    pending: &EthPendingSubmission,
    context: &ActiveWalletAccessContext,
    expected_chain_id: u64,
) -> Result<(), WalletError> {
    let expected_from: Address = context
        .eth_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    validate_payload_binding(&pending.payload, expected_chain_id, expected_from)?;
    if pending.chain_id != expected_chain_id
        || !pending
            .from_address
            .eq_ignore_ascii_case(&context.eth_address)
    {
        return Err(WalletError::SecureStorageUnavailable);
    }
    let stage_matches_payload = match (&pending.payload, pending.stage) {
        (EthPreflightPayload::Eth { .. }, EthSubmissionStage::Eth)
        | (EthPreflightPayload::Erc20 { .. }, EthSubmissionStage::Erc20)
        | (EthPreflightPayload::Bridge { .. }, EthSubmissionStage::BridgeTransfer) => true,
        (
            EthPreflightPayload::Bridge {
                source_contract, ..
            },
            EthSubmissionStage::BridgeZeroApproval | EthSubmissionStage::BridgeApproval,
        ) => source_contract
            .parse::<Address>()
            .is_ok_and(|address| address != Address::zero()),
        _ => false,
    };
    if !stage_matches_payload
        || (pending.status == EthSubmissionStatus::Confirmed && !pending.stage.requires_receipt())
    {
        return Err(WalletError::SecureStorageUnavailable);
    }
    Ok(())
}

async fn persist_pending_submission(
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
    pending: &EthPendingSubmission,
) -> Result<(), WalletError> {
    let bytes = serde_json::to_vec(pending).map_err(|_| WalletError::OperationFailed)?;
    operation
        .wait(context.stronghold_store.store_eth_pending_submission(
            &context.account_id,
            context.password_hash(),
            context.wallet_network,
            &bytes,
        ))
        .await
}

async fn clear_pending_submission(
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
) -> Result<(), WalletError> {
    operation
        .wait(
            context
                .stronghold_store
                .clear_eth_pending_submission(&context.account_id, context.wallet_network),
        )
        .await
}

async fn prepare_pending_submission(
    mut tx: TypedTransaction,
    stage: EthSubmissionStage,
    payload: EthPreflightPayload,
    preflight_id: &str,
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
    signer: &SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>,
) -> Result<EthPendingSubmission, WalletError> {
    operation
        .wait(async {
            signer
                .fill_transaction(&mut tx, None)
                .await
                .map_err(|_| WalletError::NetworkError)
        })
        .await?;

    let expected_from: Address = context
        .eth_address
        .parse()
        .map_err(|_| WalletError::InvalidAddress)?;
    if tx.from().copied() != Some(expected_from)
        || tx.chain_id().map(|value| value.as_u64()) != Some(payload_chain_and_from(&payload).0)
    {
        return Err(WalletError::InvalidPreflight);
    }

    let nonce = tx.nonce().copied().ok_or(WalletError::OperationFailed)?;
    let signature = operation
        .wait(async {
            signer
                .signer()
                .sign_transaction(&tx)
                .await
                .map_err(|_| WalletError::OperationFailed)
        })
        .await?;
    let raw = tx.rlp_signed(&signature);
    let tx_hash = H256::from(keccak256(raw.as_ref()));
    let recovery_id = uuid::Uuid::new_v4().to_string();
    let mut result = payload_result(&payload);
    result.txid = format!("{tx_hash:#x}");
    result.recovery_id = Some(recovery_id.clone());

    let pending = EthPendingSubmission {
        schema_version: 2,
        recovery_id,
        preflight_id: preflight_id.to_string(),
        chain_id: payload_chain_and_from(&payload).0,
        from_address: context.eth_address.clone(),
        nonce: nonce.to_string(),
        stage,
        status: EthSubmissionStatus::Prepared,
        raw_signed_transaction: hex::encode(raw),
        tx_hash: format!("{tx_hash:#x}"),
        payload,
        result,
    };
    persist_pending_submission(context, operation, &pending).await?;
    Ok(pending)
}

async fn broadcast_pending_submission(
    pending: &EthPendingSubmission,
    operation: &SessionBoundEthOperation,
    rpc_provider: &Provider<Http>,
) -> Result<(), WalletError> {
    let tx_hash: H256 = pending
        .tx_hash
        .parse()
        .map_err(|_| WalletError::SecureStorageUnavailable)?;
    let already_visible = operation
        .wait(async {
            rpc_provider
                .get_transaction(tx_hash)
                .await
                .map_err(|_| WalletError::NetworkError)
        })
        .await
        .map_err(|error| map_eth_uncertain(error, &pending.tx_hash))?
        .is_some();
    if already_visible {
        return Ok(());
    }

    let raw = hex::decode(&pending.raw_signed_transaction)
        .map(Bytes::from)
        .map_err(|_| WalletError::SecureStorageUnavailable)?;
    let submitted = operation
        .submit(async {
            rpc_provider
                .send_raw_transaction(raw)
                .await
                .map(|_| ())
                .map_err(|_| WalletError::NetworkError)
        })
        .await;
    match submitted {
        Ok(()) => Ok(()),
        Err(WalletError::WalletLocked) => Err(WalletError::WalletLocked),
        Err(_) => {
            let visible = operation
                .wait(async {
                    rpc_provider
                        .get_transaction(tx_hash)
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await
                .map_err(|error| map_eth_uncertain(error, &pending.tx_hash))?
                .is_some();
            if visible {
                Ok(())
            } else {
                Err(WalletError::EthBroadcastUncertain(pending.tx_hash.clone()))
            }
        }
    }
}

fn map_eth_uncertain(error: WalletError, tx_hash: &str) -> WalletError {
    match error {
        WalletError::WalletLocked => WalletError::WalletLocked,
        _ => WalletError::EthBroadcastUncertain(tx_hash.to_string()),
    }
}

async fn wait_for_approval_receipt(
    pending: &EthPendingSubmission,
    operation: &SessionBoundEthOperation,
    rpc_provider: &Provider<Http>,
) -> Result<(), WalletError> {
    let tx_hash: H256 = pending
        .tx_hash
        .parse()
        .map_err(|_| WalletError::SecureStorageUnavailable)?;
    loop {
        let receipt = operation
            .wait(async {
                rpc_provider
                    .get_transaction_receipt(tx_hash)
                    .await
                    .map_err(|_| WalletError::NetworkError)
            })
            .await
            .map_err(|error| map_eth_uncertain(error, &pending.tx_hash))?;
        if let Some(receipt) = receipt {
            if receipt.status.map(|status| status.as_u64()) == Some(1) {
                return Ok(());
            }
            return Err(WalletError::BridgeApprovalFailed);
        }
        operation
            .wait(async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(())
            })
            .await?;
    }
}

async fn submit_approval_transaction(
    tx: TypedTransaction,
    stage: EthSubmissionStage,
    payload: EthPreflightPayload,
    preflight_id: &str,
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
    network_provider: &crate::core::channels::eth::provider::EthNetworkProvider,
    signer: Arc<
        SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>,
    >,
) -> Result<(), WalletError> {
    let mut pending = prepare_pending_submission(
        tx,
        stage,
        payload,
        preflight_id,
        context,
        operation,
        signer.as_ref(),
    )
    .await?;
    broadcast_pending_submission(&pending, operation, &network_provider.rpc_provider).await?;
    pending.status = EthSubmissionStatus::BroadcastKnown;
    persist_pending_submission(context, operation, &pending).await?;
    if let Err(error) =
        wait_for_approval_receipt(&pending, operation, &network_provider.rpc_provider).await
    {
        if matches!(error, WalletError::BridgeApprovalFailed) {
            clear_pending_submission(context, operation).await?;
        }
        return Err(error);
    }
    pending.status = EthSubmissionStatus::Confirmed;
    persist_pending_submission(context, operation, &pending).await
}

async fn submit_final_transaction(
    tx: TypedTransaction,
    stage: EthSubmissionStage,
    payload: EthPreflightPayload,
    preflight_id: &str,
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
    network_provider: &crate::core::channels::eth::provider::EthNetworkProvider,
    signer: Arc<
        SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>,
    >,
) -> Result<SendResult, WalletError> {
    let mut pending = prepare_pending_submission(
        tx,
        stage,
        payload,
        preflight_id,
        context,
        operation,
        signer.as_ref(),
    )
    .await?;
    broadcast_pending_submission(&pending, operation, &network_provider.rpc_provider).await?;
    pending.status = EthSubmissionStatus::BroadcastKnown;
    persist_pending_submission(context, operation, &pending).await?;
    Ok(pending.result)
}

async fn resume_pending_submission_explicit(
    mut pending: EthPendingSubmission,
    context: &ActiveWalletAccessContext,
    operation: &SessionBoundEthOperation,
    network_provider: &crate::core::channels::eth::provider::EthNetworkProvider,
    signer: Arc<
        SignerMiddleware<ethers::providers::Provider<ethers::providers::Http>, LocalWallet>,
    >,
) -> Result<SendResult, WalletError> {
    validate_pending_binding(&pending, context, network_provider.chain_id)?;

    if pending.status == EthSubmissionStatus::Prepared {
        broadcast_pending_submission(&pending, operation, &network_provider.rpc_provider).await?;
        pending.status = EthSubmissionStatus::BroadcastKnown;
        persist_pending_submission(context, operation, &pending).await?;
    }

    if pending.stage.requires_receipt() && pending.status == EthSubmissionStatus::BroadcastKnown {
        if let Err(error) =
            wait_for_approval_receipt(&pending, operation, &network_provider.rpc_provider).await
        {
            if matches!(error, WalletError::BridgeApprovalFailed) {
                clear_pending_submission(context, operation).await?;
            }
            return Err(error);
        }
        pending.status = EthSubmissionStatus::Confirmed;
        persist_pending_submission(context, operation, &pending).await?;
    }

    if pending.stage.is_final() {
        return Ok(pending.result);
    }

    if pending.status != EthSubmissionStatus::Confirmed {
        return Err(WalletError::EthBroadcastUncertain(pending.tx_hash));
    }

    execute_payload(
        pending.payload,
        Some(pending.stage),
        &pending.preflight_id,
        context,
        operation,
        network_provider,
        signer,
    )
    .await
}

fn parse_u256(input: &str) -> Result<U256, WalletError> {
    U256::from_dec_str(input.trim()).map_err(|_| WalletError::OperationFailed)
}

fn parse_u64(input: &str) -> Result<u64, WalletError> {
    input
        .trim()
        .parse::<u64>()
        .map_err(|_| WalletError::OperationFailed)
}

fn parse_hex_bytes(input: &str) -> Result<Bytes, WalletError> {
    let trimmed = input.trim().trim_start_matches("0x");
    let raw = hex::decode(trimmed).map_err(|_| WalletError::OperationFailed)?;
    Ok(Bytes::from(raw))
}

fn fee_drift_exceeds_cap(
    gas_limit: U256,
    current_max_fee_per_gas: U256,
    max_fee_cap: U256,
) -> bool {
    gas_limit.saturating_mul(current_max_fee_per_gas) > max_fee_cap
}

#[cfg(test)]
mod tests {
    use super::{
        fee_drift_exceeds_cap, EthPendingSubmission, EthSubmissionStage, EthSubmissionStatus,
        SessionBoundEthOperation, SessionBoundSubmission,
    };
    use crate::core::auth::session::ActiveWalletAccessContext;
    use crate::core::auth::{
        capture_active_wallet_access_context, ensure_active_wallet_session, SessionManager,
    };
    use crate::core::channels::eth::preflight::EthPreflightPayload;
    use crate::core::channels::eth::EthProviderPool;
    use crate::core::channels::store::{PreflightRecord, PreflightStore};
    use crate::core::crypto::{derive_public_profile_from_material, Network};
    use crate::core::StrongholdStore;
    use crate::types::transaction::SendResult;
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
    use crate::types::WalletError;
    use ethers::providers::{Http, Provider};
    use ethers::types::U256;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::{Mutex, Notify};
    use zeroize::Zeroizing;

    async fn test_operation() -> (Arc<Mutex<SessionManager>>, SessionBoundEthOperation) {
        let path = std::env::temp_dir().join(format!(
            "lite_wallet_eth_send_session_{}",
            uuid::Uuid::new_v4()
        ));
        let session = Arc::new(Mutex::new(SessionManager::new(
            StrongholdStore::new_for_tests(path),
        )));
        unlock(&session, "initial unlock").await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("active context");
        let operation = SessionBoundEthOperation::new(&session, &context);
        (session, operation)
    }

    async fn test_session_with_stored_seed(
        material: &str,
    ) -> (Arc<Mutex<SessionManager>>, ActiveWalletAccessContext) {
        let _ = iota_stronghold::engine::snapshot::try_set_encrypt_work_factor(0);
        let path = std::env::temp_dir().join(format!(
            "lite_wallet_eth_send_caller_{}",
            uuid::Uuid::new_v4()
        ));
        let store = StrongholdStore::new_for_tests(path);
        store
            .store_seed("account-1", material, &[7u8; 32])
            .await
            .expect("store test seed");
        let session = Arc::new(Mutex::new(SessionManager::new(store)));
        unlock(&session, material).await;
        let context = capture_active_wallet_access_context(&session)
            .await
            .expect("active test context");
        (session, context)
    }

    async fn unlock(session: &Arc<Mutex<SessionManager>>, material: &str) {
        let profile = derive_public_profile_from_material(
            material,
            WalletSecretKind::SeedText,
            Network::Mainnet,
        )
        .expect("public profile");
        session.lock().await.unlock_with_profile(
            "account-1".to_string(),
            WalletNetwork::Mainnet,
            WalletSecretKind::SeedText,
            profile,
            Zeroizing::new(vec![7u8; 32]),
        );
    }

    async fn lock_and_reunlock_same_account(session: &Arc<Mutex<SessionManager>>) {
        session.lock().await.lock();
        unlock(session, "replacement unlock").await;
    }

    fn pending_submission(stage: EthSubmissionStage) -> EthPendingSubmission {
        EthPendingSubmission {
            schema_version: 2,
            recovery_id: "recovery-1".to_string(),
            preflight_id: "preflight-1".to_string(),
            chain_id: 1,
            from_address: "0x1111111111111111111111111111111111111111".to_string(),
            nonce: "7".to_string(),
            stage,
            status: EthSubmissionStatus::Prepared,
            raw_signed_transaction: "02deadbeef".to_string(),
            tx_hash: format!("0x{}", "ab".repeat(32)),
            payload: EthPreflightPayload::Eth {
                chain_id: 1,
                coin_id: "ETH".to_string(),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                value_wei: "1".to_string(),
                gas_limit: "21000".to_string(),
                max_fee_per_gas: "2".to_string(),
                max_priority_fee_per_gas: "1".to_string(),
                fee: "0.000000000000042".to_string(),
                value: "0.000000000000000001".to_string(),
            },
            result: SendResult {
                txid: format!("0x{}", "ab".repeat(32)),
                fee: "0.000000000000042".to_string(),
                value: "0.000000000000000001".to_string(),
                to_address: "0x2222222222222222222222222222222222222222".to_string(),
                from_address: "0x1111111111111111111111111111111111111111".to_string(),
                recovery_id: Some("recovery-1".to_string()),
            },
        }
    }

    fn eth_payload(from_address: &str, to_address: &str) -> EthPreflightPayload {
        EthPreflightPayload::Eth {
            chain_id: 1,
            coin_id: "ETH".to_string(),
            from_address: from_address.to_string(),
            to_address: to_address.to_string(),
            value_wei: "1".to_string(),
            gas_limit: "21000".to_string(),
            max_fee_per_gas: "2".to_string(),
            max_priority_fee_per_gas: "1".to_string(),
            fee: "0.000000000000042".to_string(),
            value: "0.000000000000000001".to_string(),
        }
    }

    fn bridge_payload(from_address: &str) -> EthPreflightPayload {
        EthPreflightPayload::Bridge {
            chain_id: 1,
            coin_id: "vETH".to_string(),
            channel_id: "erc20.vETH".to_string(),
            from_address: from_address.to_string(),
            refund_vrpc_address: "RtestRefund".to_string(),
            to_address: "RtestDestination".to_string(),
            source_contract: "0x4444444444444444444444444444444444444444".to_string(),
            source_decimals: 8,
            source_amount_sats: "1".to_string(),
            source_amount_token_raw: Some("1".to_string()),
            mapped_currency_iaddress: "iTestCurrency".to_string(),
            mapped_currency_eth_address: "0x6666666666666666666666666666666666666666".to_string(),
            reserve_transfer_version: 1,
            reserve_transfer_currency: "0x6666666666666666666666666666666666666666".to_string(),
            reserve_transfer_amount: "1".to_string(),
            reserve_transfer_flags: 0,
            reserve_transfer_fee_currency_id: "0x0000000000000000000000000000000000000000"
                .to_string(),
            reserve_transfer_fees: 0,
            reserve_transfer_destination_type: 2,
            reserve_transfer_destination_address: "11".repeat(20),
            reserve_transfer_dest_currency_id: "0x0000000000000000000000000000000000000000"
                .to_string(),
            reserve_transfer_dest_system_id: "0x0000000000000000000000000000000000000000"
                .to_string(),
            reserve_transfer_second_reserve_id: "0x0000000000000000000000000000000000000000"
                .to_string(),
            bridge_contract: "0x5555555555555555555555555555555555555555".to_string(),
            transfer_value_wei: "0".to_string(),
            gas_limit: "500000".to_string(),
            transfer_gas_limit: "300000".to_string(),
            approval_gas_limit: "80000".to_string(),
            max_fee_per_gas: "2".to_string(),
            max_priority_fee_per_gas: "1".to_string(),
            max_fee_cap: "1000000000000000000000000000000".to_string(),
            approval_zero_out: false,
            fee: "0.001".to_string(),
            value: "0.00000001".to_string(),
        }
    }

    async fn store_pending(context: &ActiveWalletAccessContext, pending: &EthPendingSubmission) {
        context
            .stronghold_store
            .store_eth_pending_submission(
                &context.account_id,
                context.password_hash(),
                context.wallet_network,
                &serde_json::to_vec(pending).expect("serialize pending"),
            )
            .await
            .expect("store pending submission");
    }

    fn put_preflight(
        store: &PreflightStore,
        context: &ActiveWalletAccessContext,
        id: &str,
        payload: EthPreflightPayload,
    ) {
        store.activate_wallet_session(&context.session_id);
        assert!(store.put(
            id.to_string(),
            PreflightRecord {
                session_id: context.session_id.clone(),
                channel_id: "eth.ETH".to_string(),
                account_id: context.account_id.clone(),
                payload: serde_json::to_value(payload).expect("serialize preflight"),
            },
        ));
    }

    async fn read_json_rpc_request(stream: &mut tokio::net::TcpStream) -> serde_json::Value {
        let mut request = Vec::new();
        let expected_len = loop {
            let mut chunk = [0u8; 1024];
            let read = stream
                .read(&mut chunk)
                .await
                .expect("read JSON-RPC request");
            assert!(read > 0, "connection closed before request completed");
            request.extend_from_slice(&chunk[..read]);
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end + 4]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .expect("content-length");
                break header_end + 4 + content_length;
            }
        };
        while request.len() < expected_len {
            let mut chunk = [0u8; 1024];
            let read = stream.read(&mut chunk).await.expect("read JSON-RPC body");
            assert!(read > 0, "connection closed before body completed");
            request.extend_from_slice(&chunk[..read]);
        }
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("header terminator")
            + 4;
        serde_json::from_slice(&request[body_start..expected_len]).expect("JSON-RPC body")
    }

    async fn write_json_rpc_response(stream: &mut tokio::net::TcpStream, body: serde_json::Value) {
        let body = body.to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write JSON-RPC response");
    }

    struct BroadcastOnSecondPoll {
        polls: Arc<AtomicUsize>,
        broadcasts: Arc<AtomicUsize>,
    }

    impl Future for BroadcastOnSecondPoll {
        type Output = Result<(), WalletError>;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.polls.fetch_add(1, Ordering::SeqCst) == 0 {
                Poll::Pending
            } else {
                self.broadcasts.fetch_add(1, Ordering::SeqCst);
                Poll::Ready(Ok(()))
            }
        }
    }

    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn fee_drift_exceeds_cap_returns_true_when_current_fee_is_higher_than_preflight_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(40u64);
        let cap = U256::from(3_900_000u64);

        assert!(fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }

    #[test]
    fn fee_drift_exceeds_cap_returns_false_when_within_cap() {
        let gas_limit = U256::from(100_000u64);
        let current_fee = U256::from(39u64);
        let cap = U256::from(3_900_000u64);

        assert!(!fee_drift_exceeds_cap(gas_limit, current_fee, cap));
    }

    #[test]
    fn pending_submission_serialization_preserves_exact_replay_for_every_stage() {
        for stage in [
            EthSubmissionStage::Eth,
            EthSubmissionStage::Erc20,
            EthSubmissionStage::BridgeZeroApproval,
            EthSubmissionStage::BridgeApproval,
            EthSubmissionStage::BridgeTransfer,
        ] {
            let expected = pending_submission(stage);
            let bytes = serde_json::to_vec(&expected).expect("serialize pending submission");
            let actual: EthPendingSubmission =
                serde_json::from_slice(&bytes).expect("deserialize pending submission");

            assert_eq!(actual.schema_version, 2);
            assert_eq!(actual.recovery_id, "recovery-1");
            assert_eq!(actual.stage, stage);
            assert_eq!(actual.status, EthSubmissionStatus::Prepared);
            assert_eq!(actual.nonce, "7");
            assert_eq!(actual.raw_signed_transaction, "02deadbeef");
            assert_eq!(actual.tx_hash, format!("0x{}", "ab".repeat(32)));
        }
    }

    #[test]
    fn approval_and_final_stage_classification_covers_all_recovery_variants() {
        assert!(EthSubmissionStage::BridgeZeroApproval.requires_receipt());
        assert!(EthSubmissionStage::BridgeApproval.requires_receipt());
        assert!(!EthSubmissionStage::Eth.requires_receipt());
        assert!(!EthSubmissionStage::Erc20.requires_receipt());
        assert!(!EthSubmissionStage::BridgeTransfer.requires_receipt());

        assert!(EthSubmissionStage::Eth.is_final());
        assert!(EthSubmissionStage::Erc20.is_final());
        assert!(EthSubmissionStage::BridgeTransfer.is_final());
        assert!(!EthSubmissionStage::BridgeZeroApproval.is_final());
        assert!(!EthSubmissionStage::BridgeApproval.is_final());
    }

    #[tokio::test]
    async fn send_validates_and_consumes_new_request_without_resuming_old_confirmed_approval() {
        let (session, context) = test_session_with_stored_seed("caller boundary seed").await;
        let mut old = pending_submission(EthSubmissionStage::BridgeApproval);
        old.status = EthSubmissionStatus::Confirmed;
        old.from_address = context.eth_address.clone();
        old.payload = bridge_payload(&context.eth_address);
        old.result = super::payload_result(&old.payload);
        old.result.txid = old.tx_hash.clone();
        old.result.recovery_id = Some(old.recovery_id.clone());
        store_pending(&context, &old).await;

        let preflights = PreflightStore::new();
        put_preflight(
            &preflights,
            &context,
            "new-preflight",
            eth_payload(
                &context.eth_address,
                "0x3333333333333333333333333333333333333333",
            ),
        );
        let providers = EthProviderPool::for_tests(WalletNetwork::Mainnet, "http://127.0.0.1:9");

        let error = super::send("new-preflight", &preflights, &session, &providers)
            .await
            .expect_err("old operation requires explicit recovery");
        assert!(matches!(
            error,
            WalletError::EthRecoveryRequired(recovery_id) if recovery_id == old.recovery_id
        ));
        assert!(preflights
            .get("new-preflight", &context.session_id)
            .is_none());
        let persisted = super::load_pending_submission(&context)
            .await
            .expect("load journal")
            .expect("journal remains");
        assert_eq!(persisted.recovery_id, old.recovery_id);
        assert_eq!(persisted.stage, EthSubmissionStage::BridgeApproval);
        assert_eq!(persisted.status, EthSubmissionStatus::Confirmed);
    }

    #[tokio::test]
    async fn terminal_result_survives_release_until_explicit_acknowledgement() {
        let (session, context) = test_session_with_stored_seed("result release seed").await;
        let mut pending = pending_submission(EthSubmissionStage::Eth);
        pending.status = EthSubmissionStatus::BroadcastKnown;
        pending.from_address = context.eth_address.clone();
        pending.payload = eth_payload(
            &context.eth_address,
            "0x2222222222222222222222222222222222222222",
        );
        pending.result.from_address = context.eth_address.clone();
        store_pending(&context, &pending).await;
        let providers = EthProviderPool::for_tests(WalletNetwork::Mainnet, "http://127.0.0.1:9");

        let first = super::resume_pending_submission(&pending.recovery_id, &session, &providers)
            .await
            .expect("release stored result");
        let second = super::resume_pending_submission(&pending.recovery_id, &session, &providers)
            .await
            .expect("release same stored result again");
        assert_eq!(first.txid, pending.result.txid);
        assert_eq!(second.txid, first.txid);
        assert!(super::load_pending_submission(&context)
            .await
            .expect("load before ack")
            .is_some());

        super::acknowledge_pending_submission(&pending.recovery_id, &session, &providers)
            .await
            .expect("acknowledge released result");
        assert!(super::load_pending_submission(&context)
            .await
            .expect("load after ack")
            .is_none());
    }

    #[tokio::test]
    async fn terminal_result_survives_lock_and_reunlock_before_release() {
        let material = "result restart seed";
        let (session, context) = test_session_with_stored_seed(material).await;
        let mut pending = pending_submission(EthSubmissionStage::Eth);
        pending.status = EthSubmissionStatus::BroadcastKnown;
        pending.from_address = context.eth_address.clone();
        pending.payload = eth_payload(
            &context.eth_address,
            "0x2222222222222222222222222222222222222222",
        );
        pending.result.from_address = context.eth_address.clone();
        store_pending(&context, &pending).await;

        session.lock().await.lock();
        unlock(&session, material).await;
        let resumed_context = capture_active_wallet_access_context(&session)
            .await
            .expect("re-unlocked context");
        assert_ne!(resumed_context.session_id, context.session_id);
        let providers = EthProviderPool::for_tests(WalletNetwork::Mainnet, "http://127.0.0.1:9");
        let result = super::resume_pending_submission(&pending.recovery_id, &session, &providers)
            .await
            .expect("release result after session restart");
        assert_eq!(result.txid, pending.result.txid);
        assert!(super::load_pending_submission(&resumed_context)
            .await
            .expect("load journal after restart")
            .is_some());
    }

    #[tokio::test]
    async fn explicit_resume_of_confirmed_approval_submits_only_the_old_final_bridge_stage() {
        let (session, context) = test_session_with_stored_seed("bridge recovery seed").await;
        let mut pending = pending_submission(EthSubmissionStage::BridgeApproval);
        pending.status = EthSubmissionStatus::Confirmed;
        pending.from_address = context.eth_address.clone();
        pending.payload = bridge_payload(&context.eth_address);
        pending.result = super::payload_result(&pending.payload);
        pending.result.txid = pending.tx_hash.clone();
        pending.result.recovery_id = Some(pending.recovery_id.clone());
        store_pending(&context, &pending).await;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mocked Ethereum RPC");
        let address = listener.local_addr().expect("mocked RPC address");
        let server = tokio::spawn(async move {
            let mut methods = Vec::new();
            loop {
                let (mut stream, _) = listener.accept().await.expect("accept RPC request");
                let request = read_json_rpc_request(&mut stream).await;
                let id = request.get("id").cloned().expect("request id");
                let method = request
                    .get("method")
                    .and_then(serde_json::Value::as_str)
                    .expect("RPC method")
                    .to_string();
                methods.push(method.clone());
                let result = match method.as_str() {
                    "eth_getBlockByNumber" => serde_json::json!({
                        "hash": null,
                        "number": "0x1",
                        "logsBloom": null,
                        "totalDifficulty": null,
                        "transactions": [],
                        "size": null,
                        "mixHash": null,
                        "nonce": null,
                        "baseFeePerGas": "0x1",
                        "gasUsed": "0x1",
                        "gasLimit": "0x2"
                    }),
                    "eth_feeHistory" => serde_json::json!({
                        "oldestBlock": "0x1",
                        "baseFeePerGas": ["0x1", "0x1"],
                        "gasUsedRatio": [0.5],
                        "reward": [["0x1"]]
                    }),
                    "eth_getTransactionCount" => serde_json::json!("0x7"),
                    "eth_getTransactionByHash" => serde_json::Value::Null,
                    "eth_sendRawTransaction" => serde_json::json!(format!("0x{}", "cd".repeat(32))),
                    other => panic!("unexpected Ethereum RPC method: {other}"),
                };
                write_json_rpc_response(
                    &mut stream,
                    serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}),
                )
                .await;
                if method == "eth_sendRawTransaction" {
                    break methods;
                }
            }
        });
        let providers =
            EthProviderPool::for_tests(WalletNetwork::Mainnet, &format!("http://{address}"));

        let result = super::resume_pending_submission(&pending.recovery_id, &session, &providers)
            .await
            .expect("explicit bridge resume");
        let methods = server.await.expect("mocked RPC server");
        assert_eq!(
            methods
                .iter()
                .filter(|method| method.as_str() == "eth_sendRawTransaction")
                .count(),
            1
        );
        assert_eq!(result.to_address, "RtestDestination");
        let recovered = super::load_pending_submission(&context)
            .await
            .expect("load recovered journal")
            .expect("terminal journal remains");
        assert_eq!(recovered.stage, EthSubmissionStage::BridgeTransfer);
        assert_eq!(recovered.status, EthSubmissionStatus::BroadcastKnown);
        assert_ne!(recovered.recovery_id, pending.recovery_id);
    }

    #[tokio::test]
    async fn invalid_preflight_does_not_touch_existing_recovery_record() {
        let (session, context) = test_session_with_stored_seed("invalid preflight seed").await;
        let mut pending = pending_submission(EthSubmissionStage::Eth);
        pending.status = EthSubmissionStatus::BroadcastKnown;
        pending.from_address = context.eth_address.clone();
        pending.payload = eth_payload(
            &context.eth_address,
            "0x2222222222222222222222222222222222222222",
        );
        pending.result.from_address = context.eth_address.clone();
        store_pending(&context, &pending).await;
        let preflights = PreflightStore::new();
        preflights.activate_wallet_session(&context.session_id);
        let providers = EthProviderPool::for_tests(WalletNetwork::Mainnet, "http://127.0.0.1:9");

        let error = super::send("missing", &preflights, &session, &providers)
            .await
            .expect_err("invalid preflight");
        assert!(matches!(error, WalletError::InvalidPreflight));
        assert!(super::load_pending_submission(&context)
            .await
            .expect("load journal")
            .is_some());
    }

    #[tokio::test]
    async fn uncertain_retry_replays_the_exact_same_signed_transaction_bytes() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mocked JSON-RPC server");
        let address = listener.local_addr().expect("mocked server address");
        let raw = "0x02deadbeef".to_string();
        let expected_hash = format!("0x{}", "ab".repeat(32));
        let server = tokio::spawn({
            let raw = raw.clone();
            let expected_hash = expected_hash.clone();
            async move {
                let mut submitted_raw = Vec::new();
                for request_index in 0..5 {
                    let (mut stream, _) = listener.accept().await.expect("accept JSON-RPC request");
                    let request = read_json_rpc_request(&mut stream).await;
                    let id = request.get("id").cloned().expect("request id");
                    match request.get("method").and_then(serde_json::Value::as_str) {
                        Some("eth_getTransactionByHash") => {
                            write_json_rpc_response(
                                &mut stream,
                                serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
                            )
                            .await;
                        }
                        Some("eth_sendRawTransaction") => {
                            let sent = request
                                .get("params")
                                .and_then(serde_json::Value::as_array)
                                .and_then(|params| params.first())
                                .and_then(serde_json::Value::as_str)
                                .expect("raw transaction parameter")
                                .to_string();
                            submitted_raw.push(sent);
                            if request_index == 1 {
                                write_json_rpc_response(
                                    &mut stream,
                                    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":"uncertain transport"}}),
                                )
                                .await;
                            } else {
                                write_json_rpc_response(
                                    &mut stream,
                                    serde_json::json!({"jsonrpc":"2.0","id":id,"result":expected_hash}),
                                )
                                .await;
                            }
                        }
                        method => panic!("unexpected JSON-RPC method: {method:?}"),
                    }
                }
                assert_eq!(submitted_raw, vec![raw.clone(), raw]);
            }
        });

        let provider = Provider::<Http>::try_from(format!("http://{address}").as_str())
            .expect("mocked provider");
        let (_session, operation) = test_operation().await;
        let pending = pending_submission(EthSubmissionStage::Eth);

        let first = super::broadcast_pending_submission(&pending, &operation, &provider).await;
        assert!(matches!(first, Err(WalletError::EthBroadcastUncertain(_))));
        super::broadcast_pending_submission(&pending, &operation, &provider)
            .await
            .expect("exact replay succeeds");
        server.await.expect("mocked JSON-RPC server");
    }

    #[tokio::test]
    async fn eth_lock_after_final_validation_prevents_submission_admission() {
        let (session, operation) = test_operation().await;
        ensure_active_wallet_session(&session, &operation.session_id)
            .await
            .expect("final validation");
        lock_and_reunlock_same_account(&session).await;

        let polls = Arc::new(AtomicUsize::new(0));
        let submission_polls = Arc::clone(&polls);
        let result = operation
            .submit_after_validation(async move {
                submission_polls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await;

        assert!(matches!(result, Err(WalletError::WalletLocked)));
        assert_eq!(polls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn eth_invalidation_between_pending_polls_prevents_later_broadcast() {
        let (session, operation) = test_operation().await;
        let polls = Arc::new(AtomicUsize::new(0));
        let broadcasts = Arc::new(AtomicUsize::new(0));
        let future = BroadcastOnSecondPoll {
            polls: Arc::clone(&polls),
            broadcasts: Arc::clone(&broadcasts),
        };
        let mut submission = Box::pin(SessionBoundSubmission::new(
            future,
            operation.submission_guard.clone(),
        ));

        {
            let waker = Waker::noop();
            let mut context = Context::from_waker(waker);
            assert!(submission.as_mut().poll(&mut context).is_pending());
        }
        assert_eq!(polls.load(Ordering::SeqCst), 1);
        assert_eq!(broadcasts.load(Ordering::SeqCst), 0);

        lock_and_reunlock_same_account(&session).await;

        let result = {
            let waker = Waker::noop();
            let mut context = Context::from_waker(waker);
            submission.as_mut().poll(&mut context)
        };
        assert!(matches!(
            result,
            Poll::Ready(Err(WalletError::WalletLocked))
        ));
        assert_eq!(polls.load(Ordering::SeqCst), 1);
        assert_eq!(broadcasts.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn erc20_lock_during_fee_wait_prevents_transfer_submission() {
        let (session, operation) = test_operation().await;
        let fee_wait_started = Arc::new(Notify::new());
        let transfer_submissions = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let fee_wait_started = Arc::clone(&fee_wait_started);
            let transfer_submissions = Arc::clone(&transfer_submissions);
            async move {
                operation
                    .wait(async move {
                        fee_wait_started.notify_one();
                        std::future::pending::<Result<(), WalletError>>().await
                    })
                    .await?;
                operation
                    .submit(async move {
                        transfer_submissions.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            }
        });

        fee_wait_started.notified().await;
        lock_and_reunlock_same_account(&session).await;

        let result = task.await.expect("fee wait task");
        assert!(matches!(result, Err(WalletError::WalletLocked)));
        assert_eq!(transfer_submissions.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn bridge_lock_during_approval_receipt_prevents_final_submission() {
        let (session, operation) = test_operation().await;
        let receipt_wait_started = Arc::new(Notify::new());
        let approval_submissions = Arc::new(AtomicUsize::new(0));
        let final_submissions = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let receipt_wait_started = Arc::clone(&receipt_wait_started);
            let approval_submissions = Arc::clone(&approval_submissions);
            let final_submissions = Arc::clone(&final_submissions);
            async move {
                operation
                    .submit(async move {
                        approval_submissions.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await?;
                operation
                    .wait(async move {
                        receipt_wait_started.notify_one();
                        std::future::pending::<Result<(), WalletError>>().await
                    })
                    .await?;
                operation
                    .submit(async move {
                        final_submissions.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            }
        });

        receipt_wait_started.notified().await;
        lock_and_reunlock_same_account(&session).await;

        let result = task.await.expect("approval receipt task");
        assert!(matches!(result, Err(WalletError::WalletLocked)));
        assert_eq!(approval_submissions.load(Ordering::SeqCst), 1);
        assert_eq!(final_submissions.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn invalidation_drops_a_pending_submission_future() {
        let (session, operation) = test_operation().await;
        let submission_started = Arc::new(Notify::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let task = tokio::spawn({
            let submission_started = Arc::clone(&submission_started);
            let dropped = Arc::clone(&dropped);
            async move {
                operation
                    .submit(async move {
                        let _drop_marker = DropMarker(dropped);
                        submission_started.notify_one();
                        std::future::pending::<Result<(), WalletError>>().await
                    })
                    .await
            }
        });

        submission_started.notified().await;
        lock_and_reunlock_same_account(&session).await;

        let result = task.await.expect("pending submission task");
        assert!(matches!(result, Err(WalletError::WalletLocked)));
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn session_bound_wait_and_submission_complete_for_active_session() {
        let (_session, operation) = test_operation().await;
        assert_eq!(
            operation
                .wait(async { Ok::<_, WalletError>(7) })
                .await
                .expect("reversible wait"),
            7
        );
        assert_eq!(
            operation
                .submit(async { Ok::<_, WalletError>(11) })
                .await
                .expect("submission"),
            11
        );
    }
}
