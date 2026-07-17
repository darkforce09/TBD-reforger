// smoke_doc_editor.mjs — T-159.16 MissionDoc host Class R gate. Serves the Leptos dist, mounts the
// editor, and asserts the Rust-hosted mission document (window.__missionDoc, a plain MissionDocCore
// in the same wasm module) is live and round-trips:
//   seeded       — slot_count() === SEED_N (the deterministic seed_random golden)
//   roundtripOk  — roundtrip_ok() === true (re-encode stable + encode→apply→materialize set-equality,
//                  all computed in Rust — the live equivalent of the core encode_decode_roundtrip test)
//   encodeStable — encode_hex() is byte-identical across two reads of the same doc
// Pass = every assertion true AND no panic. The doc bridge is registered synchronously in on_load, so
// it is GPU-independent, but the page still mounts the wgpu engine like the other editor smokes.
// Default backend = WebGPU/Dawn (no ?force=webgl). Mirrors smoke_pan_editor.mjs.
//
//   node .ai/artifacts/t159_gates/driver/smoke_doc_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const SEED_N = 8 // must match mission_doc.rs `SEED_N`

const srv = await startServer({ dir: leptosDir, port: 5302 })
const b = await launch({ debugPort: 9362 })
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
  // The doc bridge is registered synchronously in on_load (independent of RenderEngine::create).
  const ready = await page.waitFor(
    `typeof window.__missionDoc === 'object' && window.__missionDoc !== null`,
    { tries: 120 },
  )

  let slotCount = -1, roundtripOk = false, encodeStable = false, encodeHexLen = 0, encodeHexHead = ''
  if (ready) {
    slotCount = await page.evaluate(`window.__missionDoc.slot_count()`)
    roundtripOk = (await page.evaluate(`window.__missionDoc.roundtrip_ok()`)) === true
    const h1 = await page.evaluate(`window.__missionDoc.encode_hex()`)
    const h2 = await page.evaluate(`window.__missionDoc.encode_hex()`)
    encodeStable = typeof h1 === 'string' && h1.length > 0 && h1 === h2
    encodeHexLen = (h1 || '').length
    encodeHexHead = (h1 || '').slice(0, 48) // first 24 bytes — the recorded seed golden (verify log)
  } else {
    console.error('smoke_doc_editor: window.__missionDoc never appeared')
  }

  const seeded = slotCount === SEED_N
  const pass = ready && panics.length === 0 && seeded && roundtripOk && encodeStable
  console.log(JSON.stringify({
    gate: 'editor-doc-smoke', path,
    slotCount, seeded, roundtripOk, encodeStable, encodeHexLen, encodeHexHead,
    panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_doc_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
