// serve.mjs — zero-dep static file server with SPA fallback + COOP/COEP headers.
//
// Serves a built SPA (the React oracle `dist/`, or the Leptos `trunk build` output) so both apps
// run behind the SAME cross-origin-isolation headers the app expects (crossOriginIsolated === true
// for the wasm/SAB path). Any path without a file extension falls back to index.html (client
// routing). Importable (startServer) or runnable (node serve.mjs --dir <dist> --port 5198).

import { createServer } from 'node:http'
import { readFile, stat } from 'node:fs/promises'
import { join, extname, normalize } from 'node:path'

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.webp': 'image/webp',
  '.woff2': 'font/woff2',
  '.woff': 'font/woff',
  '.ico': 'image/x-icon',
  '.map': 'application/json',
}

export function startServer({ dir, port = 0 } = {}) {
  const server = createServer(async (req, res) => {
    // Identical isolation headers for oracle and target.
    res.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
    res.setHeader('Cross-Origin-Embedder-Policy', 'credentialless')
    res.setHeader('Cache-Control', 'no-store')

    try {
      const url = new URL(req.url, 'http://localhost')
      let pathname = decodeURIComponent(url.pathname)
      // Prevent path traversal.
      let rel = normalize(pathname).replace(/^(\.\.[/\\])+/, '')
      let file = join(dir, rel)
      let ext = extname(file)

      // SPA fallback: no extension (a client route) → index.html.
      if (!ext) {
        file = join(dir, 'index.html')
        ext = '.html'
      }
      try {
        await stat(file)
      } catch {
        // Missing asset with an extension = 404; missing route already handled above.
        file = join(dir, 'index.html')
        ext = '.html'
      }
      const body = await readFile(file)
      res.writeHead(200, { 'content-type': MIME[ext] ?? 'application/octet-stream' })
      res.end(body)
    } catch (e) {
      res.writeHead(500)
      res.end('serve error: ' + e.message)
    }
  })
  return new Promise((resolve) => {
    server.listen(port, () => resolve({ server, port: server.address().port, close: () => server.close() }))
  })
}

// CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  const args = process.argv.slice(2)
  const dir = args[args.indexOf('--dir') + 1]
  const port = Number(args[args.indexOf('--port') + 1] ?? 5198)
  if (!dir || args.indexOf('--dir') === -1) {
    console.error('usage: node serve.mjs --dir <dist> [--port 5198]')
    process.exit(2)
  }
  const s = await startServer({ dir, port })
  console.log(`serving ${dir} on http://localhost:${s.port}`)
}
