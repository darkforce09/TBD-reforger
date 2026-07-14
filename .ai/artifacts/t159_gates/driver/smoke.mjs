#!/usr/bin/env node
// smoke.mjs — verifies the CDP driver + freeze + DOM serializer end-to-end against the REAL React
// oracle, with no backend and no Leptos: it drives `/login` (which makes zero API calls) twice on
// two fresh page loads and asserts the two normalized-DOM captures are byte-identical. That proves
// the serializer is deterministic — the property the whole V gate rests on. Writes the oracle
// baseline (v/oracle/login.dom.json + .png) on success.
//
//   ORACLE_DIST=/path/to/apps/website/frontend/dist node .ai/artifacts/t159_gates/driver/smoke.mjs
//
// Exit 0 = deterministic; nonzero otherwise.

import { fileURLToPath } from 'node:url'
import { dirname, resolve, join } from 'node:path'
import { existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { FREEZE_SRC } from './freeze.js'
import { DOM_SERIALIZER_SRC } from './dom.js'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(HERE, '../../../../')
const GATES = resolve(HERE, '..')

const dist = process.env.ORACLE_DIST || join(ROOT, 'apps/website/frontend/dist')
if (!existsSync(join(dist, 'index.html'))) {
  console.error(`smoke: no built React dist at ${dist} (set ORACLE_DIST or run \`npm run build\`)`)
  process.exit(2)
}

const READY = `(async () => {
  const root = document.querySelector('#root');
  if (!root || root.childElementCount === 0) return false;
  await document.fonts.ready;
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  return true;
})()`

const sha = (s) => createHash('sha256').update(s).digest('hex')

async function capture(browser, url) {
  const page = await newPage(browser, url, { initScripts: [FREEZE_SRC, DOM_SERIALIZER_SRC] })
  const ok = await page.waitFor(READY /* awaited via evaluate awaitPromise below */)
  // waitFor uses non-awaited evaluate; do an explicit awaited readiness gate too.
  await page.evaluate(READY, true)
  if (!ok) throw new Error(`app never became ready at ${url}`)
  await sleep(150)
  const dom = await page.evaluate('__t159SerializeDom()')
  const png = await page.screenshot({ x: 0, y: 0, width: 1440, height: 900 })
  await page.close()
  return { dom, png }
}

async function main() {
  const srv = await startServer({ dir: dist, port: 5198 })
  const base = `http://localhost:${srv.port}`
  const browser = await launch({ debugPort: 9335 })
  const cleanup = [() => browser.kill(), () => srv.close()]
  try {
    const a = await capture(browser, `${base}/login`)
    const b = await capture(browser, `${base}/login`)

    const deterministic = a.dom === b.dom
    const outDir = join(GATES, 'v', 'oracle')
    mkdirSync(outDir, { recursive: true })
    if (deterministic) {
      writeFileSync(join(outDir, 'login.dom.json'), a.dom + '\n')
      writeFileSync(join(outDir, 'login.png'), a.png)
    }

    const report = {
      route: '/login',
      deterministic,
      domSha256_run1: sha(a.dom),
      domSha256_run2: sha(b.dom),
      domBytes: a.dom.length,
      pngBytes: a.png.length,
    }
    console.log(JSON.stringify(report, null, 2))
    process.exit(deterministic ? 0 : 1)
  } finally {
    for (const fn of cleanup) {
      try {
        fn()
      } catch {
        /* already down */
      }
    }
  }
}

main().catch((e) => {
  console.error('smoke: driver error:', e.stack || e.message)
  process.exit(3)
})
