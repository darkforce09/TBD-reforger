// smoke_hydrate_editor.mjs — T-159.26 server-hydrate data-safety gate (LIVE backend).
//
// Proves a REAL (UUID) mission opens on its SAVED version, not the 8-slot fixture seed:
//   1. dev-login (admin) → create a mission (POST /missions) → save a version whose editor block
//      has a KNOWN slot set (POST /missions/:id/versions) — all server-side via the proxy.
//   2. Open /missions/<uuid>/edit with a fresh IDB (deleted first) + the seeded tbd-auth session.
//   3. Assert the editor's doc slot_count === the saved version's slot count (NOT SEED_N=8), i.e.
//      the seed was replaced by the server payload. The dirty flag is clear (adopted).
//
// Needs: make api (:8080) + make db-up + make seed. Same-origin /api proxy via serve.mjs (a
// window.fetch override breaks trunk's wasm streaming — see smoke_mutations).
import { launch, newPage, waitHttp } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const BACKEND = 'http://127.0.0.1:8080'
const SAVED_SLOTS = 3 // the known editor.slots count we save; must differ from SEED_N (8)

if (!(await waitHttp(`${BACKEND}/healthz`))) {
  console.error('smoke_hydrate: backend not reachable on :8080'); process.exit(2)
}

// 1a. dev-login (admin) → tokens from the 302 Location fragment.
const loginRes = await fetch(`${BACKEND}/api/v1/auth/dev-login?role=admin`, { redirect: 'manual' })
const loc = loginRes.headers.get('location') || ''
const frag = loc.split('#')[1] || ''
const params = new URLSearchParams(frag)
const token = params.get('access_token')
const refresh = params.get('refresh_token')
if (!token) { console.error('smoke_hydrate: no dev-login token'); process.exit(2) }
const authHdr = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }

// 1b. create a mission.
const createRes = await fetch(`${BACKEND}/api/v1/missions`, {
  method: 'POST', headers: authHdr,
  body: JSON.stringify({ title: 'Hydrate Gate ' + Date.now(), terrain: 'everon', game_mode: 'pve_coop', weather: 'clear', time_of_day: '12:00', max_players: 32 }),
})
if (!createRes.ok) { console.error('smoke_hydrate: create failed', createRes.status); process.exit(2) }
const mission = await createRes.json()
const missionId = mission.id
if (!missionId) { console.error('smoke_hydrate: no mission id'); process.exit(2) }

// 1c. save a version with a KNOWN 3-slot editor block (schema-valid payload superset).
const mkSlot = (i) => ({
  id: `h${i}`, squadId: '', role: 'Rifleman', tag: '', index: i, stance: 'stand',
  position: { x: 6400 + i, y: 6400 + i, z: 0, rotation: 0 }, assetId: '',
})
const payload = {
  schemaVersion: 1,
  map: { terrain: 'everon', bounds: [0, 0, 12800, 12800] },
  environment: { time: '12:00', weather: 'clear' },
  loadouts: {}, objectives: [], vehicles: [], markers: [],
  editor: {
    factions: [], squads: [], editorLayers: [{ id: 'layer-1', name: 'Layer 1', parentId: null, entityIds: ['h0', 'h1', 'h2'] }],
    slots: [mkSlot(0), mkSlot(1), mkSlot(2)],
  },
}
const saveRes = await fetch(`${BACKEND}/api/v1/missions/${missionId}/versions`, {
  method: 'POST', headers: authHdr,
  body: JSON.stringify({ semver: '0.2.0', editor_notes: 'hydrate gate', payload }),
})
if (!saveRes.ok) {
  const t = await saveRes.text()
  console.error('smoke_hydrate: save version failed', saveRes.status, t.slice(0, 200)); process.exit(2)
}

// 2. serve the dist with a same-origin /api proxy; seed the session + clear IDB before boot.
const srv = await startServer({ dir: leptosDir, port: 5315, apiProxy: BACKEND })
const b = await launch({ debugPort: 9375 })
const panics = []
try {
  const page = await newPage(b, null, {})
  await page.send('Runtime.enable', {})
  await page.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false })
  page.onEvent('Runtime.consoleAPICalled', (e) => { const t = (e.args || []).map((a) => a.value || '').join(' '); if (/panic|unreachable/i.test(t)) panics.push(t.slice(0, 200)) })

  const authBlob = JSON.stringify({ state: { refreshToken: refresh, user: {
    discord_id: '00000000000000001', username: 'Dev', discord_handle: 'd#1', avatar_url: '',
    arma_id: null, arma_character: '', role: 'admin', is_banned: false,
    total_deployments: 0, attendance_rate: 0, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z',
  }, expiresAt: '2030-01-01T00:00:00Z' }, version: 0 })
  await page.send('Page.addScriptToEvaluateOnNewDocument', {
    source: `localStorage.setItem('tbd-auth', ${JSON.stringify(authBlob)});` +
      `try { indexedDB.deleteDatabase('tbd-mission-yrs'); } catch (e) {}`,
  })

  await page.navigate(`http://localhost:${srv.port}/missions/${missionId}/edit`)
  await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
  const ready = await page.waitFor(`typeof window.__missionDoc === 'object' && typeof window.__missionDoc.slot_count === 'function'`, { tries: 120 })

  const checks = {}
  if (ready) {
    // The hydrate is async (GET after the sync seed); wait for the slot count to become the saved
    // count (seed replaced). If it stayed 8 the hydrate did NOT run — the data-safety bug.
    checks.hydratedSavedSlots = await page.waitFor(`window.__missionDoc.slot_count() === ${SAVED_SLOTS}`, { tries: 120, interval: 150 })
    checks.notSeed = (await page.evaluate(`window.__missionDoc.slot_count()`)) !== 8
    // The adopted state is clean — no dirty dot (aria-label "Unsaved changes" hidden).
    checks.notDirty = await page.evaluate(`(() => { const el = document.querySelector('[aria-label="Unsaved changes"]'); return !el || el.className.includes('hidden'); })()`)
  }

  const pass = ready && panics.length === 0 && Object.values(checks).every((v) => v === true) && Object.keys(checks).length === 3
  console.log(JSON.stringify({ gate: 'editor-hydrate-smoke', missionId, checks, panics: panics.slice(0, 2), pass }, null, 2))

  // Cleanup: delete the test mission.
  await fetch(`${BACKEND}/api/v1/missions/${missionId}`, { method: 'DELETE', headers: authHdr }).catch(() => {})
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_hydrate error:', e.message)
  await fetch(`${BACKEND}/api/v1/missions/${missionId}`, { method: 'DELETE', headers: authHdr }).catch(() => {})
  process.exitCode = 2
} finally {
  b.kill(); srv.close()
}
