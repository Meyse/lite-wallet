// Used only by the standalone fixture Vite configuration, never the app build.
import { invoke } from '@tauri-apps/api/core';
declare global {
  interface Window {
    __PROFILE_FIXTURE_INVOKE__?: (
      command: string,
      args?: Record<string, unknown>
    ) => Promise<unknown>;
  }
}
export async function invokeWalletCommand<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  return (
    window.__PROFILE_FIXTURE_INVOKE__
      ? window.__PROFILE_FIXTURE_INVOKE__(command, args)
      : invoke(command, args)
  ) as Promise<T>;
}
export const invokeSessionBoundWalletCommand = invokeWalletCommand;
