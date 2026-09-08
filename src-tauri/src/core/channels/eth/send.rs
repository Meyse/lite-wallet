use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::middleware::SignerMiddleware;
use ethers::providers::Middleware;
use ethers::signers::{LocalWallet, Signer};
use ethers::types::Bytes;
use ethers::types::{Address, Eip1559TransactionRequest, U256};
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
    let record = preflight_store
        .take(preflight_id, &context.session_id)
        .ok_or(WalletError::InvalidPreflight)?;

    let payload: EthPreflightPayload =
        serde_json::from_value(record.payload).map_err(|_| WalletError::InvalidPreflight)?;

    if context.account_id != record.account_id {
        return Err(WalletError::InvalidPreflight);
    }

    let wallet_network = context.wallet_network;
    let private_key = load_primary_private_scalar_for_context(&context).await?;

    let network_provider = provider_pool.for_network(wallet_network)?;
    let wallet = LocalWallet::from_bytes(&*private_key)
        .map_err(|_| WalletError::OperationFailed)?
        .with_chain_id(network_provider.chain_id);

    let signer = Arc::new(SignerMiddleware::new(
        network_provider.rpc_provider.clone(),
        wallet,
    ));
    ensure_active_wallet_session(session_manager, &context.session_id).await?;
    let session_operation = SessionBoundEthOperation::new(session_manager, &context);

    match payload {
        EthPreflightPayload::Eth {
            from_address,
            to_address,
            value_wei,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            fee,
            value,
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

            let tx = Eip1559TransactionRequest::new()
                .from(parsed_from)
                .to(parsed_to)
                .value(value_wei)
                .gas(gas_limit)
                .max_fee_per_gas(max_fee_per_gas)
                .max_priority_fee_per_gas(max_priority_fee_per_gas)
                .chain_id(network_provider.chain_id);

            let pending = session_operation
                .submit(async {
                    signer
                        .send_transaction(tx, None)
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
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
            fee,
            value,
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

            let pending = session_operation
                .submit(async {
                    configured_call
                        .send()
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
        EthPreflightPayload::Bridge {
            from_address,
            to_address,
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
            fee,
            value,
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

                if approval_zero_out {
                    let zero_call = token_contract
                        .method::<_, bool>("approve", (bridge_contract, U256::zero()))
                        .map_err(|_| WalletError::OperationFailed)?;
                    let zero_call = zero_call
                        .from(parsed_from)
                        .gas(approval_gas_limit)
                        .gas_price(approval_gas_price);
                    let zero_pending = session_operation
                        .submit(async {
                            zero_call
                                .send()
                                .await
                                .map_err(|_| WalletError::NetworkError)
                        })
                        .await?;
                    let zero_receipt = session_operation
                        .wait(async { zero_pending.await.map_err(|_| WalletError::NetworkError) })
                        .await?;
                    let zero_ok = zero_receipt
                        .and_then(|receipt| receipt.status)
                        .map(|status| status.as_u64() == 1)
                        .unwrap_or(false);
                    if !zero_ok {
                        return Err(WalletError::BridgeApprovalFailed);
                    }
                }

                let approval_call = token_contract
                    .method::<_, bool>("approve", (bridge_contract, approval_amount_raw))
                    .map_err(|_| WalletError::OperationFailed)?;
                let approval_call = approval_call
                    .from(parsed_from)
                    .gas(approval_gas_limit)
                    .gas_price(approval_gas_price);
                let approval_pending = session_operation
                    .submit(async {
                        approval_call
                            .send()
                            .await
                            .map_err(|_| WalletError::NetworkError)
                    })
                    .await?;
                let approval_receipt = session_operation
                    .wait(async {
                        approval_pending
                            .await
                            .map_err(|_| WalletError::NetworkError)
                    })
                    .await?;
                let approval_ok = approval_receipt
                    .and_then(|receipt| receipt.status)
                    .map(|status| status.as_u64() == 1)
                    .unwrap_or(false);
                if !approval_ok {
                    return Err(WalletError::BridgeApprovalFailed);
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
            let pending = session_operation
                .submit(async {
                    send_transfer_call
                        .send()
                        .await
                        .map_err(|_| WalletError::NetworkError)
                })
                .await?;

            Ok(SendResult {
                txid: format!("{:#x}", pending.tx_hash()),
                fee,
                value,
                to_address,
                from_address,
            })
        }
    }
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
    use super::{fee_drift_exceeds_cap, SessionBoundEthOperation, SessionBoundSubmission};
    use crate::core::auth::{
        capture_active_wallet_access_context, ensure_active_wallet_session, SessionManager,
    };
    use crate::core::crypto::{derive_public_profile_from_material, Network};
    use crate::core::StrongholdStore;
    use crate::types::wallet::{WalletNetwork, WalletSecretKind};
    use crate::types::WalletError;
    use ethers::types::U256;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
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
            Zeroizing::new(vec![1, 2, 3]),
        );
    }

    async fn lock_and_reunlock_same_account(session: &Arc<Mutex<SessionManager>>) {
        session.lock().await.lock();
        unlock(session, "replacement unlock").await;
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
