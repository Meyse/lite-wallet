//
// Core business logic modules
// Last Updated: Module 7 update engine; polling and Tauri events for balances/transactions

pub mod address_book;
pub mod auth;
pub mod channels;
pub mod coins;
pub mod crypto;
pub mod identity_display;
pub mod rates;
pub mod runtime_config;
pub mod updates;
pub mod wallet;

pub use auth::{GuardSessionManager, ProvisioningSignatureStore, SessionManager, StrongholdStore};
pub use channels::{route_preflight, route_send, PreflightStore, WalletChannel};
pub use coins::{Channel, CoinDefinition, CoinRegistry, Protocol};
pub use updates::UpdateEngine;
pub use wallet::{AccountStateStore, WalletManager};
