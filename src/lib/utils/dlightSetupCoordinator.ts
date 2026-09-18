import type { DlightSeedSetupMode, SetupDlightSeedResult } from '$lib/types/wallet';

export type DlightSetupSettlement =
  | {
      status: 'success';
      configured: boolean;
      requiresRelogin: boolean;
    }
  | { status: 'error' }
  | { status: 'cancelled' };

export type DlightSetupOperation = {
  mode: DlightSeedSetupMode;
};

type InternalDlightSetupOperation = DlightSetupOperation & {
  state: 'pending' | 'settled';
  settlement: DlightSetupSettlement | null;
  listeners: Set<(settlement: DlightSetupSettlement) => void>;
};

const operationsByWalletSession = new Map<string, InternalDlightSetupOperation>();

function settleOperation(
  walletSessionKey: string,
  operation: InternalDlightSetupOperation,
  settlement: DlightSetupSettlement
): void {
  if (operation.state === 'settled') return;

  operation.state = 'settled';
  operation.settlement = settlement;
  if (operationsByWalletSession.get(walletSessionKey) === operation) {
    operationsByWalletSession.delete(walletSessionKey);
  }

  for (const listener of operation.listeners) listener(settlement);
  operation.listeners.clear();
}

export function getDlightSetupOperation(walletSessionKey: string): DlightSetupOperation | null {
  return operationsByWalletSession.get(walletSessionKey) ?? null;
}

export function waitForDlightSetupOperation(
  operation: DlightSetupOperation
): Promise<DlightSetupSettlement> {
  const internalOperation = operation as InternalDlightSetupOperation;
  if (internalOperation.settlement) return Promise.resolve(internalOperation.settlement);

  return new Promise((resolve) => {
    internalOperation.listeners.add(resolve);
  });
}

export function startDlightSetupOperation(
  walletSessionKey: string,
  mode: DlightSeedSetupMode,
  run: () => Promise<SetupDlightSeedResult>
):
  | { operation: DlightSetupOperation; started: false }
  | {
      operation: DlightSetupOperation;
      started: true;
      ownerResult: Promise<SetupDlightSeedResult>;
    } {
  const existing = operationsByWalletSession.get(walletSessionKey);
  if (existing) return { operation: existing, started: false };

  const operation: InternalDlightSetupOperation = {
    mode,
    state: 'pending',
    settlement: null,
    listeners: new Set(),
  };
  operationsByWalletSession.set(walletSessionKey, operation);

  let ownerResult: Promise<SetupDlightSeedResult>;
  try {
    ownerResult = run();
  } catch (error) {
    ownerResult = Promise.reject(error);
  }

  void ownerResult.then(
    (result) => {
      settleOperation(walletSessionKey, operation, {
        status: 'success',
        configured: result.configured,
        requiresRelogin: result.requiresRelogin,
      });
    },
    () => settleOperation(walletSessionKey, operation, { status: 'error' })
  );

  return { operation, started: true, ownerResult };
}

export function clearDlightSetupSession(walletSessionKey: string): void {
  const operation = operationsByWalletSession.get(walletSessionKey);
  if (!operation) return;
  settleOperation(walletSessionKey, operation, { status: 'cancelled' });
}
