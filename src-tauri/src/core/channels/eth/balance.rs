use ethers::abi::Abi;
use ethers::contract::Contract;
use ethers::providers::Middleware;
use ethers::types::Address;
use ethers::utils::format_units;
use std::sync::Arc;
use std::time::Duration;

use crate::core::channels::eth::provider::EthNetworkProvider;
use crate::core::coins::CoinDefinition;
use crate::types::transaction::BalanceResult;
use crate::types::WalletError;

// Bound the whole read, including chain validation, so stalled RPC calls release
// their resources and cannot hold balance refreshes open indefinitely.
const BALANCE_TIMEOUT: Duration = Duration::from_secs(15);

const ERC20_BALANCE_ABI: &str = r#"[
  {
    "constant": true,
    "inputs": [{"name": "_owner", "type": "address"}],
    "name": "balanceOf",
    "outputs": [{"name": "balance", "type": "uint256"}],
    "type": "function"
  }
]"#;

pub async fn get_eth_balance(
    provider: &EthNetworkProvider,
    address: &str,
) -> Result<BalanceResult, WalletError> {
    tokio::time::timeout(BALANCE_TIMEOUT, async {
        provider.validate_rpc_identity().await?;
        let parsed_address: Address = address.parse().map_err(|_| WalletError::InvalidAddress)?;

        let balance_wei = provider
            .rpc_provider
            .get_balance(parsed_address, None)
            .await
            .map_err(|_| WalletError::NetworkError)?;

        let balance = format_units(balance_wei, 18).map_err(|_| WalletError::OperationFailed)?;

        Ok(BalanceResult {
            confirmed: balance.clone(),
            pending: "0".to_string(),
            total: balance,
        })
    })
    .await
    .map_err(|_| WalletError::NetworkError)?
}

pub async fn get_erc20_balance(
    provider: &EthNetworkProvider,
    from_address: &str,
    coin: &CoinDefinition,
) -> Result<BalanceResult, WalletError> {
    tokio::time::timeout(BALANCE_TIMEOUT, async {
        provider.validate_rpc_identity().await?;
        let parsed_from: Address = from_address
            .parse()
            .map_err(|_| WalletError::InvalidAddress)?;
        let token_address: Address = coin
            .currency_id
            .parse()
            .map_err(|_| WalletError::InvalidAddress)?;

        let abi: Abi =
            serde_json::from_str(ERC20_BALANCE_ABI).map_err(|_| WalletError::OperationFailed)?;
        let rpc = Arc::new(provider.rpc_provider.clone());
        let contract = Contract::new(token_address, abi, rpc);

        let balance_raw = contract
            .method::<_, ethers::types::U256>("balanceOf", parsed_from)
            .map_err(|_| WalletError::OperationFailed)?
            .call()
            .await
            .map_err(|_| WalletError::NetworkError)?;

        let balance = format_units(balance_raw, coin.decimals as usize)
            .map_err(|_| WalletError::OperationFailed)?;

        Ok(BalanceResult {
            confirmed: balance.clone(),
            pending: "0".to_string(),
            total: balance,
        })
    })
    .await
    .map_err(|_| WalletError::NetworkError)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::channels::eth::EthProviderPool;
    use crate::core::coins::CoinRegistry;
    use crate::types::wallet::WalletNetwork;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn assert_stalled_read_times_out(stalled_method: &'static str, erc20: bool) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fixture listener");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let payload = loop {
                    let mut buffer = [0; 4096];
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                    let text = String::from_utf8_lossy(&request);
                    if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap();
                        if body.len() >= length {
                            break serde_json::from_str::<serde_json::Value>(&body[..length])
                                .unwrap();
                        }
                    }
                };
                if payload["method"] == stalled_method {
                    started_tx.send(()).unwrap();
                    // Keep the socket open without responding until the client times out.
                    std::future::pending::<()>().await;
                    drop(socket);
                    break;
                }
                assert_eq!(payload["method"], "eth_chainId");
                let body = serde_json::json!({"jsonrpc":"2.0", "id":payload["id"], "result":"0x1"})
                    .to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(), body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let pool = EthProviderPool::for_tests(WalletNetwork::Mainnet, &url);
        let provider = pool.for_network(WalletNetwork::Mainnet).unwrap().clone();
        let read = tokio::spawn(async move {
            let address = "0x0000000000000000000000000000000000000000";
            if erc20 {
                let coin = CoinRegistry::new().find_by_id("USDC", false).unwrap();
                get_erc20_balance(&provider, address, &coin).await
            } else {
                get_eth_balance(&provider, address).await
            }
        });
        tokio::time::timeout(Duration::from_secs(5), started_rx)
            .await
            .expect("RPC request started")
            .unwrap();
        tokio::time::pause();
        tokio::time::advance(BALANCE_TIMEOUT).await;
        assert!(matches!(
            read.await.unwrap(),
            Err(WalletError::NetworkError)
        ));
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn bounds_stalled_chain_validation() {
        assert_stalled_read_times_out("eth_chainId", true).await;
    }

    #[tokio::test]
    async fn bounds_stalled_erc20_balance() {
        assert_stalled_read_times_out("eth_call", true).await;
    }

    #[tokio::test]
    async fn bounds_stalled_eth_balance() {
        assert_stalled_read_times_out("eth_getBalance", false).await;
    }
}
