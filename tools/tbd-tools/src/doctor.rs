//! `gate doctor` — a fail-fast preflight for the editor CDP smokes (T-177).
//!
//! Why this exists: the editor gate used to depend on **unpinned external state** (a floating
//! playwright chromium, a floating toolchain, no committed runner — `t151_10` D-06), and when the
//! environment drifted it **hung 130 s** with a cryptic `cdp: ws call timed out (Runtime.evaluate)`
//! instead of failing fast — turning a routine ticket into a multi-hour reverse-engineering session.
//! (The actual T-177 root cause: `chrome-headless-shell`'s stubbed Skia font manager FATAL-crashes on
//! per-character font fallback; fixed in [`crate::cdp::find_chromium`].)
//!
//! `gate doctor` runs before the suite (a prerequisite of `make leptos-gates`) and, in ~15 s:
//! validates the resolved chromium + toolchain against the committed pins (`gate-env.json`), checks
//! free RAM + orphaned chrome processes, and runs a **short-timeout editor liveness probe** that
//! FAILS with an actionable message + a native-stack hint instead of the 130 s hang. See
//! `docs/website/EDITOR_GATE_RUNBOOK.md`.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

use crate::cdp;
use crate::serve::{ServeConfig, start_server};

const EDIT_PATH: &str = "/missions/smoke/edit?force=webgl&sat=preview";
const DEFAULT_DIST: &str = "apps/website/frontend/dist";

/// Load the committed pin manifest (crate-local, deterministic — no cwd dependence).
fn load_manifest() -> Result<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("gate-env.json");
    let raw = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).context("parse gate-env.json")
}

/// Run `<bin> --version` and return its first stdout line (or None if it can't run).
fn tool_version(bin: &str, arg: &str) -> Option<String> {
    let out = std::process::Command::new(bin).arg(arg).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines().next().map(str::trim).map(str::to_string)
}

/// `gate doctor`. `strict` promotes drift warnings to failures; a liveness failure is ALWAYS a hard
/// fail (exit 1) so `make leptos-gates` is blocked with a diagnosis rather than wedging.
pub async fn run(dist: Option<String>, strict: bool) -> Result<u8> {
    println!("== gate doctor (T-177 editor-gate preflight)");
    let manifest = match load_manifest() {
        Ok(m) => Some(m),
        Err(e) => {
            println!("  ! gate-env.json unreadable: {e}");
            None
        }
    };
    let env = manifest.as_ref();
    let mut warnings = 0u32;
    warnings += check_chromium(env);
    warnings += check_toolchain(env);
    warnings += check_resources(env);

    let dist = dist.unwrap_or_else(|| DEFAULT_DIST.to_string());
    let live = liveness_probe(&dist, env).await;
    let live_ok = match live {
        Ok(true) => {
            println!("  ✓ liveness    editor page booted; evaluate responsive");
            true
        }
        Ok(false) => {
            println!("  ✗ liveness    editor page did not become ready within the budget");
            false
        }
        Err(e) => {
            println!("  ✗ liveness    {e}");
            false
        }
    };

    if !live_ok {
        print_wedge_hint();
        println!("== gate doctor: FAIL — the editor page is unhealthy; the gate would wedge");
        return Ok(1);
    }
    if strict && warnings > 0 {
        println!("== gate doctor: FAIL (strict) — {warnings} pin/env warning(s)");
        return Ok(1);
    }
    println!("== gate doctor: OK — {warnings} warning(s)");
    Ok(0)
}

/// Resolve chromium + verify it's the full build at the pinned version (not the crashing shell).
fn check_chromium(env: Option<&Value>) -> u32 {
    let Some(bin) = cdp::find_chromium() else {
        println!(
            "  ✗ chromium    not found (set CHROME_HEADLESS_SHELL or install the playwright chromium)"
        );
        return 1;
    };
    let mut warn = 0;
    if cdp::is_headless_shell(&bin) {
        println!(
            "  ! chromium    resolved to chrome-headless-shell — it FATAL-crashes on font fallback; \
             install the full `chrome` build (chrome-linux64/chrome). {}",
            bin.display()
        );
        warn += 1;
    }
    let version = tool_version(&bin.to_string_lossy(), "--version").unwrap_or_default();
    let want = env
        .and_then(|e| e["chromium"]["version"].as_str())
        .unwrap_or("");
    if !want.is_empty() && !version.contains(want) {
        println!(
            "  ! chromium    version drift: have '{version}', pinned '{want}' (gate-env.json)"
        );
        warn += 1;
    }
    if warn == 0 {
        println!("  ✓ chromium    {version}");
    }
    warn
}

/// Toolchain versions vs the pins (rustc / trunk / wasm-bindgen best-effort).
fn check_toolchain(env: Option<&Value>) -> u32 {
    let mut warn = 0;
    let checks = [("rustc", "rustc", "rustc"), ("trunk", "trunk", "trunk")];
    for (label, bin, key) in checks {
        let have = tool_version(bin, "--version").unwrap_or_default();
        let want = env.and_then(|e| e["toolchain"][key].as_str()).unwrap_or("");
        if want.is_empty() {
            continue;
        }
        if have.contains(want) {
            println!("  ✓ {label:<11} {have}");
        } else {
            println!("  ! {label:<11} drift: have '{have}', pinned '{want}'");
            warn += 1;
        }
    }
    warn
}

