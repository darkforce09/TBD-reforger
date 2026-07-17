// smoke_editor.mjs — T-159.15 Mission Creator editor smoke (build-green is the slice gate; this
// checks the wgpu canvas mounts + the engine renders + wheel-zoom changes the view, with panic
// capture). NOT a DOM-diff gate — the editor DOM is just the canvas until the Eden shell lands.
//
//   node .ai/artifacts/t159_gates/driver/smoke_editor.mjs [leptosDir] [path]
//
// Uses the shared launch() (already passes --enable-unsafe-webgpu + --use-angle=swiftshader). See
// t159_15_render_loop_handoff.md for the render-loop blocker (readback map_async needs device.poll
// on headless — the render loop must call an engine poll() hook).
import { launch, newPage, sleep } from './cdp.mjs'
import { startServer } from './serve.mjs'
import { createHash } from 'node:crypto'

const leptosDir = process.argv[2] || 'apps/website-leptos/dist'
const path = process.argv[3] || '/missions/smoke/edit'

const srv = await startServer({ dir: leptosDir, port: 5299 })
const b = await launch({ debugPort: 9359 })
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
  await sleep(1200) // create + several rAF frames

  const shot = async () => {
    const r = await page.send('Page.captureScreenshot', { format: 'png' })
    return createHash('sha256').update(r.data).digest('hex').slice(0, 16)
  }
  const before = await shot()
  await page.evaluate(
    `(()=>{const c=document.querySelector('div.relative')||document.body;c.dispatchEvent(new WheelEvent('wheel',{deltaY:-600,clientX:720,clientY:450,bubbles:true,cancelable:true}));return 1})()`,
  )
  await sleep(600)
  const after = await shot()
  const info = await page.evaluate(
    `(()=>{const c=document.querySelector('canvas');return JSON.stringify({w:c?.width||0,h:c?.height||0})})()`,
  )
  const pass = panics.length === 0 && before !== after
  console.log(JSON.stringify({ gate: 'editor-smoke', path, canvas: JSON.parse(info), viewChangedOnWheel: before !== after, panics: panics.slice(0, 2), pass }, null, 2))
  process.exitCode = pass ? 0 : 1
} catch (e) {
  console.error('smoke_editor error:', e.message)
  process.exitCode = 2
} finally {
  b.kill()
  srv.close()
}
