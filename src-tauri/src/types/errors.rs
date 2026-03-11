//
// Error type definitions for wallet operations
// Security: Never expose internal implementation details to frontend
// Last Updated: Added InvalidPreflight and UnsupportedChannel for Module 8

use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum WalletError {
    #[error("Invalid wallet address")]
    InvalidAddress,

    #[error("Invalid seed phrase")]
    InvalidSeedPhrase,

    #[error("Invalid import text")]
    InvalidImportText,

    #[error("Invalid wallet name")]
    InvalidWalletName,

    #[error("Wallet already exists")]
    WalletExists,

    #[error("Wallet is locked")]
    WalletLocked,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Password must be at least 7 characters")]
    PasswordTooShort,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Insufficient ETH to cover transaction gas")]
    InsufficientEthForGas,

    #[error("Network error")]
    NetworkError,

    #[error("Operation failed")]
    OperationFailed,

    #[error("Secure storage is unavailable")]
    SecureStorageUnavailable,

    #[error("Invalid coin definition")]
    InvalidCoinDefinition,

    #[error("PBaaS currency already exists")]
    DuplicatePbaasCurrency,

    #[error("Asset already exists")]
    AssetAlreadyExists,

    #[error("PBaaS currency not found")]
    PbaasNotFound,

    #[error("PBaaS query is ambiguous")]
    PbaasAmbiguous,

    #[error("Invalid or expired preflight")]
    InvalidPreflight,

    #[error("Unsupported channel")]
    UnsupportedChannel,

    #[error("Unsupported network for this operation")]
    UnsupportedNetwork,

    #[error("dlight synchronizer not ready")]
    DlightSynchronizerNotReady,

    #[error("dlight prover parameters are unavailable")]
    DlightProverUnavailable,

    #[error("dlight spend cache is not ready")]
    DlightSpendCacheNotReady,

    #[error("{0}")]
    DlightBroadcastRejected(String),

    #[error("{0}")]
    DlightSendFailed(String),

    #[error("Ethereum channels are not configured")]
    EthNotConfigured,

    #[error("Invalid ERC20 contract")]
    InvalidContract,

    #[error("Bridge transfer is not implemented for this channel")]
    BridgeNotImplemented,

    #[error("Bridge route is invalid for this transfer")]
    BridgeRouteInvalid,

    #[error("Bridge destination and route combination is unsupported")]
    BridgeUnsupportedDestinationCombination,

    #[error("Bridge transfer approval failed")]
    BridgeApprovalFailed,

    #[error("Insufficient ETH to cover bridge fee envelope")]
    BridgeInsufficientEthFeeEnvelope,

    #[error("Bridge gas drift exceeded preflight fee envelope")]
    BridgeGasDriftExceeded,

    #[error("Identity not found")]
    IdentityNotFound,

    #[error("This VerusID is not controlled by this wallet address.")]
    IdentityOwnershipMismatch,

    #[error("You can only favorite up to 2 identities.")]
    IdentityFavoriteLimitReached,

    #[error("Identity has an invalid state for this operation")]
    IdentityInvalidState,

    #[error("Identity authority is unsupported")]
    IdentityUnsupportedAuthority,

    #[error("Identity request expired")]
    IdentityRequestExpired,

    #[error("Active currency status cannot be changed")]
    IdentityUnsupportedActiveCurrencyChange,

    #[error("Tokenized control status cannot be changed")]
    IdentityUnsupportedTokenizedControlChange,

    #[error("Identity RPC method unsupported by endpoint")]
    IdentityRpcUnsupported,

    #[error("Failed to build identity transaction")]
    IdentityBuildFailed,

    #[error("Failed to sign identity transaction")]
    IdentitySignFailed,

    #[error("Generic request envelope is invalid")]
    GenericRequestInvalidEnvelope,

    #[error("Generic request signature is unsupported")]
    GenericRequestUnsupportedSignature,

    #[error("Guard session not found or expired")]
    GuardSessionNotFound,

    #[error("Address book contains a duplicate destination")]
    AddressBookDuplicate,

    #[error("Address book contact was not found")]
    AddressBookContactNotFound,

    #[error("Address book input is invalid")]
    AddressBookInvalidInput,

    // Internal errors are mapped to generic ones above
    #[serde(skip)]
    #[error("Internal error: {0}")]
    Internal(String),
}

// Convert internal errors to user-safe errors
impl From<std::io::Error> for WalletError {
    fn from(_: std::io::Error) -> Self {
        WalletError::OperationFailed
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(_: serde_json::Error) -> Self {
        WalletError::OperationFailed
    }
}
