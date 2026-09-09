//
// Module 7: Update engine — interval polling for balances and transactions, Tauri event emission.
// No Tauri types in public API except AppHandle when starting; engine is constructed from setup.

mod engine;
mod events;
mod params;

pub use engine::{
    UpdateEngine, UpdateEngineStartConfig, EVENT_BALANCES_UPDATED, EVENT_ERROR, EVENT_INFO_UPDATED,
    EVENT_RATES_UPDATED, EVENT_TRANSACTIONS_UPDATED, EVENT_TX_SEND_PROGRESS,
};
pub use events::{TxSendProgressPayload, UpdateErrorPayload};
pub(crate) use params::RATES_REFRESH_SECS;
pub use params::{jitter_duration, BALANCE_EXPIRE_SECS, BALANCE_REFRESH_SECS};
