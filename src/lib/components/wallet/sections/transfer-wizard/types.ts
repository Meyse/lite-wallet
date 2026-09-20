import type { ScopeKind, WalletNetwork } from '$lib/types/wallet';

export type TransferStepId = 'details' | 'recipient' | 'review' | 'success';

export type WizardOperationalStepId = Exclude<TransferStepId, 'success'>;

export type StepStatus = 'complete' | 'current' | 'upcoming';

export type TransferStepperStep = {
  id: WizardOperationalStepId;
  label: string;
  status: StepStatus;
};

export type DestinationAddressKind = 'vrpc' | 'btc' | 'eth' | 'dlight';

export type TransferEntryContext = {
  coinId: string;
  channelId: string;
  readOnly: boolean;
  scopeKind: ScopeKind;
};

export type TransferRecipientIntent = {
  identityAddress: string;
  fullyQualifiedName: string;
  network: WalletNetwork;
  chainId: string;
};

export type TransferNavigationState = {
  mode: 'send' | 'convert';
  dirty: boolean;
  locked: boolean;
  completed: boolean;
};
