// smoke_pan_editor.mjs — T-159.15.2 Mission Creator camera-pan smoke (Class R: the gate reads the
// engine camera getters via window.__editorCam(), NOT a DOM diff). Serves the Leptos dist, mounts
// the wgpu editor, drives trusted CDP mouse input, and asserts:
//   Test A (pan)        — RMB drag moves the camera target (content-follows-cursor).
//   Test B (rebase)     — a wheel zoom DURING a pan changes zoom AND the pan continues afterward
//                         with no re-press (P5 / T-151.11.6 mid-pan rebase, incremental engine.pan).
// Pass = every assertion true AND no panic ("Buffer is already mapped" / unreachable). Default
// backend = WebGPU/Dawn (pan is pure camera math, backend-independent — no readback needed, unlike
// selfcheck_editor.mjs which forces ?force=webgl). Mirrors smoke_editor.mjs harness.
//
//   node .ai/artifacts/t159_gates/driver/smoke_pan_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5301 })
const b = await launch({ debugPort: 9361 })
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

  await page.navigate(`http://localhost:${srv.port}${path}`)
  await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
  // The cam bridge is registered after RenderEngine::create resolves (async) — poll for it.
  const ready = await page.waitFor(`typeof window.__editorCam === 'function'`, { tries: 120 })

  // Read the engine camera getters (P6): { tx, ty, z, backend }. NEVER unproject_xy (X-05).
  const cam = async () => JSON.parse(await page.evaluate(`window.__editorCam()`))
  const mouse = (type, x, y, extra = {}) => page.send('Input.dispatchMouseEvent', { type, x, y, ...extra })
  const RMB = { button: 'right', buttons: 2, clickCount: 1 }
  const HELD = { button: 'none', buttons: 2 } // right button held during a move

  let cam0 = { tx: 0, ty: 0, z: 0, backend: 'unknown' }
  let cam1 = cam0, camB1 = cam0, camB2 = cam0, camB3 = cam0
  let panMoved = false, zoomChanged = false, panContinued = false

  if (ready) {
    cam0 = await cam() // expect tx≈6400, ty≈6400, z≈-2 (INITIAL_TARGET / INITIAL_ZOOM)

    // --- Test A: RMB drag left → target moves east (target -= dx/scale; dx<0 ⇒ tx up) ---
    await mouse('mousePressed', 720, 450, RMB)
    await mouse('mouseMoved', 620, 450, HELD)
    await mouse('mouseMoved', 520, 450, HELD)
    await mouse('mouseReleased', 520, 450, RMB)
    cam1 = await cam()
    panMoved = Math.abs(cam1.tx - cam0.tx) > 1e-6

    // --- Test B: mid-pan wheel rebase — pan continues after a mid-drag zoom, no re-press ---
    await mouse('mousePressed', 720, 450, RMB)
    await mouse('mouseMoved', 680, 450, HELD)
    camB1 = await cam() // after the 1st pan move, before the wheel (z still -2)
    // Wheel anchored at the current pan pointer (680,450) — the single-pointer invariant.
    await mouse('mouseWheel', 680, 450, { deltaX: 0, deltaY: -600 })
    camB2 = await cam() // after the zoom (z changed; pan still in flight — pan_px stays Some)
    await mouse('mouseMoved', 620, 450, HELD) // 2nd pan move WITHOUT re-pressing
    await mouse('mouseReleased', 620, 450, RMB)
    camB3 = await cam()
    zoomChanged = Math.abs(camB2.z - camB1.z) > 1e-6
    panContinued = Math.abs(camB3.tx - camB2.tx) > 1e-6
  } else {
    console.error('smoke_pan_editor: window.__editorCam never appeared')
  }

  const pass = ready && panics.length === 0 && panMoved && zoomChanged && panContinued
  console.log(JSON.stringify({
    gate: 'editor-pan-smoke', path, backend: cam0.backend,
    cam0, cam1, camB1, camB2, camB3,
    panMoved, zoomChanged, panContinued,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_pan_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
