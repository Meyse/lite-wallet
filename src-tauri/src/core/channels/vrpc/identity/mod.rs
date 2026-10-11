//
// Identity transaction flow for VRPC channels.

pub(crate) mod preflight;
pub(crate) mod profile;
mod send;
pub(crate) mod validate;
pub(crate) mod verus_tx;

pub use preflight::preflight;
pub(crate) use send::sign_payload as sign_identity_payload;
pub use send::{send, send_with_signing_material};
