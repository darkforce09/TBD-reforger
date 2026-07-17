// smoke_attributes_editor.mjs — T-159.26 Attributes modal Class R gate (spec A6).
//
// Drives the REAL open paths and commits against the hosted doc:
//   A1  dbl-click a seed slot on the map (trusted CDP mouse, clickCount 2) → modal opens
//   A2t Transform: X commit (input+blur) → slots_digest changes; one undo step
//   A2i Identity: Role commit (per-input, React TextField parity) → digest changes
//   U   real Ctrl+Z (rawKeyDown+keyUp — the .22.1 driver contract) restores the digest
//   A1c Esc closes the modal
//
//   node .ai/artifacts/t159_gates/driver/smoke_attributes_editor.mjs [dist] [path]
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5311 })
const b = await launch({ debugPort: 9371 })
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
      ` && typeof window.__editorHistory === 'object' && typeof window.__editorCam === 'function' && typeof window.__missionPersist === 'object'`,
    { tries: 120 },
  )
  const checks = {}
  if (ready) {
    // Deterministic clickable seed px (centers seed 0) — the select-smoke probe.
    const { hit } = JSON.parse(await page.evaluate(`window.__editorSelection.probe()`))
    const modalOpen = () => `[...document.querySelectorAll('h2')].some(h => h.textContent === 'Attributes')`

    // A1 — trusted dbl-click on the slot (down/up ×2, clickCount 2 on the second pair).
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 2 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 2 })
    checks.a1_open = await page.waitFor(modalOpen(), { tries: 40 })
    checks.a1_selected = await page.evaluate(`window.__editorSelection.count() === 1`)

    const d0 = await page.evaluate(`window.__missionPersist.slots_digest()`)
    const depth0 = await page.evaluate(`window.__editorHistory.undo_depth()`)

    // A2t — Transform tab → X commit via input + blur (the NumberField contract).
    await page.evaluate(`[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Transform').click()`)
    checks.a2t_tab = await page.waitFor(`document.querySelectorAll('input[type=number]').length >= 4`, { tries: 20 })
    await page.evaluate(`(() => {
      const el = document.querySelectorAll('input[type=number]')[0];
      el.focus();
      el.value = '5000';
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.blur();
    })()`)
    const d1 = await page.evaluate(`window.__missionPersist.slots_digest()`)
    const depth1 = await page.evaluate(`window.__editorHistory.undo_depth()`)
    checks.a2t_digestChanged = d1 !== d0
    checks.a2t_oneUndoStep = depth1 === depth0 + 1

    // U — real Ctrl+Z (rawKeyDown + keyUp only; keyDown would double-fire — .22.1).
    await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'z', code: 'KeyZ', modifiers: 2, windowsVirtualKeyCode: 90 })
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'z', code: 'KeyZ', modifiers: 2, windowsVirtualKeyCode: 90 })
    checks.u_digestRestored = await page.waitFor(`window.__missionPersist.slots_digest() === ${JSON.stringify(d0)}`, { tries: 20 })

    // A2i — Identity tab → Role commit per input (TextField parity).
    await page.evaluate(`[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Identity').click()`)
    // Role field by placeholder (the top strip's Save-Version input is also type=text).
    checks.a2i_tab = await page.waitFor(`!!document.querySelector('input[placeholder=Rifleman]')`, { tries: 20 })
    await page.evaluate(`(() => {
      const el = document.querySelector('input[placeholder=Rifleman]');
      el.focus();
      el.value = 'Marksman';
      el.dispatchEvent(new Event('input', { bubbles: true }));
    })()`)
    checks.a2i_digestChanged = await page.waitFor(`window.__missionPersist.slots_digest() !== ${JSON.stringify(d0)}`, { tries: 20 })

    // A1c — Esc closes.
    await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 })
    checks.a1c_closed = await page.waitFor(`!(${modalOpen()})`, { tries: 20 })
  }

  const expectedCount = 9
  const pass = ready && panics.length === 0
    && Object.values(checks).every((v) => v === true)
    && Object.keys(checks).length === expectedCount
  console.log(JSON.stringify({ gate: 'editor-attributes-smoke', path, checks, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_attributes_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
