import type { IdentityWarning } from '$lib/types/wallet.js';

/**
 * Warnings Rust attaches to an identity update review or preflight
 * (`IdentityWarning.warningType` in `src-tauri/src/commands/generic_request.rs`
 * and `src-tauri/src/core/channels/vrpc/identity/preflight.rs`). Rust's
 * `message` is English, so the UI shows translated copy for each code.
 */
const WARNING_KEYS: Record<string, string> = {
  no_effect: 'genericRequest.flow.warning.noEffect',
  empty_patch: 'genericRequest.flow.warning.noEffect',
  dust_change: 'genericRequest.flow.warning.dustChange',
};

/** A code Rust adds later still shows, as a general caution. */
const UNKNOWN_WARNING_KEY = 'genericRequest.flow.warning.other';

/** The translation key for each warning, each key once, in Rust's order. */
export function requestWarningKeys(warnings: readonly IdentityWarning[]): string[] {
  return [
    ...new Set(warnings.map((warning) => WARNING_KEYS[warning.warningType] ?? UNKNOWN_WARNING_KEY)),
  ];
}
