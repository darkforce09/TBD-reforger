import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'
import tsconfigPaths from 'vite-tsconfig-paths'
import wasm from 'vite-plugin-wasm'

// 3× up from apps/website/frontend → repo root → packages/map-assets.
const mapAssets = fileURLToPath(new URL('../../../packages/map-assets', import.meta.url))

export default defineConfig({
  // '@/…' imports (tsconfig paths) — the app gets these from vite.config; tests need the
  // same resolver for modules like the T-092.2 flatten that import across features.
  // wasm(): vitest.config is standalone (does NOT extend vite.config), so the map-engine-wasm
  // parity harness needs the wasm plugin wired here too (T-145 Phase 0).
  plugins: [wasm(), tsconfigPaths()],
  test: {
    environment: 'node', // DEM tests read the committed PNG via fs
    include: ['src/**/*.test.ts'],
  },
  resolve: {
    alias: {
      'map-assets': mapAssets,
    },
  },
})
