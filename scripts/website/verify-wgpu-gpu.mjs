#!/usr/bin/env node
// T-151.11.4 (audit D-06) — committed GPU-R verification harness.
//
// Runs every `window.__selfChecks` gate on the DEV spike page (/_spike/wgpu) under headless
// chromium via raw CDP (Node ≥22 built-in WebSocket; zero npm deps), asserting `pass` on all.
// Backends: WebGL2 via SwiftShader always; the computeCull check self-skips on WebGL2 and
// proves CPU==GPU when a WebGPU adapter exists.
//
// Usage:
//   node scripts/website/verify-wgpu-gpu.mjs            # spawns `vite --port 5199` itself
//   BASE_URL=http://localhost:5173 node scripts/...     # reuse a running dev server
//   CHROME_HEADLESS_SHELL=/path/to/chrome-headless-shell  # explicit browser binary
// Exit 0 = every check pass:true; nonzero otherwise (JSON verdicts on stdout either way).

import { spawn } from 'node:child_process'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'

const ROOT = new URL('../..', import.meta.url).pathname
const FRONTEND = join(ROOT, 'apps/website/frontend')
const PORT = 5199
const DEBUG_PORT = 9333

function findChromium() {
  if (process.env.CHROME_HEADLESS_SHELL && existsSync(process.env.CHROME_HEADLESS_SHELL)) {
    return process.env.CHROME_HEADLESS_SHELL
  }
  const cache = join(homedir(), '.cache/ms-playwright')
  if (existsSync(cache)) {
    const dirs = readdirSync(cache)
      .filter((d) => d.startsWith('chromium_headless_shell-'))
      .sort()
      .reverse()
    for (const d of dirs) {
      const bin = join(cache, d, 'chrome-headless-shell-linux64/chrome-headless-shell')
      if (existsSync(bin)) return bin
    }
    const full = readdirSync(cache)
      .filter((d) => d.startsWith('chromium-'))
      .sort()
      .reverse()
    for (const d of full) {
      const bin = join(cache, d, 'chrome-linux/chrome')
      if (existsSync(bin)) return bin
    }
  }
  return null
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function waitHttp(url, tries = 60) {
  for (let i = 0; i < tries; i++) {
    try {
      const res = await fetch(url)
      if (res.ok) return true
    } catch {
      /* not up yet */
    }
    await sleep(500)
  }
  return false
}

async function main() {
  const chromium = findChromium()
  if (!chromium) {
    console.error(
      'verify-wgpu-gpu: no chromium found (set CHROME_HEADLESS_SHELL or install playwright chromium)',
    )
    process.exit(3)
  }

  let vite = null
  let base = process.env.BASE_URL
  if (!base) {
    base = `http://localhost:${PORT}`
    vite = spawn('npx', ['vite', '--port', String(PORT), '--strictPort'], {
      cwd: FRONTEND,
      stdio: ['ignore', 'pipe', 'pipe'],
    })
  }
  const cleanup = []
  cleanup.push(() => vite?.kill('SIGTERM'))

  try {
    if (!(await waitHttp(`${base}/_spike/wgpu`))) {
      console.error(`verify-wgpu-gpu: dev server not reachable at ${base}`)
      process.exit(3)
    }

    const browser = spawn(
      chromium,
      [
        '--no-sandbox',
        '--disable-gpu-sandbox',
        `--remote-debugging-port=${DEBUG_PORT}`,
        '--use-angle=swiftshader',
        '--enable-unsafe-swiftshader',
        '--enable-unsafe-webgpu',
        'about:blank',
      ],
      { stdio: ['ignore', 'pipe', 'pipe'] },
    )
    cleanup.push(() => browser.kill('SIGTERM'))
    await sleep(1500)

    const target = await (
      await fetch(`http://127.0.0.1:${DEBUG_PORT}/json/new?${encodeURIComponent(`${base}/_spike/wgpu?force=webgl`)}`, {
        method: 'PUT',
      })
    ).json()
    const ws = new WebSocket(target.webSocketDebuggerUrl)
    await new Promise((res, rej) => {
      ws.onopen = res
      ws.onerror = rej
    })
    let id = 0
    const pending = new Map()
    ws.onmessage = (ev) => {
      const msg = JSON.parse(ev.data)
      if (msg.id && pending.has(msg.id)) {
        pending.get(msg.id)(msg)
        pending.delete(msg.id)
      }
    }
    const send = (method, params = {}) =>
      new Promise((res) => {
        const mid = ++id
        pending.set(mid, res)
        ws.send(JSON.stringify({ id: mid, method, params }))
      })
    await send('Runtime.enable')
    const evalJs = async (expression, awaitPromise = false) => {
      const r = await send('Runtime.evaluate', {
        expression,
        awaitPromise,
        returnByValue: true,
        timeout: 120000,
      })
      if (r.result?.exceptionDetails) {
        return { error: r.result.exceptionDetails.text ?? 'evaluate failed' }
      }
      return { value: r.result?.result?.value }
    }

    // Engine init under SwiftShader can take a while.
    let hooked = false
    for (let i = 0; i < 120; i++) {
      const { value } = await evalJs('!!window.__selfChecks && Object.keys(window.__selfChecks).length')
      if (value) {
        hooked = true
        break
      }
      await sleep(1000)
    }
    if (!hooked) {
      console.error('verify-wgpu-gpu: __selfChecks never registered (engine init failed?)')
      process.exit(2)
    }

    const { value: names } = await evalJs('Object.keys(window.__selfChecks)')
    const results = {}
    let allPass = true
    for (const name of names) {
      const r = await evalJs(`window.__selfChecks[${JSON.stringify(name)}]()`, true)
      if (r.error) {
        results[name] = { error: r.error, pass: false }
        allPass = false
        continue
      }
      let parsed
      try {
        parsed = JSON.parse(r.value)
      } catch {
        parsed = { raw: String(r.value), pass: false }
      }
      results[name] = parsed
      allPass &&= parsed.pass === true
    }

    console.log(JSON.stringify({ page: `${base}/_spike/wgpu?force=webgl`, results, allPass }, null, 1))
    process.exit(allPass ? 0 : 1)
  } finally {
    for (const fn of cleanup) {
      try {
        fn()
      } catch {
        /* already dead */
      }
    }
  }
}

main().catch((e) => {
  console.error('verify-wgpu-gpu: driver error:', e.message)
  process.exit(3)
})
