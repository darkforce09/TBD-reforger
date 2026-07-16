// smoke_undo_editor.mjs — T-159.21 undo/redo smoke (spec C6). Serves the Leptos dist, mounts the
// wgpu editor, commits a real drag-move, then proves the MissionDocCore undo stack drives the
// editor through BOTH user paths — the keyboard shortcut and the toolbar button. Asserts:
//   A0 Seed    — on a COLD boot can_undo() === false && can_redo() === false. The seed runs under the
//                INIT origin, which the core's UndoManager does not track, so a fresh mission has no
//                history to undo (a false here means the seed leaked into the LOCAL-origin stack).
//   A1 Move    — probe_move() centres seed 0 and returns from→to px (to = from + 40 px, ≫ the 4 px
//                threshold). A trusted CDP drag commits move_entities: the SEMANTIC slot-position
//                digest (__missionPersist.slots_digest, Class R) changes, and can_undo flips true /
//                can_redo stays false.
//   A2 Undo    — a trusted Ctrl+Z (Input.dispatchKeyEvent → the real window keydown handler) restores
//                the digest EXACTLY (d2 === d0 — slots_digest is sorted rows of raw f32 to_bits, and
//                yrs restores the prior values rather than recomputing them, so this is byte
//                equality, not a tolerance), and the can_undo/can_redo pair inverts.
//   A3 Redo    — a trusted CLICK on the Redo button ([aria-label="Redo"], hit-tested via its live
//                rect) re-applies the move: d3 === d1 and the pair inverts back. A2 + A3 together
//                prove the shortcut and the button drive the SAME path (mission_history::undo/redo).
//   A4 Select  — undoing a *move* must not drop the selection: ids() still contains the dragged seed
//                (after_doc_change prunes only ids that no longer exist).
//   A5 Persist — edit_persist_count() increments across the undo, i.e. undo re-armed the debounced
//                IDB writer. The baseline is read AFTER the A1 drag, never before: schedule_edit_persist
//                bumps the counter unconditionally and the drag itself calls it, so a pre-drag
//                baseline would make this pass on the drag alone and prove nothing about undo.
//   A6 Chrome  — the C1/C4 scaffold is mounted: Undo + Redo buttons and both dock placeholders exist.
//
// Real trusted CDP input drives the actual handlers (no synthetic events, no bridge-only undo path —
// __editorHistory is READ-ONLY by design). A boot-0 __missionPersist.clear() → reload gives a
// deterministic COLD baseline (per smoke_persist_editor.mjs).
//
// Pass = ready AND all checks AND the expected assertion count AND no panic. Backend =
// WebGL2/SwiftShader (forced via ?force=webgl) — the undo LOGIC is backend-independent (a doc
// mutation + a camera-math drag), but the drag's live preview calls engine.set_drag/upload_marquee →
// wgpu create_buffer_init (mappedAtCreation), which headless WebGPU rejects; see the
// smoke_marquee_drag_editor.mjs header for the full note on that software-rasterizer limitation.
//
//   node .ai/artifacts/t159_gates/driver/smoke_undo_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const rawPath = process.argv[3] || '/missions/smoke/edit'
// Force WebGL2 (see the header note on the headless-WebGPU mappedAtCreation limitation).
const path = `${rawPath}${rawPath.includes('?') ? '&' : '?'}force=webgl`

