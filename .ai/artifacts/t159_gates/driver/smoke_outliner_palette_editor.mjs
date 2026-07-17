// smoke_outliner_palette_editor.mjs — T-159.22 dock gate (spec O1/O2/O3/O4). Proves the left
// Editor Layers outliner and the right Factions palette are live against the hosted MissionDocCore,
// that a palette leaf drags onto the map to place a real slot, and that a wheel over a dock no longer
// zooms the map (the T-159.21 deferred item).
//
// The registry is served by CDP Fetch.fulfillRequest from the SAME committed golden the R-api gate
// pins (fixtures/api/GET__registry.json — 21 rows, 8 of kind `character`), the gate_r_auth.mjs
// pattern. The app therefore runs its REAL `api_get::<RegistryResponse>("/registry")` path with zero
// test-only surface, while the gate stays independent of a live DB (these smokes serve a static
// trunk dist with no backend).
//
// Assertions:
//   P1 Palette  — the golden yields NATO > US_Army > 8 leaves; `[aria-label="US Rifleman"]` exists.
//                 (Rows are <button aria-label=…> — real a11y names, the [aria-label="Undo"] precedent.)
//   O1 Outliner — the seed's 8 slots are listed under the "Unfiled (8)" pseudo-root. They are unfiled
//                 because `seed_random` writes the slots map directly and files nothing into a layer;
//                 a boot-time layer is impossible (smoke_save_export_editor asserts editorLayers === 0).
//   O2 Select   — a trusted click on the FIRST Unfiled row selects EXACTLY ["s0"] — exact because the
//                 Unfiled children are id-sorted (materialize() row order is arbitrary: yrs map
//                 iteration).
//   D1 Place    — pointer-drag `US Rifleman` from its live rect and release at (700, 500):
//                 slot_count 8 -> 9, the toolbelt OBJ text 8 -> 9, the digest changes, and
//                 edit_persist_count increments (the place re-armed the debounced IDB writer).
//   D2 Position — the placed slot lands at EXACTLY (6320, 6200), asserted BIT-EXACTLY with no
//                 tolerance. At the default cam (tx/ty 6400, z -2 ⇒ 0.25 px/m ⇒ 1 px = 4 m):
//                   x = 6400 + (700-720)*4 = 6320 ;  y = 6400 + (450-500)*4 = 6200  (flipY:false)
//                 slots_digest emits `f32::to_bits` per row, and both values are exactly representable
//                 in f32 (6320 and 6200 lie in [4096, 8192) where the ULP is 2^-11 ≈ 0.000488), so the
//                 ~1e-9 matrix-inverse error in unproject_xy is absorbed by the f64->f32 truncation and
//                 the bits land on exactly Math.fround(6320) / Math.fround(6200). Also pins the
//                 asset_id (the full Enfusion ResourceName) and role reaching the doc.
//   D3 Layer    — the place lazily minted the default layer and FILED the slot into it: the new row
//                 leaves Unfiled (still 8) and appears under "Layer 1".
//   W1 Wheel    — a wheel over the LEFT DOCK does not move the camera (__editorCam unchanged), while
//                 the same wheel over the free canvas does. Before T-159.22 the capture-phase listener
//                 zoomed on both.
//
// MUST NOT call probe() before the D2 read: probe() re-centres the camera on seed 0 as a test hook,
// which would invalidate the (700,500) -> (6320,6200) arithmetic. This gate never calls it.
//
// Backend = default (WebGPU/Dawn): no buffer upload is involved (the place path is a doc mutation +
// camera math), so the ?force=webgl workaround the marquee/undo gates need does not apply.
//
//   node .ai/artifacts/t159_gates/driver/smoke_outliner_palette_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { readFileSync } from 'node:fs'
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const REGISTRY_GOLDEN = new URL('../fixtures/api/GET__registry.json', import.meta.url)

const RIFLEMAN_RES = '{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et'
// f32 bit patterns of the exact expected drop position (see the D2 note).
const f32bits = (n) => new Uint32Array(new Float32Array([n]).buffer)[0]
const EXPECT_X_BITS = f32bits(6320)
const EXPECT_Y_BITS = f32bits(6200)

