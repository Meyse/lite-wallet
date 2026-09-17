use std::sync::{Arc, Mutex as StdMutex};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use super::btc::BtcProviderPool;
use super::eth::EthProviderPool;
use super::route_preflight;
use super::vrpc::identity::verus_tx::codec::encode_hex;
use super::vrpc::identity::verus_tx::model::{VerusTx, VerusTxIn, VerusTxOut};
use super::vrpc::intent::decode_currency_id;
use super::vrpc::VrpcProviderPool;
use super::PreflightStore;
use crate::core::auth::{SessionManager, StrongholdStore};
use crate::core::coins::CoinRegistry;
use crate::core::crypto::DerivedPublicProfile;
use crate::types::transaction::PreflightParams;
use crate::types::wallet::{WalletNetwork, WalletSecretKind};
use crate::types::WalletError;

const VRSC_SYSTEM_ID: &str = "i5w5MuNik5NtLcYmNzcvaoixooEebB6MGV";
const TOKEN_ID: &str = "i61cV2uicKSi1rSMQCBNQeSYC3UAi9GVzd";
const OTHER_TOKEN_ID: &str = "iHog9UCTrn95qpUBFCZ7kKz7qWdMA8MQ6N";

#[derive(Clone, Copy)]
enum DestinationKind {
    Transparent,
    Identity,
}

struct VrpcFixtureState {
    resolved_currency_id: String,
    previous_tx: String,
    previous_txid: String,
    previous_script: Vec<u8>,
    funded_tx: String,
    requested_output: Option<Value>,
}

fn transparent_address(version: u8, byte: u8) -> String {
    let mut payload = vec![version];
    payload.extend_from_slice(&[byte; 20]);
    bs58::encode(payload).with_check().into_string()
}

fn push(data: &[u8]) -> Vec<u8> {
    let mut output = if data.len() <= 75 {
        vec![data.len() as u8]
    } else {
        vec![76, data.len() as u8]
    };
    output.extend_from_slice(data);
    output
}

fn reserve_output_script(destination_type: u8, destination: [u8; 20], amount: u8) -> Vec<u8> {
    let mut destination_chunk = Vec::with_capacity(21);
    if destination_type != 2 {
        destination_chunk.push(destination_type);
    }
    destination_chunk.extend_from_slice(&destination);

    let mut token = vec![1];
    token.extend_from_slice(&decode_currency_id(TOKEN_ID).expect("valid token currency id"));
    token.push(amount);

    let mut master = push(&[3, 0, 1, 1]);
    master.extend_from_slice(&push(&destination_chunk));
    let mut condition = push(&[3, 9, 1, 1]);
    condition.extend_from_slice(&push(&destination_chunk));
    condition.extend_from_slice(&push(&token));

    let mut script = push(&master);
    script.push(0xcc);
    script.extend_from_slice(&push(&condition));
    script.push(0x75);
    script
}

fn p2pkh_script(byte: u8) -> Vec<u8> {
    let mut script = vec![0x76, 0xa9, 0x14];
    script.extend_from_slice(&[byte; 20]);
    script.extend_from_slice(&[0x88, 0xac]);
    script
}

fn transaction_id(tx_hex: &str) -> (String, [u8; 32]) {
    let digest = Sha256::digest(Sha256::digest(
        hex::decode(tx_hex).expect("fixture transaction hex"),
    ));
    let mut txid_le = [0u8; 32];
    txid_le.copy_from_slice(&digest);
    let mut display = digest.to_vec();
    display.reverse();
    (hex::encode(display), txid_le)
}

