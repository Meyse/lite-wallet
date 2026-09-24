import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
      '$app/navigation': fileURLToPath(
        new URL('./src/routes/wallet/__tests__/navigationStub.ts', import.meta.url)
      ),
    },
    conditions: ['browser'],
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.mounted.ts'],
  },
});
