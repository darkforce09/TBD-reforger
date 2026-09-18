//! T-165.6 — the editor CDP smokes + auxiliary browser gates (ports of the 19 Node driver
//! scripts that lived under the t159_gates driver/ dir until T-165.6 deleted it). One async fn
//! per script; the `gate` bin
//! exposes `gate smoke <name>`, the ordered `gate editor-suite` (the Makefile glob
//! replacement), `gate r-auth`, and `gate render-check`.
//!
//! Shared contract with the Node harness (per-script headers preserved on each fn):
//! - trusted CDP Input events (mouse moves carry button:none with the held bit in `buttons`;
//!   key chords are rawKeyDown+keyUp ONLY — the T-159.22.1 double-fire contract)
//! - panic capture over console/log/exception events (`/panic|unreachable|already mapped/i`)
//! - JSON verdict on stdout; exit 0 pass · 1 fail · 2 usage/scenario error (3 = driver error
//!   for r-auth/render-check, matching the Node scripts' exit maps)

use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};

use crate::browser_testing::cdp::{self, Browser, Page};
use crate::browser_testing::server::{RunningServer, ServeConfig, repo_root, start_server};
use crate::repository_layout::MapAssetMounts;

mod outliner_drag;

const DIST_DEFAULT: &str = "apps/website/frontend/dist";
/// Default editor path for the suite. `sat=preview` keeps smokes off the 152 MB full TBDS GET
/// (which freezes headless CDP mid-suite once `/map-assets` is live).
///
/// `force=webgl` (T-166): pin the software **WebGL2/SwiftShader** backend. The default
/// WebGPU/lavapipe path is unreliable headless under memory pressure — its rAF render loop
/// intermittently stalls the page main thread long enough that a `Runtime.evaluate` (e.g. the
/// `__editorSelection.probe()` centering call) never returns, wedging the suite. `arsenal` is
/// simply the first default-backend smoke after WebGL2 `selfcheck`, so it always died first.
/// These smokes exercise doc / UI / interaction, not the GPU backend (the GPU-byte gates —
/// `selfcheck`, `fullmap`, `hillshade` — already force WebGL2), so pinning it suite-wide is safe.
const EDIT_PATH: &str = "/missions/smoke/edit?force=webgl&sat=preview";
const SEED_N: i64 = 8; // must match mission_doc.rs `SEED_N`

/// The Makefile glob `driver/*_editor.mjs` in shell-sort order (selfcheck sorts first).
pub const EDITOR_SUITE: [&str; 21] = [
    "selfcheck",
    "arsenal",
    "attributes",
    "cur",
    "doc",
    "editor",
    "entrance-motion-rect",
    "fullmap",
    "hillshade",
    "hydrate",
    "keyboard-settings",
    "marquee-drag",
    "outliner-palette",
    "pan",
    "persist",
    "save-dialog-rect",
    "save-export",
    "select",
    "t946-86",
    "undo",
    "virtual-outliner",
];

/// Locked T-166 sat bundle size — full GET of this body must never happen under `?sat=preview`.
const SAT_FULL_BYTES: u64 = 152_713_114;

struct Harness {
    srv: RunningServer,
    browser: Browser,
    page: Arc<Page>,
    panics: Arc<StdMutex<Vec<String>>>,
}

