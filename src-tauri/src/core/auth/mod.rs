//
// Authentication and session management module
// Security: Handles secure seed storage and session lifecycle with zeroization
// Last Updated: Created for Module 1 integration

pub mod guard_session;
pub mod kdf;
pub mod lifecycle;
pub mod provisioning;
pub mod session;
pub mod stronghold_store;

pub use guard_session::GuardSessionManager;
pub use lifecycle::clear_wallet_session_if_current;
pub use provisioning::{ProvisioningSignatureChallenge, ProvisioningSignatureStore};
pub(crate) use session::SessionSubmissionGuard;
pub use session::{
    capture_active_wallet_access_context, ensure_active_wallet_session,
    load_primary_private_scalar_for_context, load_primary_secret_material_for_context,
    SessionManager,
};
pub use stronghold_store::StrongholdStore;
