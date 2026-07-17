// spike_upload_140mb.mjs — T-159.24 large-upload transport spike (evidence script, not a suite gate).
//
// Question: can the browser fetch path the wasm client rides (gloo-net wraps the same fetch) move a
// ~140 MB JSON body to the backend through the NEW Trunk `/api` dev proxy (Trunk.toml [[proxy]],
// T-159.24)? React needed a direct-:8080 bypass because the Vite proxy reset large bodies
// (T-060.1.4); this decides whether the Leptos dev path needs one.
//
// Method: drive a real chromium page on the `trunk serve` origin (http://127.0.0.1:3000), fetch
// POST a ~140 MB JSON body to /api/v1/missions/<nil-uuid>/versions with a real dev-login Bearer.
// Axum's Json extractor reads + parses the FULL body before the handler can 404 the unknown
// mission (versions route body cap = 256 MB), so ANY 4xx response proves the bytes crossed;
// a network error / socket reset proves they did not. Nothing is written to the DB.
//
//   TOKEN=<access_token> node .ai/artifacts/t159_gates/driver/spike_upload_140mb.mjs
//
// Requires: `make api` on :8080 and `make leptos` (trunk serve) on :3000.
import { launch, newPage, waitHttp } from './cdp.mjs'

const token = process.env.TOKEN
if (!token) {
  console.error('spike_upload_140mb: set TOKEN (dev-login access token)')
  process.exit(2)
}
const PAD_BYTES = 140_000_000

if (!(await waitHttp('http://127.0.0.1:3000/login'))) {
  console.error('spike_upload_140mb: trunk serve not reachable on :3000')
  process.exit(2)
}

const b = await launch({ debugPort: 9399 })
try {
  const page = await newPage(b, null, {})
  await page.send('Runtime.enable', {})
  await page.navigate('http://127.0.0.1:3000/login')
  await page.waitFor(`document.readyState === 'complete'`, { tries: 120 })

  const result = await page.evaluate(
    `(async () => {
      const body = JSON.stringify({
        semver: '9.9.9',
        editor_notes: 'T-159.24 transport spike',
        payload: { schemaVersion: 1, pad: 'x'.repeat(${PAD_BYTES}) },
      })
      const t0 = performance.now()
      try {
        const resp = await fetch('/api/v1/missions/00000000-0000-0000-0000-000000000000/versions', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ${token}' },
          body,
        })
        const text = await resp.text()
        return {
          ok: true, status: resp.status, ms: Math.round(performance.now() - t0),
          bodyBytes: body.length, responseHead: text.slice(0, 200),
        }
      } catch (e) {
        return { ok: false, error: String(e), ms: Math.round(performance.now() - t0), bodyBytes: body.length }
      }
    })()`,
    true,
  )

  // Transport PASS = the server produced an HTTP status (4xx expected: full body read, then the
  // unknown mission / schema check rejects it). A TypeError/network error = transport FAIL.
  const pass = result && result.ok === true && result.status >= 400 && result.status < 500
  console.log(JSON.stringify({ gate: 'spike-upload-140mb', ...result, pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('spike_upload_140mb error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
}
