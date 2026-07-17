#!/usr/bin/env node
// gate_r_auth.mjs — the R-auth gate, in a real browser (no backend, no fixtures).
//
// Seeds localStorage["tbd-auth"] with a refresh token, then loads the Leptos app and mocks the API
// via CDP Fetch interception: /auth/refresh → a rotated pair (counted), /me → 401 on the first hit
// then a user. The app's cold-load bootstrap must therefore drive exactly the api/client.ts flow —
// GET /me → 401 → single-flight refresh (ONE call) → retry with the rotated token → /me 200 → set
// session + persist. Asserts refreshCount === 1 AND the user landed in tbd-auth (authed).
//
//   ORACLE unused; LEPTOS_DIST=…/apps/website-leptos/dist node gate_r_auth.mjs
//
// Exit 0 = the browser flow proved single-flight refresh + retry + authed; nonzero otherwise.

import { fileURLToPath } from 'node:url'
import { dirname, resolve, join } from 'node:path'
import { existsSync } from 'node:fs'
import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = resolve(HERE, '../../../../')
const DIST = process.env.LEPTOS_DIST || join(ROOT, 'apps/website-leptos/dist')
if (!existsSync(join(DIST, 'index.html'))) {
  console.error(`gate_r_auth: no Leptos dist at ${DIST} (run \`trunk build\`)`)
  process.exit(2)
}

const SAMPLE_USER = {
  discord_id: '1',
  username: 'cpl-authed',
  discord_handle: 'cpl#0001',
  avatar_url: '',
  arma_id: null,
  arma_character: '',
  role: 'enlisted',
  is_banned: false,
  total_deployments: 0,
  attendance_rate: 0.0,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
}
const SEED = `localStorage.setItem('tbd-auth', JSON.stringify({state:{refreshToken:'rt-seed',user:null,expiresAt:'2026-01-01T00:00:00Z'},version:0}));`

async function main() {
  const srv = await startServer({ dir: DIST, port: 5193 })
  const browser = await launch({ debugPort: 9341 })
  const cleanup = [() => browser.kill(), () => srv.close()]
  try {
    const page = await newPage(browser, null, { initScripts: [SEED] })

    let refreshCount = 0
    let meCount = 0
    const b64 = (o) => Buffer.from(JSON.stringify(o)).toString('base64')
    const fulfill = (requestId, code, json) =>
      page
        .send('Fetch.fulfillRequest', {
          requestId,
          responseCode: code,
          responseHeaders: [{ name: 'content-type', value: 'application/json' }],
          body: b64(json),
        })
        .catch(() => {})

    await page.send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })
    page.onEvent('Fetch.requestPaused', (p) => {
      const u = p.request.url
      if (u.includes('/api/v1/auth/refresh')) {
        refreshCount++
        fulfill(p.requestId, 200, {
          access_token: 'new-access',
          refresh_token: 'new-rt',
          expires_at: '2026-01-01T01:00:00Z',
        })
      } else if (u.includes('/api/v1/me')) {
        meCount++
        if (meCount === 1) fulfill(p.requestId, 401, { error: 'unauthorized' })
        else fulfill(p.requestId, 200, { user: SAMPLE_USER, arma_linked: false })
      } else if (u.includes('/api/v1/')) {
        fulfill(p.requestId, 401, {})
      } else {
        page.send('Fetch.continueRequest', { requestId: p.requestId }).catch(() => {})
      }
    })

    await page.navigate(`http://localhost:${srv.port}/`)

    // Bootstrap is async (spawn_local after mount); wait for the session to land in tbd-auth.
    const authedExpr = `(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user?.username || null } catch { return null } })()`
    const ok = await page.waitFor(
      `(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user != null } catch { return false } })()`,
      { tries: 80, interval: 250 },
    )
    await sleep(100)
    const username = await page.evaluate(authedExpr)

    const pass = ok && refreshCount === 1 && username === SAMPLE_USER.username
    console.log(
      JSON.stringify(
        { gate: 'R-auth', pass, refreshCount, meCount, authedUsername: username, expected: SAMPLE_USER.username },
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
  console.error('gate_r_auth: driver error:', e.stack || e.message)
  process.exit(3)
})