impl Harness {
    /// serve + launch + page + viewport + panic capture — the shared prologue of every smoke.
    async fn new(
        dist: &str,
        port: u16,
        debug_port: u16,
        map_assets: Option<MapAssetMounts>,
        api_proxy: Option<String>,
        init_scripts: &[&str],
    ) -> Result<Harness> {
        // Font cache: `cdp::launch` pins `XDG_CACHE_HOME` on the chromium child (T-339 / T-362).
        // Process-wide `ensure_gate_font_cache` still runs from `gate` main (T-354) for doctor
        // inherit-path probes.
        let srv = start_server(
            ServeConfig {
                dir: PathBuf::from(dist),
                api_proxy,
                map_assets,
            },
            port,
        )
        .await?;
        let browser = cdp::launch(debug_port, &[]).await?;
        // T-843 — always seed mission_maker/admin before any smoke navigates the editor.
        let auth_seed = editor_auth_seed()?;
        let mut scripts: Vec<&str> = Vec::with_capacity(init_scripts.len() + 1);
        scripts.push(auth_seed.as_str());
        scripts.extend_from_slice(init_scripts);
        let page = Arc::new(cdp::new_page(&browser, None, &scripts).await?);
        page.send("Runtime.enable", json!({})).await?;
        page.send("Log.enable", json!({})).await?;
        page.send(
            "Emulation.setDeviceMetricsOverride",
            json!({ "width": 1440, "height": 900, "deviceScaleFactor": 1, "mobile": false }),
        )
        .await?;
        let panics = Arc::new(StdMutex::new(Vec::<String>::new()));
        attach_panic_capture(&page, &panics).await;
        Ok(Harness {
            srv,
            browser,
            page,
            panics,
        })
    }

    fn url(&self, path: &str) -> String {
        format!("http://localhost:{}{}", self.srv.port, path)
    }

    fn panics_head(&self) -> Vec<String> {
        self.panics
            .lock()
            .unwrap()
            .iter()
            .take(2)
            .cloned()
            .collect()
    }

    fn no_panics(&self) -> bool {
        self.panics.lock().unwrap().is_empty()
    }

    async fn shutdown(self) {
        // Reap chrome (SIGTERM → wait → SIGKILL) + drop its profile dir BEFORE the next smoke,
        // so the debug port + profile lock free deterministically (T-166 suite-hang fix).
        self.browser.shutdown().await;
        self.srv.close().await;
    }
}

/* ───────────────────────────── shared scenario helpers ───────────────────────────── */

const PERSIST_READY: &str = "typeof window.__missionPersist === 'object' && window.__missionPersist !== null && window.__missionPersist.ready() === true";
const SEL_READY: &str = "typeof window.__editorSelection === 'object' && window.__editorSelection !== null && typeof window.__editorCam === 'function'";
const DOC_READY: &str = "typeof window.__missionDoc === 'object' && window.__missionDoc !== null";
const HIST_READY: &str = "typeof window.__editorHistory === 'object' && window.__editorHistory !== null && typeof window.__editorHistory.can_undo === 'function'";

/* ───────────────────────────── the registry-golden Fetch tap ───────────────────────────── */

/* ───────────────────────────── the smokes ───────────────────────────── */

const ATTR_READY: &str = "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__editorCam === 'function' && typeof window.__missionPersist === 'object'";
const MODAL_OPEN: &str =
    "[...document.querySelectorAll('h2')].some(h => h.textContent === 'Attributes')";

const BACKEND: &str = "http://127.0.0.1:8080";

/// render-check.mjs — generic "does this built SPA render X" liveness check.
/// Exit map: 0 pass · 1 fail · 2 usage · 3 driver error (mapped by the bin).
pub struct RenderCheckArgs {
    pub dir: String,
    pub path: String,
    pub expect: String,
    pub assert_js: Option<String>,
    /// Inject the v-suite admin localStorage seed before boot (T-172 behavioral probes on
    /// auth-gated pages).
    pub seed_auth: bool,
    pub port: u16,
    pub debug_port: u16,
    /// Upstream for `/api` (T-339 / T-387). Without this, `--seed-auth` leaves a half-hydrated
    /// session (`user` from localStorage, `/me` fails → `access_token` None → signed-out UI).
    /// `None` defaults to `BACKEND` so probes reach a live API when one is on :8080.
    pub api_proxy: Option<String>,
    /// T-090.12.5 — full-viewport PNG written after `assert_js` settled (`None` = no shot). The
    /// screenshot is evidence only: it never affects the verdict.
    pub shot: Option<std::path::PathBuf>,
    /// T-090.12.5 — `/map-assets/` passthrough root (`None` = the SPA fallback answers assets).
    pub map_assets: Option<std::path::PathBuf>,
    /// T-090.12.5 — extra pre-boot inject (a JS file), evaluated on every new document after
    /// the freeze and the optional `--seed-auth` seed.
    pub inject_js: Option<std::path::PathBuf>,
    /// T-090.12.5 — leave the clock and RNG alone (wall-time measurements; never goldens).
    pub no_freeze: bool,
}