const srv = await startServer({ dir: leptosDir, port: 5308 })
const b = await launch({ debugPort: 9368 })
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
  const persistReady = `typeof window.__missionPersist === 'object' && window.__missionPersist !== null && window.__missionPersist.ready() === true`
  const selReady = `typeof window.__editorSelection === 'object' && window.__editorSelection !== null && typeof window.__editorCam === 'function'`
  const histReady = `typeof window.__editorHistory === 'object' && window.__editorHistory !== null && typeof window.__editorHistory.can_undo === 'function'`
  const bootTo = async (readyExpr) => {
    await page.navigate(url)
    await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
    return page.waitFor(readyExpr, { tries: 200 })
  }

  // Trusted CDP mouse → real pointer events on the Leptos container (shape per the pan/marquee smokes:
  // a 'left' button on a MOVE does not fire a clean pointermove in headless Chrome, so moves carry
  // button:'none' with the left bit held in `buttons`).
  const mouse = (type, x, y, extra = {}) => page.send('Input.dispatchMouseEvent', { type, x, y, ...extra })
  const drag = async (x0, y0, x1, y1) => {
    await mouse('mousePressed', x0, y0, { button: 'left', buttons: 1, clickCount: 1 })
    const steps = 6 // intermediate moves guarantee a pointermove past the 4 px promote gate
    for (let i = 1; i <= steps; i++) {
      await mouse('mouseMoved', x0 + ((x1 - x0) * i) / steps, y0 + ((y1 - y0) * i) / steps, { button: 'none', buttons: 1 })
    }
    await mouse('mouseReleased', x1, y1, { button: 'left', buttons: 0, clickCount: 1 })
  }
  // A trusted Ctrl+Z / Ctrl+Shift+Z. rawKeyDown+keyDown+keyUp with modifiers:2 (CDP bit: Ctrl = 2) —
  // the browser delivers it to the focused node, and it bubbles to the window handler.
  const keyChord = async (code, vk, key, shift = false) => {
    const modifiers = shift ? 2 | 8 : 2 // Ctrl = 2, Shift = 8
    const base = { modifiers, code, key, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk }
    await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...base })
    await page.send('Input.dispatchKeyEvent', { type: 'keyDown', ...base })
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', ...base })
  }
  // Click a chrome button by its a11y name, hit-testing its LIVE rect (no hardcoded px).
  const clickSelector = async (sel) => {
    const r = JSON.parse(
      await page.evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)});
        if (!e) return 'null'; const b = e.getBoundingClientRect();
        return JSON.stringify([b.left + b.width / 2, b.top + b.height / 2]); })()`),
    )
    if (!Array.isArray(r)) return false
    await mouse('mousePressed', r[0], r[1], { button: 'left', buttons: 1, clickCount: 1 })
    await mouse('mouseReleased', r[0], r[1], { button: 'left', buttons: 0, clickCount: 1 })
    return true
  }
  const digest = () => page.evaluate(`window.__missionPersist.slots_digest()`)
  const editCount = () => page.evaluate(`window.__missionPersist.edit_persist_count()`)
  const selIds = () => page.evaluate(`JSON.parse(window.__editorSelection.ids())`)
  const canUndo = () => page.evaluate(`window.__editorHistory.can_undo()`)
  const canRedo = () => page.evaluate(`window.__editorHistory.can_redo()`)

  // ── boot 0: reach a live editor, then hard-reset IndexedDB for a deterministic COLD start.
  const ready0 = await bootTo(persistReady)
  await page.evaluate(`window.__missionPersist.clear()`, /* awaitPromise */ true)

  // ── boot 1 (COLD): seeded doc, bridges live.
  const ready = await bootTo(`${selReady} && ${persistReady} && ${histReady}`)

  const checks = {}
  let d0 = null, d1 = null, d2 = null, d3 = null
  let cu = [], cr = [], cAfterDrag = -1, cAfterUndo = -1
  let mv = null, undoIds = null, redoClicked = false

  if (ready) {
    // A0 — the INIT-origin seed is not an undo step.
    cu.push(await canUndo()); cr.push(await canRedo())
    checks.a0_seedNotUndoable = cu[0] === false && cr[0] === false

    // A6 — the chrome scaffold is mounted (C1/C4).
    checks.a6_chromeMounted =
      (await page.evaluate(`!!document.querySelector('[aria-label="Undo"]') &&
        !!document.querySelector('[aria-label="Redo"]')`)) === true
    // textContent, NOT innerText: the dock headings are `uppercase`, and innerText returns the
    // CSS-transformed text ("EDITOR LAYERS"), which would compare against the source string and fail.
    //
    // T-159.22 updated the expected strings: this asserted the .21 SCAFFOLD's placeholder headings
    // ("ORBAT / Layers" / "Assets"), which that slice's own header called out as text the outliner +
    // palette would replace. The assertion's intent — both docks are mounted — is unchanged; only the
    // headings it pins moved to the real ones (left: ORBAT stub + the live Editor Layers tree; right:
    // the Factions palette, matching React's `AssetBrowser` <h2>).
    checks.a6_docksMounted =
      (await page.evaluate(`(() => { const t = [...document.querySelectorAll('aside')]
        .map((e) => e.textContent || '').join('|');
        return t.includes('ORBAT') && t.includes('Editor Layers') && t.includes('Factions'); })()`)) === true

    d0 = await digest()
    mv = JSON.parse(await page.evaluate(`window.__editorSelection.probe_move()`))
    if (mv && typeof mv.id === 'string' && Array.isArray(mv.from) && Array.isArray(mv.to)) {
      // A1 — commit a real drag-move.
      await drag(mv.from[0], mv.from[1], mv.to[0], mv.to[1])
      d1 = await digest()
      cAfterDrag = await editCount() // baseline AFTER the drag — see the header note on A5
      cu.push(await canUndo()); cr.push(await canRedo())
      checks.a1_moveChangedDigest =
        typeof d0 === 'string' && d0.length > 0 && typeof d1 === 'string' && d1 !== d0
      checks.a1_canUndoAfterMove = cu[1] === true && cr[1] === false

      // A2 — Ctrl+Z restores the digest byte-for-byte.
      await keyChord('KeyZ', 90, 'z')
      d2 = await digest()
      cu.push(await canUndo()); cr.push(await canRedo())
      undoIds = await selIds()
      cAfterUndo = await editCount()
      checks.a2_undoRestoredDigest = d2 === d0
      checks.a2_stackInverted = cu[2] === false && cr[2] === true
      // A4 — undo of a move keeps the seed selected.
      checks.a4_selectionKept = Array.isArray(undoIds) && undoIds.includes(mv.id)
      // A5 — the undo re-armed the debounced IDB writer.
      checks.a5_undoPersisted =
        typeof cAfterDrag === 'number' && typeof cAfterUndo === 'number' && cAfterUndo > cAfterDrag

      // A3 — the Redo BUTTON re-applies the move (same path as the shortcut).
      redoClicked = await clickSelector('[aria-label="Redo"]')
      d3 = await digest()
      cu.push(await canUndo()); cr.push(await canRedo())
      checks.a3_redoClicked = redoClicked === true
      checks.a3_redoRestoredMove = d3 === d1
      checks.a3_stackInverted = cu[3] === true && cr[3] === false
    }
  } else {
    console.error('smoke_undo_editor: window.__editorHistory / __editorSelection never appeared')
  }

  const allChecks = Object.values(checks).every((v) => v === true)
  // 3 pre-move (a0 + 2 chrome) + 9 post-move. A missing key means a branch silently skipped, which
  // .every() would pass vacuously — so the count is asserted too.
  const expectedCount = 12
  const pass =
    ready0 && ready && panics.length === 0 && allChecks && Object.keys(checks).length === expectedCount

  console.log(JSON.stringify({
    gate: 'editor-undo-smoke', path,
    ready0, ready,
    moveId: mv?.id ?? null,
    digests: { d0, d1, d2, d3 },
    canUndo: cu, canRedo: cr,
    editPersist: { afterDrag: cAfterDrag, afterUndo: cAfterUndo },
    undoIds,
    checks,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_undo_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
