#!/usr/bin/env node
// gate_v_suite.mjs — the V-suite freeze + frozen verify (T-159.29.1, the React-delete prep).
//
// The single-route gate_v.mjs compares LIVE React dist vs Leptos. Once React is deleted that
// oracle is gone forever — so this suite (a) FREEZES the oracle: captures the normalized DOM
// (dom.js serializer, the same one gate_v uses) + a PNG for every leaf route from the built React
// dist into committed goldens, and (b) VERIFIES: re-captures the SAME routes from the Leptos dist
// and diffs against the frozen goldens. After deletion, (b) is the permanent V regression gate.
//
//   node gate_v_suite.mjs freeze  --oracle-dir apps/website/frontend/dist   [--only slug]
//   node gate_v_suite.mjs verify  --leptos-dir apps/website-leptos/dist     [--only slug]
//   node gate_v_suite.mjs accept  --leptos-dir apps/website-leptos/dist      --only slug --note "why"
//
// `accept`: for a route whose divergence from React is INTENDED (the T-159.25 mock→live rebuilds:
// React page showed hardcoded mock data, Leptos shows live state), re-source the golden from the
// CURRENT Leptos dist. The React capture is preserved beside it as <slug>.react.dom.json and the
// manifest row records goldenSource:'leptos' + the acceptance note. The gate stays a strict
// regression pin either way — it just pins the intended state, not React's mock.
//
// Goldens: .ai/artifacts/t159_gates/v/oracle-freeze/<slug>.dom.json (+ .png) + manifest.json
// (per-route sha256 + the oracle dist identity sha). Route table below mirrors
// manifests/routes.csv (26 rows): 25 DOM-frozen routes; /missions/:id/edit is EXCLUDED — the
// editor's DOM is one canvas + docked shell and its regression gate is the 15 CDP editor smokes
// (smoke_*_editor.mjs), which are strictly stronger than a DOM snapshot.
//
// Readiness = the same FREEZE_SRC clock + fixture-intercepted fetches as gate_v, then a
// stability loop: serialize until two consecutive captures are byte-identical (absorbs bootstrap /
// query-loading churn without per-route ready selectors). Viewport pinned 1440x900 in BOTH modes.
//
// Exit 0 = all routes green; 1 = any diff/missing; 3 = driver error.

import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { FREEZE_SRC } from './freeze.js'
import { DOM_SERIALIZER_SRC } from './dom.js'

const args = process.argv.slice(2)
const mode = args[0]
const arg = (n, d) => {
  const i = args.indexOf(n)
  return i >= 0 ? args[i + 1] : d
}
const oracleDir = arg('--oracle-dir', 'apps/website/frontend/dist')
const leptosDir = arg('--leptos-dir', 'apps/website-leptos/dist')
const only = arg('--only', '')
const note = arg('--note', '')
if (mode !== 'freeze' && mode !== 'verify' && mode !== 'accept') {
  console.error('usage: gate_v_suite.mjs <freeze|verify|accept> [--oracle-dir d] [--leptos-dir d] [--only slug] [--note why]')
  process.exit(2)
}
if (mode === 'accept' && (!only || !note)) {
  console.error('accept requires --only <slug> and --note "<why the divergence is intended>"')
  process.exit(2)
}

const GOLD = new URL('../v/oracle-freeze/', import.meta.url).pathname
const FIXTURES = new URL('../fixtures/api/', import.meta.url).pathname

// The committed seed golden ids (memory/fixtures): mission / event / event-mission.
const MISSION = '512d8658-7025-4a70-94e9-a1b44a7aa155'
const EVENT = 'c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7'
const EM = '89b1b731-37a8-4926-901a-3c7ff7de5eb3'

// slug → { path, authed }. 25 of routes.csv's 26 rows (editor excluded, see header).
const ROUTES = [
  { slug: 'notfound', path: '/this-route-does-not-exist', authed: true },
  { slug: 'dashboard', path: '/', authed: true },
  { slug: 'approvals', path: '/admin/approvals', authed: true },
  { slug: 'audit', path: '/admin/audit', authed: true },
  { slug: 'content', path: '/admin/content', authed: true },
  { slug: 'eventmgr', path: '/admin/events', authed: true },
  { slug: 'personnel', path: '/admin/personnel', authed: true },
  { slug: 'servercontrol', path: '/admin/server', authed: true },
  { slug: 'announcements', path: '/announcements', authed: true },
  { slug: 'callback', path: '/auth/callback', authed: false },
  { slug: 'deployments', path: '/deployments', authed: true },
  { slug: 'events', path: '/events', authed: true },
  { slug: 'eventhub', path: `/events/${EVENT}`, authed: true },
  { slug: 'orbat', path: `/events/${EVENT}/missions/${EM}/orbat`, authed: true },
  { slug: 'leaderboards', path: '/leaderboards', authed: true },
  { slug: 'login', path: '/login', authed: false },
  { slug: 'missions', path: '/missions', authed: true },
  { slug: 'missionview', path: `/missions/${MISSION}`, authed: true },
  { slug: 'modpacks', path: '/modpacks', authed: true },
  { slug: 'serverintel', path: '/server-intel', authed: true },
  { slug: 'settings', path: '/settings', authed: true },
  { slug: 'mortar', path: '/tools/mortar', authed: true },
  { slug: 'vehicles', path: '/vehicles', authed: true },
  { slug: 'wiki', path: '/wiki', authed: true },
  { slug: 'wikislug', path: '/wiki/field-manual', authed: true },
]

