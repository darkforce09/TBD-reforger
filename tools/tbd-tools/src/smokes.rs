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

use crate::cdp::{self, Browser, Page};
use crate::serve::{RunningServer, ServeConfig, repo_root, start_server};

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
pub const EDITOR_SUITE: [&str; 18] = [
    "selfcheck",
    "arsenal",
    "attributes",
    "cur",
    "doc",
    "editor",
    "fullmap",
    "hillshade",
    "hydrate",
    "keyboard-settings",
    "marquee-drag",
    "outliner-palette",
    "pan",
    "persist",
    "save-export",
    "select",
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
        map_assets_dir: Option<PathBuf>,
        api_proxy: Option<String>,
        init_scripts: &[&str],
    ) -> Result<Harness> {
        let srv = start_server(
            ServeConfig {
                dir: PathBuf::from(dist),
                api_proxy,
                map_assets_dir,
            },
            port,
        )
        .await?;
        let browser = cdp::launch(debug_port, &[]).await?;
        let page = Arc::new(cdp::new_page(&browser, None, init_scripts).await?);
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

/// The three panic-capture event taps every smoke installs (`grab` in the Node scripts).
async fn attach_panic_capture(page: &Arc<Page>, panics: &Arc<StdMutex<Vec<String>>>) {
    let re = regex::Regex::new("(?i)panic|unreachable|already mapped").unwrap();
    let grab = {
        let panics = Arc::clone(panics);
        move |t: String| {
            if re.is_match(&t) {
                panics.lock().unwrap().push(t.chars().take(300).collect());
            }
        }
    };
    // JS truthiness of `a.value || a.description || ''` per console arg.
    fn arg_text(a: &Value) -> String {
        match &a["value"] {
            Value::String(s) if !s.is_empty() => s.clone(),
            Value::Number(n) if n.as_f64() != Some(0.0) => n.to_string(),
            Value::Bool(true) => "true".to_string(),
            _ => a["description"].as_str().unwrap_or("").to_string(),
        }
    }
    let mut console = page.on_event("Runtime.consoleAPICalled").await;
    let mut log = page.on_event("Log.entryAdded").await;
    let mut exc = page.on_event("Runtime.exceptionThrown").await;
    let g1 = grab.clone();
    tokio::spawn(async move {
        while let Some(e) = console.recv().await {
            let joined = e["args"]
                .as_array()
                .map(|a| a.iter().map(arg_text).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            g1(joined);
        }
    });
    let g2 = grab.clone();
    tokio::spawn(async move {
        while let Some(e) = log.recv().await {
            g2(e["entry"]["text"].as_str().unwrap_or("").to_string());
        }
    });
    let g3 = grab;
    tokio::spawn(async move {
        while let Some(e) = exc.recv().await {
            g3(e["exceptionDetails"]["exception"]["description"]
                .as_str()
                .unwrap_or("")
                .to_string());
        }
    });
}

/* ───────────────────────────── shared scenario helpers ───────────────────────────── */

async fn mouse(page: &Page, ev: &str, x: f64, y: f64, extra: Value) -> Result<()> {
    let mut params = json!({ "type": ev, "x": x, "y": y });
    if let (Value::Object(b), Value::Object(e)) = (&mut params, extra) {
        for (k, v) in e {
            b.insert(k, v);
        }
    }
    page.send("Input.dispatchMouseEvent", params).await?;
    Ok(())
}

/// Trusted LMB drag with intermediate `button:none` moves (the pan/marquee smoke shape).
async fn drag(page: &Page, x0: f64, y0: f64, x1: f64, y1: f64) -> Result<()> {
    mouse(
        page,
        "mousePressed",
        x0,
        y0,
        json!({ "button": "left", "buttons": 1, "clickCount": 1 }),
    )
    .await?;
    let steps = 6.0;
    for i in 1..=6 {
        let f = f64::from(i) / steps;
        mouse(
            page,
            "mouseMoved",
            x0 + (x1 - x0) * f,
            y0 + (y1 - y0) * f,
            json!({ "button": "none", "buttons": 1 }),
        )
        .await?;
    }
    mouse(
        page,
        "mouseReleased",
        x1,
        y1,
        json!({ "button": "left", "buttons": 0, "clickCount": 1 }),
    )
    .await?;
    Ok(())
}

async fn click_at(page: &Page, x: f64, y: f64, ctrl: bool) -> Result<()> {
    let m = if ctrl {
        json!({ "modifiers": 2 })
    } else {
        json!({})
    };
    let mut down = json!({ "button": "left", "buttons": 1, "clickCount": 1 });
    let mut up = json!({ "button": "left", "buttons": 0, "clickCount": 1 });
    for v in [&mut down, &mut up] {
        if let (Value::Object(b), Some(e)) = (v, m.as_object()) {
            for (k, vv) in e {
                b.insert(k.clone(), vv.clone());
            }
        }
    }
    mouse(page, "mousePressed", x, y, down).await?;
    mouse(page, "mouseReleased", x, y, up).await?;
    Ok(())
}

/// One trusted key chord: rawKeyDown + keyUp ONLY (T-159.22.1 — keyDown would double-fire).
async fn key_chord(page: &Page, key: &str, code: &str, modifiers: u32, vk: u32) -> Result<()> {
    for ev in ["rawKeyDown", "keyUp"] {
        page.send(
            "Input.dispatchKeyEvent",
            json!({ "type": ev, "key": key, "code": code, "modifiers": modifiers,
                    "windowsVirtualKeyCode": vk, "nativeVirtualKeyCode": vk }),
        )
        .await?;
    }
    Ok(())
}

async fn eval(page: &Page, expr: &str) -> Result<Value> {
    page.evaluate(expr, false).await
}

async fn eval_i64(page: &Page, expr: &str) -> Result<i64> {
    Ok(eval(page, expr).await?.as_i64().unwrap_or(-1))
}

async fn eval_str(page: &Page, expr: &str) -> Result<String> {
    Ok(eval(page, expr)
        .await?
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn eval_bool(page: &Page, expr: &str) -> Result<bool> {
    Ok(eval(page, expr).await?.as_bool() == Some(true))
}

/// `document.querySelector(sel)` centre rect, or None.
async fn rect_of(page: &Page, sel: &str) -> Result<Option<(f64, f64)>> {
    let expr = format!(
        "(() => {{ const e = document.querySelector({sel:?});
      if (!e) return 'null'; const b = e.getBoundingClientRect();
      return JSON.stringify([b.left + b.width / 2, b.top + b.height / 2]); }})()"
    );
    let raw = eval_str(page, &expr).await?;
    let v: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    Ok(v.as_array()
        .and_then(|a| Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))))
}

async fn click_selector(page: &Page, sel: &str) -> Result<bool> {
    match rect_of(page, sel).await? {
        Some((x, y)) => {
            click_at(page, x, y, false).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

fn checks_pass(checks: &Map<String, Value>, expected: usize) -> bool {
    checks.len() == expected && checks.values().all(|v| *v == json!(true))
}

fn print_verdict(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn to_code(pass: bool) -> u8 {
    u8::from(!pass)
}

const PERSIST_READY: &str = "typeof window.__missionPersist === 'object' && window.__missionPersist !== null && window.__missionPersist.ready() === true";
const SEL_READY: &str = "typeof window.__editorSelection === 'object' && window.__editorSelection !== null && typeof window.__editorCam === 'function'";
const DOC_READY: &str = "typeof window.__missionDoc === 'object' && window.__missionDoc !== null";
const HIST_READY: &str = "typeof window.__editorHistory === 'object' && window.__editorHistory !== null && typeof window.__editorHistory.can_undo === 'function'";

fn force_webgl(path: &str) -> String {
    // Idempotent — EDIT_PATH already pins `force=webgl` (T-166), so callers that wrap it must not
    // double-append.
    if path.contains("force=webgl") {
        return path.to_string();
    }
    format!(
        "{path}{}force=webgl",
        if path.contains('?') { '&' } else { '?' }
    )
}

/* ───────────────────────────── the registry-golden Fetch tap ───────────────────────────── */

/// The gate_r_auth.mjs interception pattern used by the arsenal + outliner smokes: /registry →
/// the committed golden; other /api/v1/ → 401 {}; everything else continues. Returns a counter.
async fn serve_registry_golden(page: &Arc<Page>) -> Result<Arc<StdMutex<u64>>> {
    let golden = std::fs::read_to_string(
        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__registry.json"),
    )?;
    let golden: Value = serde_json::from_str(&golden)?;
    let hits = Arc::new(StdMutex::new(0u64));
    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let rp = Arc::clone(page);
    let hits_task = Arc::clone(&hits);
    tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let u = p["request"]["url"].as_str().unwrap_or_default();
            let res = if u.contains("/api/v1/registry") {
                *hits_task.lock().unwrap() += 1;
                rp.fulfill_json(request_id, 200, &golden).await
            } else if u.contains("/api/v1/") {
                rp.fulfill_json(request_id, 401, &json!({})).await
            } else {
                rp.continue_request(request_id).await
            };
            let _ = res;
        }
    });
    Ok(hits)
}

/// T-167 Smart-Arsenal tap: the committed registry golden **augmented** with a compat optic +
/// magazine (kind + weight), plus `/registry/compat` edges linking the golden's M16A2 to them and
/// `/factions` from its golden. Inline JSON so the r_api-pinned `GET__registry.json` stays byte-exact.
/// `(registry_hits, compat_hits, faction_post_hits)`.
async fn serve_arsenal_golden(
    page: &Arc<Page>,
    m16a2: &str,
) -> Result<(Arc<StdMutex<u64>>, Arc<StdMutex<u64>>, Arc<StdMutex<u64>>)> {
    const OPTIC: &str = "{ARSENAL_OPTIC}Prefabs/Weapons/Attachments/Optic_ACOG.et";
    const MAG: &str = "{ARSENAL_MAG}Prefabs/Weapons/Magazines/Mag_STANAG_30.et";
    let root = repo_root();
    let mut registry: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("apps/website/frontend/tests/fixtures/api/GET__registry.json"),
    )?)?;
    let mp = registry["modpack_id"].clone();
    let mk = |rn: &str, name: &str, kind: &str, wkg: f64| {
        json!({
            "id": format!("arsenal-{kind}"), "modpack_id": mp.clone(),
            "resource_name": rn, "display_name": name, "category": "Weapons/Attachments",
            "kind": kind, "weight_kg": wkg, "sort_order": 900,
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
        })
    };
    if let Some(arr) = registry["data"].as_array_mut() {
        arr.push(mk(OPTIC, "ACOG", "gear_optic", 0.6));
        arr.push(mk(MAG, "STANAG 30rd", "gear_magazine", 0.45));
    }
    let compat = json!({
        "data": [
            { "id": "e1", "modpack_id": mp.clone(), "from_node": m16a2, "to_node": OPTIC,
              "edge_type": "optic_on_weapon", "evidence": "",
              "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
            { "id": "e2", "modpack_id": mp.clone(), "from_node": m16a2, "to_node": MAG,
              "edge_type": "mag_in_weapon", "evidence": "",
              "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
        ],
        "etag": "W/\"arsenal-compat\"", "modpack_id": mp, "modpack_version": "test",
    });
    let factions: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("apps/website/frontend/tests/fixtures/api/GET__factions.json"),
    )?)?;

    let reg_hits = Arc::new(StdMutex::new(0u64));
    let compat_hits = Arc::new(StdMutex::new(0u64));
    let post_hits = Arc::new(StdMutex::new(0u64));
    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let rp = Arc::clone(page);
    let (rh, ch, ph) = (
        Arc::clone(&reg_hits),
        Arc::clone(&compat_hits),
        Arc::clone(&post_hits),
    );
    tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let u = p["request"]["url"].as_str().unwrap_or_default();
            let method = p["request"]["method"].as_str().unwrap_or("GET");
            let res = if u.contains("/api/v1/registry/compat") {
                *ch.lock().unwrap() += 1;
                rp.fulfill_json(request_id, 200, &compat).await
            } else if u.contains("/api/v1/registry") {
                *rh.lock().unwrap() += 1;
                rp.fulfill_json(request_id, 200, &registry).await
            } else if u.contains("/api/v1/factions") {
                if method == "POST" || method == "PUT" {
                    *ph.lock().unwrap() += 1;
                    rp.fulfill_json(request_id, 201, &json!({
                        "id": "fnew", "owner_id": "smoke", "side": "BLUFOR", "name": "Smoke Bn",
                        "doc": { "side": "BLUFOR", "name": "Smoke Bn", "roles": [], "vehicles": [] },
                        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
                    })).await
                } else {
                    rp.fulfill_json(request_id, 200, &factions).await
                }
            } else if u.contains("/api/v1/") {
                rp.fulfill_json(request_id, 401, &json!({})).await
            } else {
                rp.continue_request(request_id).await
            };
            let _ = res;
        }
    });
    Ok((reg_hits, compat_hits, post_hits))
}

