// Updated: remove unused ts-expect-error on TAURI_DEV_HOST access.
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';

const require = createRequire(import.meta.url);
const host = process.env.TAURI_DEV_HOST;
const bufferAlias = require.resolve('buffer/');
const createHashAlias = fileURLToPath(new URL('./src/lib/shims/create-hash.cjs', import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit(), tailwindcss()],
  resolve: {
    alias: {
      buffer: bufferAlias,
      'buffer/': bufferAlias,
      crypto: createHashAlias,
      'create-hash': createHashAlias,
    },
  },
  define: {
    global: 'globalThis',
  },
  optimizeDeps: {
    esbuildOptions: {
      define: {
        global: 'globalThis',
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
}));
