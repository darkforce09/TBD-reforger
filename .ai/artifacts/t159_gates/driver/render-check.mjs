#!/usr/bin/env node
// render-check.mjs — generic "does this built SPA actually render X in a real browser" check.
//
// Serves a dist dir, loads a route under headless chromium (freeze applied), waits for the app to
// mount, and asserts the rendered body text contains an expected marker. Used to prove the Leptos
// wasm scaffold mounts (G-gate render proof) and reusable as a quick liveness check for any slice.
//
//   node render-check.mjs --dir <dist> --path /login --expect "some text"
//
// Exit 0 = marker present; nonzero otherwise.

import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { FREEZE_SRC } from './freeze.js'

const args = process.argv.slice(2)
const arg = (name, def) => {
  const i = args.indexOf(name)
  return i >= 0 ? args[i + 1] : def
}
const dir = arg('--dir')
const path = arg('--path', '/')
const expect = arg('--expect', '')
if (!dir) {
  console.error('usage: node render-check.mjs --dir <dist> [--path /] --expect "text"')
  process.exit(2)
}

async function main() {
  const srv = await startServer({ dir, port: Number(arg('--port', 5197)) })
  const browser = await launch({ debugPort: Number(arg('--debug-port', 9337)) })
  const cleanup = [() => browser.kill(), () => srv.close()]
  try {
    const url = `http://localhost:${srv.port}${path}`
    const page = await newPage(browser, url, { initScripts: [FREEZE_SRC] })
    const ready = await page.waitFor(
      `!!document.body && document.body.innerText.trim().length > 0`,
      { tries: 80, interval: 250 },
    )
    await sleep(150)
    const text = (await page.evaluate('document.body.innerText')) || ''
    const html = (await page.evaluate('document.body.innerHTML')) || ''
    await page.close()

    const pass = ready && (!expect || text.includes(expect))
    console.log(
      JSON.stringify(
        { url, ready, expect, found: expect ? text.includes(expect) : null, textPreview: text.slice(0, 200), htmlBytes: html.length },
        null,
        2,
      ),
    )
    process.exit(pass ? 0 : 1)
  } finally {
    for (const fn of cleanup) {
      try {
        fn()
      } catch {
        /* down */
      }
    }
  }
}

main().catch((e) => {
  console.error('render-check: driver error:', e.stack || e.message)
  process.exit(3)
})
