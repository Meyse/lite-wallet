/**
 * Frontend types mirroring backend serialized shapes (camelCase).
 * Used by stores, services, and UI. No sensitive fields.
 * Last Updated: Module 9 — balance, transaction, coin, preflight, send types.
 */

export interface BalanceResult {
  confirmed: string;
  pending: string;
  total: string;
}

export interface Transaction {
  txid: string;
  amount: string;
  fromAddress: string;
  toAddress: string;
  confirmations: number;
  timestamp?: number;
  pending: boolean;
}

export interface TransactionHistoryPageRequest {
  channelId: string;
  coinId?: string;
  cursor?: string;
  limit?: number;
}

export interface TransactionHistoryPage {
  transactions: Transaction[];
  nextCursor?: string | null;
  hasMore: boolean;
  warning?: string | null;
}

export type WalletNetwork = 'mainnet' | 'testnet';

export interface ActiveAssetsState {
  network: WalletNetwork;
  initialized: boolean;
  coinIds: string[];
}

export type Protocol = 'vrsc' | 'btc' | 'eth' | 'erc20';
export type Channel = 'vrpc' | 'btc' | 'eth' | 'erc20' | 'dlight_private';

export interface CoinDefinition {
  id: string;
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  coinPaprikaId?: string | null;
  proto: Protocol;
  compatibleChannels: Channel[];
  decimals: number;
  vrpcEndpoints: string[];
  dlightEndpoints?: string[] | null;
  electrumEndpoints?: string[] | null;
  secondsPerBlock: number;
  mappedTo?: string | null;
  isTestnet: boolean;
}

export type ScopeKind = 'transparent' | 'shielded';

export type WalletEntryKind = 'coin' | 'private_verus';

export interface WalletEntrySelection {
  walletEntryKind: WalletEntryKind;
  coinId: string;
  baseCoinId?: string;
  scopeFilterMode: ScopeKind;
  displayName?: string;
}

export interface CoinScope {
  channelId: string;
  coinId: string;
  address: string;
  addressLabel: string;
  systemId: string;
  systemTicker: string;
  systemDisplayName: string;
  isPrimaryAddress: boolean;
  isReadOnly: boolean;
  scopeKind: ScopeKind;
}

export interface CoinScopesResult {
  coinId: string;
  scopes: CoinScope[];
}

export interface PbaasCandidate {
  currencyId: string;
  systemId: string;
  displayTicker: string;
  displayName: string;
  fullyQualifiedName?: string | null;
}

export type PbaasResolveResult =
  | {
      status: 'resolved';
      coin: CoinDefinition;
    }
  | {
      status: 'ambiguous';
      candidates: PbaasCandidate[];
    };

export type Erc20ResolveResult = {
  status: 'resolved';
  coin: CoinDefinition;
};

export interface PreflightWarning {
  warningType: string;
  message: string;
}

export interface PreflightResult {
  preflightId: string;
  fee: string;
  feeCurrency: string;
  value: string;
  amountSubmitted: string;
  toAddress: string;
  fromAddress: string;
  feeTakenFromAmount: boolean;
  feeTakenMessage?: string | null;
  warnings: PreflightWarning[];
  memo?: string | null;
}

export interface PreflightParams {
  coinId: string;
  channelId: string;
  toAddress: string;
  amount: string;
  memo?: string | null;
}

export interface SendRequest {
  preflightId: string;
}

export interface SendResult {
  txid: string;
  fee: string;
  value: string;
  toAddress: string;
  fromAddress: string;
}

export type IdentityOperation = 'update' | 'revoke' | 'recover';

export interface IdentityPatch {
  primaryAddresses?: string[] | null;
  recoveryAuthority?: string | null;
  revocationAuthority?: string | null;
  privateAddress?: string | null;
}

export interface HighRiskChange {
  changeType: string;
  beforeValue?: string | null;
  afterValue?: string | null;
}

export interface IdentityWarning {
  warningType: string;
  message: string;
}

export interface IdentityPreflightParams {
  coinId: string;
  channelId: string;
  operation: IdentityOperation;
  targetIdentity: string;
  patch?: IdentityPatch | null;
  memo?: string | null;
}

export interface IdentityPreflightResult {
  preflightId: string;
  operation: IdentityOperation;
  targetIdentity: string;
  fromAddress: string;
  fee: string;
  feeCurrency: string;
  highRiskChanges: HighRiskChange[];
  warnings: IdentityWarning[];
  memo?: string | null;
}

