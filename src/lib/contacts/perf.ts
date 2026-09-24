// Per-window, opt-in diagnostics. Labels are fixed by callers; never pass contact data.
let nextToken = 1;

type ContactOperation =
  'form_save' | 'form_delete' | 'watchlist_contact_check' | 'watchlist_identity_save';
type ContactPhase =
  | 'busy_frame'
  | 'input_validation'
  | 'address_validation'
  | 'contacts_load'
  | 'mutation_response'
  | 'result_frame'
  | 'error_frame'
  | 'mutation_failed'
  | 'check_failed';

export function startContactPerf(operation: ContactOperation) {
  let enabled = false;
  try {
    enabled =
      typeof window !== 'undefined' &&
      window.sessionStorage.getItem('lite-wallet-contact-perf') === '1';
  } catch {
    // Diagnostics must never affect wallet actions when browser storage is unavailable.
  }
  const token = nextToken++;
  const started = performance.now();
  const mark = (phase: ContactPhase) => {
    if (!enabled) return;
    console.info(
      `[CONTACT_PERF] operation=${operation} token=${token} phase=${phase} elapsed_ms=${(performance.now() - started).toFixed(1)}`
    );
  };
  return {
    mark,
    nextPaint(phase: ContactPhase) {
      if (!enabled) return;
      // The second callback runs after the first frame has had a paint opportunity.
      requestAnimationFrame(() => requestAnimationFrame(() => mark(phase)));
    },
  };
}
