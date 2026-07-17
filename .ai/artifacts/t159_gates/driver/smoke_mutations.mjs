// smoke_mutations.mjs — T-159.25 suite-mutation Class T/R gate. Drives a REAL suite mutation
// (Settings "Generate Link Code" → POST /me/link) end to end against the running dev backend,
// with a real dev-login session seeded into localStorage, and asserts the live response reaches
// the DOM (the mono "Link code: …" panel) + a success toast.
//
// This proves the T-159.25 mutation wiring (api_post + single-flight client + toast) works against
// the live API — not just that it compiles. Needs: `make api` on :8080 + `make db-up`, and a
// dev-login token (the harness mints one via the backend's /auth/dev-login).
//
//   TOKEN=<access> REFRESH=<refresh> node .ai/artifacts/t159_gates/driver/smoke_mutations.mjs
//
import { launch, newPage, waitHttp } from './cdp.mjs'
import { startServer } from './serve.mjs'

const token = process.env.TOKEN
const refresh = process.env.REFRESH
if (!token || !refresh) {
  console.error('smoke_mutations: set TOKEN + REFRESH (dev-login tokens)')
  process.exit(2)
}
const leptosDir = process.argv[2] || 'apps/website-leptos/dist'

// Seed the exact tbd-auth blob (auth.rs PersistedAuth: {state:{refreshToken,user,expiresAt},version:0}).
const authBlob = JSON.stringify({
  state: {
    refreshToken: refresh,
    // The full auth.rs `User` shape — missing a required field (arma_character/is_banned/
    // created_at/updated_at) makes `from_persist_json` return None and bootstrap early-return.
    user: {
      discord_id: '00000000000000001',
      username: 'Dev Operator',
      discord_handle: 'dev#0001',
      avatar_url: '',
      arma_id: null,
      arma_character: '',
      role: 'admin',
      is_banned: false,
      total_deployments: 0,
      attendance_rate: 0,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
    expiresAt: '2030-01-01T00:00:00Z',
  },
  version: 0,
})

if (!(await waitHttp('http://127.0.0.1:8080/healthz'))) {
  console.error('smoke_mutations: backend not reachable on :8080')
  process.exit(2)
}

// The static server proxies /api → the real backend same-origin, so the app boots untouched (a
// window.fetch override breaks trunk's WebAssembly.instantiateStreaming).
const srv = await startServer({ dir: leptosDir, port: 5320, apiProxy: 'http://127.0.0.1:8080' })
const b = await launch({ debugPort: 9380 })
const panics = []
try {
  const page = await newPage(b, null, {})
  await page.send('Runtime.enable', {})
  await page.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false })
  const grab = (t) => { if (/panic|unreachable/i.test(t || '')) panics.push(t.slice(0, 200)) }
  page.onEvent('Runtime.consoleAPICalled', (e) => grab((e.args || []).map((a) => a.value || '').join(' ')))

  // Seed the real dev-login session into localStorage before the app boots.
  await page.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `localStorage.setItem('tbd-auth', ${JSON.stringify(authBlob)});`,
  })

  await page.navigate(`http://localhost:${srv.port}/settings`)
  // Wait for the authed Settings render (Generate button present, session bootstrapped).
  const ready = await page.waitFor(
    `[...document.querySelectorAll('button')].some(b => b.textContent.includes('Generate Link Code'))`,
    { tries: 160 },
  )

  const checks = {}
  checks.authedRender = ready
  if (ready) {
    // No pending panel before the mutation.
    checks.noCodeBefore = await page.evaluate(
      `!document.body.textContent.includes('Link code:')`,
    )
    // Click Generate Link Code.
    await page.evaluate(
      `[...document.querySelectorAll('button')].find(b => b.textContent.includes('Generate Link Code')).click()`,
    )
    // The live POST /me/link resolves → the mono code panel appears.
    checks.codePanelAfter = await page.waitFor(
      `document.body.textContent.includes('Link code:')`,
      { tries: 80 },
    )
    // A success toast (role=status) rendered.
    checks.toastShown = await page.waitFor(
      `[...document.querySelectorAll('[role=status]')].some(n => /Link code generated/i.test(n.textContent))`,
      { tries: 40 },
    )
  }

  const pass = ready && panics.length === 0 && Object.values(checks).every((v) => v === true)
  console.log(JSON.stringify({ gate: 'suite-mutations-smoke', checks, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_mutations error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
