//
// Type definition re-exports
// Last Updated: Added transaction module and Module 8 preflight/send types

pub mod address_book;
pub mod bridge;
pub mod errors;
pub mod generic_request;
pub mod guard;
pub mod identity;
pub mod transaction;
pub mod vrpc_transfer;
pub mod wallet;
pub mod watchlist;

pub use address_book::{
    AddressBookContact, AddressBookEndpoint, AddressBookSnapshot, AddressEndpointKind,
    SaveAddressBookContactRequest, SaveAddressBookEndpointInput, ValidateDestinationAddressRequest,
    ValidateDestinationAddressResult,
};
pub use bridge::{
    BridgeCapabilitiesRequest, BridgeCapabilitiesResult, BridgeConversionEstimateRequest,
    BridgeConversionEstimateResult, BridgeConversionPathQuote, BridgeConversionPathRequest,
    BridgeConversionPathsResult, BridgeExecutionHint, BridgeExportFeeEstimateRequest,
    BridgeExportFeeEstimateResult, BridgeTransferPreflightParams, BridgeTransferPreflightResult,
    BridgeTransferRoute,
};
pub use errors::WalletError;
pub use generic_request::{
    BuildAndSignGenericResponseRequest, BuildAndSignGenericResponseResult,
    GenericAuthenticationResponseInput, GenericIdentityAuthorities,
    GenericIdentityPrimaryAddressEntry, GenericIdentityPrimaryAddressInfo,
    GenericIdentityUpdatePreflightResult, GenericIdentityUpdateRequestMeta,
    GenericIdentityUpdateResponseInput, GenericIdentityUpdateReviewResult,
    GenericRequestVerificationResult, GenericResponseSignerInput, LinkReadyProvisioningJobResult,
    ProvisioningJobRecord,
};
pub use guard::{
    BeginGuardSessionRequest, BeginGuardSessionResult, EndGuardSessionRequest,
    EndGuardSessionResult, GuardIdentityLookupRequest, GuardIdentityLookupResult,
    GuardIdentityPreflightRequest, GuardIdentitySendRequest, GuardImportMode, GuardPreflightResult,
    GuardSendResult,
};
pub use identity::{
    HighRiskChange, IdentityDetailWarning, IdentityDetails, IdentityOperation, IdentityPatch,
    IdentityPreflightParams, IdentityPreflightResult, IdentityProfileAvatar,
    IdentityProfileAvatarChange, IdentityProfileDescriptionChange, IdentityProfileField,
    IdentityProfileIssue, IdentityProfileLoadResult, IdentityProfilePreflightRequest,
    IdentityProfilePreflightResult, IdentityProfileSnapshot, IdentityProfileSource,
    IdentityProfileState, IdentitySendRequest, IdentitySendResult, IdentityWarning,
    LinkIdentityRequest, LinkableIdentity, LinkedIdentity, PendingIdentityProfileUpdate,
    SetLinkedIdentityFavoriteRequest, UnlinkIdentityRequest,
};
pub use transaction::{
    BalanceResult, DirectSendFeeMode, PreflightParams, PreflightResult, PreflightWarning,
    SendRequest, SendResult, Transaction, TransactionHistoryPage, TransactionHistoryPageRequest,
};
pub use vrpc_transfer::{VrpcTransferPreflightParams, VrpcTransferPreflightResult};
pub use wallet::{
    AccountRecord, ActiveAssetsState, ActiveWalletResponse, AddressResponse, AssetPreferencesState,
    CoinScope, CoinScopesResult, CreateWalletRequest, CreateWalletResult,
    DlightProverFileStatusResult, DlightProverStatusResult, DlightRecoverySecretKind,
    DlightRuntimeStatusResult, DlightSeedStatusResult, GenerateMnemonicRequest,
    ImportWalletTextRequest, MnemonicResult, RecoverySecretKind, ScopeKind, SetupDlightSeedRequest,
    SetupDlightSeedResult, WalletListItem, WalletMetadata, WalletRecoverySecretsResult,
    WalletSecretKind,
};
pub use watchlist::{
    ResolveWatchlistTargetRequest, WatchlistEntry, WatchlistEntrySnapshot, WatchlistHolding,
    WatchlistRefreshResult, WatchlistResolvedTarget, WatchlistSource, WatchlistTargetKind,
};
