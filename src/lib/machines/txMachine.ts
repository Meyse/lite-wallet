/**
 * Send transaction flow: idle -> preflighting -> confirming -> sending -> success | error.
 * No tx hex or signing data in context; preflight_id only for send.
 */

import { assign, fromPromise, setup } from 'xstate';
import * as txService from '$lib/services/txService.js';
import type { PreflightParams, PreflightResult, SendResult } from '$lib/types/wallet.js';

interface TxContext {
  params: PreflightParams | null;
  preflightResult: PreflightResult | null;
  sendResult: SendResult | null;
  error: string | null;
}

type TxEvent =
  | { type: 'SUBMIT_FORM'; params: PreflightParams }
  | { type: 'CONFIRM' }
  | { type: 'RESET' };

const preflightActor = fromPromise(async ({ input }: { input: PreflightParams | null }) => {
  if (!input) {
    throw new Error('Missing preflight params');
  }
  return txService.preflightSend(input);
});

const sendActor = fromPromise(async ({ input }: { input: { preflightId: string } | null }) => {
  if (!input?.preflightId) {
    throw new Error('Missing preflight id');
  }
  return txService.sendTransaction({ preflightId: input.preflightId });
});

export const txMachine = setup({
  actors: {
    preflight: preflightActor,
    send: sendActor
  }
}).createMachine({
  id: 'tx',
  types: {} as {
    context: TxContext;
    events: TxEvent;
  },
  context: {
    params: null,
    preflightResult: null,
    sendResult: null,
    error: null
  },
  initial: 'idle',
  states: {
    idle: {
      on: {
        SUBMIT_FORM: {
          target: 'preflighting',
          actions: assign({
            params: ({ event }) => (event.type === 'SUBMIT_FORM' ? event.params : null),
            error: () => null
          })
        }
      }
    },
    preflighting: {
      invoke: {
        src: 'preflight',
        input: ({ context }) => context.params,
        onDone: {
          target: 'confirming',
          actions: assign({ preflightResult: ({ event }) => event.output })
        },
        onError: {
          target: 'error',
          actions: assign({
            error: ({ event }) => (event.error instanceof Error ? event.error.message : 'Preflight failed')
          })
        }
      }
    },
    confirming: {
      on: {
        CONFIRM: { target: 'sending' },
        RESET: { target: 'idle', actions: assign({ params: () => null, preflightResult: () => null, error: () => null }) }
      }
    },
    sending: {
      invoke: {
        src: 'send',
        input: ({ context }) =>
          context.preflightResult ? { preflightId: context.preflightResult.preflightId } : null,
        onDone: {
          target: 'success',
          actions: assign({ sendResult: ({ event }) => event.output })
        },
        onError: {
          target: 'error',
          actions: assign({
            error: ({ event }) => (event.error instanceof Error ? event.error.message : 'Send failed')
          })
        }
      }
    },
    success: {
      on: {
        RESET: {
          target: 'idle',
          actions: assign({
            params: () => null,
            preflightResult: () => null,
            sendResult: () => null,
            error: () => null
          })
        }
      }
    },
    error: {
      on: {
        RESET: {
          target: 'idle',
          actions: assign({
            params: () => null,
            preflightResult: () => null,
            sendResult: () => null,
            error: () => null
          })
        }
      }
    }
  }
});