const sha = (s) => createHash('sha256').update(s).digest('hex')

// /api/v1/<path>[?…] → fixture file (gate_v's mapping).
function fixtureFor(url) {
  const m = url.match(/\/api\/v1\/([^?#]*)/)
  if (!m) return null
  const slug = 'GET__' + m[1].replace(/\/+$/, '').replace(/\//g, '__') + '.json'
  const f = join(FIXTURES, slug)
  return existsSync(f) ? f : null
}

const me = JSON.parse(readFileSync(join(FIXTURES, 'GET__me.json'), 'utf8'))
const SEED = `localStorage.setItem('tbd-auth', ${JSON.stringify(
  JSON.stringify({ state: { refreshToken: 'rt-seed', user: me.user, expiresAt: '2026-01-01T00:00:00Z' }, version: 0 }),
)});`

const SETTLE = `(async()=>{await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()`

async function captureRoute(browser, dir, port, route) {
  const srv = await startServer({ dir, port })
  try {
    const initScripts = [FREEZE_SRC, DOM_SERIALIZER_SRC]
    if (route.authed) initScripts.push(SEED)
    const page = await newPage(browser, null, { initScripts })
    await page.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false })

    await page.send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })
    page.onEvent('Fetch.requestPaused', (p) => {
      const u = p.request.url
      const b64 = (o) => Buffer.from(JSON.stringify(o)).toString('base64')
      const fulfill = (code, json) =>
        page
          .send('Fetch.fulfillRequest', {
            requestId: p.requestId,
            responseCode: code,
            responseHeaders: [{ name: 'content-type', value: 'application/json' }],
            body: b64(json),
          })
          .catch(() => {})
      if (u.includes('/api/v1/auth/refresh')) {
        return fulfill(200, { access_token: 'acc-v', refresh_token: 'rt-v2', expires_at: '2026-01-01T01:00:00Z' })
      }
      if (u.includes('/api/v1/auth/logout')) return fulfill(200, {})
      const f = fixtureFor(u)
      if (f) return fulfill(200, JSON.parse(readFileSync(f, 'utf8')))
      if (u.includes('/api/v1/')) return fulfill(200, {})
      page.send('Fetch.continueRequest', { requestId: p.requestId }).catch(() => {})
    })

    await page.navigate(`http://localhost:${srv.port}${route.path}`)
    const ok = await page.waitFor(`!!document.querySelector('body')`, { tries: 80 })
    if (!ok) throw new Error(`body never appeared at ${route.path}`)

    // Stability loop: two consecutive byte-identical serializations = settled.
    let dom = ''
    let prev = null
    for (let i = 0; i < 60; i++) {
      await page.evaluate(SETTLE, true)
      // Scope = the app root's first child (the whole app UI on both sides). React's #root has a
      // 2nd child — the always-mounted sonner toaster portal (empty <section aria-live>) — with no
      // Leptos twin (its toaster mounts on demand); toast DOM is out of the frozen gate's scope.
      dom = await page.evaluate(`__t159SerializeDom('#root>:first-child', null)`)
      if (dom !== null && dom === prev) break
      prev = dom
      await sleep(300)
      if (i === 59) throw new Error(`DOM never stabilized at ${route.path}`)
    }
    const png = await page.screenshot()
    await page.close()
    return { dom, png }
  } finally {
    srv.close()
  }
}

// Structural tree-diff (gate_v's diffNode).
function diffNode(o, l, path, out, cap = 40) {
  if (out.length >= cap) return
  if (o === null || l === null || typeof o !== 'object' || typeof l !== 'object') {
    if (JSON.stringify(o) !== JSON.stringify(l)) out.push({ path, oracle: o, leptos: l })
    return
  }
  if (o.tag !== l.tag) {
    out.push({ path: `${path}/tag`, oracle: o.tag, leptos: l.tag })
    return
  }
  const A = (x) => x || {}
  for (const k of new Set([...Object.keys(A(o.attrs)), ...Object.keys(A(l.attrs))])) {
    if (A(o.attrs)[k] !== A(l.attrs)[k]) out.push({ path: `${path}/@${k}`, oracle: A(o.attrs)[k], leptos: A(l.attrs)[k] })
  }
  for (const k of new Set([...Object.keys(A(o.style)), ...Object.keys(A(l.style))])) {
    if (A(o.style)[k] !== A(l.style)[k]) out.push({ path: `${path}/style.${k}`, oracle: A(o.style)[k], leptos: A(l.style)[k] })
  }
  const oc = o.children || []
  const lc = l.children || []
  if (oc.length !== lc.length) out.push({ path: `${path}/children.length`, oracle: oc.length, leptos: lc.length })
  for (let i = 0; i < Math.min(oc.length, lc.length); i++) {
    const oi = oc[i]
    const li = lc[i]
    if (typeof oi === 'string' || typeof li === 'string') {
      if (oi !== li) out.push({ path: `${path}/text[${i}]`, oracle: oi, leptos: li })
    } else {
      diffNode(oi, li, `${path}/${(li && li.tag) || (oi && oi.tag) || '?'}[${i}]`, out, cap)
    }
  }
}