fn vrpc_fixture_state(
    resolved_currency_id: &str,
    destination_kind: DestinationKind,
) -> VrpcFixtureState {
    let previous_script = reserve_output_script(2, [1; 20], 75);
    let previous_tx = encode_hex(&VerusTx {
        version: 4,
        overwintered: true,
        version_group_id: 0x892f2085,
        inputs: vec![],
        outputs: vec![VerusTxOut {
            value: 20_000,
            script_pub_key: previous_script.clone(),
        }],
        lock_time: 0,
        expiry_height: 0,
        value_balance: 0,
    })
    .expect("previous transaction");
    let (previous_txid, previous_txid_le) = transaction_id(&previous_tx);
    let destination_type = match destination_kind {
        DestinationKind::Transparent => 2,
        DestinationKind::Identity => 4,
    };
    let funded_tx = encode_hex(&VerusTx {
        version: 4,
        overwintered: true,
        version_group_id: 0x892f2085,
        inputs: vec![VerusTxIn {
            prevout_txid_le: previous_txid_le,
            prevout_vout: 0,
            script_sig: vec![],
            sequence: u32::MAX,
        }],
        outputs: vec![
            VerusTxOut {
                value: 0,
                script_pub_key: reserve_output_script(destination_type, [2; 20], 50),
            },
            VerusTxOut {
                value: 0,
                script_pub_key: reserve_output_script(2, [1; 20], 25),
            },
            VerusTxOut {
                value: 10_000,
                script_pub_key: p2pkh_script(1),
            },
        ],
        lock_time: 0,
        expiry_height: 0,
        value_balance: 0,
    })
    .expect("funded transaction");

    VrpcFixtureState {
        resolved_currency_id: resolved_currency_id.to_string(),
        previous_tx,
        previous_txid,
        previous_script,
        funded_tx,
        requested_output: None,
    }
}

async fn start_vrpc_fixture(
    state: VrpcFixtureState,
) -> (
    String,
    Arc<StdMutex<VrpcFixtureState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let url = format!("http://{}", listener.local_addr().expect("fixture address"));
    let state = Arc::new(StdMutex::new(state));
    let server_state = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.expect("accept fixture request");
            let state = server_state.clone();
            tokio::spawn(async move {
                let mut request_bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                let (body_start, content_length) = loop {
                    let read = socket
                        .read(&mut buffer)
                        .await
                        .expect("read fixture request");
                    if read == 0 {
                        return;
                    }
                    request_bytes.extend_from_slice(&buffer[..read]);
                    if let Some(header_end) =
                        request_bytes.windows(4).position(|w| w == b"\r\n\r\n")
                    {
                        let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request_bytes.len() >= header_end + 4 + content_length {
                            break (header_end + 4, content_length);
                        }
                    }
                };
                let request: Value =
                    serde_json::from_slice(&request_bytes[body_start..body_start + content_length])
                        .expect("JSON-RPC request");
                let method = request["method"].as_str().expect("RPC method");
                let result = {
                    let mut state = state.lock().expect("fixture state");
                    match method {
                        "getidentity" => json!({
                            "identity": {
                                "identityaddress": transparent_address(102, 2),
                                "primaryaddresses": [transparent_address(60, 2)]
                            }
                        }),
                        "getaddressutxos" => json!([{
                            "txid": state.previous_txid,
                            "vout": 0,
                            "satoshis": 20_000,
                            "scriptPubKey": hex::encode(&state.previous_script)
                        }]),
                        "getcurrency" => json!({ "currencyid": state.resolved_currency_id }),
                        "sendcurrency" => {
                            state.requested_output = request["params"][1][0]
                                .as_object()
                                .map(|_| request["params"][1][0].clone());
                            json!(state.funded_tx)
                        }
                        "fundrawtransaction" => json!({
                            "hex": state.funded_tx,
                            "fee": 0.0001
                        }),
                        "getrawtransaction" => json!(state.previous_tx),
                        _ => panic!("unexpected VRPC method: {method}"),
                    }
                };
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": result
                })
                .to_string();
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.len(), response
                        )
                        .as_bytes(),
                    )
                    .await
                    .expect("write fixture response");
            });
        }
    });
    (url, state, task)
}

fn session() -> (Arc<Mutex<SessionManager>>, String, String) {
    let vrpc_address = transparent_address(60, 1);
    let mut session = SessionManager::new(StrongholdStore::new_for_tests(
        std::env::temp_dir().join(format!("preflight-router-{}", uuid::Uuid::new_v4())),
    ));
    let session_id = session.unlock_with_profile(
        "preflight-router-account".to_string(),
        WalletNetwork::Mainnet,
        WalletSecretKind::SeedText,
        DerivedPublicProfile {
            address: vrpc_address.clone(),
            eth_address: format!("0x{}", "11".repeat(20)),
            btc_address: transparent_address(0, 1),
            pub_hex: String::new(),
        },
        zeroize::Zeroizing::new(vec![]),
    );
    (Arc::new(Mutex::new(session)), session_id, vrpc_address)
}

