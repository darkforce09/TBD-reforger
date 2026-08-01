// cdp2.mjs <out.png> <url> [waitMs] [url waitMs ...]
// Same as cdpshot but: dumps page diagnostics, and tries fromSurface:false when the
// surface capture hangs (a WebGPU canvas under swiftshader can stall the surface path).
import { writeFileSync } from 'node:fs';
const PORT = process.env.CDP_PORT || 9222;
const a = process.argv.slice(2);
const out = a.shift();
const steps = [];
for (let i = 0; i < a.length; i += 2) steps.push([a[i], Number(a[i + 1] || 3000)]);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const log = (s) => process.stderr.write(s + '\n');

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const page = list.find((t) => t.type === 'page');
if (!page) throw new Error('no page target');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r, j) => { ws.addEventListener('open', r, {once:true}); ws.addEventListener('error', j, {once:true}); });

let id = 0;
function rpc(method, params = {}, timeoutMs = 30000) {
  const msgId = ++id;
  return new Promise((resolve, reject) => {
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
}
const evalJs = async (expr) => {
  try {
    const r = await rpc('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: false }, 15000);
    return r.result?.value;
  } catch (e) { return `<eval failed: ${e.message}>`; }
};

const consoleLines = [];
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.method === 'Runtime.consoleAPICalled') {
    const txt = (m.params.args || []).map((a) => a.value ?? a.description ?? a.type).join(' ');
    consoleLines.push(`[${m.params.type}] ${txt}`);
  } else if (m.method === 'Log.entryAdded') {
    consoleLines.push(`[${m.params.entry.level}] ${m.params.entry.text}`);
  } else if (m.method === 'Runtime.exceptionThrown') {
    const d = m.params.exceptionDetails;
    consoleLines.push(`[EXCEPTION] ${d.text} ${d.exception?.description || ''}`);
  }
});

await rpc('Page.enable'); await rpc('Runtime.enable'); await rpc('Log.enable');
await rpc('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });

for (const [url, waitMs] of steps) {
  log(`→ ${url} (wait ${waitMs}ms)`);
  await rpc('Page.navigate', { url }, 30000);
  await sleep(waitMs);
  log(`  href      : ${await evalJs('location.href')}`);
}

// The editor boots behind a full-bleed loading overlay. Screenshotting before it clears
// captures the spinner, not the editor — poll it out rather than guessing a fixed wait.
const OVERLAY = '[class*="animate-overlay-fade"], [class*="z-50"][class*="backdrop-blur"]';
for (let i = 0; i < 25; i++) {
  const state = await evalJs(`(() => {
    const o = document.querySelector('${OVERLAY}');
    return o ? (o.innerText || '').replace(/\\s+/g, ' ').trim() : null;
  })()`);
  if (state === null) { log(`  overlay cleared after ${i}s`); break; }
  if (i % 10 === 0) log(`  [${i}s] overlay: ${state}`);
  if (i === 89) log(`  overlay STILL PRESENT after 90s: ${state}`);
  await sleep(1000);
}

log(`  readyState: ${await evalJs('document.readyState')}`);
log(`  title     : ${await evalJs('document.title')}`);
log(`  canvases  : ${await evalJs('JSON.stringify([...document.querySelectorAll("canvas")].map(c=>({w:c.width,h:c.height,cls:c.className})))')}`);
log(`  bodyTextLen: ${await evalJs('document.body.innerText.length')}`);
log(`  bodyHead  : ${JSON.stringify(await evalJs('document.body.innerText.slice(0,600)'))}`);
log(`  overlay   : ${await evalJs('JSON.stringify([...document.querySelectorAll("[class*=overlay],[class*=loading],[role=progressbar]")].slice(0,5).map(e=>e.className+" :: "+(e.innerText||"").slice(0,80)))')}`);

log('  ---- console (last 40) ----');
for (const l of consoleLines.slice(-40)) log('  ' + l.slice(0, 300));
log('  ---------------------------');

// If the boot overlay never cleared, take it out of the DOM so the shot shows the chrome
// underneath. The map may be blank — that is itself the finding — but the panels are readable.
if (process.env.FORCE_HIDE_OVERLAY === '1') {
  const removed = await evalJs(`(() => {
    const els = [...document.querySelectorAll('${OVERLAY}')];
    els.forEach(e => e.remove());
    return els.length;
  })()`);
  log(`  force-removed ${removed} overlay element(s)`);
  await sleep(1500);
}

async function shoot(params, label) {
  try {
    const r = await rpc('Page.captureScreenshot', params, 25000);
    writeFileSync(out, Buffer.from(r.data, 'base64'));
    log(`OK via ${label} → ${out}`);
    return true;
  } catch (e) { log(`FAIL ${label}: ${e.message}`); return false; }
}
// Read the WebGPU canvas directly. Headless chrome's compositor can fail to present the GPU
// layer ("Failed to initialize vulkan surface") while the engine itself renders fine — that
// path yields a black map, which is indistinguishable from a dead engine. toDataURL asks the
// canvas for its own pixels and sidesteps the compositor entirely.
if (process.env.CANVAS_CAPTURE === '1') {
  const dataUrl = await evalJs(`(() => {
    const c = document.querySelector('canvas');
    if (!c) return 'NO_CANVAS';
    try { return c.toDataURL('image/png'); } catch (e) { return 'ERR: ' + e.message; }
  })()`);
  if (typeof dataUrl === 'string' && dataUrl.startsWith('data:image/png;base64,')) {
    const buf = Buffer.from(dataUrl.slice('data:image/png;base64,'.length), 'base64');
    // An all-black canvas still encodes to a valid PNG, just a tiny one. Size is the tell.
    log(`  canvas toDataURL → ${buf.length} bytes`);
    if (buf.length > 20000) {
      writeFileSync(out.replace(/\.png$/, '_canvas.png'), buf);
      log(`  wrote ${out.replace(/\.png$/, '_canvas.png')}`);
    } else log('  canvas looks blank (too few bytes) — not written');
  } else log(`  canvas capture failed: ${String(dataUrl).slice(0, 120)}`);
}

// fromSurface:true FIRST — it is the only path that composites the WebGPU canvas. The
// fromSurface:false path renders DOM only and hands back a black map, which looks exactly
// like a broken engine when the engine is fine.
(await shoot({ format: 'png', fromSurface: true, captureBeyondViewport: false }, 'fromSurface:true'))
  || (await shoot({ format: 'png', captureBeyondViewport: false, fromSurface: false }, 'fromSurface:false'))
  || (await shoot({ format: 'jpeg', quality: 80, fromSurface: false }, 'jpeg/fromSurface:false'));
ws.close(); process.exit(0);
