// smoke_persist_editor.mjs — T-159.17 yrs IDB persist Class R gate. Serves the Leptos dist, mounts
// the editor, and proves the Rust IndexedDB persistence layer keeps the local doc across a reload:
//
//   boot 0        — reach a live editor, then HARD-RESET IndexedDB + sessionStorage (window.__missionPersist.clear())
//                   so the run starts COLD regardless of the browser profile.
//   boot 1 (COLD) — no blob → seed; loaded_from_storage()===false; slot_count()===SEED_N;
//                   roundtrip_ok()===true; capture the SEMANTIC slot digest (slots_digest()); flush() → IDB.
//   boot 2 (WARM) — blob present → Rust SWAPs a fresh core with the applied blob; loaded_from_storage()===true;
//                   slot_count()===SEED_N; roundtrip_ok()===true; slots_digest()===coldDigest; warm() marker present.
//
// Class R is asserted on the SEMANTIC slot digest (id-sorted, bit-exact floats, interned indices
// resolved to strings), NOT the encode bytes: `yrs` re-encodes a fresh peer to semantically-identical
// but byte-different bytes (only the materialization is equal — see mission_doc.rs / the core's
// `encode_decode_roundtrip_is_stable`). A byte compare would be a false negative; the digest is sound.
//
// Pass = COLD ok AND WARM ok AND no panic. __missionDoc (T-159.16) gives slot_count/roundtrip_ok;
// __missionPersist (T-159.17) gives ready/loaded_from_storage/warm/slots_digest/flush/clear.
// Default backend = WebGPU/Dawn (no ?force=webgl). Mirrors smoke_doc_editor.mjs.
//
//   node .ai/artifacts/t159_gates/driver/smoke_persist_editor.mjs [leptosDir=apps/website-leptos/dist] [path=/missions/smoke/edit]
//
import { launch, newPage } from './cdp.mjs'
import { startServer } from './serve.mjs'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'
const SEED_N = 8 // must match mission_doc.rs `SEED_N`
const MISSION_ID = 'smoke' // the `:id` route segment on the gate route

const srv = await startServer({ dir: leptosDir, port: 5303 })
const b = await launch({ debugPort: 9363 })
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
  const docReady = `typeof window.__missionDoc === 'object' && window.__missionDoc !== null`
  const bootTo = async (readyExpr) => {
    await page.navigate(url) // CDP navigate reloads even to the same URL; same origin ⇒ IDB + sessionStorage survive
    await page.waitFor(`!!document.querySelector('canvas')`, { tries: 80 })
    const ready = await page.waitFor(readyExpr, { tries: 200 })
    await page.waitFor(docReady, { tries: 120 })
    return ready
  }

  // ── boot 0: reach a live editor, then hard-reset for a deterministic COLD start.
  const ready0 = await bootTo(persistReady)
  await page.evaluate(`window.__missionPersist.clear()`, /* awaitPromise */ true)

  // ── boot 1 (COLD): no blob → seed. Capture the semantic slot digest, flush the seed blob to IDB.
  const readyCold = await bootTo(persistReady)
  const coldLoaded = await page.evaluate(`window.__missionPersist.loaded_from_storage()`)
  const coldSlots = await page.evaluate(`window.__missionDoc.slot_count()`)
  const coldDocRt = (await page.evaluate(`window.__missionDoc.roundtrip_ok()`)) === true
  const coldDigest = await page.evaluate(`window.__missionPersist.slots_digest()`)
  const encodeHexLen = ((await page.evaluate(`window.__missionDoc.encode_hex()`)) || '').length // info only
  await page.evaluate(`window.__missionPersist.flush()`, /* awaitPromise */ true)

  // ── boot 2 (WARM): blob present → SWAP restore. Slot digest must equal the cold digest.
  const readyWarm = await bootTo(`${persistReady} && window.__missionPersist.loaded_from_storage() === true`)
  const warmLoaded = await page.evaluate(`window.__missionPersist.loaded_from_storage()`)
  const warmSlots = await page.evaluate(`window.__missionDoc.slot_count()`)
  const warmDocRt = (await page.evaluate(`window.__missionDoc.roundtrip_ok()`)) === true
  const warmDigest = await page.evaluate(`window.__missionPersist.slots_digest()`)
  const warmJson = await page.evaluate(`window.__missionPersist.warm()`)
  let warm = null
  try { warm = warmJson ? JSON.parse(warmJson) : null } catch { warm = null }

  const digestMatch = typeof coldDigest === 'string' && coldDigest.length > 0 && warmDigest === coldDigest
  const coldOk = ready0 && readyCold && coldLoaded === false && coldSlots === SEED_N && coldDocRt && typeof coldDigest === 'string' && coldDigest.length > 0
  const warmOk =
    readyWarm && warmLoaded === true && warmSlots === SEED_N && warmDocRt && digestMatch &&
    warm !== null && warm.missionId === MISSION_ID && warm.slotCount === SEED_N
  const pass = coldOk && warmOk && panics.length === 0

  console.log(JSON.stringify({
    gate: 'editor-persist-smoke', path,
    coldLoaded, coldSlots, coldDocRt,
    warmLoaded, warmSlots, warmDocRt,
    digestMatch, digestLen: (coldDigest || '').length, encodeHexLen,
    warm, coldOk, warmOk, panics: panics.slice(0, 2), pass,
  }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_persist_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