/// T-173 — in-page perf probe: rAF frame sampler + longtask observer + counter diffs around four
/// scripted gesture scenarios (pan / zoom / settle-thrash / bench). Synthetic DOM events (not CDP
/// `Input.*`): CDP round-trips can't sustain one-move-per-frame cadence, and the editor's
/// capture-phase container listeners accept untrusted events. Returns one JSON string.
const PERF_PROBE: &str = r#"(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const raf = () => new Promise(r => requestAnimationFrame(r));
  const canvas = document.querySelector('canvas');
  if (!canvas) return JSON.stringify({ error: 'no canvas' });
  const M = () => window.__mapAssets || {};
  const counters = () => ({
    icon: M().icon_lane_uploads || 0, poly: M().polygon_lane_uploads || 0,
    strip: M().strip_lane_uploads || 0, bld: M().building_uploads || 0,
    rev: M().buffers_revision || 0, glyphR: M().glyph_recomposes || 0, fillR: M().fill_recomposes || 0,
  });
  let frames = []; let sampling = true;
  (async () => { await raf(); let p = performance.now();
    while (sampling) { await raf(); const t = performance.now(); frames.push(t - p); p = t; } })();
  let longtaskMs = 0;
  try {
    new PerformanceObserver(l => l.getEntries().forEach(e => { longtaskMs += e.duration; }))
      .observe({ entryTypes: ['longtask'] });
  } catch (e) {}
  const statsOf = a => {
    const s = [...a].sort((x, y) => x - y);
    const avg = a.reduce((x, y) => x + y, 0) / Math.max(1, a.length);
    return { frames: a.length, avg_fps: +(1000 / Math.max(0.01, avg)).toFixed(1),
      p95_ms: +(s[Math.min(s.length - 1, Math.floor(s.length * 0.95))] || 0).toFixed(2),
      max_ms: +(s[s.length - 1] || 0).toFixed(2), hitches: a.filter(x => x > 33.4).length };
  };
  const seg = async fn => {
    frames = []; const c0 = counters(); const lt0 = longtaskMs;
    await fn();
    const f = frames.slice(); const c1 = counters();
    const d = {}; for (const k of Object.keys(c0)) d[k] = c1[k] - c0[k];
    return { ...statsOf(f), longtask_ms: +(longtaskMs - lt0).toFixed(1), counters: d };
  };
  const pev = (type, x, y, buttons, button) => canvas.dispatchEvent(new PointerEvent(type, {
    bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse', isPrimary: true,
    clientX: x, clientY: y, buttons, button }));
  const wev = (x, y, dy) => canvas.dispatchEvent(new WheelEvent('wheel', {
    bubbles: true, cancelable: true, clientX: x, clientY: y, deltaY: dy, deltaMode: 0 }));
  const r = canvas.getBoundingClientRect();
  const cx = r.left + r.width / 2, cy = r.top + r.height / 2;

  window.__editorCamSet(6400, 6400, 0.5); await sleep(1500);
  // warmup discard: sampler restarted by each seg()

  const pan = await seg(async () => {
    pev('pointerdown', cx, cy, 2, 2);
    let x = cx, y = cy; const t0 = performance.now();
    while (performance.now() - t0 < 6000) {
      const ph = ((performance.now() - t0) / 1000) * (Math.PI / 2);
      x = cx + 220 * Math.cos(ph); y = cy + 140 * Math.sin(ph);
      pev('pointermove', x, y, 2, -1);
      await raf();
    }
    pev('pointerup', x, y, 0, 2);
    await sleep(1300);
  });

  const zoom = await seg(async () => {
    for (let i = 0; i < 16; i++) { wev(cx, cy, -120); await sleep(150); }
    for (let i = 0; i < 16; i++) { wev(cx, cy, 120); await sleep(150); }
    await sleep(1300);
  });

  const urls = new Map(); let fetchTotal = 0;
  const origFetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const u = String(typeof input === 'string' ? input : (input && input.url) || '');
    if (u.includes('/objects/chunks/')) { fetchTotal++; urls.set(u, (urls.get(u) || 0) + 1); }
    return origFetch(input, init);
  };
  for (let i = 0; i < 12; i++) {
    const off = (i % 2) ? 800 : -800;
    window.__editorCamSet(6400 + off, 6400 + off, 1.5);
    await sleep(700);
  }
  const activeFetches = fetchTotal;
  await sleep(3500);
  const idleFetches = fetchTotal - activeFetches;
  window.fetch = origFetch;
  let dupes = 0; for (const n of urls.values()) if (n > 1) dupes += n - 1;

  const bench = {};
  if (typeof window.__editorBench === 'function') {
    const cams = { town: [4870, 7760, 2], forest: [6400, 6400, 0.5], mid: [6400, 6400, -1], max: [4870, 7760, 4] };
    for (const k of Object.keys(cams)) {
      const c = cams[k];
      window.__editorCamSet(c[0], c[1], c[2]); await sleep(1200);
      try { bench[k] = JSON.parse(await window.__editorBench(200)); }
      catch (e) { bench[k] = { error: String(e).slice(0, 120) }; }
    }
  }
  sampling = false; await raf();
  return JSON.stringify({
    pan, zoom,
    thrash: { chunk_fetches: activeFetches, duplicate_fetches: dupes, idle_fetches: idleFetches },
    bench, render_cpu_ms_ema: (M().render_cpu_ms_ema || 0),
  });
})()"#;

