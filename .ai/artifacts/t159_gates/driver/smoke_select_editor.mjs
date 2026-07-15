// smoke_select_editor.mjs — T-159.18 Select / LMB pick-foundation smoke. Serves the Leptos dist,
// mounts the wgpu editor, and proves LMB click-select on the seeded slots via the frozen-viewport
// unproject + Rust PointIndex pick (X-05: NO live RenderEngine::unproject_xy). Asserts:
//   Class S  — window.__editorSelection.pick_selfcheck() === true (PointIndex box-nearest == a
//              brute-force box scan over the real seeded SoA).
//   Test 1   — a plain LMB click on a seed slot selects it (ids == [id], count 1).
//   Test 2   — a plain LMB click on empty map clears the selection (count 0).
//   Test 3   — a Ctrl+LMB click on the seed toggles it ON (count 1).
//   Test 4   — a second Ctrl+LMB click on the seed toggles it OFF (count 0).
//
// probe() centres seed 0 in the view (a test hook) and returns a guaranteed-clickable `hit` px + a
// guaranteed-empty `empty` px, so the smoke is deterministic and independent of where the fixed seed
// lands. Real trusted CDP mouse input drives the actual Leptos pointer handlers (Input.dispatchMouseEvent
// synthesizes pointerdown/up — same path the pan smoke exercises); no synthetic pick shortcut.
//
// Pass = ready AND selfcheck AND Tests 1–4 AND no panic. Default backend = WebGPU/Dawn (pick is pure
// camera math + a CPU spatial index — backend-independent). Mirrors smoke_pan_editor / smoke_persist_editor.
//
//   node .ai/artifacts/t159_gates/driver/smoke_select_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5304 })
const b = await launch({ debugPort: 9364 })
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
  // __editorSelection is registered synchronously; wait for __editorCam too (engine up ⇒ probe()'s
  // set_view/project works). The pick self-check needs only the synchronously-seeded doc.
  const ready = await page.waitFor(
    `typeof window.__editorSelection === 'object' && window.__editorSelection !== null && typeof window.__editorCam === 'function'`,
    { tries: 200 },
  )

  // Trusted CDP mouse → real pointerdown/up on the Leptos container.
  const mouse = (type, x, y, extra = {}) => page.send('Input.dispatchMouseEvent', { type, x, y, ...extra })
  const clickAt = async (x, y, ctrl = false) => {
    const mod = ctrl ? { modifiers: 2 } : {} // CDP modifier bit: Ctrl = 2
    await mouse('mousePressed', x, y, { button: 'left', buttons: 1, clickCount: 1, ...mod })
    await mouse('mouseReleased', x, y, { button: 'left', buttons: 0, clickCount: 1, ...mod })
  }
  const ids = () => page.evaluate(`JSON.parse(window.__editorSelection.ids())`)
  const count = () => page.evaluate(`window.__editorSelection.count()`)

  let selfcheck = false
  let probe = null, probeOk = false
  let t1 = false, t2 = false, t3 = false, t4 = false
  let selIds = null, selCount = -1, clrCount = -1, onCount = -1, offCount = -1

  if (ready) {
    selfcheck = (await page.evaluate(`window.__editorSelection.pick_selfcheck()`)) === true
    probe = JSON.parse(await page.evaluate(`window.__editorSelection.probe()`))
    probeOk =
      probe && typeof probe.id === 'string' && Array.isArray(probe.hit) && Array.isArray(probe.empty)

    if (probeOk) {
      const [hx, hy] = probe.hit
      const [ex, ey] = probe.empty

      // Test 1 — plain click on the seed selects it.
      await clickAt(hx, hy)
      selIds = await ids(); selCount = await count()
      t1 = selCount === 1 && Array.isArray(selIds) && selIds.length === 1 && selIds[0] === probe.id

      // Test 2 — plain click on empty map clears.
      await clickAt(ex, ey)
      clrCount = await count()
      t2 = clrCount === 0

      // Test 3 — Ctrl+click the seed toggles it ON.
      await clickAt(hx, hy, true)
      onCount = await count()
      t3 = onCount === 1

      // Test 4 — a second Ctrl+click toggles it OFF.
      await clickAt(hx, hy, true)
      offCount = await count()
      t4 = offCount === 0
    }
  } else {
    console.error('smoke_select_editor: window.__editorSelection / __editorCam never appeared')
  }

  const pass = ready && selfcheck && probeOk && t1 && t2 && t3 && t4 && panics.length === 0
  console.log(JSON.stringify({
    gate: 'editor-select-smoke', path,
    ready, selfcheck, probeOk, probeId: probe?.id ?? null,
    hit: probe?.hit ?? null, empty: probe?.empty ?? null,
    selIds, selCount, clrCount, onCount, offCount,
    t1_select: t1, t2_clear: t2, t3_toggleOn: t3, t4_toggleOff: t4,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_select_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
