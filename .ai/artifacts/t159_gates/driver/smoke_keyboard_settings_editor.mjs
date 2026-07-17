// smoke_keyboard_settings_editor.mjs — T-159.26 editor keyboard + Mission Settings Class R gate.
//
//   K-del  Delete removes the selection (slot_count 8 → 7, one undo step); Ctrl+Z restores
//   K-cv   Ctrl+C on a selected slot then Ctrl+V pastes at the cursor (slot_count +1)
//   S-env  gear opens Mission Settings; changing Weather commits to the doc (digest changes)
//
// Drives trusted CDP mouse + key input against the seeded doc; no ?force=webgl needed (bridges are
// GPU-independent). Digest via __missionPersist.slots_digest, undo depth via __editorHistory.
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5316 })
const b = await launch({ debugPort: 9376 })
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
  const ready = await page.waitFor(
    `typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object'` +
      ` && typeof window.__editorHistory === 'object' && typeof window.__missionPersist === 'object'` +
      ` && typeof window.__editorCam === 'function'`,
    { tries: 120 },
  )
  const checks = {}
  const keyChord = async (key, code, mods, vk) => {
    await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code, modifiers: mods, windowsVirtualKeyCode: vk })
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, modifiers: mods, windowsVirtualKeyCode: vk })
  }

  if (ready) {
    const { hit } = JSON.parse(await page.evaluate(`window.__editorSelection.probe()`))
    // Select seed 0 with a single click (sub-threshold).
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 1 })
    checks.selected1 = await page.waitFor(`window.__editorSelection.count() === 1`, { tries: 20 })

    // K-del — Delete removes it; slot_count 8 → 7, one undo step; Ctrl+Z restores.
    const n0 = await page.evaluate(`window.__missionDoc.slot_count()`)
    const depth0 = await page.evaluate(`window.__editorHistory.undo_depth()`)
    await keyChord('Delete', 'Delete', 0, 46)
    checks.delRemoved = await page.waitFor(`window.__missionDoc.slot_count() === ${n0 - 1}`, { tries: 20 })
    checks.delOneUndo = (await page.evaluate(`window.__editorHistory.undo_depth()`)) === depth0 + 1
    await keyChord('z', 'KeyZ', 2, 90) // Ctrl+Z
    checks.delUndoRestored = await page.waitFor(`window.__missionDoc.slot_count() === ${n0}`, { tries: 20 })

    // K-cv — reselect, Ctrl+C then move cursor over canvas and Ctrl+V. slot_count +1.
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 1 })
    await page.waitFor(`window.__editorSelection.count() === 1`, { tries: 20 })
    await keyChord('c', 'KeyC', 2, 67) // Ctrl+C
    // Move the cursor so paste has a world anchor (a plain mousemove over the canvas centre).
    await page.dispatchMouse('mouseMoved', 720, 470)
    const nBefore = await page.evaluate(`window.__missionDoc.slot_count()`)
    await keyChord('v', 'KeyV', 2, 86) // Ctrl+V
    checks.pasteAdded = await page.waitFor(`window.__missionDoc.slot_count() === ${nBefore + 1}`, { tries: 30 })

    // S-env — open Mission Settings, change Weather, assert the doc env changed.
    await page.evaluate(`document.querySelector('button[aria-label="Mission settings"]').click()`)
    checks.settingsOpen = await page.waitFor(`[...document.querySelectorAll('h2')].some(h => h.textContent === 'Mission Settings')`, { tries: 30 })
    const dEnv0 = await page.evaluate(`window.__missionDoc.slots_digest ? '' : window.__missionPersist.slots_digest()`)
    // Weather is the only <select> in the dialog; set to overcast + change event.
    await page.evaluate(`(() => {
      const sel = [...document.querySelectorAll('select')].find(s => [...s.options].some(o => o.value === 'overcast'));
      sel.value = 'overcast';
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()`)
    // Environment lives in meta (not slots_digest), so assert via a fresh save-compile: the
    // editor command bridge compiles environment into the payload — check it reflects overcast.
    checks.weatherCommitted = await page.waitFor(
      `JSON.parse(window.__editorCommands.compile_save_json()).environment.weather === 'overcast'`,
      { tries: 20 },
    )
    const _ = dEnv0
  }

  const expectedCount = 7
  const pass = ready && panics.length === 0
    && Object.values(checks).every((v) => v === true)
    && Object.keys(checks).length === expectedCount
  console.log(JSON.stringify({ gate: 'editor-keyboard-settings-smoke', path, checks, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_keyboard_settings_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