/// Pan-only rerun with dock `backdrop-filter` stripped — isolates the compositor blur cost
/// (same env, same gesture; the delta vs the main run's pan row is the blur bill).
const PERF_PROBE_NOBLUR_PAN: &str = r#"(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const raf = () => new Promise(r => requestAnimationFrame(r));
  const canvas = document.querySelector('canvas');
  if (!canvas) return JSON.stringify({ error: 'no canvas' });
  const st = document.createElement('style');
  st.textContent = '*{backdrop-filter:none !important;-webkit-backdrop-filter:none !important;}';
  document.head.appendChild(st);
  let frames = []; let sampling = true;
  (async () => { await raf(); let p = performance.now();
    while (sampling) { await raf(); const t = performance.now(); frames.push(t - p); p = t; } })();
  const pev = (type, x, y, buttons, button) => canvas.dispatchEvent(new PointerEvent(type, {
    bubbles: true, cancelable: true, pointerId: 1, pointerType: 'mouse', isPrimary: true,
    clientX: x, clientY: y, buttons, button }));
  const r = canvas.getBoundingClientRect();
  const cx = r.left + r.width / 2, cy = r.top + r.height / 2;
  window.__editorCamSet(6400, 6400, 0.5); await sleep(1200);
  frames = [];
  pev('pointerdown', cx, cy, 2, 2);
  let x = cx, y = cy; const t0 = performance.now();
  while (performance.now() - t0 < 6000) {
    const ph = ((performance.now() - t0) / 1000) * (Math.PI / 2);
    x = cx + 220 * Math.cos(ph); y = cy + 140 * Math.sin(ph);
    pev('pointermove', x, y, 2, -1);
    await raf();
  }
  pev('pointerup', x, y, 0, 2);
  await sleep(1300);
  sampling = false; await raf();
  const a = frames; const s = [...a].sort((x2, y2) => x2 - y2);
  const avg = a.reduce((x2, y2) => x2 + y2, 0) / Math.max(1, a.length);
  st.remove();
  return JSON.stringify({ frames: a.length, avg_fps: +(1000 / Math.max(0.01, avg)).toFixed(1),
    p95_ms: +(s[Math.min(s.length - 1, Math.floor(s.length * 0.95))] || 0).toFixed(2),
    max_ms: +(s[s.length - 1] || 0).toFixed(2), hitches: a.filter(x2 => x2 > 33.4).length });
})()"#;

