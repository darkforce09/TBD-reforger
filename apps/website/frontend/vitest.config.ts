import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'
import tsconfigPaths from 'vite-tsconfig-paths'

// 3× up from apps/website/frontend → repo root → packages/map-assets.
const mapAssets = fileURLToPath(new URL('../../../packages/map-assets', import.meta.url))

export default defineConfig({
  // '@/…' imports (tsconfig paths) — the app gets these from vite.config; tests need the
  // same resolver for modules like the T-092.2 flatten that import across features.
  plugins: [tsconfigPaths()],
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
