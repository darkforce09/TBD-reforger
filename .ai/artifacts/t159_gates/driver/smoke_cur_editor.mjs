// smoke_cur_editor.mjs — T-159.22 CUR read-out gate (spec O5). Commits the toolbelt cursor math
// T-159.21 could only check by hand (its verify log §CUR read-out: "not a committed gate; the
// toolbelt has no gate of its own yet").
//
// The expected numbers are DERIVED, not copied from that log:
//   OrthoCamera::new sets `scale = zoom.exp2()` (camera/ortho.rs:127) and the editor boots
//   INITIAL_TARGET = (6400, 6400) / INITIAL_ZOOM = -2 (mission_editor.rs) ⇒ scale = 2^-2 = 0.25 px/m,
//   i.e. 1 px = 4 m. `clamp_target` cannot bite: at 1440x900 the half-extents are 2880 m / 1800 m and
//   6400 sits inside both [2880, 9920] and [1800, 11000]. The container is `h-screen w-screen` and
//   this gate forces a 1440x900 viewport at dsf 1, so rect = (0,0,1440,900) and client px == container
//   px. Hence:
//     C1 centre (720, 450)  -> X 6400.000  Y 6400.000   (the pointer is ON the camera target)
//     C2        (600, 300)  -> X 5920.000  Y 7000.000   (dx = -120 px * 4 = -480 m; dy = -150 px ->
//                                                        +600 m, because flipY:false is north-up)
//   C0 asserts that premise FIRST via __editorCam(), so the gate proves its own basis rather than
//   assuming a camera it never checked.
//
// Assertions:
//   C0 Camera  — __editorCam() reports tx/ty = 6400, z = -2 (the default view this math rests on).
//   C1 Centre  — a trusted mouseMoved at the container centre reads X 6400.000 / Y 6400.000.
//   C2 Offset  — (600, 300) reads X 5920.000 / Y 7000.000 — the arithmetic proof above, and the one
//                that would catch a sign flip (a flipY regression reads Y 5800.000).
//   C3 Off-map — at BOOT, before any pointer has moved, both cells render the em dash. `cursor` is
//                `RwSignal::new(None)` (mission_editor.rs) and renders through the same
//                `fmt_coord(None)` arm the shipped `pointerleave -> None` handler feeds, so this pins
//                the off-map rendering deterministically — where driving a real pointer-leave out of a
//                full-viewport container is not something CDP can do reliably.
//
// Read via [title="Cursor X"] / [title="Cursor Y"] — a real tooltip on a roleless <span> (an
// aria-label there is ignored by AT), matching the toolbelt's existing title= idiom. The 3-dp text is
// also what absorbs any ULP from unproject's matrix inverse, so these are exact string compares.
//
// MUST run before any probe(): probe()/probe_move()/probe_marquee() re-centre the camera on seed 0 as
// a test hook. This gate never calls them.
//
// Backend = default (WebGPU/Dawn): CUR is pure camera math off the engine's target/zoom, and no
// buffer upload is involved, so the ?force=webgl workaround the marquee/undo gates need does not apply.
//
//   node .ai/artifacts/t159_gates/driver/smoke_cur_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5310 })
const b = await launch({ debugPort: 9370 })
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

  const url = `http://localhost:${srv.port}${path}`
  const camReady = `typeof window.__editorCam === 'function'`
  await page.navigate(url)
  await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
  const ready = await page.waitFor(camReady, { tries: 200 })

  // The toolbelt cells. `textContent` is "X" + the padded number; the leading pad is collapsed by
  // HTML, so trim and drop the axis letter.
  const cell = async (axis) =>
    page.evaluate(`(() => { const e = document.querySelector('[title="Cursor ${axis}"]');
      return e ? (e.textContent || '').replace(/^\\s*${axis}\\s*/, '').trim() : null; })()`)
  const read = async () => ({ x: await cell('X'), y: await cell('Y') })
  const move = (x, y) => page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', buttons: 0 })

  const checks = {}
  let cam = null, boot = null, centre = null, offset = null
  const EM_DASH = '—'

  if (ready) {
    // C3 — off-map BEFORE any pointer move (see the header note).
    boot = await read()
    checks.c3_offMapEmDash = boot.x === EM_DASH && boot.y === EM_DASH

    // C0 — pin the camera this math rests on. `__editorCam()` returns a JSON *string* (the
    // smoke_pan_editor idiom) — parse it once, don't re-stringify.
    cam = JSON.parse(await page.evaluate(`window.__editorCam()`))
    checks.c0_defaultCamera = cam.tx === 6400 && cam.ty === 6400 && cam.z === -2

    // C1 — the container centre is the camera target.
    await move(720, 450)
    centre = await read()
    checks.c1_centreIsTarget = centre.x === '6400.000' && centre.y === '6400.000'

    // C2 — the offset proof (1 px = 4 m, north-up).
    await move(600, 300)
    offset = await read()
    checks.c2_offsetMath = offset.x === '5920.000' && offset.y === '7000.000'
  } else {
    console.error('smoke_cur_editor: window.__editorCam never appeared')
  }

  const allChecks = Object.values(checks).every((v) => v === true)
  // A missing key means a branch silently skipped, which .every() would pass vacuously.
  const expectedCount = 4
  const pass = ready && panics.length === 0 && allChecks && Object.keys(checks).length === expectedCount

  console.log(JSON.stringify({
    gate: 'editor-cur-smoke', path,
    ready, backend: cam?.backend ?? null,
    cam, readouts: { boot, centre, offset },
    expected: { centre: ['6400.000', '6400.000'], offset: ['5920.000', '7000.000'] },
    checks,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_cur_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
