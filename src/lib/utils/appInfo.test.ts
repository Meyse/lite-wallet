import { beforeEach, expect, it, vi } from 'vitest';
const app = vi.hoisted(() => ({ getName: vi.fn(), getVersion: vi.fn(), getIdentifier: vi.fn() }));
vi.mock('@tauri-apps/api/app', () => app);
import { loadRuntimeAppInfo } from './appInfo.js';

beforeEach(() => {
  app.getName.mockResolvedValue('Verus Express');
  app.getIdentifier.mockResolvedValue('com.example.wallet');
});

it('reports unavailable metadata instead of inventing an app version after failure', async () => {
  app.getVersion.mockRejectedValueOnce(new Error('Unavailable'));
  await expect(loadRuntimeAppInfo()).rejects.toThrow('App version unavailable');
  app.getVersion.mockResolvedValueOnce('1.2.3');
  await expect(loadRuntimeAppInfo()).resolves.toMatchObject({ version: '1.2.3' });
});
