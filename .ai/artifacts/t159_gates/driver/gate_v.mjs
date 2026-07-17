#!/usr/bin/env node
// gate_v.mjs — the V gate: normalized DOM + computed-style byte-equality, oracle vs Leptos.
//
// Serves both built apps, loads them at the same route under one headless browser (freeze applied),
// extracts the normalized DOM of a selector-scoped region from each (dom.js), and reports a
// structural tree-diff: the path of every mismatch with the oracle vs leptos value. Empty diff =
// PASS (the primary, deterministic V proof). Pixel ε=0 is a separate secondary check.
//
//   node gate_v.mjs --oracle-dir <react dist> --leptos-dir <leptos dist> \
//                   --path / --selector aside [--label sidebar]
//
// Exit 0 = byte-equal; 1 = diffs (printed); 3 = driver error.

import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { FREEZE_SRC } from './freeze.js'
import { DOM_SERIALIZER_SRC } from './dom.js'

const args = process.argv.slice(2)
const arg = (n, d) => {
  const i = args.indexOf(n)
  return i >= 0 ? args[i + 1] : d
}
const oracleDir = arg('--oracle-dir')
const leptosDir = arg('--leptos-dir')
const path = arg('--path', '/')
const selector = arg('--selector', '')
const exclude = arg('--exclude', '')
const label = arg('--label', selector || 'body')
// Authed mode: point at the golden corpus and the gate seeds an authed tbd-auth blob (from the /me
// golden's user) + Fetch-intercepts every /api/v1/** from that corpus, so both apps bootstrap to the
// SAME authed session + data with no live backend. `--ready <sel>` waits for a post-bootstrap
// element (e.g. the page's own content root) before capturing, so we never snapshot the loading state.
const apiFixtures = arg('--api-fixtures', '')
const readySel = arg('--ready', '')
if (!oracleDir || !leptosDir) {
  console.error('usage: node gate_v.mjs --oracle-dir <dist> --leptos-dir <dist> [--path /] [--selector aside]')
  console.error('  authed: [--api-fixtures <goldenDir>] [--ready <selector>]')
  process.exit(2)
}

// /api/v1/<path>[?…] → "<goldenDir>/GET__<path with '/'→'__'>.json" (or null if no golden).
function fixtureFor(url) {
  const m = url.match(/\/api\/v1\/([^?#]*)/)
  if (!m) return null
  const slug = 'GET__' + m[1].replace(/\/+$/, '').replace(/\//g, '__') + '.json'
  const f = join(apiFixtures, slug)
  return existsSync(f) ? f : null
}

// The authed seed: reuse the exact user the /me golden carries so the persisted user matches what
// bootstrap re-fetches (no divergence). accessToken is intentionally absent (never persisted) — the
// app mints one via the intercepted /auth/refresh, exactly like production cold-load.
let SEED = ''
if (apiFixtures) {
  const me = JSON.parse(readFileSync(join(apiFixtures, 'GET__me.json'), 'utf8'))
  const blob = {
    state: { refreshToken: 'rt-seed', user: me.user, expiresAt: '2026-01-01T00:00:00Z' },
    version: 0,
  }
  SEED = `localStorage.setItem('tbd-auth', ${JSON.stringify(JSON.stringify(blob))});`
}

const SETTLE = `(async()=>{await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()`

async function capture(browser, dir, port) {
  const srv = await startServer({ dir, port })
  const initScripts = [FREEZE_SRC, DOM_SERIALIZER_SRC]
  if (SEED) initScripts.push(SEED)
  // Create the page WITHOUT navigating, so the auth seed + Fetch interception are armed first.
  const page = await newPage(browser, null, { initScripts })

  if (apiFixtures) {
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
        return fulfill(200, {
          access_token: 'acc-v',
          refresh_token: 'rt-v2',
          expires_at: '2026-01-01T01:00:00Z',
        })
      }
      if (u.includes('/api/v1/auth/logout')) return fulfill(200, {})
      const f = fixtureFor(u)
      if (f) return fulfill(200, JSON.parse(readFileSync(f, 'utf8')))
      if (u.includes('/api/v1/')) return fulfill(200, {}) // unknown API → empty 200 (never a live hit)
      page.send('Fetch.continueRequest', { requestId: p.requestId }).catch(() => {})
    })
  }

  await page.navigate(`http://localhost:${srv.port}${path}`)
  const sel = selector || 'body'
  const ok = await page.waitFor(`!!document.querySelector(${JSON.stringify(sel)})`, { tries: 80 })
  if (!ok) throw new Error(`selector ${sel} never appeared at ${dir}${path}`)
  // In authed mode wait for the post-bootstrap content root before snapshotting (skip the AuthGate
  // "Loading session…" / QueryState "Loading…" transient).
  if (readySel) {
    const ready = await page.waitFor(`!!document.querySelector(${JSON.stringify(readySel)})`, {
      tries: 160,
      interval: 250,
    })
    if (!ready) throw new Error(`ready selector ${readySel} never appeared at ${dir}${path}`)
  }
  await page.evaluate(SETTLE, true)
  await sleep(150)
  const dom = await page.evaluate(
    `__t159SerializeDom(${JSON.stringify(selector || null)}, ${JSON.stringify(exclude || null)})`,
  )
  await page.close()
  srv.close()
  return dom
}

// Structural tree-diff. Reports up to `cap` mismatches with their path.
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
  const oc = o.children || [],
    lc = l.children || []
  if (oc.length !== lc.length) out.push({ path: `${path}/children.length`, oracle: oc.length, leptos: lc.length })
  for (let i = 0; i < Math.min(oc.length, lc.length); i++) {
    const oi = oc[i],
      li = lc[i]
    if (typeof oi === 'string' || typeof li === 'string') {
      if (oi !== li) out.push({ path: `${path}/text[${i}]`, oracle: oi, leptos: li })
    } else {
      diffNode(oi, li, `${path}/${(li && li.tag) || (oi && oi.tag) || '?'}[${i}]`, out, cap)
    }
  }
}

async function main() {
  const browser = await launch({ debugPort: 9339 })
  try {
    const oracle = await capture(browser, oracleDir, 5191)
    const leptos = await capture(browser, leptosDir, 5192)
    if (oracle === null || leptos === null) {
      console.error(`gate_v[${label}]: selector produced null (oracle=${oracle === null}, leptos=${leptos === null})`)
      process.exit(1)
    }
    const out = []
    diffNode(JSON.parse(oracle), JSON.parse(leptos), selector || 'body', out)
    const pass = out.length === 0
    console.log(
      JSON.stringify(
        { gate: 'V', label, path, selector: selector || 'body', pass, diffs: out.length, oracleBytes: oracle.length, leptosBytes: leptos.length },
        null,
        2,
      ),
    )
    if (!pass) {
      console.log('\n─── first mismatches (oracle → leptos) ───')
      for (const d of out.slice(0, 40)) {
        console.log(`  ${d.path}\n    oracle: ${JSON.stringify(d.oracle)}\n    leptos: ${JSON.stringify(d.leptos)}`)
      }
    }
    process.exit(pass ? 0 : 1)
  } finally {
    browser.kill()
  }
}

main().catch((e) => {
  console.error('gate_v: driver error:', e.stack || e.message)
  process.exit(3)
})
