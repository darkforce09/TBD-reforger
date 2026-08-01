//! T-661 — the editor-capture harness, ported from the Node scripts under `tools/editor-capture/`.
//!
//! This is the operator's headless screenshot rig for the live Mission Creator: it drives the
//! running editor over CDP and captures both the DOM chrome (`Page.captureScreenshot`) and the
//! `wgpu` map (`canvas.toDataURL`). It replaces three files that tripped the language gates:
//!
//!   * `cdp2.mjs`         → [`shot`]      (navigate + boot-overlay poll + diagnostics + capture)
//!   * `zoomsweep.mjs`    → [`zoomsweep`] (per-zoom `__editorCamSet` + canvas read)
//!   * `run_shot_gpu.sh`  → absorbed: chrome launch (ANGLE/Vulkan on the real device), the KB-002
//!     font-cache workaround, the CDP wait and the teardown are all `cdp::launch_with_gpu` +
//!     `Browser::shutdown` now. `crop.sh` → [`crop`] (the `image` crate, already a dependency).
//!
//! The hard-won environment knowledge lives in `tools/editor-capture/README.md` — the three
//! non-obvious things (writable `XDG_CACHE_HOME`, `--use-angle=vulkan`, read the map off the canvas
//! not the compositor) are preserved here. `cdp::launch_with_gpu(_, GpuBackend::Vulkan, _)` carries
//! the vulkan flags and pins `XDG_CACHE_HOME` on the chromium child (KB-002); this module carries
//! the driver logic.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use serde_json::{Value, json};

use crate::cdp::{self, GpuBackend, Page, sleep_ms};

/// The capture viewport, from `cdp2.mjs`'s `Emulation.setDeviceMetricsOverride` (1920×1080, dsf=1).
/// Deliberately larger than the gate harness's 1440×900 — these shots frame the whole editor.
const CAPTURE_VIEWPORT: (u32, u32) = (1920, 1080);

/// Debug port for the capture chromium. Distinct from the gate harness ports (9337/9341/9399) so a
/// capture never collides with a concurrent gate run; matches the `run_shot_gpu.sh` / `cdp2.mjs`
/// default of 9222.
const CAPTURE_DEBUG_PORT: u16 = 9222;

/// The boot-overlay selector, verbatim from `cdp2.mjs` / `zoomsweep.mjs`. The editor boots behind a
/// full-bleed loading overlay; screenshotting before it clears captures the spinner, not the map.
const OVERLAY_SELECTOR: &str =
    r#"[class*="animate-overlay-fade"], [class*="z-50"][class*="backdrop-blur"]"#;

/// An all-black canvas still encodes to a valid PNG — just a tiny one. `cdp2.mjs` uses byte count as
/// the tell: ~45 KB is a black rectangle, ~3.7 MB is the real map. Refuse to write below this.
const CANVAS_MIN_BYTES: usize = 20_000;

/// One capture "step": a URL to navigate to and how long (ms) to settle afterwards. The `cdp2.mjs`
/// positional pairs `<url> <waitMs> [url waitMs ...]`.
#[derive(Clone, Debug)]
pub struct Step {
    pub url: String,
    pub wait_ms: u64,
}

/// Behaviour switches that were environment variables on the Node scripts (`CANVAS_CAPTURE`,
/// `FORCE_HIDE_OVERLAY`). Named so the `capture` CLI can expose them as flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShotOptions {
    /// `CANVAS_CAPTURE=1` — also write `<out>_canvas.png` via `toDataURL`. **Required to see the
    /// map**: headless chrome's compositor returns a black GPU layer even when the engine renders.
    pub canvas_capture: bool,
    /// `FORCE_HIDE_OVERLAY=1` — remove the boot overlay from the DOM before capturing, to read the
    /// chrome behind a stuck boot. The map may be blank — that is itself the finding.
    pub force_hide_overlay: bool,
}

/// Console/log/exception lines the page emitted, captured the way `cdp2.mjs`'s message listener did
/// (for the "last 40" diagnostics dump). Kept behind an `Arc<Mutex>` because CDP events arrive on
/// the reader task.
type ConsoleLog = Arc<StdMutex<Vec<String>>>;

/// `location.href`, `document.readyState`, etc. — `Runtime.evaluate` returning the value, with the
/// same "never throw, fold the error into the string" contract as `cdp2.mjs`'s `evalJs`.
async fn eval_js(page: &Page, expr: &str) -> String {
    match page.evaluate(expr, false).await {
        Ok(Value::String(s)) => s,
        Ok(Value::Null) => "null".to_string(),
        Ok(v) => v.to_string(),
        Err(e) => format!("<eval failed: {e}>"),
    }
}

