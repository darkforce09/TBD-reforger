// cdp.mjs — reusable zero-dep Chrome DevTools Protocol client for the T-159 gate harness.
//
// Generalized from scripts/website/verify-wgpu-gpu.mjs: raw CDP over Node's built-in WebSocket,
// no npm deps. Drives both the React oracle and (later) the Leptos target identically, one browser
// process, so V/R/T captures are apples-to-apples.
//
// Exports: findChromium, sleep, waitHttp, launch(), newPage(). A page exposes evaluate(),
// screenshot(), dispatchMouse(), dispatchKey(), interceptFetch(), and close().

import { spawn } from 'node:child_process'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

export function findChromium() {
  if (process.env.CHROME_HEADLESS_SHELL && existsSync(process.env.CHROME_HEADLESS_SHELL)) {
    return process.env.CHROME_HEADLESS_SHELL
  }
  const cache = join(homedir(), '.cache/ms-playwright')
  if (!existsSync(cache)) return null
  for (const prefix of ['chromium_headless_shell-', 'chromium-']) {
    const dirs = readdirSync(cache)
      .filter((d) => d.startsWith(prefix))
      .sort()
      .reverse()
    for (const d of dirs) {
      for (const rel of [
        'chrome-headless-shell-linux64/chrome-headless-shell',
        'chrome-linux/chrome',
      ]) {
        const bin = join(cache, d, rel)
        if (existsSync(bin)) return bin
      }
    }
  }
  return null
}

export async function waitHttp(url, tries = 60) {
  for (let i = 0; i < tries; i++) {
    try {
      const res = await fetch(url)
      if (res.ok || res.status === 404) return true // 404 = server up, SPA route
    } catch {
      /* not up yet */
    }
    await sleep(250)
  }
  return false
}