export interface IdentitySendRequest {
  preflightId: string;
}

export interface IdentitySendResult {
  txid: string;
  operation: IdentityOperation;
  targetIdentity: string;
  fee: string;
  fromAddress: string;
}

export interface LinkableIdentity {
  identityAddress: string;
  name?: string | null;
  fullyQualifiedName?: string | null;
  status?: string | null;
  linked: boolean;
}

export interface LinkedIdentity {
  identityAddress: string;
  name?: string | null;
  fullyQualifiedName?: string | null;
  status?: string | null;
  systemId?: string | null;
  favorite: boolean;
}

export interface LinkIdentityRequest {
  identityAddress: string;
}

export interface UnlinkIdentityRequest {
  identityAddress: string;
}

export interface SetLinkedIdentityFavoriteRequest {
  identityAddress: string;
  favorite: boolean;
}

export interface IdentityDetailWarning {
  warningType: string;
  message: string;
}

export interface IdentityDetails {
  identityAddress: string;
  name?: string | null;
  fullyQualifiedName?: string | null;
  status?: string | null;
  system?: string | null;
  parent?: string | null;
  revocationAuthority?: string | null;
  recoveryAuthority?: string | null;
  primaryAddresses: string[];
  privateAddress?: string | null;
  ownedByPrimaryAddress: boolean;
  warnings: IdentityDetailWarning[];
}

export interface GenericRequestVerificationResult {
  valid: boolean;
  signerSystemId: string;
  signerIdentityId: string;
  signatureBlockHeight: number;
}

export interface GenericResponseSignerInput {
  systemId: string;
  identityId: string;
}

export interface GenericAuthenticationResponseInput {
  requestId?: string | null;
}

export interface GenericIdentityUpdateResponseInput {
  requestId?: string | null;
  txid: string;
}

export interface BuildAndSignGenericResponseRequest {
  requestHex: string;
  signer: GenericResponseSignerInput;
  authentication?: GenericAuthenticationResponseInput | null;
  identityUpdate?: GenericIdentityUpdateResponseInput | null;
}

export interface BuildAndSignGenericResponseResult {
  signedResponseHex: string;
}

export interface GenericIdentityUpdateRequestMeta {
  requestId?: string | null;
  signerSystemId?: string | null;
  signerIdentityId?: string | null;
  expiryHeight?: number | null;
}

export interface GenericIdentityPrimaryAddressEntry {
  address: string;
  inWallet: boolean;
}

export interface GenericIdentityPrimaryAddressInfo {
  addresses: GenericIdentityPrimaryAddressEntry[];
  walletCount: number;
  externalCount: number;
}

export interface GenericIdentityAuthorities {
  revocation?: string | null;
  recovery?: string | null;
}

export interface GenericIdentityUpdatePreflightResult {
  preflightId: string;
  targetIdentity: string;
  fromAddress: string;
  fee: string;
  feeCurrency: string;
  warnings: IdentityWarning[];
  highRiskChanges: HighRiskChange[];
  currentIdentity: Record<string, unknown>;
  requestedIdentity: Record<string, unknown>;
  fullyQualifiedName?: string | null;
  friendlyNames: Record<string, string>;
  signerCmmKeyLabels: Record<string, string>;
  primaryAddressAfterUpdateInfo: GenericIdentityPrimaryAddressInfo;
  currentAuthorities: GenericIdentityAuthorities;
}

export interface ProvisioningJobRecord {
  jobId: string;
  requestType: string;
  requestHex: string;
  requestedIdentityAddress?: string | null;
  requestedFqn: string;
  signingId: string;
  hasResponseUris: boolean;
  infoUri?: string | null;
  status: string;
  createdAt: number;
  error?: string | null;
}

export interface StoreGenericProvisioningJobRequest {
  requestHex: string;
  requestedIdentityAddress?: string | null;
  requestedFqn: string;
  signingId: string;
  hasResponseUris: boolean;
  infoUri?: string | null;
  error?: string | null;
  status?: string | null;
}

export interface LinkReadyProvisioningJobResult {
  job: ProvisioningJobRecord;
  linkedIdentities: LinkedIdentity[];
}