async fn run_token_preflight(
    provider_currency: &str,
    destination: &str,
    destination_kind: DestinationKind,
) -> (
    Result<crate::types::transaction::PreflightResult, WalletError>,
    PreflightStore,
    String,
    Arc<StdMutex<VrpcFixtureState>>,
) {
    let (url, fixture_state, task) =
        start_vrpc_fixture(vrpc_fixture_state(provider_currency, destination_kind)).await;
    let (session, session_id, vrpc_address) = session();
    let store = PreflightStore::new();
    store.activate_wallet_session(&session_id);
    let coins = CoinRegistry::new();
    let channel_id = format!("vrpc.{vrpc_address}.{VRSC_SYSTEM_ID}");
    let params = PreflightParams {
        coin_id: TOKEN_ID.to_string(),
        channel_id: channel_id.clone(),
        to_address: destination.to_string(),
        amount: "0.00000050".to_string(),
        memo: None,
        fee_mode: None,
    };
    let result = route_preflight(
        &channel_id,
        params,
        &store,
        &session,
        &coins,
        &VrpcProviderPool::for_tests(&url),
        &BtcProviderPool::new(),
        &EthProviderPool::disabled_for_tests(),
    )
    .await;
    task.abort();
    (result, store, session_id, fixture_state)
}

#[tokio::test]
async fn actual_router_authenticates_the_selected_token_and_transparent_destination() {
    let destination = transparent_address(60, 2);
    let (result, store, session_id, fixture) =
        run_token_preflight(TOKEN_ID, &destination, DestinationKind::Transparent).await;
    let result = result.expect("valid authenticated token preflight");
    let record = store
        .get(&result.preflight_id, &session_id)
        .expect("stored preflight");

    assert_eq!(record.payload["intent"]["kind"], "token_payment");
    assert_eq!(record.payload["entered_to_address"], destination);
    assert_eq!(record.payload["to_address"], destination);
    assert_eq!(result.fee_currency, VRSC_SYSTEM_ID);
    assert_eq!(
        fixture
            .lock()
            .expect("fixture")
            .requested_output
            .as_ref()
            .unwrap()["currency"],
        TOKEN_ID
    );
}

#[tokio::test]
async fn actual_router_authenticates_resolved_identity_token_destination() {
    let (result, store, session_id, _) =
        run_token_preflight(TOKEN_ID, "alice@", DestinationKind::Identity).await;
    let result = result.expect("valid identity token preflight");
    let record = store
        .get(&result.preflight_id, &session_id)
        .expect("stored preflight");

    assert_eq!(record.payload["intent"]["kind"], "token_payment");
    assert_eq!(
        record.payload["intent"]["destination"]["destination_type"],
        4
    );
    assert_eq!(record.payload["entered_to_address"], "alice@");
    assert_eq!(record.payload["to_address"], transparent_address(102, 2));
    assert_eq!(result.warnings[0].warning_type, "resolved_destination");
}

#[tokio::test]
async fn actual_router_rejects_native_or_other_token_provider_substitution() {
    for substituted_currency in [VRSC_SYSTEM_ID, OTHER_TOKEN_ID] {
        let destination = transparent_address(60, 2);
        let (result, _, _, fixture) = run_token_preflight(
            substituted_currency,
            &destination,
            DestinationKind::Transparent,
        )
        .await;
        assert!(matches!(result, Err(WalletError::CurrencyMetadataMismatch)));
        assert!(
            fixture.lock().expect("fixture").requested_output.is_none(),
            "a conflicting currency must be rejected before sendcurrency"
        );
    }
}

struct EvmFixtureState {
    estimate_mode: String,
    balance_wei: u128,
    estimate_calls: usize,
    estimated_values: Vec<String>,
}

