import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'

// 3× up from apps/website/frontend → repo root → packages/map-assets.
const mapAssets = fileURLToPath(new URL('../../../packages/map-assets', import.meta.url))

export default defineConfig({
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