// Identity of the frozen oracle build: sha over the sorted (name, sha) of index.html + assets/*.
function distIdentity(dir) {
  const rows = []
  const add = (p, rel) => rows.push(`${rel}:${sha(readFileSync(p))}`)
  add(join(dir, 'index.html'), 'index.html')
  const assets = join(dir, 'assets')
  if (existsSync(assets)) {
    for (const f of readdirSync(assets).sort()) {
      const p = join(assets, f)
      if (statSync(p).isFile()) add(p, `assets/${f}`)
    }
  }
  return sha(rows.join('\n'))
}

async function main() {
  const routes = only ? ROUTES.filter((r) => r.slug === only) : ROUTES
  const browser = await launch({ debugPort: 9341 })
  const rows = []
  let fail = 0
  try {
    for (const route of routes) {
      if (mode === 'freeze') {
        const { dom, png } = await captureRoute(browser, oracleDir, 5195, route)
        mkdirSync(GOLD, { recursive: true })
        writeFileSync(join(GOLD, `${route.slug}.dom.json`), dom)
        writeFileSync(join(GOLD, `${route.slug}.png`), png)
        rows.push({ slug: route.slug, path: route.path, authed: route.authed, goldenSource: 'react', bytes: dom.length, sha256: sha(dom) })
        console.log(`froze  ${route.slug.padEnd(14)} ${String(dom.length).padStart(8)} B  ${sha(dom).slice(0, 12)}`)
      } else if (mode === 'accept') {
        // Preserve the React capture as the historical reference, then re-golden from Leptos.
        const goldFile = join(GOLD, `${route.slug}.dom.json`)
        const reactRef = join(GOLD, `${route.slug}.react.dom.json`)
        if (existsSync(goldFile) && !existsSync(reactRef)) writeFileSync(reactRef, readFileSync(goldFile))
        const { dom, png } = await captureRoute(browser, leptosDir, 5197, route)
        writeFileSync(goldFile, dom)
        writeFileSync(join(GOLD, `${route.slug}.png`), png)
        const manifest = JSON.parse(readFileSync(join(GOLD, 'manifest.json'), 'utf8'))
        const row = manifest.routes.find((r) => r.slug === route.slug)
        Object.assign(row, { goldenSource: 'leptos', bytes: dom.length, sha256: sha(dom), acceptedDelta: note })
        writeFileSync(join(GOLD, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n')
        console.log(`accept ${route.slug.padEnd(14)} ${String(dom.length).padStart(8)} B  ${sha(dom).slice(0, 12)}  (react ref kept)`)
      } else {
        const goldFile = join(GOLD, `${route.slug}.dom.json`)
        if (!existsSync(goldFile)) {
          rows.push({ slug: route.slug, pass: false, error: 'missing golden' })
          fail++
          continue
        }
        const golden = readFileSync(goldFile, 'utf8')
        const { dom } = await captureRoute(browser, leptosDir, 5196, route)
        const out = []
        diffNode(JSON.parse(golden), JSON.parse(dom), 'approot', out)
        const pass = out.length === 0
        if (!pass) fail++
        rows.push({ slug: route.slug, path: route.path, pass, diffs: out.length, goldenBytes: golden.length, leptosBytes: dom.length, first: out.slice(0, 5) })
        console.log(`${pass ? 'PASS' : 'FAIL'}   ${route.slug.padEnd(14)} diffs=${out.length}  ${golden.length}→${dom.length} B`)
      }
    }
    if (mode === 'freeze') {
      const manifest = {
        frozenFrom: 'react dist (apps/website/frontend/dist)',
        distSha256: distIdentity(oracleDir),
        viewport: '1440x900',
        excluded: { '/missions/:id/edit': 'editor gate = the 15 CDP smokes (smoke_*_editor.mjs)' },
        scope: "#root>:first-child — the app UI root on both apps; React's empty sonner toaster portal (#root 2nd child) is outside the frozen scope",
        routes: rows,
      }
      writeFileSync(join(GOLD, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n')
      console.log(`\nfroze ${rows.length} routes · dist ${manifest.distSha256.slice(0, 12)} → ${GOLD}`)
    } else if (mode === 'accept') {
      console.log(`\naccepted ${routes.length} route(s) — golden re-sourced from Leptos, React reference kept`)
    } else {
      console.log(`\n${rows.length - fail}/${rows.length} routes match the frozen oracle`)
      if (fail) {
        for (const r of rows.filter((x) => !x.pass)) console.log(JSON.stringify(r, null, 2))
      }
    }
    process.exit(fail ? 1 : 0)
  } finally {
    browser.kill()
  }
}

main().catch((e) => {
  console.error('gate_v_suite: driver error:', e.stack || e.message)
  process.exit(3)
})
