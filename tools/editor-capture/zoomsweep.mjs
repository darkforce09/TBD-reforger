// zoomsweep.mjs <outPrefix> <missionId> <zoom,zoom,...>
// Boots the editor, then for each zoom: __editorCamSet(tx,ty,z), settle, read the wgpu
// canvas via toDataURL. Used to inspect zoom-banded map layers (contours, height labels)
// without a human at the wheel.
//
// Canvas capture, not Page.captureScreenshot — headless chrome fails to present the
// vulkan surface and returns a black map over correct DOM. See ./README.md.
import { writeFileSync } from 'node:fs';
const PORT = process.env.CDP_PORT || 9222;
const [outPrefix, missionId, zoomCsv] = process.argv.slice(2);
const zooms = zoomCsv.split(',').map(Number);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (s) => process.stderr.write(s + '\n');

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const page = list.find((t) => t.type === 'page');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r, j) => { ws.addEventListener('open', r, {once:true}); ws.addEventListener('error', j, {once:true}); });

let id = 0;
const rpc = (method, params = {}, timeoutMs = 40000) => new Promise((resolve, reject) => {
  const msgId = ++id;
  const to = setTimeout(() => { ws.removeEventListener('message', onMsg); reject(new Error(`TIMEOUT ${method}`)); }, timeoutMs);
  const onMsg = (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id !== msgId) return;
    clearTimeout(to); ws.removeEventListener('message', onMsg);
    m.error ? reject(new Error(`${method}: ${JSON.stringify(m.error)}`)) : resolve(m.result);
  };
  ws.addEventListener('message', onMsg);
  ws.send(JSON.stringify({ id: msgId, method, params }));
});
const evalJs = async (expr) => {
  try { return (await rpc('Runtime.evaluate', { expression: expr, returnByValue: true }, 20000)).result?.value; }
  catch (e) { return `<eval failed: ${e.message}>`; }
};

const errs = [];
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') {
    errs.push((m.params.args || []).map((a) => a.value ?? a.description ?? '').join(' ').slice(0, 220));
  } else if (m.method === 'Runtime.exceptionThrown') {
    errs.push('EXC ' + (m.params.exceptionDetails?.text || '').slice(0, 220));
  }
});

await rpc('Page.enable'); await rpc('Runtime.enable');
await rpc('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });

await rpc('Page.navigate', { url: `http://localhost:8080/api/v1/auth/dev-login?role=admin` });
await sleep(6000);
await rpc('Page.navigate', { url: `http://localhost:3000/missions/${missionId}/edit` });
await sleep(15000);

// Boot overlay must clear or the canvas is not yet drawing the world.
const OVERLAY = '[class*="animate-overlay-fade"], [class*="z-50"][class*="backdrop-blur"]';
for (let i = 0; i < 60; i++) {
  const s = await evalJs(`document.querySelector('${OVERLAY}') ? 1 : 0`);
  if (s === 0) { log(`overlay cleared after ${i}s`); break; }
  await sleep(1000);
}

log(`cam api: ${await evalJs('typeof window.__editorCamSet')} / ${await evalJs('typeof window.__editorCam')}`);
log(`height layer on: ${await evalJs(`(localStorage.getItem('tbd-mc-world-layers')||'(default)')`)}`);

// Everon centre-ish; the peaks worth reading sit inland.
const [TX, TY] = [6400, 6400];
for (const z of zooms) {
  await evalJs(`window.__editorCamSet(${TX}, ${TY}, ${z})`);
  await sleep(3500);
  const cam = await evalJs('JSON.stringify(window.__editorCam())');
  const d = await evalJs(`document.querySelector('canvas').toDataURL('image/png')`);
  if (typeof d === 'string' && d.startsWith('data:image/png;base64,')) {
    const buf = Buffer.from(d.slice('data:image/png;base64,'.length), 'base64');
    const f = `${outPrefix}_z${String(z).replace('.', 'p').replace('-', 'm')}.png`;
    writeFileSync(f, buf);
    log(`z=${z}  ${buf.length} bytes  cam=${cam}  -> ${f}`);
  } else log(`z=${z}  CAPTURE FAILED: ${String(d).slice(0, 100)}`);
  if (errs.length) { log(`   errors since last: ${errs.splice(0).slice(0,2).join(' | ')}`); }
}
ws.close(); process.exit(0);