/** Spawn a headless chromium with SwiftShader WebGL2 + lavapipe WebGPU. */
export async function launch({ debugPort = 9333, extraArgs = [] } = {}) {
  const chromium = findChromium()
  if (!chromium) throw new Error('cdp: no chromium (set CHROME_HEADLESS_SHELL or install playwright)')
  const proc = spawn(
    chromium,
    [
      '--no-sandbox',
      '--disable-gpu-sandbox',
      `--remote-debugging-port=${debugPort}`,
      '--use-angle=swiftshader',
      '--enable-unsafe-swiftshader',
      '--enable-unsafe-webgpu',
      '--hide-scrollbars',
      '--force-device-scale-factor=1',
      'about:blank',
      ...extraArgs,
    ],
    { stdio: ['ignore', 'pipe', 'pipe'] },
  )
  // Wait for the debug endpoint instead of a fixed sleep.
  for (let i = 0; i < 80; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${debugPort}/json/version`)
      if (r.ok) break
    } catch {
      /* not up */
    }
    await sleep(125)
  }
  return { proc, debugPort, kill: () => proc.kill('SIGTERM') }
}

/**
 * Open a fresh page, apply init scripts + a fixed 1440×900 dsf=1 viewport BEFORE navigation,
 * then navigate and wait for load. Init scripts run on document-start (freeze/determinism, the DOM
 * serializer definition).
 */
export async function newPage(browser, url, { initScripts = [], viewport = { width: 1440, height: 900 } } = {}) {
  const target = await (
    await fetch(`http://127.0.0.1:${browser.debugPort}/json/new?about:blank`, { method: 'PUT' })
  ).json()
  const ws = new WebSocket(target.webSocketDebuggerUrl)
  await new Promise((res, rej) => {
    ws.onopen = res
    ws.onerror = rej
  })

  let id = 0
  const pending = new Map()
  const eventWaiters = new Map() // method -> resolver[] (one-shot)
  const persistentHandlers = new Map() // method -> cb[] (called for EVERY matching event — no race)
  ws.onmessage = (ev) => {
    const m = JSON.parse(ev.data)
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m)
      pending.delete(m.id)
      return
    }
    if (!m.method) return
    const ph = persistentHandlers.get(m.method)
    if (ph) for (const cb of ph) cb(m.params)
    if (eventWaiters.has(m.method)) {
      const arr = eventWaiters.get(m.method)
      eventWaiters.set(m.method, [])
      for (const r of arr) r(m.params)
    }
  }
  const send = (method, params = {}) =>
    new Promise((res) => {
      const mid = ++id
      pending.set(mid, res)
      ws.send(JSON.stringify({ id: mid, method, params }))
    }).then((m) => {
      if (m.error) throw new Error(`${method}: ${JSON.stringify(m.error)}`)
      return m.result
    })
  const waitEvent = (method, timeout = 30000) =>
    new Promise((res, rej) => {
      const t = setTimeout(() => rej(new Error(`cdp: timeout waiting for ${method}`)), timeout)
      const arr = eventWaiters.get(method) ?? []
      arr.push((v) => {
        clearTimeout(t)
        res(v)
      })
      eventWaiters.set(method, arr)
    })

  await send('Page.enable')
  await send('Runtime.enable')
  await send('Emulation.setDeviceMetricsOverride', {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
    screenWidth: viewport.width,
    screenHeight: viewport.height,
  })
  for (const s of initScripts) await send('Page.addScriptToEvaluateOnNewDocument', { source: s })

  const evaluate = (expression, awaitPromise = false) =>
    send('Runtime.evaluate', { expression, awaitPromise, returnByValue: true, timeout: 120000 }).then(
      (r) => {
        if (r.exceptionDetails) throw new Error(r.exceptionDetails.text ?? 'cdp: evaluate failed')
        return r.result?.value
      },
    )

  const page = {
    send,
    evaluate,
    /** Register a callback fired for EVERY event of `method` (persistent — for Fetch interception). */
    onEvent: (method, cb) => {
      const arr = persistentHandlers.get(method) ?? []
      arr.push(cb)
      persistentHandlers.set(method, arr)
    },
    async navigate(to) {
      const loaded = waitEvent('Page.loadEventFired')
      await send('Page.navigate', { url: to })
      await loaded
    },
    /** Poll a boolean expression until true (app-ready, engine-ready, …). */
    async waitFor(expr, { tries = 120, interval = 250 } = {}) {
      for (let i = 0; i < tries; i++) {
        if (await evaluate(expr)) return true
        await sleep(interval)
      }
      return false
    },
    async screenshot(clip) {
      const r = await send('Page.captureScreenshot', {
        format: 'png',
        clip: clip ? { ...clip, scale: 1 } : undefined,
        captureBeyondViewport: false,
      })
      return Buffer.from(r.data, 'base64')
    },
    dispatchMouse: (type, x, y, extra = {}) =>
      send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1, ...extra }),
    dispatchKey: (type, key, extra = {}) => send('Input.dispatchKeyEvent', { type, key, ...extra }),
    /** Fulfill matching requests from a map of url-substring → {status, json}. */
    async interceptFetch(handler) {
      await send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })
      eventWaiters.set('__fetch', []) // placeholder; real routing below
      // Continuous handler: CDP delivers Fetch.requestPaused as events.
      const route = async (params) => {
        const out = handler(params.request)
        if (out) {
          const body = Buffer.from(JSON.stringify(out.json)).toString('base64')
          await send('Fetch.fulfillRequest', {
            requestId: params.requestId,
            responseCode: out.status ?? 200,
            responseHeaders: [{ name: 'content-type', value: 'application/json' }],
            body,
          })
        } else {
          await send('Fetch.continueRequest', { requestId: params.requestId })
        }
      }
      const loop = () =>
        waitEvent('Fetch.requestPaused', 600000).then((p) => {
          route(p).catch(() => {})
          loop()
        })
      loop().catch(() => {})
    },
    close: () => fetch(`http://127.0.0.1:${browser.debugPort}/json/close/${target.id}`).catch(() => {}),
  }

  if (url) await page.navigate(url)
  return page
}
