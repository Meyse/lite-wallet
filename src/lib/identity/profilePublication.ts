import { writable } from 'svelte/store';
import type {
  IdentityProfileLoadResult,
  IdentityProfilePreflightRequest,
  IdentityProfilePreflightResult,
  PendingIdentityProfileUpdate,
  ProfilePublicationState,
} from '$lib/types/wallet';
import { pendingProfileMatches } from '$lib/utils/identityProfileUpdate';

export interface PublicationPresentation {
  plan: ProfilePublicationState | null;
  receipt: PendingIdentityProfileUpdate | null;
  submitting: boolean;
  uncertain: boolean;
  checking: boolean;
  error: boolean;
  readFailures: number;
  failureSince: number | null;
  completed: boolean;
}
export const emptyPublication = (): PublicationPresentation => ({
  plan: null,
  receipt: null,
  submitting: false,
  uncertain: false,
  checking: false,
  error: false,
  readFailures: 0,
  failureSince: null,
  completed: false,
});

/** One owner per selected identity/session. Local editor stages never own chain state. */
export class ProfilePublicationController {
  private value = emptyPublication();
  private store = writable(this.value);
  subscribe = this.store.subscribe;
  private generation = 0;
  private alive = true;
  private queued = false;
  private running: Promise<void> | null = null;
  private settled = new Set<string>();

  constructor(
    private readonly identity: string,
    private readonly io: {
      current: () => boolean;
      read: () => Promise<ProfilePublicationState | null>;
      confirm: (txid: string) => Promise<IdentityProfileLoadResult | null>;
      submitted: (receipt: PendingIdentityProfileUpdate) => void;
      settled: (txids: string[]) => void;
      confirmed: (
        receipt: PendingIdentityProfileUpdate,
        profile: IdentityProfileLoadResult,
        final: boolean
      ) => void;
    }
  ) {}

  private active() {
    return this.alive && this.io.current();
  }
  private set(patch: Partial<PublicationPresentation>) {
    this.value = { ...this.value, ...patch };
    this.store.set(this.value);
  }
  remember(receipt: PendingIdentityProfileUpdate | null) {
    if (
      !this.active() ||
      !receipt ||
      receipt.identityAddress !== this.identity ||
      this.value.receipt ||
      this.value.completed ||
      this.settled.has(receipt.txid)
    )
      return;
    this.set({ receipt });
  }
  reviewed(review: IdentityProfilePreflightResult, request: IdentityProfilePreflightRequest) {
    if (!this.active() || this.value.submitting) return;
    this.generation++;
    this.set({
      plan: {
        planId: review.publication.planId,
        identityAddress: this.identity,
        step: review.publication.step,
        totalSteps: review.publication.totalSteps,
        status: 'ready',
        request,
        pending: null,
        settledTxids: this.value.plan?.settledTxids ?? [],
        firstReceipt: this.value.plan?.firstReceipt,
      },
      receipt: null,
      uncertain: false,
      completed: false,
      error: false,
    });
  }
  beginSubmission() {
    if (
      !this.active() ||
      this.value.submitting ||
      this.value.uncertain ||
      this.value.plan?.status !== 'ready' ||
      this.value.receipt
    )
      return false;
    // Invalidate reads begun before admission, including a poll still in flight.
    this.generation++;
    this.set({ submitting: true, error: false });
    return true;
  }
  submitted(receipt: PendingIdentityProfileUpdate) {
    if (!this.active() || receipt.identityAddress !== this.identity || !this.value.plan) return;
    this.generation++;
    this.set({
      submitting: false,
      uncertain: false,
      receipt,
      plan: { ...this.value.plan, status: 'waiting', pending: receipt },
    });
    this.io.submitted(receipt);
    void this.refresh();
  }
  submissionFailed() {
    if (!this.active()) return;
    this.generation++;
    // Transport failure is not evidence that broadcast failed. Resolve it before actions return.
    this.set({ submitting: false, uncertain: true });
    void this.refresh();
  }
  discarded() {
    if (!this.active()) return;
    this.generation++;
    this.set(emptyPublication());
  }
  private settle(txids: string[]) {
    const fresh = txids.filter((txid) => !this.settled.has(txid));
    fresh.forEach((txid) => this.settled.add(txid));
    if (fresh.length) this.io.settled(fresh);
  }
  refresh(): Promise<void> {
    if (!this.active() || this.value.completed) return Promise.resolve();
    this.queued = true;
    if (!this.running) {
      this.running = this.drain().finally(() => {
        this.running = null;
      });
    }
    return this.running;
  }
  private async drain() {
    while (this.queued && this.active() && !this.value.completed) {
      this.queued = false;
      const generation = this.generation;
      const current = () => this.active() && generation === this.generation;
      this.set({ checking: true });
      try {
        const plan = await this.io.read();
        if (!current()) continue;
        this.set({ readFailures: 0, failureSince: null });
        if (this.value.submitting) continue;
        if (
          plan &&
          (plan.identityAddress !== this.identity ||
            (this.value.plan && plan.planId !== this.value.plan.planId))
        )
          continue;
        const previousReceipt = this.value.receipt;
        const receipt = previousReceipt ?? plan?.pending ?? plan?.completedReceipt ?? null;
        // The old ready plan may still be visible after a transport failure. It
        // cannot establish that broadcast failed, so keep repeat submission closed.
        if (
          this.value.uncertain &&
          plan?.status === 'ready' &&
          plan.step === this.value.plan?.step &&
          !plan.pending
        )
          continue;
        // A post-send ready observation for the same step cannot erase a receipt.
        // Fresh stale/reorg evidence and progression to step 2 remain admissible.
        if (
          plan?.status === 'ready' &&
          receipt &&
          this.value.plan &&
          plan.step === this.value.plan.step
        )
          continue;
        if ((!plan || plan.status === 'complete') && receipt) {
          // Null/settled IDs never imply success. Recheck the exact revision and block,
          // then use production value/digest/removal matching before closing the editor.
          const profile = await this.io.confirm(receipt.txid);
          if (!current()) continue;
          if (profile && pendingProfileMatches(receipt, profile)) {
            this.set({
              plan: null,
              receipt: null,
              completed: true,
              uncertain: false,
              error: false,
            });
            this.settle([receipt.txid]);
            this.io.confirmed(receipt, profile, true);
          } else {
            this.set({ error: true });
          }
          continue;
        }
        if (!plan) {
          this.set({ error: Boolean(receipt) || this.value.uncertain });
          continue;
        }
        if (plan.status === 'complete') {
          // Preserve explicit evidence until a matching retained receipt is available.
          this.set({ plan, error: true });
          continue;
        }
        this.set({ plan, receipt: plan.pending, uncertain: false, error: false });
        if (plan.pending && plan.pending.txid !== previousReceipt?.txid)
          this.io.submitted(plan.pending);
        if (plan.status === 'ready' && plan.step === 2 && receipt) {
          const profile = await this.io.confirm(receipt.txid);
          if (!current()) continue;
          if (profile && pendingProfileMatches(receipt, profile))
            this.io.confirmed(receipt, profile, false);
        }
        // Conflict/expiry markers settle local pending caches, never signal success.
        this.settle(plan.settledTxids);
      } catch {
        if (current())
          this.set({
            error: true,
            readFailures: this.value.readFailures + 1,
            failureSince: this.value.failureSince ?? Date.now(),
          });
      } finally {
        if (this.active()) this.set({ checking: false });
      }
    }
  }
  dispose() {
    this.alive = false;
    this.generation++;
    this.queued = false;
  }
}
