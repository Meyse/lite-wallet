//
// Fiat rates domain: CoinPaprika direct prices, ECB fiat crosses, and PBaaS fallback derivation.
// Mirrors valu-mobile GENERAL channel behavior (excluding Wyre).

use reqwest::Client;

pub mod cache;
pub mod coinpaprika;
pub mod ecb;
pub mod pbaas;

/// Shared HTTP client for public fiat rate sources.
pub fn build_rates_http_client() -> Client {
    Client::builder()
        .use_rustls_tls()
        .no_proxy()
        .http1_only()
        .connect_timeout(std::time::Duration::from_secs(4))
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .unwrap_or_else(|_| Client::new())
}