/* ───────────────────────────── the smokes ───────────────────────────── */

/// smoke_editor.mjs — T-159.15: canvas mounts + engine renders + wheel-zoom changes the view.
pub async fn smoke_editor(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5299, 9359, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        // Wait for engine + `__editorCam` (T-166 host bootstrap can outlast a fixed 1.2s sleep).
        h.page
            .wait_for("typeof window.__editorCam==='function'", 80, 250)
            .await?;
        cdp::sleep_ms(400).await;

        // `__editorCam()` returns a JSON *string* `{"tx","ty","z","backend"}` (not an object).
        let cam_z = "(()=>{try{const raw=window.__editorCam&&window.__editorCam();const c=typeof raw==='string'?JSON.parse(raw):raw;return String(c&&c.z!=null?c.z:NaN)}catch(e){return 'NaN'}})()";
        let z_before: f64 = eval_str(&h.page, cam_z).await?.parse().unwrap_or(f64::NAN);
        // Dispatch on the canvas (gesture host), not the first `div.relative` (chrome shell) —
        // a target outside the capture listener never reaches `zoom_at`.
        eval(
            &h.page,
            "(()=>{const c=document.querySelector('canvas');if(!c)return 0;const r=c.getBoundingClientRect();c.dispatchEvent(new WheelEvent('wheel',{deltaY:-600,clientX:r.left+r.width/2,clientY:r.top+r.height/2,bubbles:true,cancelable:true}));return 1})()",
        )
        .await?;
        cdp::sleep_ms(400).await;
        let z_after: f64 = eval_str(&h.page, cam_z).await?.parse().unwrap_or(f64::NAN);
        let info = eval_str(
            &h.page,
            "(()=>{const c=document.querySelector('canvas');return JSON.stringify({w:c?.width||0,h:c?.height||0})})()",
        )
        .await?;
        let canvas: Value = serde_json::from_str(&info).unwrap_or(json!({}));
        let changed =
            z_before.is_finite() && z_after.is_finite() && (z_after - z_before).abs() > 1e-6;
        let pass = h.no_panics() && changed;
        print_verdict(&json!({
            "gate": "editor-smoke", "path": path, "canvas": canvas,
            "zoomBefore": z_before, "zoomAfter": z_after,
            "viewChangedOnWheel": changed, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// selfcheck_editor.mjs — T-159.15.1: byte-exact GPU readback self-checks (?force=webgl).
pub async fn smoke_selfcheck(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5300, 9360, None, None, &[]).await?;
    let run = async {
        let nav = force_webgl(path);
        h.page.navigate(&h.url(&nav)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for(
                "!!(window.__selfChecks && window.__selfChecks.calibration)",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let mut backend = "unknown".to_string();
        let mut all_pass = ready;
        if !ready {
            eprintln!("selfcheck_editor: window.__selfChecks never appeared");
        } else {
            for name in ["calibration", "texture"] {
                match h
                    .page
                    .evaluate(&format!("window.__selfChecks[{name:?}]()"), true)
                    .await
                {
                    Ok(raw) => {
                        let parsed: Value = serde_json::from_str(raw.as_str().unwrap_or_default())
                            .unwrap_or(Value::Null);
                        let ok = parsed["pass"] == json!(true);
                        checks.insert(
                            name.to_string(),
                            json!({ "pass": ok, "backend": parsed["backend"] }),
                        );
                        if let Some(b) = parsed["backend"].as_str() {
                            backend = b.to_string();
                        }
                        all_pass = all_pass && ok;
                    }
                    Err(err) => {
                        checks.insert(
                            name.to_string(),
                            json!({ "pass": false, "error": err.to_string() }),
                        );
                        all_pass = false;
                    }
                }
            }
        }
        let pass = all_pass && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-selfcheck", "path": path, "backend": backend, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// T-166 — full W1–W5 host wiring Class-R matrix (`?force=webgl&sat=preview`).
pub async fn smoke_fullmap(dist: &str, map_assets: &str) -> Result<u8> {
    let path = "/missions/smoke/edit?force=webgl&sat=preview";
    let h = Harness::new(dist, 5318, 9378, Some(PathBuf::from(map_assets)), None, &[]).await?;
    let run = async {
        // Track whether any Network response delivered the full sat body (A_sat_bytes).
        h.page.send("Network.enable", json!({})).await?;
        let mut net = h.page.on_event("Network.responseReceived").await;
        let sat_full_hits = Arc::new(StdMutex::new(0u32));
        let hits = Arc::clone(&sat_full_hits);
        tokio::spawn(async move {
            while let Some(e) = net.recv().await {
                let url = e["response"]["url"].as_str().unwrap_or("");
                if !url.contains(".tbd-sat") {
                    continue;
                }
                let len = e["response"]["headers"]
                    .as_object()
                    .and_then(|hdrs| {
                        hdrs.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    })
                    .and_then(|(_, v)| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| e["response"]["encodedDataLength"].as_u64())
                    .unwrap_or(0);
                if len == SAT_FULL_BYTES {
                    *hits.lock().unwrap() += 1;
                }
            }
        });

        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 160, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            // Wait for host bootstrap + residency drain (hillshade first, then world pins).
            let bridge = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.hillshadeW > 0 && window.__mapAssets.road_segments === 888 && window.__mapAssets.landcover_polygons === 36 && window.__mapAssets.sea_polygons > 0 && window.__mapAssets.contour_segments > 0 && window.__mapAssets.world_building_instances > 0 && window.__mapAssets.world_chunks_drawn > 0 && window.__mapAssets.forest_polygons > 0 && (window.__mapAssets.atlas_bytes > 0 || window.__mapAssets.glyphAtlas === true) && window.__mapAssets.tree_glyphs === 0",
                    400,
                    250,
                )
                .await?;
            checks.insert("bridgeSettled".into(), json!(bridge));

            let a_hs = eval_bool(
                &h.page,
                "!!window.__mapAssets && window.__mapAssets.hillshadeW > 0 && window.__mapAssets.hillshadeH > 0",
            )
            .await?;
            let a_sat = eval_bool(
                &h.page,
                "!!window.__mapAssets && window.__mapAssets.satW > 0 && window.__mapAssets.satH > 0 && window.__mapAssets.satMode === 'single'",
            )
            .await?;
            let a_roads = eval_bool(&h.page, "window.__mapAssets.road_segments === 888").await?;
            let a_lc = eval_bool(&h.page, "window.__mapAssets.landcover_polygons === 36").await?;
            let a_sea = eval_bool(&h.page, "window.__mapAssets.sea_polygons > 0").await?;
            let a_cont = eval_bool(&h.page, "window.__mapAssets.contour_segments > 0").await?;
            let a_bld = eval_bool(
                &h.page,
                "window.__mapAssets.world_building_instances > 0 && window.__mapAssets.world_chunks_drawn > 0",
            )
            .await?;
            let a_forest = eval_bool(&h.page, "window.__mapAssets.forest_polygons > 0").await?;
            let a_atlas = eval_bool(
                &h.page,
                "window.__mapAssets.atlas_bytes > 0 || window.__mapAssets.glyphAtlas === true",
            )
            .await?;
            let a_trees_off = eval_bool(&h.page, "window.__mapAssets.tree_glyphs === 0").await?;

            checks.insert("A_hs".into(), json!(a_hs));
            checks.insert("A_sat".into(), json!(a_sat));
            checks.insert("A_roads".into(), json!(a_roads));
            checks.insert("A_lc".into(), json!(a_lc));
            checks.insert("A_sea".into(), json!(a_sea));
            checks.insert("A_cont".into(), json!(a_cont));
            checks.insert("A_bld".into(), json!(a_bld));
            checks.insert("A_forest".into(), json!(a_forest));
            checks.insert("A_atlas".into(), json!(a_atlas));
            checks.insert("A_trees_off".into(), json!(a_trees_off));

            // Zoom probe ≥ 0 → tree glyphs on. Use z=2 (not 0): at island-center z=0 the
            // exact-count heatmap rung (INSTANCE_BUDGET) clears tree glyphs by design — smoke would
            // false-red on a correct LOD ladder. z=2 shrinks the draw-set under budget.
            let set_ok = eval_bool(
                &h.page,
                "typeof window.__editorCamSet === 'function' && (window.__editorCamSet(6400, 6400, 2), true)",
            )
            .await?;
            checks.insert("A_trees_probe".into(), json!(set_ok));
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let a_trees_on = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.tree_glyphs > 0",
                    160,
                    250,
                )
                .await?;
            checks.insert("A_trees_on".into(), json!(a_trees_on));
            let sat_hits = *sat_full_hits.lock().unwrap();
            checks.insert("A_sat_bytes".into(), json!(sat_hits == 0));
            checks.insert("A_panic".into(), json!(h.no_panics()));

            let pass = ready
                && h.no_panics()
                && checks.values().all(|v| *v == json!(true))
                && checks.len() >= 12;
            print_verdict(&json!({
                "gate": "editor-fullmap-smoke",
                "path": path,
                "pins": {
                    "sat_full_bytes": SAT_FULL_BYTES,
                    "roads": 888,
                    "landcover": 36,
                    "default_zoom": -2.0,
                    "tree_glyph_min_zoom": 0.0,
                },
                "checks": checks,
                "panics": h.panics_head(),
                "pass": pass,
            }));
            return Ok::<u8, anyhow::Error>(to_code(pass));
        }

        let pass = false;
        print_verdict(&json!({
            "gate": "editor-fullmap-smoke",
            "path": path,
            "pins": {
                "sat_full_bytes": SAT_FULL_BYTES,
                "roads": 888,
                "landcover": 36,
                "default_zoom": -2.0,
                "tree_glyph_min_zoom": 0.0,
            },
            "checks": checks,
            "panics": h.panics_head(),
            "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_hillshade_editor.mjs — T-159.28: DEM fetched + Rust-decoded + hillshade uploaded.
pub async fn smoke_hillshade(dist: &str, map_assets: &str) -> Result<u8> {
    let path = "/missions/smoke/edit?force=webgl&sat=preview";
    let h = Harness::new(dist, 5317, 9377, Some(PathBuf::from(map_assets)), None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 160, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            let uploaded = h.page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.hillshadeH > 0 && window.__mapAssets.hillshadeW > 0",
                    200,
                    250,
                )
                .await?;
            checks.insert("hillshadeUploaded".into(), json!(uploaded));
            if uploaded {
                let dims = eval_str(&h.page, "JSON.stringify([window.__mapAssets.hillshadeW, window.__mapAssets.hillshadeH])").await?;
                let dims: Vec<i64> = serde_json::from_str(&dims).unwrap_or_default();
                checks.insert("dimsPositive".into(), json!(dims.iter().all(|d| *d > 0)));
            }
            checks.insert(
                "laneDrawn".into(),
                json!(checks["hillshadeUploaded"] == json!(true)),
            );
        }
        let pass = ready
            && h.no_panics()
            && checks.values().all(|v| *v == json!(true))
            && checks.len() >= 2;
        print_verdict(&json!({
            "gate": "editor-hillshade-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_doc_editor.mjs — T-159.16: hosted MissionDocCore live + seeded + round-trips.
pub async fn smoke_doc(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5302, 9362, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(DOC_READY, 120, 250).await?;

        let (mut slot_count, mut roundtrip_ok, mut encode_stable) = (-1i64, false, false);
        let (mut hex_len, mut hex_head) = (0usize, String::new());
        if ready {
            slot_count = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            roundtrip_ok = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
            let h1 = eval_str(&h.page, "window.__missionDoc.encode_hex()").await?;
            let h2 = eval_str(&h.page, "window.__missionDoc.encode_hex()").await?;
            encode_stable = !h1.is_empty() && h1 == h2;
            hex_len = h1.len();
            hex_head = h1.chars().take(48).collect();
        } else {
            eprintln!("smoke_doc_editor: window.__missionDoc never appeared");
        }
        // T-172 B4 — the slot glyph lane must be live: atlas uploaded at mount and the seeded
        // SoA bound (the pre-T-172 editor never called ensure_slot_atlas → invisible slots).
        let slot_stats: Value = {
            let engine_up = h
                .page
                .wait_for("typeof window.__wgpuSlotStats === 'function'", 120, 250)
                .await?;
            if engine_up {
                let raw = eval_str(&h.page, "window.__wgpuSlotStats()").await?;
                serde_json::from_str(&raw).unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        };
        let atlas_ready = slot_stats["atlas_ready"].as_bool() == Some(true);
        let lane_bound = slot_stats["slot_len"].as_i64() == Some(SEED_N);
        let seeded = slot_count == SEED_N;
        let pass = ready
            && h.no_panics()
            && seeded
            && roundtrip_ok
            && encode_stable
            && atlas_ready
            && lane_bound;
        print_verdict(&json!({
            "gate": "editor-doc-smoke", "path": path,
            "slotCount": slot_count, "seeded": seeded, "roundtripOk": roundtrip_ok,
            "encodeStable": encode_stable, "encodeHexLen": hex_len, "encodeHexHead": hex_head,
            "slotAtlasReady": atlas_ready, "slotLaneBound": lane_bound,
            "slotStats": slot_stats,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_pan_editor.mjs — T-159.15.2: RMB pan + mid-pan wheel rebase via __editorCam.
pub async fn smoke_pan(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5301, 9361, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 120, 250)
            .await?;

        let cam = || async {
            let raw = eval_str(&h.page, "window.__editorCam()").await?;
            Ok::<Value, anyhow::Error>(serde_json::from_str(&raw).unwrap_or(json!({})))
        };
        let rmb = json!({ "button": "right", "buttons": 2, "clickCount": 1 });
        let held = json!({ "button": "none", "buttons": 2 });

        let zero = json!({ "tx": 0, "ty": 0, "z": 0, "backend": "unknown" });
        let (mut cam0, mut cam1) = (zero.clone(), zero.clone());
        let (mut cam_b1, mut cam_b2, mut cam_b3) = (zero.clone(), zero.clone(), zero.clone());
        let (mut pan_moved, mut zoom_changed, mut pan_continued) = (false, false, false);
        let f = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0);

        if ready {
            cam0 = cam().await?;
            // Test A: RMB drag left → target moves east.
            mouse(&h.page, "mousePressed", 720.0, 450.0, rmb.clone()).await?;
            mouse(&h.page, "mouseMoved", 620.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseMoved", 520.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseReleased", 520.0, 450.0, rmb.clone()).await?;
            cam1 = cam().await?;
            pan_moved = (f(&cam1, "tx") - f(&cam0, "tx")).abs() > 1e-6;

            // Test B: mid-pan wheel rebase — pan continues after a mid-drag zoom, no re-press.
            mouse(&h.page, "mousePressed", 720.0, 450.0, rmb.clone()).await?;
            mouse(&h.page, "mouseMoved", 680.0, 450.0, held.clone()).await?;
            cam_b1 = cam().await?;
            mouse(
                &h.page,
                "mouseWheel",
                680.0,
                450.0,
                json!({ "deltaX": 0, "deltaY": -600 }),
            )
            .await?;
            cam_b2 = cam().await?;
            mouse(&h.page, "mouseMoved", 620.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseReleased", 620.0, 450.0, rmb).await?;
            cam_b3 = cam().await?;
            zoom_changed = (f(&cam_b2, "z") - f(&cam_b1, "z")).abs() > 1e-6;
            pan_continued = (f(&cam_b3, "tx") - f(&cam_b2, "tx")).abs() > 1e-6;
        } else {
            eprintln!("smoke_pan_editor: window.__editorCam never appeared");
        }
        let pass = ready && h.no_panics() && pan_moved && zoom_changed && pan_continued;
        print_verdict(&json!({
            "gate": "editor-pan-smoke", "path": path, "backend": cam0["backend"],
            "cam0": cam0, "cam1": cam1, "camB1": cam_b1, "camB2": cam_b2, "camB3": cam_b3,
            "panMoved": pan_moved, "zoomChanged": zoom_changed, "panContinued": pan_continued,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_persist_editor.mjs — T-159.17: IDB persist across reload (COLD seed → WARM restore).
pub async fn smoke_persist(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5303, 9363, None, None, &[]).await?;
    let run = async {
        let url = h.url(path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                let ready = page.wait_for(&ready_expr, 200, 250).await?;
                page.wait_for(DOC_READY, 120, 250).await?;
                Ok::<bool, anyhow::Error>(ready)
            }
        };

        // boot 0: reach a live editor, then hard-reset for a deterministic COLD start.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;

        // boot 1 (COLD): no blob → seed.
        let ready_cold = boot_to(PERSIST_READY.to_string()).await?;
        let cold_loaded = eval(&h.page, "window.__missionPersist.loaded_from_storage()").await?;
        let cold_slots = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
        let cold_doc_rt = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
        let cold_digest = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
        let encode_hex_len = eval_str(&h.page, "window.__missionDoc.encode_hex()")
            .await?
            .len();
        h.page
            .evaluate("window.__missionPersist.flush()", true)
            .await?;

        // boot 2 (WARM): blob present → SWAP restore.
        let ready_warm = boot_to(format!(
            "{PERSIST_READY} && window.__missionPersist.loaded_from_storage() === true"
        ))
        .await?;
        let warm_loaded = eval(&h.page, "window.__missionPersist.loaded_from_storage()").await?;
        let warm_slots = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
        let warm_doc_rt = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
        let warm_digest = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
        let warm_json = eval_str(&h.page, "window.__missionPersist.warm()").await?;
        let warm: Value = serde_json::from_str(&warm_json).unwrap_or(Value::Null);

        let digest_match = !cold_digest.is_empty() && warm_digest == cold_digest;
        let cold_ok = ready0
            && ready_cold
            && cold_loaded == json!(false)
            && cold_slots == SEED_N
            && cold_doc_rt
            && !cold_digest.is_empty();
        let warm_ok = ready_warm
            && warm_loaded == json!(true)
            && warm_slots == SEED_N
            && warm_doc_rt
            && digest_match
            && !warm.is_null()
            && warm["missionId"] == json!("smoke")
            && warm["slotCount"] == json!(SEED_N);
        let pass = cold_ok && warm_ok && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-persist-smoke", "path": path,
            "coldLoaded": cold_loaded, "coldSlots": cold_slots, "coldDocRt": cold_doc_rt,
            "warmLoaded": warm_loaded, "warmSlots": warm_slots, "warmDocRt": warm_doc_rt,
            "digestMatch": digest_match, "digestLen": cold_digest.len(), "encodeHexLen": encode_hex_len,
            "warm": warm, "coldOk": cold_ok, "warmOk": warm_ok,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_select_editor.mjs — T-159.18: LMB pick foundation (selfcheck + click/toggle battery).
pub async fn smoke_select(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5304, 9364, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(SEL_READY, 200, 250).await?;

        let ids = || async { eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await };
        let count = || async { eval_i64(&h.page, "window.__editorSelection.count()").await };

        let mut selfcheck = false;
        let mut probe = Value::Null;
        let mut probe_ok = false;
        let (mut t1, mut t2, mut t3, mut t4) = (false, false, false, false);
        let mut sel_ids = Value::Null;
        let (mut sel_count, mut clr_count, mut on_count, mut off_count) =
            (-1i64, -1i64, -1i64, -1i64);

        if ready {
            selfcheck = eval_bool(&h.page, "window.__editorSelection.pick_selfcheck()").await?;
            probe =
                serde_json::from_str(&eval_str(&h.page, "window.__editorSelection.probe()").await?)
                    .unwrap_or(Value::Null);
            probe_ok =
                probe["id"].is_string() && probe["hit"].is_array() && probe["empty"].is_array();
            if probe_ok {
                let (hx, hy) = (
                    probe["hit"][0].as_f64().unwrap_or(0.0),
                    probe["hit"][1].as_f64().unwrap_or(0.0),
                );
                let (ex, ey) = (
                    probe["empty"][0].as_f64().unwrap_or(0.0),
                    probe["empty"][1].as_f64().unwrap_or(0.0),
                );

                click_at(&h.page, hx, hy, false).await?;
                sel_ids = ids().await?;
                sel_count = count().await?;
                t1 = sel_count == 1
                    && sel_ids
                        .as_array()
                        .map(|a| a.len() == 1 && a[0] == probe["id"])
                        == Some(true);

                click_at(&h.page, ex, ey, false).await?;
                clr_count = count().await?;
                t2 = clr_count == 0;

                click_at(&h.page, hx, hy, true).await?;
                on_count = count().await?;
                t3 = on_count == 1;

                click_at(&h.page, hx, hy, true).await?;
                off_count = count().await?;
                t4 = off_count == 0;
            }
        } else {
            eprintln!("smoke_select_editor: window.__editorSelection / __editorCam never appeared");
        }
        let pass = ready && selfcheck && probe_ok && t1 && t2 && t3 && t4 && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-select-smoke", "path": path,
            "ready": ready, "selfcheck": selfcheck, "probeOk": probe_ok, "probeId": probe["id"],
            "hit": probe["hit"], "empty": probe["empty"],
            "selIds": sel_ids, "selCount": sel_count, "clrCount": clr_count,
            "onCount": on_count, "offCount": off_count,
            "t1_select": t1, "t2_clear": t2, "t3_toggleOn": t3, "t4_toggleOff": t4,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_save_export_editor.mjs — T-159.20: Rust compile bridges produce the schema payloads.
pub async fn smoke_save_export(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5307, 9367, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__editorCommands === 'object' && window.__editorCommands !== null && typeof window.__missionDoc === 'object' && window.__missionDoc !== null",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let (mut save_len, mut export_len, mut slot_count) = (0usize, 0usize, -1i64);
        if ready {
            let s1 = eval_str(&h.page, "window.__editorCommands.compile_save_json()").await?;
            let s2 = eval_str(&h.page, "window.__editorCommands.compile_save_json()").await?;
            let e1 = eval_str(&h.page, "window.__editorCommands.compile_export_json()").await?;
            let e2 = eval_str(&h.page, "window.__editorCommands.compile_export_json()").await?;
            save_len = s1.len();
            export_len = e1.len();
            slot_count = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;

            checks.insert(
                "saveDeterministic".into(),
                json!(!s1.is_empty() && s1 == s2),
            );
            checks.insert(
                "exportDeterministic".into(),
                json!(!e1.is_empty() && e1 == e2),
            );

            let save: Value = serde_json::from_str(&s1).unwrap_or(Value::Null);
            let exp: Value = serde_json::from_str(&e1).unwrap_or(Value::Null);
            let is_obj = |v: &Value| v.is_object();
            checks.insert("saveParsed".into(), json!(is_obj(&save)));
            checks.insert("exportParsed".into(), json!(is_obj(&exp)));
            if is_obj(&save) {
                let schema_version_re = regex::Regex::new(r#""schemaVersion":1[,}]"#).unwrap();
                checks.insert(
                    "schemaVersionInt".into(),
                    json!(save["schemaVersion"] == json!(1) && schema_version_re.is_match(&s1)),
                );
                checks.insert(
                    "terrainEveron".into(),
                    json!(save["map"]["terrain"] == json!("everon")),
                );
                checks.insert(
                    "boundsExact".into(),
                    json!(save["map"]["bounds"] == json!([0, 0, 12800, 12800])),
                );
                checks.insert("saveOmitsOrbat".into(), json!(save.get("orbat").is_none()));
                checks.insert("editorObj".into(), json!(save["editor"].is_object()));
                checks.insert(
                    "slotsMatchDoc".into(),
                    json!(
                        save["editor"]["slots"].as_array().map(|a| a.len() as i64)
                            == Some(slot_count)
                    ),
                );
                let empty_arr = |v: &Value| v.as_array().map(Vec::len) == Some(0);
                checks.insert(
                    "emptyGraph".into(),
                    json!(
                        empty_arr(&save["editor"]["factions"])
                            && empty_arr(&save["editor"]["squads"])
                            && empty_arr(&save["editor"]["editorLayers"])
                    ),
                );
                checks.insert(
                    "objectShapes".into(),
                    json!(save["loadouts"].is_object() && save["environment"].is_object()),
                );
                checks.insert(
                    "arrayShapes".into(),
                    json!(
                        save["objectives"].is_array()
                            && save["vehicles"].is_array()
                            && save["markers"].is_array()
                    ),
                );
            }
            if is_obj(&exp) {
                checks.insert(
                    "exportFormatVersion".into(),
                    json!(exp["exportFormatVersion"] == json!(1)),
                );
                checks.insert(
                    "exportOrbatEmpty".into(),
                    json!(
                        exp["payload"].is_object()
                            && exp["payload"]["orbat"].as_array().map(Vec::len) == Some(0)
                    ),
                );
                checks.insert(
                    "exportWrapsPayload".into(),
                    json!(
                        exp["payload"].is_object() && exp["payload"]["schemaVersion"] == json!(1)
                    ),
                );
            }
        } else {
            eprintln!("smoke_save_export_editor: window.__editorCommands never appeared");
        }
        let seeded = slot_count == SEED_N;
        let pass = ready && h.no_panics() && seeded && checks_pass(&checks, 16);
        print_verdict(&json!({
            "gate": "editor-save-export-smoke", "path": path,
            "slotCount": slot_count, "seeded": seeded, "checks": checks,
            "saveLen": save_len, "exportLen": export_len,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_cur_editor.mjs — T-159.22: CUR toolbelt read-out (C0 camera + C1/C2 math + C3 em dash).
/// MUST NOT call probe() (it re-centres the camera and would invalidate the arithmetic).
pub async fn smoke_cur(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5310, 9370, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 200, 250)
            .await?;

        let cell = |axis: &'static str| {
            let page = Arc::clone(&h.page);
            async move {
                let expr = format!(
                    "(() => {{ const e = document.querySelector('[title=\"Cursor {axis}\"]');
      return e ? (e.textContent || '').replace(/^\\s*{axis}\\s*/, '').trim() : null; }})()"
                );
                page.evaluate(&expr, false).await
            }
        };
        let read = || async {
            Ok::<Value, anyhow::Error>(json!({ "x": cell("X").await?, "y": cell("Y").await? }))
        };
        let mv = |x: f64, y: f64| {
            mouse(
                &h.page,
                "mouseMoved",
                x,
                y,
                json!({ "button": "none", "buttons": 0 }),
            )
        };

        let mut checks = Map::new();
        let (mut cam, mut boot_r, mut centre, mut offset) =
            (Value::Null, Value::Null, Value::Null, Value::Null);
        if ready {
            // C3 — off-map BEFORE any pointer move.
            boot_r = read().await?;
            checks.insert(
                "c3_offMapEmDash".into(),
                json!(boot_r["x"] == json!("—") && boot_r["y"] == json!("—")),
            );
            // C0 — pin the camera this math rests on.
            cam = serde_json::from_str(&eval_str(&h.page, "window.__editorCam()").await?)
                .unwrap_or(Value::Null);
            checks.insert(
                "c0_defaultCamera".into(),
                json!(
                    cam["tx"].as_f64() == Some(6400.0)
                        && cam["ty"].as_f64() == Some(6400.0)
                        && cam["z"].as_f64() == Some(-2.0)
                ),
            );
            // C1 — the container centre is the camera target.
            mv(720.0, 450.0).await?;
            centre = read().await?;
            checks.insert(
                "c1_centreIsTarget".into(),
                json!(centre["x"] == json!("6400.000") && centre["y"] == json!("6400.000")),
            );
            // C2 — the offset proof (1 px = 4 m, north-up).
            mv(600.0, 300.0).await?;
            offset = read().await?;
            checks.insert(
                "c2_offsetMath".into(),
                json!(offset["x"] == json!("5920.000") && offset["y"] == json!("7000.000")),
            );
        } else {
            eprintln!("smoke_cur_editor: window.__editorCam never appeared");
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 4);
        print_verdict(&json!({
            "gate": "editor-cur-smoke", "path": path,
            "ready": ready, "backend": cam["backend"],
            "cam": cam, "readouts": { "boot": boot_r, "centre": centre, "offset": offset },
            "expected": { "centre": ["6400.000", "6400.000"], "offset": ["5920.000", "7000.000"] },
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

const ATTR_READY: &str = "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__editorCam === 'function' && typeof window.__missionPersist === 'object'";
const MODAL_OPEN: &str =
    "[...document.querySelectorAll('h2')].some(h => h.textContent === 'Attributes')";

async fn probe_hit(page: &Page) -> Result<(f64, f64)> {
    let probe: Value =
        serde_json::from_str(&eval_str(page, "window.__editorSelection.probe()").await?)
            .unwrap_or(Value::Null);
    Ok((
        probe["hit"][0].as_f64().unwrap_or(0.0),
        probe["hit"][1].as_f64().unwrap_or(0.0),
    ))
}

async fn dbl_click(page: &Page, x: f64, y: f64) -> Result<()> {
    // down/up ×2, clickCount 2 on the second pair (the Node smokes' shape via page.dispatchMouse).
    for (ev, cc) in [
        ("mousePressed", 1),
        ("mouseReleased", 1),
        ("mousePressed", 2),
        ("mouseReleased", 2),
    ] {
        page.send(
            "Input.dispatchMouseEvent",
            json!({ "type": ev, "x": x, "y": y, "button": "left", "clickCount": cc }),
        )
        .await?;
    }
    Ok(())
}

/// smoke_attributes_editor.mjs — T-159.26 Attributes modal (A1/A2t/A2i/U/A1c).
pub async fn smoke_attributes(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5311, 9371, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(ATTR_READY, 120, 250).await?;

        let mut checks = Map::new();
        if ready {
            let (hx, hy) = probe_hit(&h.page).await?;

            // A1 — trusted dbl-click on the slot.
            dbl_click(&h.page, hx, hy).await?;
            checks.insert(
                "a1_open".into(),
                json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
            );
            checks.insert(
                "a1_selected".into(),
                json!(eval_bool(&h.page, "window.__editorSelection.count() === 1").await?),
            );

            let d0 = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;

            // A2t — Transform tab → X commit via input + blur.
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Transform').click()").await?;
            checks.insert(
                "a2t_tab".into(),
                json!(
                    h.page
                        .wait_for(
                            "document.querySelectorAll('input[type=number]').length >= 4",
                            20,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                "(() => {
      const el = document.querySelectorAll('input[type=number]')[0];
      el.focus();
      el.value = '5000';
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.blur();
    })()",
            )
            .await?;
            let d1 = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
            let depth1 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            checks.insert("a2t_digestChanged".into(), json!(d1 != d0));
            checks.insert("a2t_oneUndoStep".into(), json!(depth1 == depth0 + 1));

            // U — real Ctrl+Z restores the digest.
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            let d0_json = serde_json::to_string(&d0)?;
            checks.insert(
                "u_digestRestored".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionPersist.slots_digest() === {d0_json}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // A2i — Identity tab → Role commit per input.
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Identity').click()").await?;
            checks.insert(
                "a2i_tab".into(),
                json!(
                    h.page
                        .wait_for(
                            "!!document.querySelector('input[placeholder=Rifleman]')",
                            20,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                "(() => {
      const el = document.querySelector('input[placeholder=Rifleman]');
      el.focus();
      el.value = 'Marksman';
      el.dispatchEvent(new Event('input', { bubbles: true }));
    })()",
            )
            .await?;
            checks.insert(
                "a2i_digestChanged".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionPersist.slots_digest() !== {d0_json}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // A1c — Esc closes.
            for ev in ["rawKeyDown", "keyUp"] {
                h.page.send(
                    "Input.dispatchKeyEvent",
                    json!({ "type": ev, "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27 }),
                ).await?;
            }
            checks.insert(
                "a1c_closed".into(),
                json!(
                    h.page
                        .wait_for(&format!("!({MODAL_OPEN})"), 20, 250)
                        .await?
                ),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 9);
        print_verdict(&json!({
            "gate": "editor-attributes-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_keyboard_settings_editor.mjs — T-159.26: Delete/undo, copy/paste, Mission Settings.
pub async fn smoke_keyboard_settings(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5316, 9376, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__missionPersist === 'object' && typeof window.__editorCam === 'function'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        if ready {
            let (hx, hy) = probe_hit(&h.page).await?;
            // Select seed 0 with a single click (sub-threshold).
            click_at(&h.page, hx, hy, false).await?;
            checks.insert(
                "selected1".into(),
                json!(
                    h.page
                        .wait_for("window.__editorSelection.count() === 1", 20, 250)
                        .await?
                ),
            );

            // K-del — Delete removes it; one undo step; Ctrl+Z restores.
            let n0 = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            key_chord(&h.page, "Delete", "Delete", 0, 46).await?;
            checks.insert(
                "delRemoved".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {}", n0 - 1),
                            20,
                            250
                        )
                        .await?
                ),
            );
            checks.insert(
                "delOneUndo".into(),
                json!(
                    eval_i64(&h.page, "window.__editorHistory.undo_depth()").await? == depth0 + 1
                ),
            );
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            checks.insert(
                "delUndoRestored".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {n0}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // K-cv — reselect, Ctrl+C then cursor over canvas and Ctrl+V.
            click_at(&h.page, hx, hy, false).await?;
            h.page
                .wait_for("window.__editorSelection.count() === 1", 20, 250)
                .await?;
            key_chord(&h.page, "c", "KeyC", 2, 67).await?;
            mouse(&h.page, "mouseMoved", 720.0, 470.0, json!({})).await?;
            let n_before = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            key_chord(&h.page, "v", "KeyV", 2, 86).await?;
            checks.insert(
                "pasteAdded".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {}", n_before + 1),
                            30,
                            250
                        )
                        .await?
                ),
            );

            // S-env — open Mission Settings, change Weather, assert the compiled env changed.
            eval(
                &h.page,
                "document.querySelector('button[aria-label=\"Mission settings\"]').click()",
            )
            .await?;
            checks.insert(
                "settingsOpen".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('h2')].some(h => h.textContent === 'Mission Settings')", 30, 250).await?),
            );
            eval(&h.page, "(() => {
      const sel = [...document.querySelectorAll('select')].find(s => [...s.options].some(o => o.value === 'overcast'));
      sel.value = 'overcast';
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()").await?;
            checks.insert(
                "weatherCommitted".into(),
                json!(h.page.wait_for("JSON.parse(window.__editorCommands.compile_save_json()).environment.weather === 'overcast'", 20, 250).await?),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 7);
        print_verdict(&json!({
            "gate": "editor-keyboard-settings-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_arsenal_editor.mjs — T-159.27 Arsenal loadout tab (R1–R5, registry golden intercepted).
pub async fn smoke_arsenal(dist: &str, path: &str) -> Result<u8> {
    const M16A2: &str = "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et";
    let h = Harness::new(dist, 5314, 9374, None, None, &[]).await?;
    let run = async {
        let (hits, compat_hits, post_hits) = serve_arsenal_golden(&h.page, M16A2).await?;
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__editorCommands === 'object' && typeof window.__missionPersist === 'object'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        if ready {
            h.page.wait_for("(() => { try { return JSON.parse(window.__editorSelection.probe()).hit !== null } catch (e) { return false } })()", 80, 250).await?;
            let (hx, hy) = probe_hit(&h.page).await?;

            // R1 — dbl-click seed slot → modal; then the Arsenal tab.
            dbl_click(&h.page, hx, hy).await?;
            checks.insert(
                "r1_open".into(),
                json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
            );
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Arsenal').click()").await?;

            // R2 (T-172 B10) — registry resolved → the Forge layout: 14-region rail + the item
            // list for the default-active Primary region.
            checks.insert(
                "r2_registryFetched".into(),
                json!(*hits.lock().unwrap() >= 1),
            );
            checks.insert(
                "r2_railRendered".into(),
                json!(
                    h.page
                        .wait_for(
                            "document.querySelectorAll('[data-arsenal-rail]').length >= 14",
                            40,
                            250
                        )
                        .await?
                ),
            );

            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;

            // R3 — the Primary item list carries the golden's M16A2; click-pick it.
            let m16_json = serde_json::to_string(M16A2)?;
            checks.insert(
                "r3_m16Listed".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!(
                                "!![...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {m16_json})"
                            ),
                            40,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                &format!(
                    "[...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {m16_json})?.click()"
                ),
            )
            .await?;

            // R4 — compiled save payload carries the canonical SlotLoadoutV2.
            h.page.wait_for("JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout && s.loadout.weapons && s.loadout.weapons.length)", 40, 250).await?;
            let lo_json = eval_str(
                &h.page,
                "(() => {
        const p = JSON.parse(window.__editorCommands.compile_save_json());
        const s = (p.editor?.slots || []).find(s => s.loadout);
        return s ? JSON.stringify(s.loadout) : '';
      })()",
            )
            .await?;
            let lo: Value = serde_json::from_str(&lo_json).unwrap_or(Value::Null);
            checks.insert("r4_version2".into(), json!(lo["version"] == json!(2)));
            checks.insert(
                "r4_weaponSlot".into(),
                json!(
                    lo["weapons"][0]["slotIndex"] == json!(0)
                        && lo["weapons"][0]["slotType"] == json!("primary")
                ),
            );
            checks.insert(
                "r4_weaponIsPick".into(),
                json!(lo["weapons"][0]["weapon"] == json!(M16A2)),
            );

            // R5 — one undo step; real Ctrl+Z clears it.
            let depth1 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            checks.insert("r5_oneUndoStep".into(), json!(depth1 == depth0 + 1));
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            checks.insert(
                "r5_undoClears".into(),
                json!(h.page.wait_for("!JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout)", 20, 250).await?),
            );

            // R6 (T-167 compat / T-172 Forge) — R5's undo bumped `doc_tick`, which re-creates the
            // modal body and resets the tab to Identity; re-open the Arsenal tab, re-pick primary
            // from the item list, then the compat PANEL lists the edge's ACOG under OPTIC; click
            // it → saved weapons[0] carries `optic`.
            const OPTIC: &str = "{ARSENAL_OPTIC}Prefabs/Weapons/Attachments/Optic_ACOG.et";
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Arsenal').click()").await?;
            h.page
                .wait_for(
                    "document.querySelectorAll('[data-arsenal-rail]').length >= 14",
                    40,
                    250,
                )
                .await?;
            let pick_value = |val: &str| {
                format!(
                    "(() => {{ const b=[...document.querySelectorAll('[data-value]')].find(b=>b.getAttribute('data-value')==={val:?}); if(!b)return false; b.click(); return true }})()"
                )
            };
            h.page.wait_for(&pick_value(M16A2), 40, 250).await?;
            checks.insert(
                "r6_compatFetched".into(),
                json!(*compat_hits.lock().unwrap() >= 1),
            );
            let optic_json = serde_json::to_string(OPTIC)?;
            checks.insert(
                "r6_opticListed".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!(
                                "!![...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {optic_json})"
                            ),
                            40,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                &format!(
                    "[...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {optic_json})?.click()"
                ),
            )
            .await?;
            checks.insert(
                "r6_opticSaved".into(),
                json!(h.page.wait_for("(() => { const s=(JSON.parse(window.__editorCommands.compile_save_json()).editor?.slots||[]).find(s=>s.loadout); return !!(s && s.loadout.weapons && s.loadout.weapons[0] && s.loadout.weapons[0].optic) })()", 40, 250).await?),
            );

            // R7 (T-172 B10 — 3D doll) — the DollEngine canvas mounts (long wait: SwiftShader
            // create is slow headless); its window hooks report a live backend, the active-region
            // anchor projects, and a CPU pick at that anchor resolves a region. If create failed
            // (no GL at all), the SVG paper-doll fallback must be up instead — the T-154 contract.
            let doll_3d = h
                .page
                .wait_for(
                    "!!document.querySelector('[data-arsenal-doll] canvas') && typeof window.__arsenalDoll === 'object'",
                    120,
                    250,
                )
                .await?;
            if doll_3d {
                checks.insert(
                    "r7_dollBackend".into(),
                    json!(
                        h.page
                            .wait_for(
                                "typeof window.__arsenalDoll.backend() === 'string' && window.__arsenalDoll.backend().length > 0",
                                40,
                                250
                            )
                            .await?
                    ),
                );
                checks.insert(
                    "r7_dollAnchorPick".into(),
                    json!(
                        h.page
                            .wait_for(
                                "(() => { const a = window.__arsenalDoll.anchor(0); return a && a.length === 2 && window.__arsenalDoll.pick(a[0], a[1]) >= 0 })()",
                                40,
                                250
                            )
                            .await?
                    ),
                );
                checks.insert(
                    "r7_dollCallout".into(),
                    json!(
                        eval_bool(&h.page, "!!document.querySelector('[data-doll-callout]')")
                            .await?
                    ),
                );
            } else {
                // Fallback branch: SVG hotspots (the old R7) prove the fallback path works.
                checks.insert(
                    "r7_dollBackend".into(),
                    json!(
                        eval_i64(
                            &h.page,
                            "document.querySelectorAll('svg [role=\"button\"]').length"
                        )
                        .await?
                            >= 8
                    ),
                );
                checks.insert("r7_dollAnchorPick".into(), json!(true));
                checks.insert("r7_dollCallout".into(), json!(true));
                eprintln!("smoke_arsenal: DollEngine unavailable — verified the SVG fallback");
            }

            // R8 (weight) — the honest weight readout renders (contains a kg figure).
            checks.insert(
                "r8_weightReadout".into(),
                json!(eval_bool(&h.page, "[...document.querySelectorAll('p')].some(p => /\\bkg\\b/.test(p.textContent||''))").await?),
            );

            // R9 (Faction Manager) — close the modal, open the manager from the Factions dock, and a
            // create round-trips a POST to /factions.
            key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
            eval(&h.page, "document.querySelector('[aria-label=\"Manage factions\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true}))").await?;
            checks.insert(
                "r9_fmOpens".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('h2')].some(h => h.textContent === 'Faction Manager')", 40, 250).await?),
            );
            eval(&h.page, "(() => { const i=document.querySelector('input[placeholder^=\"e.g.\"]'); if(i){ i.value='Smoke Bn'; i.dispatchEvent(new Event('input',{bubbles:true})); } document.querySelector('[aria-label=\"Save faction\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true})); })()").await?;
            cdp::sleep_ms(600).await; // let the POST round-trip through the Fetch tap
            checks.insert("r9_fmPost".into(), json!(*post_hits.lock().unwrap() >= 1));
        }
        let registry_hits = *hits.lock().unwrap();
        let pass = ready && h.no_panics() && checks_pass(&checks, 18);
        print_verdict(&json!({
            "gate": "editor-arsenal-smoke", "path": path, "registryHits": registry_hits,
            "compatHits": *compat_hits.lock().unwrap(), "factionPosts": *post_hits.lock().unwrap(),
            "checks": checks, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

fn set_eq(a: &Value, b: &Value) -> bool {
    match (a.as_array(), b.as_array()) {
        (Some(x), Some(y)) => {
            let mut xs: Vec<String> = x
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect();
            let mut ys: Vec<String> = y
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect();
            xs.sort();
            ys.sort();
            xs.len() == ys.len() && xs == ys
        }
        _ => false,
    }
}

/// smoke_marquee_drag_editor.mjs — T-159.19: marquee select + drag-move (?force=webgl).
pub async fn smoke_marquee_drag(dist: &str, raw_path: &str) -> Result<u8> {
    let path = force_webgl(raw_path);
    let h = Harness::new(dist, 5305, 9365, None, None, &[]).await?;
    let run = async {
        let url = h.url(&path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                page.wait_for(&ready_expr, 200, 250).await
            }
        };
        let digest = || async { eval_str(&h.page, "window.__missionPersist.slots_digest()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let sel_ids =
            || async { eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await };

        // boot 0: hard-reset IDB for a deterministic COLD start.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        // boot 1 (COLD).
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY}")).await?;

        let mut marquee_selfcheck = false;
        let mut pm = Value::Null;
        let (mut marquee_ok, mut marquee_count) = (false, -1i64);
        let mut marquee_ids = Value::Null;
        let (mut c0, mut c1) = (-1i64, -1i64);
        let mut mv = Value::Null;
        let (mut move_digest_changed, mut move_selected, mut edit_persist_fired) =
            (false, false, false);

        if ready {
            marquee_selfcheck =
                eval_bool(&h.page, "window.__editorSelection.marquee_selfcheck()").await?;

            // Marquee — drag the probe box; selection must equal the oracle's expect set.
            pm = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_marquee()").await?,
            )
            .unwrap_or(Value::Null);
            if pm["rect"].is_array() && pm["expect_ids"].is_array() {
                let r = &pm["rect"];
                drag(
                    &h.page,
                    r[0].as_f64().unwrap_or(0.0),
                    r[1].as_f64().unwrap_or(0.0),
                    r[2].as_f64().unwrap_or(0.0),
                    r[3].as_f64().unwrap_or(0.0),
                )
                .await?;
                marquee_count = eval_i64(&h.page, "window.__editorSelection.count()").await?;
                marquee_ids = sel_ids().await?;
                marquee_ok = pm["expect_count"].as_i64().unwrap_or(0) >= 1
                    && marquee_count == pm["expect_count"].as_i64().unwrap_or(-2)
                    && set_eq(&marquee_ids, &pm["expect_ids"]);
            }

            // Reset the selection to none (plain click on a guaranteed-empty px).
            let probe: Value =
                serde_json::from_str(&eval_str(&h.page, "window.__editorSelection.probe()").await?)
                    .unwrap_or(Value::Null);
            if probe["empty"].is_array() {
                click_at(
                    &h.page,
                    probe["empty"][0].as_f64().unwrap_or(0.0),
                    probe["empty"][1].as_f64().unwrap_or(0.0),
                    false,
                )
                .await?;
            }

            // Move (Class R + M5).
            let d0 = digest().await?;
            c0 = edit_count().await?;
            mv = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
            )
            .unwrap_or(Value::Null);
            if mv["id"].is_string() && mv["from"].is_array() && mv["to"].is_array() {
                drag(
                    &h.page,
                    mv["from"][0].as_f64().unwrap_or(0.0),
                    mv["from"][1].as_f64().unwrap_or(0.0),
                    mv["to"][0].as_f64().unwrap_or(0.0),
                    mv["to"][1].as_f64().unwrap_or(0.0),
                )
                .await?;
                let d1 = digest().await?;
                c1 = edit_count().await?;
                let move_ids = sel_ids().await?;
                move_digest_changed = !d0.is_empty() && !d1.is_empty() && d1 != d0;
                move_selected = move_ids.as_array().map(|a| a.contains(&mv["id"])) == Some(true);
                edit_persist_fired = c1 > c0;
            }
        } else {
            eprintln!("smoke_marquee_drag_editor: bridges never appeared");
        }

        let pass = ready0
            && ready
            && marquee_selfcheck
            && marquee_ok
            && move_digest_changed
            && move_selected
            && edit_persist_fired
            && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-marquee-drag-smoke", "path": path,
            "ready0": ready0, "ready": ready, "marqueeSelfcheck": marquee_selfcheck,
            "marquee": { "rect": pm["rect"], "expectCount": pm["expect_count"], "count": marquee_count, "ids": marquee_ids, "ok": marquee_ok },
            "move": { "id": mv["id"], "from": mv["from"], "to": mv["to"], "digestChanged": move_digest_changed, "selected": move_selected, "editPersistFired": edit_persist_fired, "c0": c0, "c1": c1 },
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_undo_editor.mjs — T-159.21/.22.1: two drags, undo boundary, redo button, A7 keydown guard.
pub async fn smoke_undo(dist: &str, raw_path: &str) -> Result<u8> {
    let path = force_webgl(raw_path);
    let h = Harness::new(dist, 5308, 9368, None, None, &[]).await?;
    let run = async {
        let url = h.url(&path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                page.wait_for(&ready_expr, 200, 250).await
            }
        };
        let digest = || async { eval_str(&h.page, "window.__missionPersist.slots_digest()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let can_undo = || async { eval_bool(&h.page, "window.__editorHistory.can_undo()").await };
        let can_redo = || async { eval_bool(&h.page, "window.__editorHistory.can_redo()").await };
        let undo_depth =
            || async { eval_i64(&h.page, "window.__editorHistory.undo_depth()").await };

        // boot 0 + hard reset; boot 1 COLD.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY} && {HIST_READY}")).await?;

        let mut checks = Map::new();
        let (mut d0, mut d1, mut d2) = (String::new(), String::new(), String::new());
        let (mut du1, mut du2, mut d3) = (String::new(), String::new(), String::new());
        let (mut cu, mut cr, mut depth) = (Vec::new(), Vec::new(), Vec::new());
        let (mut c_after_drag, mut c_after_undo) = (-1i64, -1i64);
        let (mut mv, mut mv2) = (Value::Null, Value::Null);
        let mut undo_ids = Value::Null;
        let mut kd = -1i64;

        if ready {
            eval(&h.page, "window.__kd = 0; window.addEventListener('keydown', () => { window.__kd++ }, true); 'ok'").await?;
            // A0 — the INIT-origin seed is not an undo step.
            cu.push(can_undo().await?);
            cr.push(can_redo().await?);
            depth.push(undo_depth().await?);
            checks.insert("a0_seedNotUndoable".into(), json!(!cu[0] && !cr[0]));
            checks.insert("a0_depthZero".into(), json!(depth[0] == 0));

            // A6 — the chrome scaffold is mounted.
            checks.insert(
                "a6_chromeMounted".into(),
                json!(eval_bool(&h.page, "!!document.querySelector('[aria-label=\"Undo\"]') &&\n        !!document.querySelector('[aria-label=\"Redo\"]')").await?),
            );
            checks.insert(
                "a6_docksMounted".into(),
                json!(eval_bool(&h.page, "(() => { const t = [...document.querySelectorAll('aside')]\n        .map((e) => e.textContent || '').join('|');\n        return t.includes('ORBAT') && t.includes('Editor Layers') && t.includes('Factions'); })()").await?),
            );

            d0 = digest().await?;
            mv = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
            )
            .unwrap_or(Value::Null);
            if mv["id"].is_string() && mv["from"].is_array() && mv["to"].is_array() {
                let dxy = |v: &Value, k: &str, i: usize| v[k][i].as_f64().unwrap_or(0.0);
                // A1 — commit a real drag-move.
                drag(
                    &h.page,
                    dxy(&mv, "from", 0),
                    dxy(&mv, "from", 1),
                    dxy(&mv, "to", 0),
                    dxy(&mv, "to", 1),
                )
                .await?;
                d1 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert(
                    "a1_moveChangedDigest".into(),
                    json!(!d0.is_empty() && d1 != d0),
                );
                checks.insert("a1_canUndoAfterMove".into(), json!(cu[1] && !cr[1]));
                checks.insert("a1_depthOne".into(), json!(depth[1] == 1));

                // A1b — a SECOND drag on the same slot.
                mv2 = serde_json::from_str(
                    &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
                )
                .unwrap_or(Value::Null);
                drag(
                    &h.page,
                    dxy(&mv2, "from", 0),
                    dxy(&mv2, "from", 1),
                    dxy(&mv2, "to", 0),
                    dxy(&mv2, "to", 1),
                )
                .await?;
                d2 = digest().await?;
                depth.push(undo_depth().await?);
                c_after_drag = edit_count().await?;
                checks.insert("a1b_move2ChangedDigest".into(), json!(d2 != d1));
                checks.insert("a1b_depthTwo".into(), json!(depth[2] == 2));

                // A2 — THE BOUNDARY: one Ctrl+Z reverts ONLY the 2nd drag.
                key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
                du1 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                undo_ids = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
                c_after_undo = edit_count().await?;
                checks.insert("a2_undoLandsOnD1".into(), json!(du1 == d1));
                checks.insert("a2_undoDidNotLandOnD0".into(), json!(du1 != d0));
                checks.insert("a2_depthOne".into(), json!(depth[3] == 1));
                checks.insert("a2_stillUndoable".into(), json!(cu[2] && cr[2]));

                // A2b — a second Ctrl+Z empties the stack.
                key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
                du2 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert("a2b_undoRestoredDigest".into(), json!(du2 == d0));
                checks.insert(
                    "a2b_stackEmptied".into(),
                    json!(!cu[3] && cr[3] && depth[4] == 0),
                );

                // A4 — undo of a move keeps the seed selected.
                checks.insert(
                    "a4_selectionKept".into(),
                    json!(undo_ids.as_array().map(|a| a.contains(&mv["id"])) == Some(true)),
                );
                // A5 — the undo re-armed the debounced IDB writer.
                checks.insert(
                    "a5_undoPersisted".into(),
                    json!(c_after_undo > c_after_drag),
                );

                // A3 — the Redo BUTTON re-applies one step.
                let redo_clicked = click_selector(&h.page, "[aria-label=\"Redo\"]").await?;
                d3 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert("a3_redoClicked".into(), json!(redo_clicked));
                checks.insert("a3_redoRestoredMove".into(), json!(d3 == d1));
                checks.insert(
                    "a3_oneStepBack".into(),
                    json!(cu[4] && cr[4] && depth[5] == 1),
                );

                // A7 — exactly one keydown per chord (2 chords above → 2).
                kd = eval_i64(&h.page, "window.__kd").await?;
                checks.insert("a7_oneKeydownPerChord".into(), json!(kd == 2));
            }
        } else {
            eprintln!(
                "smoke_undo_editor: window.__editorHistory / __editorSelection never appeared"
            );
        }

        let pass = ready0 && ready && h.no_panics() && checks_pass(&checks, 21);
        print_verdict(&json!({
            "gate": "editor-undo-smoke", "path": path,
            "ready0": ready0, "ready": ready,
            "moveId": mv["id"], "moveId2": mv2["id"],
            "digests": { "d0": d0, "d1": d1, "d2": d2, "du1": du1, "du2": du2, "d3": d3 },
            "canUndo": cu, "canRedo": cr, "undoDepth": depth,
            "keydownEvents": kd,
            "editPersist": { "afterDrag": c_after_drag, "afterUndo": c_after_undo },
            "undoIds": undo_ids,
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_outliner_palette_editor.mjs — T-159.22 dock gate (P1/O1/O2/D1/D2/D3/W1).
/// MUST NOT call probe() before the D2 read (it would re-centre the camera).
pub async fn smoke_outliner_palette(dist: &str, path: &str) -> Result<u8> {
    const RIFLEMAN_LABEL: &str = "[aria-label=\"US Rifleman\"]";
    let expect_x_bits = 6320.0f32.to_bits();
    let expect_y_bits = 6200.0f32.to_bits();
    let h = Harness::new(dist, 5309, 9369, None, None, &[]).await?;
    let run = async {
        let hits = serve_registry_golden(&h.page).await?;
        let url = h.url(path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                page.wait_for(&ready_expr, 200, 250).await
            }
        };
        let digest = || async { eval_str(&h.page, "window.__missionPersist.slots_digest()").await };
        let slot_count = || async { eval_i64(&h.page, "window.__missionDoc.slot_count()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let cam = || async {
            Ok::<Value, anyhow::Error>(
                serde_json::from_str(&eval_str(&h.page, "window.__editorCam()").await?)
                    .unwrap_or(Value::Null),
            )
        };
        let dock_text = || async {
            eval_str(&h.page, "(() => [...document.querySelectorAll('aside')].map((e) => e.textContent || '').join('|'))()").await
        };
        let obj_text = || async {
            eval(&h.page, "(() => { const e = document.querySelector('[title^=\"Placed slots\"]');\n    const m = (e?.textContent || '').match(/OBJ\\s*(\\d+)/); return m ? m[1] : null; })()").await
        };
        // slots_digest rows: id|x_bits|y_bits|z_bits|rot_bits|stance|role|tag|squad|layer
        let row_of = |d: &str, id: &str| -> Option<Vec<String>> {
            d.split('\n')
                .map(|r| r.split('|').map(str::to_string).collect::<Vec<_>>())
                .find(|c| c.first().map(String::as_str) == Some(id))
        };

        // boot 0 + reset; boot 1 COLD.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY} && {DOC_READY}")).await?;
        // T-172 B6 — palette folders below depth 0 boot collapsed (`default_expanded` rule 3:
        // only faction roots open). Expand US_Army before waiting on its leaves.
        let us_army = h
            .page
            .wait_for(
                "!!document.querySelector('aside [aria-label=\"US_Army\"]')",
                200,
                250,
            )
            .await?;
        if us_army {
            h.page
                .evaluate(
                    "document.querySelector('aside [aria-label=\"US_Army\"]').click()",
                    true,
                )
                .await?;
        }
        let palette_ready = h
            .page
            .wait_for(
                &format!("!!document.querySelector('{RIFLEMAN_LABEL}')"),
                200,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let (mut count0, mut count1) = (-1i64, -1i64);
        let (mut obj0, mut obj1) = (Value::Null, Value::Null);
        let (mut ec0, mut ec1) = (-1i64, -1i64);
        let mut first_row_ids = Value::Null;
        let mut placed_row: Option<Vec<String>> = None;
        let (mut cam_before, mut cam_dock, mut cam_canvas) =
            (Value::Null, Value::Null, Value::Null);

        if ready && palette_ready {
            // P1 — the palette tree from the golden.
            let docks0 = dock_text().await?;
            checks.insert(
                "p1_paletteTree".into(),
                json!(
                    docks0.contains("Factions")
                        && docks0.contains("NATO")
                        && docks0.contains("US_Army")
                ),
            );
            checks.insert(
                "p1_eightLeaves".into(),
                json!(
                    eval_i64(
                        &h.page,
                        "document.querySelectorAll('aside [aria-label^=\"US \"]').length"
                    )
                    .await?
                        == 8
                ),
            );
            checks.insert(
                "p1_registryFetched".into(),
                json!(*hits.lock().unwrap() >= 1),
            );

            // O1 — the seed's 8 slots are listed, unfiled.
            count0 = slot_count().await?;
            checks.insert(
                "o1_unfiledRoot".into(),
                json!(docks0.contains("Unfiled (8)") && count0 == 8),
            );

            // O2 — clicking the first Unfiled row selects exactly s0.
            click_selector(&h.page, "aside [aria-label=\"Rifleman\"]").await?;
            first_row_ids = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
            checks.insert(
                "o2_rowSelectsS0".into(),
                json!(
                    first_row_ids
                        .as_array()
                        .map(|a| a.len() == 1 && a[0] == json!("s0"))
                        == Some(true)
                ),
            );

            // D1/D2/D3 — drag the palette leaf onto the canvas at (700, 500).
            let d0 = digest().await?;
            ec0 = edit_count().await?;
            obj0 = obj_text().await?;
            if let Some((lx, ly)) = rect_of(&h.page, RIFLEMAN_LABEL).await? {
                drag(&h.page, lx, ly, 700.0, 500.0).await?;
                count1 = slot_count().await?;
                let d1 = digest().await?;
                ec1 = edit_count().await?;
                obj1 = obj_text().await?;
                let docks1 = dock_text().await?;

                checks.insert("d1_slotAdded".into(), json!(count0 == 8 && count1 == 9));
                checks.insert(
                    "d1_objReadout".into(),
                    json!(obj0 == json!("8") && obj1 == json!("9")),
                );
                checks.insert("d1_digestChanged".into(), json!(!d0.is_empty() && d1 != d0));
                checks.insert("d1_persistArmed".into(), json!(ec1 > ec0));

                // The placed slot is the one row present in d1 but not d0.
                let ids0: std::collections::HashSet<&str> = d0
                    .split('\n')
                    .map(|r| r.split('|').next().unwrap_or(""))
                    .collect();
                let new_id = d1
                    .split('\n')
                    .map(|r| r.split('|').next().unwrap_or(""))
                    .find(|id| !ids0.contains(id))
                    .map(str::to_string);
                placed_row = new_id.as_deref().and_then(|id| row_of(&d1, id));
                let bits = |row: &Option<Vec<String>>, i: usize| {
                    row.as_ref()
                        .and_then(|r| r.get(i))
                        .and_then(|v| v.parse::<u32>().ok())
                };
                checks.insert(
                    "d2_positionBitExact".into(),
                    json!(
                        placed_row.is_some()
                            && bits(&placed_row, 1) == Some(expect_x_bits)
                            && bits(&placed_row, 2) == Some(expect_y_bits)
                    ),
                );
                checks.insert(
                    "d2_roleFromPalette".into(),
                    json!(
                        placed_row
                            .as_ref()
                            .and_then(|r| r.get(6))
                            .map(String::as_str)
                            == Some("US Rifleman")
                    ),
                );
                checks.insert(
                    "d3_filedInDefaultLayer".into(),
                    json!(
                        placed_row
                            .as_ref()
                            .and_then(|r| r.get(9))
                            .map(String::as_str)
                            == Some("layer-1")
                    ),
                );
                checks.insert(
                    "d3_layerInOutliner".into(),
                    json!(docks1.contains("Layer 1") && docks1.contains("Unfiled (8)")),
                );

                // O3/O4/O5 (T-168) — the place minted a default squad; the ORBAT tree shows it,
                // its slot leaf selects, and dbl-click opens Attributes (SEL-ORBAT-DBL-001).
                checks.insert(
                    "o3_orbatSquadMinted".into(),
                    json!(docks1.contains("Squad 1 (1)") && docks1.contains("Faction 1")),
                );
                // The ORBAT slot leaf = the first slot button in the ORBAT div (the div right after
                // the "ORBAT" h2, before "Editor Layers") of the left dock.
                const ORBAT_LEAF: &str = "(() => { const d=[...document.querySelectorAll('aside')].find(a=>(a.textContent||'').includes('ORBAT')&&(a.textContent||'').includes('Editor Layers')); const o=d&&d.querySelector('div'); return o?o.querySelector('button[aria-label=\"US Rifleman\"]'):null; })()";
                eval(
                    &h.page,
                    &format!(
                        "{ORBAT_LEAF}?.dispatchEvent(new MouseEvent('click',{{bubbles:true}}))"
                    ),
                )
                .await?;
                let orbat_sel = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
                checks.insert(
                    "o4_orbatLeafSelects".into(),
                    json!(
                        orbat_sel
                            .as_array()
                            .map(|a| a.len() == 1 && a[0].as_str() == new_id.as_deref())
                            == Some(true)
                    ),
                );
                eval(
                    &h.page,
                    &format!(
                        "{ORBAT_LEAF}?.dispatchEvent(new MouseEvent('dblclick',{{bubbles:true}}))"
                    ),
                )
                .await?;
                checks.insert(
                    "o5_orbatDblAttributes".into(),
                    json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
                );
                // Close the modal so its overlay does not swallow the W1 wheel checks below.
                key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
            }

            // W1 — wheel over a dock must not zoom; over the canvas it must.
            cam_before = cam().await?;
            mouse(
                &h.page,
                "mouseWheel",
                120.0,
                500.0,
                json!({ "deltaX": 0, "deltaY": -240 }),
            )
            .await?;
            cam_dock = cam().await?;
            mouse(
                &h.page,
                "mouseWheel",
                700.0,
                500.0,
                json!({ "deltaX": 0, "deltaY": -240 }),
            )
            .await?;
            cam_canvas = cam().await?;
            checks.insert(
                "w1_dockWheelNoZoom".into(),
                json!(cam_dock["z"] == cam_before["z"]),
            );
            checks.insert(
                "w1_canvasWheelZooms".into(),
                json!(cam_canvas["z"] != cam_before["z"]),
            );
        } else {
            eprintln!("smoke_outliner_palette_editor: bridges/palette never appeared");
        }

        let registry_hits = *hits.lock().unwrap();
        let pass = ready0 && ready && palette_ready && h.no_panics() && checks_pass(&checks, 18);
        let placed = placed_row.as_ref().map(|r| {
            json!({
                "id": r.first(), "xBits": r.get(1).and_then(|v| v.parse::<u32>().ok()),
                "yBits": r.get(2).and_then(|v| v.parse::<u32>().ok()),
                "role": r.get(6), "layer": r.get(9),
            })
        });
        print_verdict(&json!({
            "gate": "editor-outliner-palette-smoke", "path": path,
            "ready0": ready0, "ready": ready, "paletteReady": palette_ready, "registryHits": registry_hits,
            "counts": { "slots": [count0, count1], "obj": [obj0, obj1], "editPersist": [ec0, ec1] },
            "selectedFirstRow": first_row_ids,
            "placed": placed,
            "expectedBits": { "x": expect_x_bits, "y": expect_y_bits },
            "cam": { "before": cam_before, "afterDockWheel": cam_dock, "afterCanvasWheel": cam_canvas },
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// T-169 — the VirtualOutliner gate. Seeds a mission past `VIRTUAL_SLOT_THRESHOLD` (via the
/// `__missionDoc.seed_slots` hook) and asserts the dock trees WINDOW: `window.__outlinerStats`
/// reports `rendered < total` above the threshold (and `rendered === total` below it), for both
/// the Editor Layers and ORBAT trees, while a windowed slot row still selects.
pub async fn smoke_virtual_outliner(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5320, 9380, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__missionDoc.seed_slots === 'function' && typeof window.__editorSelection === 'object'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        // A stats getter for one tree key → `{total, rendered, threshold}` (or nulls).
        let stat = |key: &str, field: &str| {
            format!(
                "(() => {{ const s = window.__outlinerStats && window.__outlinerStats.{key}; return (s && typeof s.{field} === 'number') ? s.{field} : -1 }})()"
            )
        };
        if ready {
            // Editor Layers has the 8 unfiled seeds now → below threshold → eager (rendered==total).
            h.page
                .wait_for(&format!("{} >= 0", stat("editorLayers", "total")), 40, 250)
                .await?;
            let e_total0 = eval_i64(&h.page, &stat("editorLayers", "total")).await?;
            let e_rend0 = eval_i64(&h.page, &stat("editorLayers", "rendered")).await?;
            checks.insert(
                "v1_eagerBelowThreshold".into(),
                json!(e_total0 > 0 && e_total0 <= 50 && e_rend0 == e_total0),
            );

            // Push both trees past the threshold.
            eval(&h.page, "window.__missionDoc.seed_slots(80)").await?;
            h.page
                .wait_for("window.__missionDoc.slot_count() >= 80", 40, 250)
                .await?;
            // Wait for the windowed re-render to publish rendered < total.
            let windowed = format!(
                "{} > 50 && {} < {}",
                stat("editorLayers", "total"),
                stat("editorLayers", "rendered"),
                stat("editorLayers", "total")
            );
            checks.insert(
                "v2_editorLayersWindowed".into(),
                json!(h.page.wait_for(&windowed, 40, 250).await?),
            );
            let e_total1 = eval_i64(&h.page, &stat("editorLayers", "total")).await?;
            let e_rend1 = eval_i64(&h.page, &stat("editorLayers", "rendered")).await?;
            checks.insert(
                "v3_windowRendersSubset".into(),
                json!(e_rend1 > 0 && e_rend1 < e_total1 && e_rend1 <= 60),
            );
            checks.insert(
                "v4_thresholdIs50".into(),
                json!(eval_i64(&h.page, &stat("editorLayers", "threshold")).await? == 50),
            );
            let orbat_windowed = format!(
                "{} > 50 && {} < {}",
                stat("orbat", "total"),
                stat("orbat", "rendered"),
                stat("orbat", "total")
            );
            checks.insert(
                "v5_orbatWindowed".into(),
                json!(h.page.wait_for(&orbat_windowed, 40, 250).await?),
            );

            // A windowed slot row still selects (the virtualization keeps interaction intact).
            eval(&h.page, "[...document.querySelectorAll('aside button[aria-label=\"Rifleman\"]')][0]?.dispatchEvent(new MouseEvent('click',{bubbles:true}))").await?;
            checks.insert(
                "v6_windowedRowSelects".into(),
                json!(
                    h.page
                        .wait_for("window.__editorSelection.count() >= 1", 20, 250)
                        .await?
                ),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 6);
        print_verdict(&json!({
            "gate": "editor-virtual-outliner-smoke", "path": path,
            "checks": checks, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

const BACKEND: &str = "http://127.0.0.1:8080";

/// smoke_hydrate_editor.mjs — T-159.26 server-hydrate data-safety gate (LIVE backend on :8080).
pub async fn smoke_hydrate(dist: &str) -> Result<u8> {
    const SAVED_SLOTS: i64 = 3; // must differ from SEED_N (8)
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    if !cdp::wait_http(&http, &format!("{BACKEND}/healthz"), 60).await {
        eprintln!("smoke_hydrate: backend not reachable on :8080");
        return Ok(2);
    }

    // 1a. dev-login (admin) → tokens from the 302 Location fragment.
    let login = http
        .get(format!("{BACKEND}/api/v1/auth/dev-login?role=admin"))
        .send()
        .await?;
    let loc = login
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let frag = loc.split('#').nth(1).unwrap_or("");
    let param = |k: &str| {
        frag.split('&')
            .find_map(|kv| kv.strip_prefix(&format!("{k}=")))
            .map(str::to_string)
    };
    let Some(token) = param("access_token") else {
        eprintln!("smoke_hydrate: no dev-login token");
        return Ok(2);
    };
    let refresh = param("refresh_token").unwrap_or_default();

    // 1b. create a mission (title varies per-run via the token suffix — no clock in harness).
    let create = http
        .post(format!("{BACKEND}/api/v1/missions"))
        .bearer_auth(&token)
        .json(&json!({
            "title": format!("Hydrate Gate {}", &token[token.len().saturating_sub(8)..]),
            "terrain": "everon", "game_mode": "pve_coop", "weather": "clear",
            "time_of_day": "12:00", "max_players": 32
        }))
        .send()
        .await?;
    if !create.status().is_success() {
        eprintln!("smoke_hydrate: create failed {}", create.status());
        return Ok(2);
    }
    let mission: Value = create.json().await?;
    let Some(mission_id) = mission["id"].as_str().map(str::to_string) else {
        eprintln!("smoke_hydrate: no mission id");
        return Ok(2);
    };

    // 1c. save a version with a KNOWN 3-slot editor block.
    let mk_slot = |i: i64| {
        json!({
            "id": format!("h{i}"), "squadId": "", "role": "Rifleman", "tag": "", "index": i,
            "stance": "stand",
            "position": { "x": 6400 + i, "y": 6400 + i, "z": 0, "rotation": 0 }, "assetId": "",
        })
    };
    let payload = json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon", "bounds": [0, 0, 12800, 12800] },
        "environment": { "time": "12:00", "weather": "clear" },
        "loadouts": {}, "objectives": [], "vehicles": [], "markers": [],
        "editor": {
            "factions": [], "squads": [],
            "editorLayers": [{ "id": "layer-1", "name": "Layer 1", "parentId": null, "entityIds": ["h0", "h1", "h2"] }],
            "slots": [mk_slot(0), mk_slot(1), mk_slot(2)],
        },
    });
    let save = http
        .post(format!("{BACKEND}/api/v1/missions/{mission_id}/versions"))
        .bearer_auth(&token)
        .json(&json!({ "semver": "0.2.0", "editor_notes": "hydrate gate", "payload": payload }))
        .send()
        .await?;
    if !save.status().is_success() {
        let status = save.status();
        let t = save.text().await.unwrap_or_default();
        eprintln!(
            "smoke_hydrate: save version failed {status} {}",
            t.chars().take(200).collect::<String>()
        );
        let _ = http
            .delete(format!("{BACKEND}/api/v1/missions/{mission_id}"))
            .bearer_auth(&token)
            .send()
            .await;
        return Ok(2);
    }

    // 2. serve with the same-origin /api proxy; seed the session + clear IDB before boot.
    let h = Harness::new(dist, 5315, 9375, None, Some(BACKEND.to_string()), &[]).await?;
    let run = async {
        let auth_blob = serde_json::to_string(&json!({
            "state": {
                "refreshToken": refresh,
                "user": {
                    "discord_id": "00000000000000001", "username": "Dev", "discord_handle": "d#1",
                    "avatar_url": "", "arma_id": null, "arma_character": "", "role": "admin",
                    "is_banned": false, "total_deployments": 0, "attendance_rate": 0,
                    "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
                },
                "expiresAt": "2030-01-01T00:00:00Z",
            },
            "version": 0,
        }))?;
        h.page
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": format!(
                    "localStorage.setItem('tbd-auth', {});try {{ indexedDB.deleteDatabase('tbd-mission-yrs'); }} catch (e) {{}}",
                    serde_json::to_string(&auth_blob)?
                ) }),
            )
            .await?;

        // T-172 B4 follow-up: pin force=webgl like every other editor smoke — with the slot
        // atlas live, the first hydrated slots_bind_soa allocates a GPU instance buffer, and
        // headless Chromium's software WebGPU device rejects any createBuffer (the known
        // wedge the suite avoids via WebGL2/SwiftShader).
        h.page
            .navigate(&h.url(&force_webgl(&format!("/missions/{mission_id}/edit"))))
            .await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for("typeof window.__missionDoc === 'object' && typeof window.__missionDoc.slot_count === 'function'", 120, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            checks.insert(
                "hydratedSavedSlots".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {SAVED_SLOTS}"),
                            120,
                            150
                        )
                        .await?
                ),
            );
            checks.insert(
                "notSeed".into(),
                json!(eval_i64(&h.page, "window.__missionDoc.slot_count()").await? != 8),
            );
            checks.insert(
                "notDirty".into(),
                json!(eval_bool(&h.page, "(() => { const el = document.querySelector('[aria-label=\"Unsaved changes\"]'); return !el || el.className.includes('hidden'); })()").await?),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 3);
        print_verdict(&json!({
            "gate": "editor-hydrate-smoke", "missionId": mission_id, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    // Cleanup: delete the test mission (both paths, as in the Node script).
    let _ = http
        .delete(format!("{BACKEND}/api/v1/missions/{mission_id}"))
        .bearer_auth(&token)
        .send()
        .await;
    code
}

/// smoke_mutations.mjs — T-159.25 live suite-mutation gate (TOKEN/REFRESH envs, backend on :8080).
pub async fn smoke_mutations(dist: &str) -> Result<u8> {
    let Ok(_token) = std::env::var("TOKEN") else {
        eprintln!("smoke_mutations: set TOKEN + REFRESH (dev-login tokens)");
        return Ok(2);
    };
    let Ok(refresh) = std::env::var("REFRESH") else {
        eprintln!("smoke_mutations: set TOKEN + REFRESH (dev-login tokens)");
        return Ok(2);
    };
    let http = reqwest::Client::new();
    if !cdp::wait_http(&http, &format!("{BACKEND}/healthz"), 60).await {
        eprintln!("smoke_mutations: backend not reachable on :8080");
        return Ok(2);
    }

    let auth_blob = serde_json::to_string(&json!({
        "state": {
            "refreshToken": refresh,
            "user": {
                "discord_id": "00000000000000001", "username": "Dev Operator", "discord_handle": "dev#0001",
                "avatar_url": "", "arma_id": null, "arma_character": "", "role": "admin",
                "is_banned": false, "total_deployments": 0, "attendance_rate": 0,
                "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
            },
            "expiresAt": "2030-01-01T00:00:00Z",
        },
        "version": 0,
    }))?;

    let h = Harness::new(dist, 5320, 9380, None, Some(BACKEND.to_string()), &[]).await?;
    let run = async {
        h.page
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": format!("localStorage.setItem('tbd-auth', {});", serde_json::to_string(&auth_blob)?) }),
            )
            .await?;
        h.page.navigate(&h.url("/settings")).await?;
        let ready = h.page
            .wait_for("[...document.querySelectorAll('button')].some(b => b.textContent.includes('Generate Link Code'))", 160, 250)
            .await?;

        let mut checks = Map::new();
        checks.insert("authedRender".into(), json!(ready));
        if ready {
            checks.insert(
                "noCodeBefore".into(),
                json!(
                    eval_bool(&h.page, "!document.body.textContent.includes('Link code:')").await?
                ),
            );
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.textContent.includes('Generate Link Code')).click()").await?;
            checks.insert(
                "codePanelAfter".into(),
                json!(
                    h.page
                        .wait_for("document.body.textContent.includes('Link code:')", 80, 250)
                        .await?
                ),
            );
            checks.insert(
                "toastShown".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('[role=status]')].some(n => /Link code generated/i.test(n.textContent))", 40, 250).await?),
            );
        }
        let pass = ready && h.no_panics() && checks.values().all(|v| *v == json!(true));
        print_verdict(
            &json!({ "gate": "suite-mutations-smoke", "checks": checks, "panics": h.panics_head(), "pass": pass }),
        );
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// gate_r_auth.mjs — the R-auth single-flight refresh gate (no backend; Fetch-mocked).
/// Exit map: 0 pass · 1 fail · 2 no dist · 3 driver error (mapped by the bin).
pub async fn r_auth(dist_override: Option<String>) -> Result<u8> {
    let dist = dist_override
        .or_else(|| std::env::var("LEPTOS_DIST").ok())
        .unwrap_or_else(|| {
            repo_root()
                .join(DIST_DEFAULT)
                .to_string_lossy()
                .into_owned()
        });
    if !PathBuf::from(&dist).join("index.html").exists() {
        eprintln!("gate_r_auth: no Leptos dist at {dist} (run `trunk build`)");
        return Ok(2);
    }
    const SEED: &str = "localStorage.setItem('tbd-auth', JSON.stringify({state:{refreshToken:'rt-seed',user:null,expiresAt:'2026-01-01T00:00:00Z'},version:0}));";
    let sample_user = json!({
        "discord_id": "1", "username": "cpl-authed", "discord_handle": "cpl#0001",
        "avatar_url": "", "arma_id": null, "arma_character": "", "role": "enlisted",
        "is_banned": false, "total_deployments": 0, "attendance_rate": 0.0,
        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
    });

    let h = Harness::new(&dist, 5193, 9341, None, None, &[SEED]).await?;
    let run = async {
        let refresh_count = Arc::new(StdMutex::new(0u64));
        let me_count = Arc::new(StdMutex::new(0u64));
        h.page
            .send(
                "Fetch.enable",
                json!({ "patterns": [{ "urlPattern": "*" }] }),
            )
            .await?;
        let mut paused = h.page.on_event("Fetch.requestPaused").await;
        let rp = Arc::clone(&h.page);
        let (rc_task, mc_task) = (Arc::clone(&refresh_count), Arc::clone(&me_count));
        let user_task = sample_user.clone();
        tokio::spawn(async move {
            while let Some(p) = paused.recv().await {
                let Some(request_id) = p["requestId"].as_str() else {
                    continue;
                };
                let u = p["request"]["url"].as_str().unwrap_or_default();
                let res = if u.contains("/api/v1/auth/refresh") {
                    *rc_task.lock().unwrap() += 1;
                    rp.fulfill_json(
                        request_id,
                        200,
                        &json!({
                            "access_token": "new-access", "refresh_token": "new-rt",
                            "expires_at": "2026-01-01T01:00:00Z",
                        }),
                    )
                    .await
                } else if u.contains("/api/v1/me") {
                    let n = {
                        let mut m = mc_task.lock().unwrap();
                        *m += 1;
                        *m
                    };
                    if n == 1 {
                        rp.fulfill_json(request_id, 401, &json!({ "error": "unauthorized" }))
                            .await
                    } else {
                        rp.fulfill_json(
                            request_id,
                            200,
                            &json!({ "user": user_task, "arma_linked": false }),
                        )
                        .await
                    }
                } else if u.contains("/api/v1/") {
                    // 200 {} — a 401 catch-all would loop every post-boot dashboard query
                    // through refresh (the gate pins the BOOTSTRAP single-flight; /me above
                    // still 401s exactly once).
                    rp.fulfill_json(request_id, 200, &json!({})).await
                } else {
                    rp.continue_request(request_id).await
                };
                let _ = res;
            }
        });

        h.page.navigate(&h.url("/")).await?;
        let ok = h.page
            .wait_for("(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user != null } catch { return false } })()", 80, 250)
            .await?;
        cdp::sleep_ms(100).await;
        let username = eval(&h.page, "(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user?.username || null } catch { return null } })()").await?;

        let rc = *refresh_count.lock().unwrap();
        let mc = *me_count.lock().unwrap();
        let pass = ok && rc == 1 && username == json!("cpl-authed");
        print_verdict(&json!({
            "gate": "R-auth", "pass": pass, "refreshCount": rc, "meCount": mc,
            "authedUsername": username, "expected": "cpl-authed",
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

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
}

pub async fn render_check(a: &RenderCheckArgs) -> Result<u8> {
    let seed = if a.seed_auth {
        Some(crate::vsuite::seed_script()?)
    } else {
        None
    };
    let mut injects: Vec<&str> = vec![crate::inject::FREEZE_SRC];
    if let Some(s) = seed.as_deref() {
        injects.push(s);
    }
    let h = Harness::new(&a.dir, a.port, a.debug_port, None, None, &injects).await?;
    let run = async {
        let url = h.url(&a.path);
        h.page.navigate(&url).await?;
        let ready = h
            .page
            .wait_for(
                "!!document.body && document.body.innerText.trim().length > 0",
                80,
                250,
            )
            .await?;
        cdp::sleep_ms(150).await;
        let text = eval_str(&h.page, "document.body.innerText").await?;
        let html = eval_str(&h.page, "document.body.innerHTML").await?;
        // awaitPromise so async-IIFE probes can settle reactive updates between steps
        // (T-172 behavioral probes); plain values pass through unchanged. The raw value is
        // echoed in the verdict so a diagnostic probe can return a JSON string.
        let assert_value = match &a.assert_js {
            Some(js) => Some(h.page.evaluate(js, true).await?),
            None => None,
        };
        let assert_ok = assert_value.as_ref().map(|v| {
            v.as_bool() == Some(true)
                || !(v.is_null() || *v == json!(false) || *v == json!(0) || *v == json!(""))
        });
        h.page.close().await;

        let found = if a.expect.is_empty() {
            Value::Null
        } else {
            json!(text.contains(&a.expect))
        };
        let pass = ready
            && (a.expect.is_empty() || text.contains(&a.expect))
            && a.assert_js.as_ref().is_none_or(|_| assert_ok == Some(true));
        print_verdict(&json!({
            "url": url, "ready": ready, "expect": a.expect, "found": found,
            "assertJs": a.assert_js, "assertOk": assert_ok, "assertValue": assert_value,
            "textPreview": text.chars().take(200).collect::<String>(),
            "htmlBytes": crate::vsuite::js_len(&html),
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Dispatch one smoke by suite name. `dist`/`path` fall back to the Node defaults.
pub async fn run_smoke(name: &str, dist: Option<String>, path: Option<String>) -> Result<u8> {
    let dist = dist.unwrap_or_else(|| DIST_DEFAULT.to_string());
    let path = path.unwrap_or_else(|| EDIT_PATH.to_string());
    match name {
        "editor" => smoke_editor(&dist, &path).await,
        "selfcheck" => smoke_selfcheck(&dist, &path).await,
        "fullmap" => smoke_fullmap(&dist, "packages/map-assets").await,
        "hillshade" => smoke_hillshade(&dist, "packages/map-assets").await,
        "doc" => smoke_doc(&dist, &path).await,
        "pan" => smoke_pan(&dist, &path).await,
        "persist" => smoke_persist(&dist, &path).await,
        "select" => smoke_select(&dist, &path).await,
        "save-export" => smoke_save_export(&dist, &path).await,
        "cur" => smoke_cur(&dist, &path).await,
        "attributes" => smoke_attributes(&dist, &path).await,
        "keyboard-settings" => smoke_keyboard_settings(&dist, &path).await,
        "arsenal" => smoke_arsenal(&dist, &path).await,
        "marquee-drag" => smoke_marquee_drag(&dist, &path).await,
        "undo" => smoke_undo(&dist, &path).await,
        "outliner-palette" => smoke_outliner_palette(&dist, &path).await,
        "virtual-outliner" => smoke_virtual_outliner(&dist, &path).await,
        "hydrate" => smoke_hydrate(&dist).await,
        "mutations" => smoke_mutations(&dist).await,
        other => Err(anyhow!("unknown smoke '{other}' (see gate smoke --help)")),
    }
}

/// The `make leptos-gates` smoke chain: every editor smoke in glob order, first failure stops
/// (the Makefile `set -e` semantics). Returns the first non-zero code, else 0.
pub async fn editor_suite(dist: Option<String>) -> Result<u8> {
    for name in EDITOR_SUITE {
        println!("== gate smoke {name}");
        let code = run_smoke(name, dist.clone(), None).await?;
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}