async fn start_evm_fixture(
    estimate_mode: &str,
) -> (
    String,
    Arc<StdMutex<EvmFixtureState>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind EVM fixture server");
    let url = format!("http://{}", listener.local_addr().expect("fixture address"));
    let state = Arc::new(StdMutex::new(EvmFixtureState {
        estimate_mode: estimate_mode.to_string(),
        balance_wei: 1_000_000_000_000_000_000,
        estimate_calls: 0,
        estimated_values: Vec::new(),
    }));
    let server_state = state.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.expect("accept EVM request");
            let state = server_state.clone();
            tokio::spawn(async move {
                let mut request_bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                let (body_start, content_length) = loop {
                    let read = socket.read(&mut buffer).await.expect("read EVM request");
                    if read == 0 {
                        return;
                    }
                    request_bytes.extend_from_slice(&buffer[..read]);
                    if let Some(header_end) =
                        request_bytes.windows(4).position(|w| w == b"\r\n\r\n")
                    {
                        let headers = String::from_utf8_lossy(&request_bytes[..header_end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request_bytes.len() >= header_end + 4 + content_length {
                            break (header_end + 4, content_length);
                        }
                    }
                };
                let request: Value =
                    serde_json::from_slice(&request_bytes[body_start..body_start + content_length])
                        .expect("EVM JSON-RPC request");
                let method = request["method"].as_str().expect("EVM RPC method");
                let (result, error) = {
                    let mut state = state.lock().expect("EVM fixture state");
                    match method {
                        "eth_chainId" => (Some(json!("0x1")), None),
                        "eth_getBlockByNumber" => {
                            (Some(json!({ "baseFeePerGas": "0x3b9aca00" })), None)
                        }
                        "eth_feeHistory" => (
                            Some(json!({
                                "oldestBlock": "0x1",
                                "baseFeePerGas": ["0x3b9aca00", "0x3b9aca00"],
                                "gasUsedRatio": [0.5],
                                "reward": [["0x3b9aca00"]]
                            })),
                            None,
                        ),
                        "eth_getBalance" => {
                            (Some(json!(format!("0x{:x}", state.balance_wei))), None)
                        }
                        "eth_call" => (Some(json!(format!("0x{:064x}", 100_000_000u64))), None),
                        "eth_estimateGas" => {
                            state.estimate_calls += 1;
                            if let Some(value) = request["params"][0]["value"].as_str() {
                                state.estimated_values.push(value.to_string());
                            }
                            match state.estimate_mode.as_str() {
                                "zero" => (Some(json!("0x0")), None),
                                "below" => (Some(json!("0x5207")), None),
                                "malformed" => (Some(json!("not-a-quantity")), None),
                                "revert" => (
                                    None,
                                    Some(json!({
                                        "code": 3,
                                        "message": "execution reverted"
                                    })),
                                ),
                                "reject_full_then_valid" if state.estimate_calls == 1 => (
                                    None,
                                    Some(json!({
                                        "code": 3,
                                        "message": "insufficient funds for full-balance estimate"
                                    })),
                                ),
                                "reject_full_then_reject_adjusted" => (
                                    None,
                                    Some(json!({
                                        "code": 3,
                                        "message": "estimate rejected"
                                    })),
                                ),
                                "erc20_valid" => (Some(json!("0x186a0")), None),
                                _ => (Some(json!("0x5208")), None),
                            }
                        }
                        _ => panic!("unexpected EVM method: {method}"),
                    }
                };
                let response = match error {
                    Some(error) => json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "error": error
                    }),
                    None => json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "result": result.expect("fixture result")
                    }),
                }
                .to_string();
                socket
                    .write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.len(), response
                        )
                        .as_bytes(),
                    )
                    .await
                    .expect("write EVM response");
            });
        }
    });
    (url, state, task)
}

async fn run_evm_preflight(
    estimate_mode: &str,
    channel_id: &str,
    amount: &str,
) -> (
    Result<crate::types::transaction::PreflightResult, WalletError>,
    PreflightStore,
    String,
    Arc<StdMutex<EvmFixtureState>>,
) {
    let (url, fixture_state, task) = start_evm_fixture(estimate_mode).await;
    let (session, session_id, _) = session();
    let store = PreflightStore::new();
    store.activate_wallet_session(&session_id);
    let coin_id = channel_id.split('.').nth(1).expect("coin id");
    let params = PreflightParams {
        coin_id: coin_id.to_string(),
        channel_id: channel_id.to_string(),
        to_address: format!("0x{}", "22".repeat(20)),
        amount: amount.to_string(),
        memo: None,
        fee_mode: None,
    };
    let result = route_preflight(
        channel_id,
        params,
        &store,
        &session,
        &CoinRegistry::new(),
        &VrpcProviderPool::for_tests(&url),
        &BtcProviderPool::new(),
        &EthProviderPool::for_tests(WalletNetwork::Mainnet, &url),
    )
    .await;
    task.abort();
    (result, store, session_id, fixture_state)
}

