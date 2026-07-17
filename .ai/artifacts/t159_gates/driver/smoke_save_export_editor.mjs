// smoke_save_export_editor.mjs — T-159.20 Save Version + Export compile Class R gate. Serves the
// Leptos dist, mounts the editor, and asserts the Rust compile (window.__editorCommands, a peer of
// __missionDoc) produces the schema-shaped MissionPayload + MissionExport for the seeded doc:
//   Save   — compile_save_json(): schemaVersion===1, map.bounds===[0,0,12800,12800], terrain 'everon',
//            NO `orbat` key, editor.slots.length===slot_count() (8), factions/squads/editorLayers empty,
//            loadouts/environment objects, objectives/vehicles/markers arrays.
//   Export — compile_export_json(): exportFormatVersion===1, payload.orbat === [] (seed has no factions).
//   Determinism — each bridge is byte-identical across two calls (Save has no clock; Export pins a
//                 fixed exportedAt in the bridge, so it is deterministic too).
// Pass = every assertion true AND no panic. The compile bridge is registered synchronously in on_load
// (GPU-independent), so no ?force=webgl is needed. Mirrors smoke_doc_editor.mjs.
//
//   node .ai/artifacts/t159_gates/driver/smoke_save_export_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const SEED_N = 8 // must match mission_doc.rs `SEED_N`
const EXPECT_BOUNDS = [0, 0, 12800, 12800] // everon (coords/terrains.ts)

const srv = await startServer({ dir: leptosDir, port: 5307 })
const b = await launch({ debugPort: 9367 })
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
    `typeof window.__editorCommands === 'object' && window.__editorCommands !== null` +
      ` && typeof window.__missionDoc === 'object' && window.__missionDoc !== null`,
    { tries: 120 },
  )

  const checks = {}
  let saveStr = '', exportStr = '', slotCount = -1
  if (ready) {
    // Compile twice for determinism, then parse once.
    const s1 = await page.evaluate(`window.__editorCommands.compile_save_json()`)
    const s2 = await page.evaluate(`window.__editorCommands.compile_save_json()`)
    const e1 = await page.evaluate(`window.__editorCommands.compile_export_json()`)
    const e2 = await page.evaluate(`window.__editorCommands.compile_export_json()`)
    saveStr = s1 || ''
    exportStr = e1 || ''
    slotCount = await page.evaluate(`window.__missionDoc.slot_count()`)

    checks.saveDeterministic = typeof s1 === 'string' && s1.length > 0 && s1 === s2
    checks.exportDeterministic = typeof e1 === 'string' && e1.length > 0 && e1 === e2

    // Structural / schema-shape assertions (dep-free mirror of mission-editor-payload.schema.json).
    const isObj = (v) => v !== null && typeof v === 'object' && !Array.isArray(v)
    let save = null, exp = null
    try { save = JSON.parse(s1) } catch { /* leave null */ }
    try { exp = JSON.parse(e1) } catch { /* leave null */ }

    checks.saveParsed = isObj(save)
    checks.exportParsed = isObj(exp)
    if (isObj(save)) {
      checks.schemaVersionInt = save.schemaVersion === 1 && /"schemaVersion":1[,}]/.test(s1)
      checks.terrainEveron = save.map && save.map.terrain === 'everon'
      checks.boundsExact = save.map && JSON.stringify(save.map.bounds) === JSON.stringify(EXPECT_BOUNDS)
      checks.saveOmitsOrbat = !('orbat' in save)
      checks.editorObj = isObj(save.editor)
      checks.slotsMatchDoc = isObj(save.editor) && Array.isArray(save.editor.slots) && save.editor.slots.length === slotCount
      checks.emptyGraph = isObj(save.editor)
        && Array.isArray(save.editor.factions) && save.editor.factions.length === 0
        && Array.isArray(save.editor.squads) && save.editor.squads.length === 0
        && Array.isArray(save.editor.editorLayers) && save.editor.editorLayers.length === 0
      checks.objectShapes = isObj(save.loadouts) && isObj(save.environment)
      checks.arrayShapes = Array.isArray(save.objectives) && Array.isArray(save.vehicles) && Array.isArray(save.markers)
    }
    if (isObj(exp)) {
      checks.exportFormatVersion = exp.exportFormatVersion === 1
      checks.exportOrbatEmpty = isObj(exp.payload) && Array.isArray(exp.payload.orbat) && exp.payload.orbat.length === 0
      checks.exportWrapsPayload = isObj(exp.payload) && exp.payload.schemaVersion === 1
    }
  } else {
    console.error('smoke_save_export_editor: window.__editorCommands never appeared')
  }

  const seeded = slotCount === SEED_N
  const allChecks = Object.values(checks).every((v) => v === true)
  const expectedCount = 16 // total assertions when ready (4 top-level + 9 save + 3 export)
  const pass = ready && panics.length === 0 && seeded && allChecks && Object.keys(checks).length === expectedCount
  console.log(JSON.stringify({
    gate: 'editor-save-export-smoke', path,
    slotCount, seeded, checks,
    saveLen: saveStr.length, exportLen: exportStr.length,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_save_export_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
