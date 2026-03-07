/**
 * Wallet session state machine: locked | unlocking | unlocked | locking.
 * No secrets in context. Uses walletService for unlock/lock.
 */

import { assign, fromPromise, setup } from 'xstate';
import * as walletService from '$lib/services/walletService.js';

interface WalletContext {
  accountId: string | null;
  unlockError: string | null;
}

type WalletEvent =
  | { type: 'UNLOCK'; account_id: string; password: string }
  | { type: 'LOCK' };

const unlockActor = fromPromise(async ({ input }: { input: { account_id: string; password: string } }) => {
  await walletService.unlockWallet({
    account_id: input.account_id,
    password: input.password
  });
});

const lockActor = fromPromise(async () => {
  await walletService.lockWallet();
});

export const walletMachine = setup({
  actors: {
    unlock: unlockActor,
    lock: lockActor
  }
}).createMachine({
  id: 'wallet',
  types: {} as {
    context: WalletContext;
    events: WalletEvent;
  },
  context: {
    accountId: null,
    unlockError: null
  },
  initial: 'locked',
  states: {
    locked: {
      on: {
        UNLOCK: {
          target: 'unlocking',
          actions: assign({
            accountId: ({ event }) => (event.type === 'UNLOCK' ? event.account_id : null),
            unlockError: () => null
          })
        }
      }
    },
    unlocking: {
      invoke: {
        src: 'unlock',
        input: ({ event }) =>
          event.type === 'UNLOCK' ? { account_id: event.account_id, password: event.password } : { account_id: '', password: '' },
        onDone: { target: 'unlocked', actions: assign({ unlockError: () => null }) },
        onError: {
          target: 'locked',
          actions: assign({
            unlockError: () => 'Wrong password. Please try again.'
          })
        }
      }
    },
    unlocked: {
      on: {
        LOCK: { target: 'locking' }
      }
    },
    locking: {
      invoke: {
        src: 'lock',
        onDone: { target: 'locked', actions: assign({ accountId: () => null, unlockError: () => null }) },
        onError: { target: 'locked', actions: assign({ accountId: () => null }) }
      }
    }
  }
});