/// Free RAM vs the floor + a scan for orphaned chrome processes (a documented wedge trigger:
/// `cdp.rs` — a prior crashed run's orphans peg every core under software GL and starve the next
/// smoke's `Runtime.evaluate`).
fn check_resources(env: Option<&Value>) -> u32 {
    let mut warn = 0;
    let floor = env
        .and_then(|e| e["limits"]["min_mem_available_mib"].as_u64())
        .unwrap_or(1024);
    if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
        let avail_mib = meminfo
            .lines()
            .find_map(|l| l.strip_prefix("MemAvailable:"))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|kb| kb.parse::<u64>().ok())
            .map(|kb| kb / 1024)
            .unwrap_or(0);
        if avail_mib < floor {
            println!(
                "  ! memory      {avail_mib} MiB available < {floor} MiB floor (SwiftShader may thrash)"
            );
            warn += 1;
        } else {
            println!("  ✓ memory      {avail_mib} MiB available");
        }
    }
    let orphans = count_chrome_processes();
    if orphans > 0 {
        println!(
            "  ! processes   {orphans} stray chrome process(es) — kill them (they starve the gate): pkill -9 -f chrome-headless-shell; pkill -9 -f 'chrome-linux64/chrome'"
        );
        warn += 1;
    } else {
        println!("  ✓ processes   no stray chrome");
    }
    warn
}

/// Count live chromium processes by `/proc/*/comm` (avoids matching our own command line).
fn count_chrome_processes() -> u32 {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return 0;
    };
    let mut n = 0;
    for e in entries.flatten() {
        let comm_path = e.path().join("comm");
        if let Ok(comm) = std::fs::read_to_string(&comm_path) {
            let comm = comm.trim();
            if comm == "chrome-headless" || comm == "chrome" {
                n += 1;
            }
        }
    }
    n
}

/// The ~15 s editor liveness probe: serve the dist, launch chromium, navigate the editor, and run a
/// short-timeout `1+1` then a bounded readiness poll. A pegged/dead main thread fails here in seconds
/// (via [`cdp::Page::evaluate_with_timeout`]) instead of the suite's 130 s hang. The whole probe is
/// wrapped in an overall timeout so it can never inherit the wedge it exists to catch.
async fn liveness_probe(dist: &str, env: Option<&Value>) -> Result<bool> {
    let budget = env
        .and_then(|e| e["limits"]["liveness_timeout_secs"].as_u64())
        .unwrap_or(15);
    let srv = start_server(
        ServeConfig {
            dir: PathBuf::from(dist),
            api_proxy: Some("http://127.0.0.1:8080".to_string()),
            map_assets_dir: Some(PathBuf::from("packages/map-assets")),
        },
        5299,
    )
    .await?;
    let browser = cdp::launch(9399, &[]).await?;
    let page = cdp::new_page(&browser, None, &[]).await?;
    let url = format!("http://localhost:{}{}", srv.port, EDIT_PATH);

    let probe = async {
        page.send("Runtime.enable", json!({})).await?;
        page.navigate(&url).await?;
        let short = Duration::from_secs(8);
        if page
            .evaluate_with_timeout("1+1", false, short)
            .await?
            .as_i64()
            != Some(2)
        {
            return Ok(false);
        }
        for _ in 0..budget {
            let ready = page
                .evaluate_with_timeout(
                    "!!document.querySelector('canvas') && typeof window.__editorCam === 'function'",
                    false,
                    short,
                )
                .await?;
            if ready.as_bool() == Some(true) {
                return Ok::<bool, anyhow::Error>(true);
            }
            cdp::sleep_ms(1000).await;
        }
        Ok(false)
    };
    // Hard cap so a wedge (dead renderer / pegged main thread) can't exceed the budget.
    let result = match tokio::time::timeout(Duration::from_secs(budget + 12), probe).await {
        Ok(inner) => inner,
        Err(_) => Ok(false),
    };
    browser.shutdown().await;
    srv.close().await;
    result
}

fn print_wedge_hint() {
    println!("  ─ the editor page wedged or crashed the headless renderer. To diagnose:");
    println!(
        "    • capture chrome's own stderr:  launch chromium with --enable-logging=stderr --v=1"
    );
    println!("      on the served editor and grep for FATAL/SkFontMgr/Received signal");
    println!(
        "    • native stack of the hung renderer:  pgrep -f 'type=renderer' | while read p; do"
    );
    println!(
        "        gdb -p $p -batch -ex 'thread apply all bt'; done   (or /proc/<pid>/task/*/stat)"
    );
    println!(
        "    • verify the resolved chromium is the FULL chrome build (not chrome-headless-shell)"
    );
    println!(
        "    • see docs/website/EDITOR_GATE_RUNBOOK.md (known wedge modes + the P0–P6 recipe)"
    );
}
