import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import tsconfigPaths from 'vite-tsconfig-paths'

export default defineConfig({
  plugins: [react(), tailwindcss(), tsconfigPaths()],
  server: {
    proxy: {
      // 10-min timeouts (incoming + outgoing socket) so a large mission-version upload
      // (hundreds of MB @ 360k slots) isn't dropped by the dev proxy — T-060.1.
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        timeout: 600_000,
        proxyTimeout: 600_000,
      },
      '/uploads': { target: 'http://localhost:8080' },
    },
  },
})