#[tokio::test]
async fn evm_routes_reject_zero_below_intrinsic_malformed_and_reverted_estimates() {
    for channel_id in ["eth.ETH", "erc20.USDC"] {
        for estimate_mode in ["zero", "below", "malformed", "revert"] {
            let (result, _, _, _) = run_evm_preflight(estimate_mode, channel_id, "0.1").await;
            assert!(
                matches!(result, Err(WalletError::GasEstimationFailed)),
                "{channel_id} should reject {estimate_mode} gas estimates: {result:?}"
            );
        }
    }
}

#[tokio::test]
async fn eth_max_recovers_from_full_balance_estimate_failure_at_the_exact_adjusted_value() {
    let (result, store, session_id, fixture) =
        run_evm_preflight("reject_full_then_valid", "eth.ETH", "1").await;
    let result = result.expect("adjusted ETH Max preflight");
    assert!(result.fee_taken_from_amount);
    let record = store
        .get(&result.preflight_id, &session_id)
        .expect("stored ETH Max preflight");
    let final_value = record.payload["value_wei"]
        .as_str()
        .expect("stored final value")
        .parse::<u128>()
        .expect("numeric final value");
    let estimated_values = &fixture.lock().expect("EVM fixture").estimated_values;
    assert_eq!(estimated_values.len(), 2);
    assert_eq!(estimated_values[0], "0xde0b6b3a7640000");
    assert_eq!(estimated_values[1], format!("0x{final_value:x}"));
}

#[tokio::test]
async fn eth_max_returns_no_preflight_when_the_adjusted_estimate_also_fails() {
    let (result, _, _, fixture) =
        run_evm_preflight("reject_full_then_reject_adjusted", "eth.ETH", "1").await;
    assert!(matches!(result, Err(WalletError::GasEstimationFailed)));
    let estimated_values = &fixture.lock().expect("EVM fixture").estimated_values;
    assert_eq!(estimated_values.len(), 2);
    assert_ne!(estimated_values[0], estimated_values[1]);
}

#[tokio::test]
async fn eth_near_balance_success_is_reestimated_at_the_exact_final_value() {
    let (result, store, session_id, fixture) =
        run_evm_preflight("valid", "eth.ETH", "0.99999").await;
    let result = result.expect("adjusted ETH preflight");
    assert!(result.fee_taken_from_amount);
    let record = store
        .get(&result.preflight_id, &session_id)
        .expect("stored ETH preflight");
    let final_value = record.payload["value_wei"]
        .as_str()
        .expect("stored final value")
        .parse::<u128>()
        .expect("numeric final value");
    let estimated_values = &fixture.lock().expect("EVM fixture").estimated_values;
    assert_eq!(estimated_values.len(), 2);
    assert_eq!(estimated_values[1], format!("0x{final_value:x}"));
}

#[tokio::test]
async fn erc20_route_accepts_a_realistic_successful_gas_estimate() {
    let (result, store, session_id, fixture) =
        run_evm_preflight("erc20_valid", "erc20.USDC", "1.25").await;
    let result = result.expect("successful ERC20 preflight");
    assert_eq!(result.value, "1.250000");
    assert!(!result.fee_taken_from_amount);
    let record = store
        .get(&result.preflight_id, &session_id)
        .expect("stored ERC20 preflight");
    assert_eq!(record.payload["token_value_raw"], "1250000");
    assert_eq!(record.payload["gas_limit"], "133333");
    assert!(result.fee.parse::<f64>().expect("numeric ERC20 fee") > 0.0);
    let fixture = fixture.lock().expect("EVM fixture");
    assert_eq!(fixture.estimate_calls, 1);
    assert!(fixture.estimated_values.is_empty());
}

#[tokio::test]
async fn eth_overbalance_is_rejected_before_gas_estimation() {
    let (result, _, _, fixture) =
        run_evm_preflight("valid", "eth.ETH", "1.000000000000000001").await;
    assert!(matches!(result, Err(WalletError::InsufficientFunds)));
    let fixture = fixture.lock().expect("EVM fixture");
    assert_eq!(fixture.estimate_calls, 0);
    assert!(fixture.estimated_values.is_empty());
}
