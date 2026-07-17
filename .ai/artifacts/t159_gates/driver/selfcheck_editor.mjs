// selfcheck_editor.mjs — T-159.15.1 Mission Creator editor GPU readback gate (the map-lane gate;
// GPU readback, NOT DOM diff). Serves the Leptos dist, mounts the wgpu editor, and awaits the
// byte-exact self-checks the editor exposes on `window.__selfChecks`:
//   calibration = probe.rs fixed 7-probe calibration scene (north-up proof)
//   texture     = synthetic 2×2 texture, 3 corners (north-up UV proof)
// Both are scene-independent, so they pass on the empty 15.1 editor. Pass = every check's JSON
// `pass:true` AND no panic ("Buffer is already mapped" / unreachable). Prints the backend the engine
// picked headless (webgl2 vs webgpu). Mirrors scripts/website/verify-wgpu-gpu.mjs on the t159 harness.
//
//   node .ai/artifacts/t159_gates/driver/selfcheck_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const CHECKS = ['calibration', 'texture']

const srv = await startServer({ dir: leptosDir, port: 5300 })
const b = await launch({ debugPort: 9360 })
const panics = []
try {
  const page = await newPage(b, null, {})
  await page.send('Runtime.enable', {})
  await page.send('Log.enable', {})
  await page.send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false })
  const grab = (t) => { if (/panic|unreachable|already mapped/i.test(t || '')) panics.push(t.slice(0, 300)) }
  page.onEvent('Runtime.consoleAPICalled', (e) => grab((e.args || []).map((a) => a.value || a.description || '').join(' ')))
  page.onEvent('Log.entryAdded', (e) => grab(e.entry?.text))
  page.onEvent('Runtime.exceptionThrown', (e) => grab(e.exceptionDetails?.exception?.description))

  // Force WebGL2/SwiftShader: the byte-exact self_check readback (map_read_4 → device.poll) only
  // resolves on WebGL2 headless; on webgpu/Dawn the offscreen map never fires. Real browsers use
  // webgpu (readback resolves via the event loop) — this gate just needs a deterministic backend.
  const nav = `${path}${path.includes('?') ? '&' : '?'}force=webgl`
  await page.navigate(`http://localhost:${srv.port}${nav}`)
  await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
  // The bridge is registered after RenderEngine::create resolves (async) — poll for it.
  const ready = await page.waitFor(`!!(window.__selfChecks && window.__selfChecks.calibration)`, { tries: 120 })

  const checks = {}
  let backend = 'unknown'
  let allPass = ready
  if (!ready) {
    console.error('selfcheck_editor: window.__selfChecks never appeared')
  } else {
    for (const name of CHECKS) {
      try {
        // awaitPromise=true resolves the js_sys::Promise to its JSON-string report.
        const raw = await page.evaluate(`window.__selfChecks[${JSON.stringify(name)}]()`, true)
        const parsed = JSON.parse(raw)
        checks[name] = { pass: parsed.pass === true, backend: parsed.backend }
        if (parsed.backend) backend = parsed.backend
        allPass = allPass && parsed.pass === true
      } catch (err) {
        checks[name] = { pass: false, error: String(err?.message || err) }
        allPass = false
      }
    }
  }

  const pass = allPass && panics.length === 0
  console.log(JSON.stringify({ gate: 'editor-selfcheck', path, backend, checks, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('selfcheck_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
