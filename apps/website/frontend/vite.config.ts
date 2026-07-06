import { defineConfig, type Plugin, type Connect } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import tsconfigPaths from 'vite-tsconfig-paths'
import wasm from 'vite-plugin-wasm'

// Cross-origin isolation (T-145 Phase 0). COOP:same-origin + COEP:credentialless make
// `crossOriginIsolated === true`, which unlocks `SharedArrayBuffer` — the zero-copy wasm↔deck.gl
// path (Phase 3) and growable-memory views. Applied via an explicit middleware because Vite 8's
// `server.headers` did not attach to the index.html response. `credentialless` (not `require-corp`)
// so cross-origin no-cors subresources (Discord avatars) still load without CDN CORP headers.
// Prod parity: the SPA static host must send these same two headers.
function crossOriginIsolation(): Plugin {
  const setHeaders: Connect.NextHandleFunction = (_req, res, next) => {
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
    res.setHeader('Cross-Origin-Embedder-Policy', 'credentialless')
    next()
  }
  return {
    name: 'tbd-cross-origin-isolation',
    configureServer(server) {
      server.middlewares.use(setHeaders)
    },
    configurePreviewServer(server) {
      server.middlewares.use(setHeaders)
    },
  }
}

export default defineConfig({
  // crossOriginIsolation() first so its headers land on every response. wasm() transforms the
  // wasm-bindgen (bundler-target) `*.wasm` import + top-level instantiation; no
  // vite-plugin-top-level-await (node 26 + Vite 8's modern target support top-level await natively).
  plugins: [crossOriginIsolation(), wasm(), react(), tailwindcss(), tsconfigPaths()],
  // The module workers (worldObjects/compiler) import the wasm core, whose ESM integration uses
  // top-level await — unsupported in the default IIFE worker bundle. `format: 'es'` emits ESM
  // workers (they are already `new Worker(..., { type: 'module' })`), and wasm()/tsconfigPaths()
  // must be re-applied to the worker build (T-145 Phase 1).
  worker: {
    format: 'es',
    plugins: () => [wasm(), tsconfigPaths()],
  },
  // (T-145 Phase 1) The pngjs → pngjs/browser alias is gone: DEM PNG decode now runs in the
  // Rust/wasm core (dem::png_decode), so the app no longer bundles pngjs or the buffer polyfill.
  // pngjs remains a devDependency used only by the DEM parity tests (node resolver, no alias).
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
