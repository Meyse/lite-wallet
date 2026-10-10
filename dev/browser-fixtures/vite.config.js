import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// The same browser shims as the app build, for pages that load verus-typescript-primitives.
const require = createRequire(import.meta.url);
const bufferAlias = require.resolve('buffer/');
const createHashAlias = fileURLToPath(
  new URL('../../src/lib/shims/create-hash.cjs', import.meta.url)
);

export default defineConfig({
  plugins: [
    {
      name: 'fixture-wallet-commands',
      enforce: 'pre',
      resolveId(source) {
        if (/(^|\/)invokeWalletCommand(\.[jt]s)?$/.test(source))
          return fileURLToPath(new URL('./invoke-fixture.ts', import.meta.url));
      },
    },
    svelte({ configFile: false, preprocess: vitePreprocess() }),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '$app/navigation': fileURLToPath(new URL('./navigation.js', import.meta.url)),
      $lib: fileURLToPath(new URL('../../src/lib', import.meta.url)),
      buffer: bufferAlias,
      'buffer/': bufferAlias,
      crypto: createHashAlias,
      'create-hash': createHashAlias,
    },
  },
  publicDir: 'static',
  define: { global: 'globalThis' },
  optimizeDeps: { esbuildOptions: { define: { global: 'globalThis' } } },
  server: {
    host: '127.0.0.1',
    port: 1428,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/output/**'] },
  },
});