export interface VrpcTransferPreflightParams {
  coinId: string;
  channelId: string;
  sourceAddress?: string | null;
  destination: string;
  amount: string;
  convertTo?: string | null;
  exportTo?: string | null;
  via?: string | null;
  feeCurrency?: string | null;
  feeSatoshis?: string | null;
  preconvert?: boolean | null;
  mapTo?: string | null;
  vdxfTag?: string | null;
  memo?: string | null;
}

export interface VrpcTransferPreflightResult {
  preflightId: string;
  fee: string;
  feeCurrency: string;
  value: string;
  amountSubmitted: string;
  amountAdjusted?: string | null;
  toAddress: string;
  fromAddress: string;
  warnings: PreflightWarning[];
  memo?: string | null;
}

export interface BridgeConversionPathRequest {
  coinId: string;
  channelId: string;
  sourceCurrency: string;
  destinationCurrency?: string | null;
}

export interface BridgeCapabilitiesRequest {
  coinId: string;
  channelId: string;
}

export interface BridgeCapabilitiesResult {
  conversionSupported: boolean;
  executionEngine: string;
  reasonCode?: string | null;
}

export interface BridgeConversionPathQuote {
  destinationId: string;
  destinationDisplayName?: string | null;
  destinationDisplayTicker?: string | null;
  convertTo?: string | null;
  convertToDisplayName?: string | null;
  exportTo?: string | null;
  exportToDisplayName?: string | null;
  via?: string | null;
  viaDisplayName?: string | null;
  mapTo?: string | null;
  price?: string | null;
  viaPriceInRoot?: string | null;
  destPriceInVia?: string | null;
  gateway: boolean;
  mapping: boolean;
  bounceback: boolean;
  ethDestination: boolean;
  prelaunch?: boolean;
}

export interface BridgeConversionPathsResult {
  sourceCurrency: string;
  paths: Record<string, BridgeConversionPathQuote[]>;
}

export interface BridgeConversionEstimateRequest {
  coinId: string;
  channelId: string;
  sourceCurrency: string;
  convertTo: string;
  amount: string;
  via?: string | null;
  preconvert?: boolean | null;
}

export interface BridgeConversionEstimateResult {
  estimatedCurrencyOut?: string | null;
  price?: string | null;
}

export interface BridgeExportFeeEstimateRequest {
  coinId: string;
  channelId: string;
}

export interface BridgeExportFeeEstimateResult {
  feeCoins: string;
  feeSats: string;
  balanceCoins: string;
  systemId: string;
  sourceAddress: string;
  currencyTicker: string;
}

export interface BridgeTransferPreflightParams {
  coinId: string;
  channelId: string;
  sourceAddress?: string | null;
  destination: string;
  amount: string;
  convertTo?: string | null;
  exportTo?: string | null;
  via?: string | null;
  feeCurrency?: string | null;
  feeSatoshis?: string | null;
  preconvert?: boolean | null;
  mapTo?: string | null;
  vdxfTag?: string | null;
  memo?: string | null;
}

export interface BridgeTransferRoute {
  convertTo?: string | null;
  exportTo?: string | null;
  via?: string | null;
  mapTo?: string | null;
}

export interface BridgeExecutionHint {
  engine: string;
  requiresTokenApproval: boolean;
  bridgeContract?: string | null;
}

export interface BridgeTransferPreflightResult {
  preflightId: string;
  fee: string;
  feeCurrency: string;
  value: string;
  amountSubmitted: string;
  amountAdjusted?: string | null;
  toAddress: string;
  fromAddress: string;
  warnings: PreflightWarning[];
  memo?: string | null;
  route: BridgeTransferRoute;
  execution: BridgeExecutionHint;
}

export interface BeginGuardSessionRequest {
  importText: string;
  importMode: GuardImportMode;
  network: WalletNetwork;
}

export interface BeginGuardSessionResult {
  guardSessionId: string;
  secretKind: 'seed_text' | 'wif' | 'private_key_hex' | string;
  vrscAddress: string;
  ethAddress: string;
  btcAddress: string;
  network: WalletNetwork;
}

export interface EndGuardSessionRequest {
  guardSessionId: string;
}

export interface EndGuardSessionResult {
  ended: boolean;
}

export interface GuardIdentityPreflightRequest {
  guardSessionId: string;
  params: IdentityPreflightParams;
}

export interface GuardIdentityLookupRequest {
  guardSessionId: string;
  targetIdentity: string;
}

