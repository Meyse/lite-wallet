import type { Snippet } from 'svelte';
import type {
  ApprovalStageId,
  ApprovalStageStatus,
  RequestImpact,
  RequestStepId,
} from '$lib/genericRequest/flowPlan';
import type { ContactIdentity } from '$lib/types/addressBook';

/** Label and value rows, one fact per row. A row's value may be text or a snippet. */
export type RequestFact = { label: string; value?: string; content?: Snippet };

/** The requester as the header and rail show it. */
export type RequesterView = {
  /** The response address host, or the requester name when there is none. */
  appLabel: string;
  /** The signing VerusID with its profile preview, when the wallet can resolve it. */
  identity: ContactIdentity | null;
  /** The signing VerusID as text, for when no profile is available. */
  identityLabel: string;
  signed: boolean;
};

/** One line on Confirm: what a detail does, with its impact. */
export type PlanLine = {
  id: string;
  icon: RequestStepId;
  text: string;
  detail?: string;
  impact: RequestImpact;
};

/** One step in the rail. */
export type RailStep = {
  id: RequestStepId | 'done';
  label: string;
  status: 'done' | 'current' | 'upcoming';
};

/** One stage after Approval, as Done shows it. */
export type StageView = {
  id: ApprovalStageId;
  status: ApprovalStageStatus;
  label: string;
  error?: string;
};

/** An address that can pay the network fee: never a watched address. */
export type FundingSource = {
  channelId: string;
  address: string;
  addressLabel: string;
  systemId: string;
  balanceValue: number;
  balanceDisplay: string;
  isPrimaryAddress: boolean;
};
