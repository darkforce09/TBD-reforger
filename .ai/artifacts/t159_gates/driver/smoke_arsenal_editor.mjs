// smoke_arsenal_editor.mjs — T-159.27 Arsenal loadout tab Class R gate.
//
// Drives the REAL Forge path against the hosted doc + the committed /registry golden (the
// gate_r_auth.mjs interception, same 21-row fixture the R-api gate pins — 4 of kind gear_primary):
//   R1  dbl-click a seed slot → Attributes opens; click the Arsenal tab
//   R2  the tab renders one <select> per gear row once the registry resolves (not "Loading catalog…")
//   R3  the Primary <select> lists the golden's M16A2; pick it (native change)
//   R4  compile_save_json().editor.slots — the edited slot's loadout is the canonical SlotLoadoutV2
//       (version 2, weapons[0] slotIndex 0 / slotType primary / weapon = the picked resource_name)
//   R5  the pick is ONE undo step; real Ctrl+Z clears the loadout back to absent
//
//   node .ai/artifacts/t159_gates/driver/smoke_arsenal_editor.mjs [dist] [path]
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { readFileSync } from 'node:fs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const REGISTRY_GOLDEN = new URL('../fixtures/api/GET__registry.json', import.meta.url)
// The M16A2 row in the golden (kind gear_primary) — the pick under test.
const M16A2 = '{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et'

const srv = await startServer({ dir: leptosDir, port: 5314 })
const b = await launch({ debugPort: 9374 })
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

  // Serve the registry golden to the app's real `api_get::<RegistryResponse>("/registry")`.
  const golden = readFileSync(REGISTRY_GOLDEN, 'utf8')
  let registryHits = 0
  await page.send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })
  page.onEvent('Fetch.requestPaused', (p) => {
    const u = p.request.url
    if (u.includes('/api/v1/registry')) {
      registryHits++
      page.send('Fetch.fulfillRequest', {
        requestId: p.requestId, responseCode: 200,
        responseHeaders: [{ name: 'content-type', value: 'application/json' }],
        body: Buffer.from(golden).toString('base64'),
      }).catch(() => {})
    } else if (u.includes('/api/v1/')) {
      page.send('Fetch.fulfillRequest', {
        requestId: p.requestId, responseCode: 401,
        responseHeaders: [{ name: 'content-type', value: 'application/json' }],
        body: Buffer.from('{}').toString('base64'),
      }).catch(() => {})
    } else {
      page.send('Fetch.continueRequest', { requestId: p.requestId }).catch(() => {})
    }
  })

  await page.navigate(`http://localhost:${srv.port}${path}`)
  await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
  const ready = await page.waitFor(
    `typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object'` +
      ` && typeof window.__editorHistory === 'object' && typeof window.__editorCommands === 'object'` +
      ` && typeof window.__missionPersist === 'object'`,
    { tries: 120 },
  )
  const checks = {}
  if (ready) {
    // Wait for the seed slots to land so the probe returns a clickable pixel (not null).
    await page.waitFor(`(() => { try { return JSON.parse(window.__editorSelection.probe()).hit !== null } catch (e) { return false } })()`, { tries: 80 })
    const { hit } = JSON.parse(await page.evaluate(`window.__editorSelection.probe()`))
    const modalOpen = () => `[...document.querySelectorAll('h2')].some(h => h.textContent === 'Attributes')`

    // R1 — trusted dbl-click on the seed slot → modal; then the Arsenal tab.
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 1 })
    await page.dispatchMouse('mousePressed', hit[0], hit[1], { clickCount: 2 })
    await page.dispatchMouse('mouseReleased', hit[0], hit[1], { clickCount: 2 })
    checks.r1_open = await page.waitFor(modalOpen(), { tries: 40 })
    await page.evaluate(`[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Arsenal').click()`)

    // R2 — registry resolved → one <select> per gear row (12), not the loading hint.
    checks.r2_registryFetched = registryHits >= 1
    checks.r2_selectsRendered = await page.waitFor(`document.querySelectorAll('select').length >= 12`, { tries: 40 })

    const depth0 = await page.evaluate(`window.__editorHistory.undo_depth()`)

    // R3 — the Primary select (its label span reads 'Primary') lists M16A2; pick it.
    checks.r3_m16Listed = await page.evaluate(
      `[...document.querySelectorAll('select')].some(s => [...s.options].some(o => o.value === ${JSON.stringify(M16A2)}))`,
    )
    await page.evaluate(`(() => {
      const sel = [...document.querySelectorAll('label')]
        .find(l => l.querySelector('span')?.textContent === 'Primary')?.querySelector('select');
      sel.value = ${JSON.stringify(M16A2)};
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()`)

    // R4 — the compiled save payload carries the canonical SlotLoadoutV2 on the edited slot.
    await page.waitFor(
      `JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout && s.loadout.weapons && s.loadout.weapons.length)`,
      { tries: 40 },
    )
    const loJson = await page.evaluate(
      `(() => {
        const p = JSON.parse(window.__editorCommands.compile_save_json());
        const s = (p.editor?.slots || []).find(s => s.loadout);
        return s ? JSON.stringify(s.loadout) : '';
      })()`,
    )
    const lo = loJson ? JSON.parse(loJson) : null
    checks.r4_version2 = lo?.version === 2
    checks.r4_weaponSlot = lo?.weapons?.[0]?.slotIndex === 0 && lo?.weapons?.[0]?.slotType === 'primary'
    checks.r4_weaponIsPick = lo?.weapons?.[0]?.weapon === M16A2

    // R5 — one undo step; real Ctrl+Z (rawKeyDown+keyUp — the .22.1 driver contract) clears it.
    const depth1 = await page.evaluate(`window.__editorHistory.undo_depth()`)
    checks.r5_oneUndoStep = depth1 === depth0 + 1
    await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'z', code: 'KeyZ', modifiers: 2, windowsVirtualKeyCode: 90 })
    await page.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'z', code: 'KeyZ', modifiers: 2, windowsVirtualKeyCode: 90 })
    checks.r5_undoClears = await page.waitFor(
      `!JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout)`,
      { tries: 20 },
    )
  }

  const expectedCount = 9
  const pass = ready && panics.length === 0
    && Object.values(checks).every((v) => v === true)
    && Object.keys(checks).length === expectedCount
  console.log(JSON.stringify({ gate: 'editor-arsenal-smoke', path, registryHits, checks, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_arsenal_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