/// Attach the console/log/exception taps that `cdp2.mjs` installs on the raw WS, using the CDP
/// plumbing's persistent-event streams. Mirrors that script's arg-joining
/// (`a.value ?? a.description ?? a.type`).
async fn attach_console_capture(page: &Arc<Page>) -> ConsoleLog {
    let lines: ConsoleLog = Arc::new(StdMutex::new(Vec::new()));

    let mut console = page.on_event("Runtime.consoleAPICalled").await;
    let mut log = page.on_event("Log.entryAdded").await;
    let mut exc = page.on_event("Runtime.exceptionThrown").await;

    let push = |lines: &ConsoleLog, s: String| {
        if let Ok(mut v) = lines.lock() {
            v.push(s);
        }
    };

    let l1 = Arc::clone(&lines);
    tokio::spawn(async move {
        while let Some(p) = console.recv().await {
            let ty = p["type"].as_str().unwrap_or("log");
            let txt = p["args"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|arg| {
                            // JS `a.value ?? a.description ?? a.type`.
                            if !arg["value"].is_null() {
                                json_scalar(&arg["value"])
                            } else if let Some(d) = arg["description"].as_str() {
                                d.to_string()
                            } else {
                                arg["type"].as_str().unwrap_or("").to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            push(&l1, format!("[{ty}] {txt}"));
        }
    });

    let l2 = Arc::clone(&lines);
    tokio::spawn(async move {
        while let Some(p) = log.recv().await {
            let level = p["entry"]["level"].as_str().unwrap_or("info");
            let text = p["entry"]["text"].as_str().unwrap_or("");
            push(&l2, format!("[{level}] {text}"));
        }
    });

    let l3 = Arc::clone(&lines);
    tokio::spawn(async move {
        while let Some(p) = exc.recv().await {
            let d = &p["exceptionDetails"];
            let text = d["text"].as_str().unwrap_or("");
            let desc = d["exception"]["description"].as_str().unwrap_or("");
            push(&l3, format!("[EXCEPTION] {text} {desc}"));
        }
    });

    lines
}

/// JS `String(value)` for the scalar arg values CDP returns by value (`cdp2.mjs` joined these raw).
fn json_scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

/// Poll the boot overlay out, then dump the same page diagnostics `cdp2.mjs` logs. Returns nothing;
/// everything goes to stderr, exactly as the Node driver did (stdout stays clean for the caller).
async fn poll_overlay_and_diagnostics(page: &Page, console: &ConsoleLog) {
    // The editor boots behind a full-bleed loading overlay. Poll it out rather than guessing a
    // fixed wait (cdp2.mjs: 25 one-second iterations).
    for i in 0..25u32 {
        let state = eval_js(
            page,
            &format!(
                r#"(() => {{ const o = document.querySelector('{OVERLAY_SELECTOR}'); return o ? (o.innerText || '').replace(/\s+/g, ' ').trim() : null; }})()"#
            ),
        )
        .await;
        if state == "null" {
            eprintln!("  overlay cleared after {i}s");
            break;
        }
        if i % 10 == 0 {
            eprintln!("  [{i}s] overlay: {state}");
        }
        if i == 24 {
            eprintln!("  overlay STILL PRESENT after 25s: {state}");
        }
        sleep_ms(1000).await;
    }

    eprintln!(
        "  readyState: {}",
        eval_js(page, "document.readyState").await
    );
    eprintln!("  title     : {}", eval_js(page, "document.title").await);
    eprintln!(
        "  canvases  : {}",
        eval_js(
            page,
            r#"JSON.stringify([...document.querySelectorAll("canvas")].map(c=>({w:c.width,h:c.height,cls:c.className})))"#
        )
        .await
    );
    eprintln!(
        "  bodyTextLen: {}",
        eval_js(page, "document.body.innerText.length").await
    );
    eprintln!(
        "  bodyHead  : {:?}",
        eval_js(page, "document.body.innerText.slice(0,600)").await
    );
    eprintln!(
        "  overlay   : {}",
        eval_js(
            page,
            r#"JSON.stringify([...document.querySelectorAll("[class*=overlay],[class*=loading],[role=progressbar]")].slice(0,5).map(e=>e.className+" :: "+(e.innerText||"").slice(0,80)))"#
        )
        .await
    );

    eprintln!("  ---- console (last 40) ----");
    let tail: Vec<String> = console
        .lock()
        .map(|v| v.iter().rev().take(40).rev().cloned().collect())
        .unwrap_or_default();
    for l in tail {
        eprintln!("  {}", l.chars().take(300).collect::<String>());
    }
    eprintln!("  ---------------------------");
}

/// Read the WebGPU canvas directly via `toDataURL` and write `<out>_canvas.png` if it is not blank.
///
/// Headless chrome's compositor can fail to present the GPU layer ("Failed to initialize vulkan
/// surface") while the engine itself renders fine — that path yields a black map, indistinguishable
/// from a dead engine. `toDataURL` asks the canvas for its own pixels and sidesteps the compositor
/// entirely (README §3). The byte count is the tell (`CANVAS_MIN_BYTES`).
async fn capture_canvas(page: &Page, out: &Path) -> Result<()> {
    let data_url = eval_js(
        page,
        r#"(() => { const c = document.querySelector('canvas'); if (!c) return 'NO_CANVAS'; try { return c.toDataURL('image/png'); } catch (e) { return 'ERR: ' + e.message; } })()"#,
    )
    .await;
    const PREFIX: &str = "data:image/png;base64,";
    if let Some(b64) = data_url.strip_prefix(PREFIX) {
        let buf = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .context("decode canvas data URL")?;
        eprintln!("  canvas toDataURL → {} bytes", buf.len());
        if buf.len() > CANVAS_MIN_BYTES {
            let canvas_out = canvas_path(out);
            std::fs::write(&canvas_out, &buf)
                .with_context(|| format!("write {}", canvas_out.display()))?;
            eprintln!("  wrote {}", canvas_out.display());
        } else {
            eprintln!("  canvas looks blank (too few bytes) — not written");
        }
    } else {
        eprintln!(
            "  canvas capture failed: {}",
            data_url.chars().take(120).collect::<String>()
        );
    }
    Ok(())
}

/// `<out>.png` → `<out>_canvas.png` (`cdp2.mjs`'s `out.replace(/\.png$/, '_canvas.png')`).
fn canvas_path(out: &Path) -> PathBuf {
    let s = out.to_string_lossy();
    let stem = s.strip_suffix(".png").unwrap_or(&s);
    PathBuf::from(format!("{stem}_canvas.png"))
}

/// `Page.captureScreenshot` with the given params, writing `out` on success. Returns whether it
/// wrote (the `cdp2.mjs` `shoot` helper — used to build the fallback chain).
async fn shoot(page: &Page, out: &Path, params: Value, label: &str) -> bool {
    match page.send("Page.captureScreenshot", params).await {
        Ok(r) => {
            let data = r["data"].as_str().unwrap_or_default();
            match base64::engine::general_purpose::STANDARD.decode(data) {
                Ok(bytes) => match std::fs::write(out, &bytes) {
                    Ok(()) => {
                        eprintln!("OK via {label} → {}", out.display());
                        true
                    }
                    Err(e) => {
                        eprintln!("FAIL {label}: write {}: {e}", out.display());
                        false
                    }
                },
                Err(e) => {
                    eprintln!("FAIL {label}: decode: {e}");
                    false
                }
            }
        }
        Err(e) => {
            eprintln!("FAIL {label}: {e}");
            false
        }
    }
}

/// `cdp2.mjs` — navigate through the steps, poll the boot overlay out, dump diagnostics, then
/// capture the chrome (and, with `canvas_capture`, the map).
///
/// Launches chromium itself (ANGLE/Vulkan on the real device) and tears it down — absorbing what
/// `run_shot_gpu.sh` did around the driver. Returns the process exit code (0 on success).
pub async fn shot(out: &Path, steps: &[Step], opts: ShotOptions) -> Result<u8> {
    if steps.is_empty() {
        eprintln!("capture shot: need at least one <url> <waitMs> step");
        return Ok(2);
    }

    // ANGLE/Vulkan on the real device — the only backend the live wgpu engine boots on (README §2).
    // launch_with_gpu pins XDG_CACHE_HOME on the chromium child, carrying the KB-002 fontconfig
    // workaround (README §1) that run_shot_gpu.sh did with `export XDG_CACHE_HOME=…`.
    let browser = cdp::launch_with_gpu(CAPTURE_DEBUG_PORT, GpuBackend::Vulkan, &[]).await?;
    // Open a page with no initial navigation; enable Log so `Log.entryAdded` reaches the tap.
    let page = Arc::new(cdp::new_page(&browser, None, &[]).await?);
    page.send("Log.enable", json!({})).await?;
    // Override the CDP default (1440×900) with the capture viewport BEFORE navigating.
    page.set_viewport(CAPTURE_VIEWPORT.0, CAPTURE_VIEWPORT.1)
        .await?;
    let console = attach_console_capture(&page).await;

    for step in steps {
        eprintln!("→ {} (wait {}ms)", step.url, step.wait_ms);
        // Navigate WITHOUT waiting on Page.loadEventFired: the editor's SPA boot keeps loading long
        // past the load event, so cdp2.mjs relied on the fixed per-step sleep, not the load event.
        page.send("Page.navigate", json!({ "url": step.url }))
            .await?;
        sleep_ms(step.wait_ms).await;
        eprintln!("  href      : {}", eval_js(&page, "location.href").await);
    }

    poll_overlay_and_diagnostics(&page, &console).await;

    // If the boot overlay never cleared, take it out of the DOM so the shot shows the chrome
    // underneath. The map may be blank — that is itself the finding — but the panels are readable.
    if opts.force_hide_overlay {
        let removed = eval_js(
            &page,
            &format!(
                r#"(() => {{ const els = [...document.querySelectorAll('{OVERLAY_SELECTOR}')]; els.forEach(e => e.remove()); return els.length; }})()"#
            ),
        )
        .await;
        eprintln!("  force-removed {removed} overlay element(s)");
        sleep_ms(1500).await;
    }

    if opts.canvas_capture {
        capture_canvas(&page, out).await?;
    }

    // fromSurface:true FIRST — the only path that composites the WebGPU canvas. fromSurface:false
    // renders DOM only and hands back a black map, which looks exactly like a broken engine when the
    // engine is fine (README §3). Fall through to jpeg as a last resort.
    let wrote = shoot(
        &page,
        out,
        json!({ "format": "png", "fromSurface": true, "captureBeyondViewport": false }),
        "fromSurface:true",
    )
    .await
        || shoot(
            &page,
            out,
            json!({ "format": "png", "captureBeyondViewport": false, "fromSurface": false }),
            "fromSurface:false",
        )
        .await
        || shoot(
            &page,
            out,
            json!({ "format": "jpeg", "quality": 80, "fromSurface": false }),
            "jpeg/fromSurface:false",
        )
        .await;

    browser.shutdown().await;
    Ok(u8::from(!wrote))
}

/// `zoomsweep.mjs` — boot the editor, then for each zoom set the camera and read the wgpu canvas.
///
/// Boots via the hardcoded two-step dev-login → edit navigation (6 s + 15 s settle), waits the boot
/// overlay out (60 s cap), then for each zoom calls `__editorCamSet(tx, ty, z)`, settles 3.5 s, and
/// writes `<prefix>_z<z>.png` from `toDataURL`.
///
/// ── KNOWN CAPTURE-HARNESS ARTIFACT (do NOT file/fix) ─────────────────────────────────────────
/// Under headless ANGLE/Vulkan, `window.__editorCamSet(...)` **panics the render engine**
/// (`wgpu-29.0.4/src/backend/webgpu.rs`), which poisons the `RefCell` in `mission_editor.rs`'s
/// `cam_set`; every subsequent `__editorCam()` returns `undefined` and every canvas read returns a
/// ~44 KB **black rectangle** instead of the ~3.7 MB map. Reproduced across multiple runs and zooms,
/// inside and outside the height-label band — it is NOT a zoom-range guard. It is confirmed **fine
/// in a real browser** (147 FPS), so this is a headless artifact of the vulkan surface, not an
/// engine bug. See `.ai/artifacts/parity/camset_panic_finding.md`. This tool carries the finding as
/// a comment; it does not work around the panic (headless zoom would need `mouseWheel` events
/// instead), and no ticket is filed against the engine from here.
pub async fn zoomsweep(out_prefix: &str, mission_id: &str, zooms: &[f64]) -> Result<u8> {
    // Everon centre-ish; the peaks worth reading sit inland (zoomsweep.mjs `[TX, TY] = [6400, 6400]`).
    const TX: i64 = 6400;
    const TY: i64 = 6400;

    let browser = cdp::launch_with_gpu(CAPTURE_DEBUG_PORT, GpuBackend::Vulkan, &[]).await?;
    let page = Arc::new(cdp::new_page(&browser, None, &[]).await?);
    page.set_viewport(CAPTURE_VIEWPORT.0, CAPTURE_VIEWPORT.1)
        .await?;

    // dev-login (admin), then the mission edit route — the two fixed navigations from zoomsweep.mjs.
    page.send(
        "Page.navigate",
        json!({ "url": "http://localhost:8080/api/v1/auth/dev-login?role=admin" }),
    )
    .await?;
    sleep_ms(6000).await;
    page.send(
        "Page.navigate",
        json!({ "url": format!("http://localhost:3000/missions/{mission_id}/edit") }),
    )
    .await?;
    sleep_ms(15000).await;

    // Boot overlay must clear or the canvas is not yet drawing the world (60 s cap).
    for i in 0..60u32 {
        let present = eval_js(
            &page,
            &format!(r#"document.querySelector('{OVERLAY_SELECTOR}') ? 1 : 0"#),
        )
        .await;
        if present == "0" {
            eprintln!("overlay cleared after {i}s");
            break;
        }
        sleep_ms(1000).await;
    }

    eprintln!(
        "cam api: {} / {}",
        eval_js(&page, "typeof window.__editorCamSet").await,
        eval_js(&page, "typeof window.__editorCam").await
    );
    eprintln!(
        "height layer on: {}",
        eval_js(
            &page,
            r#"(localStorage.getItem('tbd-mc-world-layers')||'(default)')"#
        )
        .await
    );

    let mut wrote_any = false;
    for &z in zooms {
        // NOTE: this call is the one documented to panic the engine headless (see the artifact note
        // on this fn). It is issued verbatim as zoomsweep.mjs did; a black canvas below is that
        // artifact, not a driver bug.
        let _ = eval_js(&page, &format!("window.__editorCamSet({TX}, {TY}, {z})")).await;
        sleep_ms(3500).await;
        let cam = eval_js(&page, "JSON.stringify(window.__editorCam())").await;
        let data_url = eval_js(
            &page,
            "document.querySelector('canvas').toDataURL('image/png')",
        )
        .await;
        const PREFIX: &str = "data:image/png;base64,";
        if let Some(b64) = data_url.strip_prefix(PREFIX) {
            let buf = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .context("decode canvas data URL")?;
            // z formatting: `String(z).replace('.', 'p').replace('-', 'm')`.
            let ztag = z.to_string().replacen('.', "p", 1).replacen('-', "m", 1);
            let f = PathBuf::from(format!("{out_prefix}_z{ztag}.png"));
            std::fs::write(&f, &buf).with_context(|| format!("write {}", f.display()))?;
            eprintln!("z={z}  {} bytes  cam={cam}  -> {}", buf.len(), f.display());
            wrote_any = true;
        } else {
            eprintln!(
                "z={z}  CAPTURE FAILED: {}",
                data_url.chars().take(100).collect::<String>()
            );
        }
    }

    browser.shutdown().await;
    Ok(u8::from(!wrote_any))
}

/// `crop.sh` — crop a rectangle out of a screenshot (and optionally nearest-neighbour upscale it) so
/// it can be Read at full detail. Ported to the `image` crate (already a dependency); no ffmpeg and
/// no python.
///
/// The Read tool downscales any image over ~190,000 px, which makes small UI text unreadable — keep
/// `w * h * scale²` under that. This warns (does not fail) when the output would exceed it, matching
/// the shell tool's behaviour.
pub fn crop(img_path: &Path, x: u32, y: u32, w: u32, h: u32, scale: u32, out: &Path) -> Result<u8> {
    use image::GenericImageView as _;

    let img = image::ImageReader::open(img_path)
        .with_context(|| format!("open {}", img_path.display()))?
        .decode()
        .with_context(|| format!("decode {}", img_path.display()))?;
    let (iw, ih) = img.dimensions();
    if x + w > iw || y + h > ih {
        return Err(anyhow!(
            "crop {w}x{h}+{x}+{y} out of bounds for {iw}x{ih} image {}",
            img_path.display()
        ));
    }
    // ffmpeg `crop=W:H:X:Y` — the sub-rectangle at (x, y).
    let cropped = img.crop_imm(x, y, w, h);
    let (ow, oh) = (w * scale, h * scale);
    let scaled = if scale == 1 {
        cropped
    } else {
        // ffmpeg `scale=iw*S:ih*S:flags=neighbor` — integer upscale, nearest neighbour (crisp text).
        cropped.resize_exact(ow, oh, image::imageops::FilterType::Nearest)
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    scaled
        .save(out)
        .with_context(|| format!("write {}", out.display()))?;

    let px = (ow as u64) * (oh as u64);
    println!("{}  ({ow}x{oh} = {px}px)", out.display());
    if px > 190_000 {
        eprintln!(
            "WARNING: over ~190000px, Read will downscale this. Use a smaller region or scale."
        );
    }
    Ok(0)
}