#[cfg(test)]
#[path = "tests/editor_smoke_tests/assert_js_ok_tests.rs"]
mod assert_js_ok_tests;

#[path = "editor_smoke_tests/fullmap.rs"]
mod fullmap;
pub use fullmap::smoke_doc;
pub use fullmap::smoke_fullmap;
pub use fullmap::smoke_hillshade;

#[path = "editor_smoke_tests/pan.rs"]
mod pan;
pub use pan::smoke_pan;
pub use pan::smoke_persist;
pub use pan::smoke_save_export;
pub use pan::smoke_select;

#[path = "editor_smoke_tests/save_dialog_rect.rs"]
mod save_dialog_rect;
pub use save_dialog_rect::smoke_entrance_motion_rect;
pub use save_dialog_rect::smoke_save_dialog_rect;

#[path = "editor_smoke_tests/cur.rs"]
mod cur;
use cur::probe_hit;
pub use cur::smoke_attributes;
pub use cur::smoke_cur;
pub use cur::smoke_keyboard_settings;

#[path = "editor_smoke_tests/arsenal.rs"]
mod arsenal;
use arsenal::set_eq;
pub use arsenal::smoke_arsenal;

#[path = "editor_smoke_tests/marquee_drag.rs"]
mod marquee_drag;
pub use marquee_drag::smoke_marquee_drag;
pub use marquee_drag::smoke_undo;

#[path = "editor_smoke_tests/outliner_palette.rs"]
mod outliner_palette;
pub use outliner_palette::smoke_outliner_palette;

#[path = "editor_smoke_tests/virtual_outliner.rs"]
mod virtual_outliner;
pub use virtual_outliner::smoke_hydrate;
pub use virtual_outliner::smoke_virtual_outliner;

#[path = "editor_smoke_tests/mutations.rs"]
mod mutations;
pub use mutations::r_auth;
pub use mutations::render_check;
pub use mutations::smoke_mutations;
pub use mutations::smoke_perf;

#[path = "editor_smoke_tests/run_smoke.rs"]
mod run_smoke;
pub use run_smoke::editor_suite;
pub use run_smoke::run_smoke;

#[cfg(test)]
pub(crate) use mutations::assert_js_ok;

mod browser_fixture_helpers;
use browser_fixture_helpers::editor_auth_seed;
use browser_fixture_helpers::serve_arsenal_golden;
use browser_fixture_helpers::serve_registry_golden;

mod browser_input_helpers;
use browser_input_helpers::click_at;
use browser_input_helpers::click_selector;
use browser_input_helpers::dbl_click;
use browser_input_helpers::drag;
use browser_input_helpers::key_chord;
use browser_input_helpers::mouse;

mod browser_assertion_helpers;
use browser_assertion_helpers::attach_panic_capture;
use browser_assertion_helpers::checks_pass;
use browser_assertion_helpers::eval;
use browser_assertion_helpers::eval_bool;
use browser_assertion_helpers::eval_i64;
use browser_assertion_helpers::eval_str;
use browser_assertion_helpers::force_webgl;
use browser_assertion_helpers::print_verdict;
use browser_assertion_helpers::rect_of;
use browser_assertion_helpers::to_code;

mod editor_boot_scenarios;
pub use editor_boot_scenarios::smoke_editor;
pub use editor_boot_scenarios::smoke_selfcheck;