export interface GuardIdentityLookupResult {
  exists: boolean;
}

export interface GuardIdentitySendRequest {
  guardSessionId: string;
  preflightId: string;
}

export type GuardImportMode = 'mnemonic24' | 'textAuto';
export type GuardFlowMode = 'revoke' | 'recover';
export type GuardFlowStep = 'secret' | 'target' | 'patch' | 'review' | 'result';

export interface GuardRecoverDraft {
  primaryAddress: string;
  recoveryAuthority: string;
  revocationAuthority: string;
  privateAddress: string;
}

export type GuardFlowErrorCode =
  | 'InvalidImportText'
  | 'GuardSessionNotFound'
  | 'IdentityNotFound'
  | 'IdentityInvalidState'
  | 'IdentityUnsupportedAuthority'
  | 'InvalidPreflight'
  | 'InsufficientFunds'
  | 'NetworkError'
  | 'OperationFailed'
  | 'IdentityBuildFailed'
  | 'IdentitySignFailed'
  | 'Unknown';

export type GuardPreflightResult = IdentityPreflightResult;
export type GuardSendResult = IdentitySendResult;

/** Placeholder for wallet://info-updated (Module 7). */
export interface ChainInfo {
  channel?: string;
  percent?: number;
  blocks?: number;
  longestChain?: number;
  syncing?: boolean;
  statusKind?: string;
  lastUpdated?: number;
  lastProgressAt?: number;
  stalled?: boolean;
  scanRateBlocksPerSec?: number;
}

export interface DlightSeedStatusResult {
  configured: boolean;
  shieldedAddress?: string | null;
}

export type RecoverySecretKind = 'seed_text' | 'wif' | 'private_key_hex';
export type DlightRecoverySecretKind = 'mnemonic' | 'spending_key';

export interface WalletRecoverySecretsResult {
  primarySecretKind: RecoverySecretKind;
  primarySecret: string;
  verusWif: string;
  btcWif: string;
  ethPrivateKey: string;
  verusAddress: string;
  btcAddress: string;
  ethAddress: string;
  dlightSecret?: string | null;
  dlightSecretKind?: DlightRecoverySecretKind | null;
  dlightShieldedAddress?: string | null;
  dlightDerivedSpendingKey?: string | null;
}

export interface DlightRuntimeStatusResult {
  channelId: string;
  runtimeKey: string;
  statusKind: string;
  percent?: number | null;
  scannedHeight: number;
  tipHeight?: number | null;
  estimatedTipHeight?: number | null;
  syncing: boolean;
  lastUpdated: number;
  lastProgressAt?: number | null;
  lastTipProbeAt?: number | null;
  consecutiveFailures: number;
  scanRateBlocksPerSec?: number | null;
  stalled: boolean;
  lastError?: string | null;
  spendCacheReady?: boolean | null;
  spendCacheStatusKind?: string | null;
  spendCachePercent?: number | null;
  spendCacheLagBlocks?: number | null;
  spendCacheLastError?: string | null;
  spendCacheScannedHeight?: number | null;
  spendCacheTipHeight?: number | null;
  spendCacheLastUpdated?: number | null;
  spendCacheNoteCount?: number | null;
}

export interface DlightProverFileStatusResult {
  path: string;
  exists: boolean;
  sizeBytes?: number | null;
  minSizeBytes: number;
  checksumAlgorithm: string;
  expectedChecksum: string;
  actualChecksum?: string | null;
  checksumMatches: boolean;
  placeholderDetected: boolean;
  errors: string[];
}

export interface DlightProverStatusResult {
  ready: boolean;
  paramsDir?: string | null;
  spend: DlightProverFileStatusResult;
  output: DlightProverFileStatusResult;
  errors: string[];
}

export type TxSendProgressStage =
  | 'syncing_spend_state'
  | 'loading_prover'
  | 'building_proof'
  | 'broadcasting';

export interface TxSendProgressEventPayload {
  channel?: string;
  coinId?: string;
  stage?: TxSendProgressStage;
}

export type DlightSeedSetupMode = 'reuse_primary' | 'create_new' | 'import_text';

export interface SetupDlightSeedRequest {
  mode: DlightSeedSetupMode;
  importText?: string | null;
}

export interface SetupDlightSeedResult {
  configured: boolean;
  generatedSeedPhrase?: string | null;
  requiresRelogin: boolean;
}