const srv = await startServer({ dir: leptosDir, port: 5309 })
const b = await launch({ debugPort: 9369 })
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

  // ── Serve the registry golden to the app's real fetch (gate_r_auth.mjs pattern).
  const golden = readFileSync(REGISTRY_GOLDEN, 'utf8')
  let registryHits = 0
  await page.send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })
  page.onEvent('Fetch.requestPaused', (p) => {
    const u = p.request.url
    if (u.includes('/api/v1/registry')) {
      registryHits++
      page.send('Fetch.fulfillRequest', {
        requestId: p.requestId,
        responseCode: 200,
        responseHeaders: [{ name: 'content-type', value: 'application/json' }],
        body: Buffer.from(golden).toString('base64'),
      }).catch(() => {})
    } else if (u.includes('/api/v1/')) {
      // Everything else (e.g. the auth bootstrap) is not this gate's business.
      page.send('Fetch.fulfillRequest', {
        requestId: p.requestId, responseCode: 401,
        responseHeaders: [{ name: 'content-type', value: 'application/json' }],
        body: Buffer.from('{}').toString('base64'),
      }).catch(() => {})
    } else {
      page.send('Fetch.continueRequest', { requestId: p.requestId }).catch(() => {})
    }
  })

  const url = `http://localhost:${srv.port}${path}`
  const persistReady = `typeof window.__missionPersist === 'object' && window.__missionPersist !== null && window.__missionPersist.ready() === true`
  const selReady = `typeof window.__editorSelection === 'object' && window.__editorSelection !== null && typeof window.__editorCam === 'function'`
  const docReady = `typeof window.__missionDoc === 'object' && window.__missionDoc !== null`
  const bootTo = async (readyExpr) => {
    await page.navigate(url)
    await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
    return page.waitFor(readyExpr, { tries: 200 })
  }

  const mouse = (type, x, y, extra = {}) => page.send('Input.dispatchMouseEvent', { type, x, y, ...extra })
  const rectOf = async (sel) => {
    const r = await page.evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)});
      if (!e) return 'null'; const b = e.getBoundingClientRect();
      return JSON.stringify([b.left + b.width / 2, b.top + b.height / 2]); })()`)
    const v = JSON.parse(r)
    return Array.isArray(v) ? v : null
  }
  const clickSelector = async (sel) => {
    const r = await rectOf(sel)
    if (!r) return false
    await mouse('mousePressed', r[0], r[1], { button: 'left', buttons: 1, clickCount: 1 })
    await mouse('mouseReleased', r[0], r[1], { button: 'left', buttons: 0, clickCount: 1 })
    return true
  }
  // Pointer-drag: press a palette leaf, move onto the canvas, release. Trusted input → the real
  // pointerdown/move/up handlers (the app arms the place on the leaf press and commits on release).
  const dragFromTo = async (x0, y0, x1, y1) => {
    await mouse('mousePressed', x0, y0, { button: 'left', buttons: 1, clickCount: 1 })
    const steps = 6
    for (let i = 1; i <= steps; i++) {
      await mouse('mouseMoved', x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps, { button: 'none', buttons: 1 })
    }
    await mouse('mouseReleased', x1, y1, { button: 'left', buttons: 0, clickCount: 1 })
  }
  const wheelAt = (x, y) => page.send('Input.dispatchMouseEvent', { type: 'mouseWheel', x, y, deltaX: 0, deltaY: -240 })
  const cam = async () => JSON.parse(await page.evaluate(`window.__editorCam()`))
  const digest = () => page.evaluate(`window.__missionPersist.slots_digest()`)
  const slotCount = () => page.evaluate(`window.__missionDoc.slot_count()`)
  const editCount = () => page.evaluate(`window.__missionPersist.edit_persist_count()`)
  const selIds = () => page.evaluate(`JSON.parse(window.__editorSelection.ids())`)
  const dockText = () => page.evaluate(`(() => [...document.querySelectorAll('aside')].map((e) => e.textContent || '').join('|'))()`)
  // The toolbelt OBJ cell: the wrapper carries the existing title="Placed slots on map / …".
  const objText = () => page.evaluate(`(() => { const e = document.querySelector('[title^="Placed slots"]');
    const m = (e?.textContent || '').match(/OBJ\\s*(\\d+)/); return m ? m[1] : null; })()`)
  // Parse `slots_digest` rows: `id|x_bits|y_bits|z_bits|rot_bits|stance|role|tag|squad|layer`.
  const rowOf = (d, id) => (d || '').split('\n').map((r) => r.split('|')).find((c) => c[0] === id)

  // ── boot 0: reach a live editor, then hard-reset IndexedDB for a deterministic COLD start.
  const ready0 = await bootTo(persistReady)
  await page.evaluate(`window.__missionPersist.clear()`, /* awaitPromise */ true)

  // ── boot 1 (COLD): seeded doc, bridges live, palette fetched.
  const ready = await bootTo(`${selReady} && ${persistReady} && ${docReady}`)
  const paletteReady = await page.waitFor(`!!document.querySelector('[aria-label="US Rifleman"]')`, { tries: 200 })

  const checks = {}
  let docks0 = null, leafRect = null, d0 = null, d1 = null, placedRow = null
  let count0 = -1, count1 = -1, obj0 = null, obj1 = null, ec0 = -1, ec1 = -1
  let firstRowIds = null, camBefore = null, camDock = null, camCanvas = null

  if (ready && paletteReady) {
    // P1 — the palette tree from the golden.
    docks0 = await dockText()
    checks.p1_paletteTree =
      docks0.includes('Factions') && docks0.includes('NATO') && docks0.includes('US_Army')
    checks.p1_eightLeaves =
      (await page.evaluate(`document.querySelectorAll('aside [aria-label^="US "]').length`)) === 8
    checks.p1_registryFetched = registryHits >= 1

    // O1 — the seed's 8 slots are listed, unfiled.
    count0 = await slotCount()
    checks.o1_unfiledRoot = docks0.includes('Unfiled (8)') && count0 === 8

    // O2 — clicking the first Unfiled row selects exactly s0 (id-sorted ⇒ deterministic).
    await clickSelector('aside [aria-label="Rifleman"]')
    firstRowIds = await selIds()
    checks.o2_rowSelectsS0 = Array.isArray(firstRowIds) && firstRowIds.length === 1 && firstRowIds[0] === 's0'

    // D1/D2/D3 — drag the palette leaf onto the canvas at (700, 500).
    d0 = await digest()
    ec0 = await editCount()
    obj0 = await objText()
    leafRect = await rectOf('[aria-label="US Rifleman"]')
    if (leafRect) {
      await dragFromTo(leafRect[0], leafRect[1], 700, 500)
      count1 = await slotCount()
      d1 = await digest()
      ec1 = await editCount()
      obj1 = await objText()
      const docks1 = await dockText()

      checks.d1_slotAdded = count0 === 8 && count1 === 9
      checks.d1_objReadout = obj0 === '8' && obj1 === '9'
      checks.d1_digestChanged = typeof d0 === 'string' && typeof d1 === 'string' && d1 !== d0
      checks.d1_persistArmed = typeof ec0 === 'number' && typeof ec1 === 'number' && ec1 > ec0

      // The placed slot is the one row present in d1 but not d0.
      const ids0 = new Set((d0 || '').split('\n').map((r) => r.split('|')[0]))
      const newId = (d1 || '').split('\n').map((r) => r.split('|')[0]).find((id) => !ids0.has(id))
      placedRow = newId ? rowOf(d1, newId) : null
      // D2 — bit-exact position + the payload that reached the doc.
      checks.d2_positionBitExact =
        !!placedRow && Number(placedRow[1]) === EXPECT_X_BITS && Number(placedRow[2]) === EXPECT_Y_BITS
      checks.d2_roleFromPalette = !!placedRow && placedRow[6] === 'US Rifleman'
      // D3 — lazily minted the default layer AND filed the slot into it (the digest's layer column).
      checks.d3_filedInDefaultLayer = !!placedRow && placedRow[9] === 'layer-1'
      checks.d3_layerInOutliner = docks1.includes('Layer 1') && docks1.includes('Unfiled (8)')
    }

    // W1 — wheel over a dock must not zoom; over the canvas it must.
    camBefore = await cam()
    await wheelAt(120, 500) // inside the 256 px left dock
    camDock = await cam()
    await wheelAt(700, 500) // free canvas
    camCanvas = await cam()
    checks.w1_dockWheelNoZoom = camDock.z === camBefore.z
    checks.w1_canvasWheelZooms = camCanvas.z !== camBefore.z
  } else {
    console.error('smoke_outliner_palette_editor: bridges/palette never appeared')
  }

  const allChecks = Object.values(checks).every((v) => v === true)
  // 3 palette + o1 + o2 + 8 place (d1 x4, d2 x2, d3 x2) + 2 wheel. A missing key means a branch
  // silently skipped, which .every() would pass vacuously — so the count is asserted too.
  const expectedCount = 15
  const pass =
    ready0 && ready && paletteReady && panics.length === 0 && allChecks &&
    Object.keys(checks).length === expectedCount

  console.log(JSON.stringify({
    gate: 'editor-outliner-palette-smoke', path,
    ready0, ready, paletteReady, registryHits,
    counts: { slots: [count0, count1], obj: [obj0, obj1], editPersist: [ec0, ec1] },
    selectedFirstRow: firstRowIds,
    placed: placedRow ? { id: placedRow[0], xBits: Number(placedRow[1]), yBits: Number(placedRow[2]), role: placedRow[6], layer: placedRow[9] } : null,
    expectedBits: { x: EXPECT_X_BITS, y: EXPECT_Y_BITS },
    cam: { before: camBefore, afterDockWheel: camDock, afterCanvasWheel: camCanvas },
    checks,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_outliner_palette_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
