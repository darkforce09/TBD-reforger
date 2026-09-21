use super::*;
use crate::repository_layout::MapAssetMounts;

/// **Can chromium resolve a font at all?**
///
/// Launch chromium on `about:blank`, watch its own log for [`NO_FONT_MARKER`], kill it. That one line
/// is the difference between "the editor gate works" and "the browser process SIGABRTs the first time
/// the editor asks for a fallback glyph" — see [`ensure_gate_font_cache`]. It names the cause instead
/// of leaving a CDP timeout to be reverse-engineered.
///
/// Three things this must not do, all three measured while building it:
///   * **Do not `Command::output()`.** It waits for EOF on the stdout/stderr *pipes*, which chrome's
///     zygote/crashpad children inherit — it can block long after the browser itself exited.
///   * **Do not wait for chromium to exit.** It does not reliably exit on a box without outbound
///     Google access (it sits retrying `google_apis/gcm` registration). Both streams go to one file,
///     the file is polled, then the process **group** is SIGKILLed.
///   * **Do not use `--dump-dom` as the healthy signal.** For the same reason: the DOM is printed at
///     exit, so on this box a healthy run prints nothing at all. The verdict is the *marker*.
///
/// Timing measured here: chromium emits the font errors ~250–400 ms after launch, before the page
/// exists at all, so [`FONT_PROBE_WINDOW_MS`] of silence is a sound "fonts are fine".
///
/// This probe deliberately **inherits** `XDG_CACHE_HOME` instead of passing its own via
/// `Command::env`. It has to measure what every other browser launch in the run will get, not what
/// [`ensure_gate_font_cache`] intended; a probe that forces its own cache would pass while the
/// smokes inherited a poisoned one, which is the exact failure this check exists to catch.
pub(super) async fn check_fonts() -> FontProbe {
    let Some(bin) = cdp::find_chromium() else {
        return FontProbe::Ok; // already reported by check_chromium
    };
    let tag = format!("tbd-fontprobe-{}", std::process::id());
    let profile = std::env::temp_dir().join(&tag);
    let log_path = std::env::temp_dir().join(format!("{tag}.log"));
    let _ = std::fs::remove_dir_all(&profile);
    let Ok(log) = std::fs::File::create(&log_path) else {
        println!("  ! fonts       could not open the font-probe log");
        return FontProbe::Inconclusive;
    };
    let Ok(log2) = log.try_clone() else {
        println!("  ! fonts       could not open the font-probe log");
        return FontProbe::Inconclusive;
    };
    let mut cmd = tokio::process::Command::new(&bin);
    if !cdp::is_headless_shell(&bin) {
        cmd.arg("--headless=new");
    }
    let spawned = cmd
        .args(["--no-sandbox", "--disable-gpu-sandbox", "about:blank"])
        .arg(format!("--user-data-dir={}", profile.display()))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(log))
        .stderr(std::process::Stdio::from(log2))
        .process_group(0)
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            println!("  ! fonts       could not run the chromium font probe: {e}");
            return FontProbe::Inconclusive;
        }
    };
    let mut broken = false;
    let mut died_early = false;
    for _ in 0..(FONT_PROBE_WINDOW_MS / 250) {
        if std::fs::read_to_string(&log_path)
            .unwrap_or_default()
            .contains(NO_FONT_MARKER)
        {
            broken = true;
            break;
        }
        if matches!(child.try_wait(), Ok(Some(_))) {
            // Exited before the window closed — chromium never really came up (a bad binary, a
            // profile it could not lock). Say so rather than reading silence as health.
            died_early = true;
            break;
        }
        cdp::sleep_ms(250).await;
    }
    if let Some(p) = child.id() {
        // SAFETY: signalling the group we created above (`process_group(0)`), never our own.
        unsafe { libc::kill(-(p as i32), libc::SIGKILL) };
    }
    let _ = child.wait().await;
    let log_text = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&profile);
    let _ = std::fs::remove_file(&log_path);
    if broken {
        // Name the cache on the FAILURE path too. Which directory chromium was reading is
        // the first thing anyone needs and the one thing the old message omitted.
        println!(
            "  ✗ fonts       chromium resolves NO font ('{NO_FONT_MARKER}') — the editor page will \
             SIGABRT the browser on its first fallback glyph"
        );
        println!("                cache: {}", font_cache_report());
        return FontProbe::NoFonts;
    }
    if died_early {
        println!(
            "  ! fonts       chromium exited during the font probe; its log said: {}",
            log_text.lines().last().unwrap_or("(nothing)")
        );
        println!("                cache: {}", font_cache_report());
        return FontProbe::Inconclusive;
    }
    println!(
        "  ✓ fonts       chromium resolves fonts (cache: {})",
        font_cache_report()
    );
    FontProbe::Ok
}

/// The dist has to exist. A missing `--dist` used to serve 500s to every request and then fail on
/// **liveness**, which reads as "the editor page wedged" — the wrong diagnosis for a wrong path
/// (this is what a `gate doctor` run inside a worktree with no build looks like).
pub(super) fn check_dist(dist: &str) -> u32 {
    let index = PathBuf::from(dist).join("index.html");
    if index.exists() {
        println!("  ✓ dist        {dist}");
        0
    } else {
        println!(
            "  ! dist        no {} — every request will 500 and liveness will fail for the wrong \
             reason (pass --dist, or build one)",
            index.display()
        );
        1
    }
}

/// Count live chromium processes by `/proc/*/comm` (avoids matching our own command line).
pub(super) fn count_chrome_processes() -> u32 {
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
///
/// Whatever the outcome, before reporting we ask the browser's own `/json/version` whether it
/// is still alive. A dead endpoint turns "timed out" into "crashed", which is the difference between
/// hunting the app and hunting the environment.
pub(super) async fn liveness_probe(dist: &str, env: Option<&Value>) -> Result<Liveness> {
    ensure_gate_font_cache();
    let budget = env
        .and_then(|e| e["limits"]["liveness_timeout_secs"].as_u64())
        .unwrap_or(15);
    // Liveness must enter the editor. Seed the same admin `tbd-auth` blob the
    // smokes use, and do NOT proxy `/api` to :8080 here: a failed refresh against a live API
    // clears the seeded session and the probe never sees `__editorCam` (measured).
    let srv = start_server(
        ServeConfig {
            dir: PathBuf::from(dist),
            api_proxy: None,
            // A relative serving directory resolves against the gate's working directory.
            map_assets: Some(MapAssetMounts::from_root(Path::new(""))),
        },
        5299,
    )
    .await?;
    let browser = cdp::launch(9399, &[]).await?;
    let auth_seed = crate::browser_testing::dom_oracle::seed_script()?;
    let page = cdp::new_page(&browser, None, &[auth_seed.as_str()]).await?;
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
    // Ask the browser itself. `Runtime.evaluate` "timing out" is what a SIGABRT'd browser
    // looks like from the client: the ws reader sees a close, the pending call is never answered.
    let alive = browser
        .http
        .get(format!(
            "http://127.0.0.1:{}/json/version",
            browser.debug_port
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success());
    browser.shutdown().await;
    srv.close().await;
    match result {
        Ok(true) => Ok(Liveness::Ready),
        Ok(false) if !alive => Ok(Liveness::BrowserDied),
        Ok(false) => Ok(Liveness::NotReady),
        Err(e) if !alive => {
            println!("  ─ (the probe error under a dead browser was: {e})");
            Ok(Liveness::BrowserDied)
        }
        Err(e) => Err(e),
    }
}

/// The font remedy, printed when `check_fonts` says chromium has no fonts.
pub(super) fn print_font_wedge_hint() {
    println!(
        "  ─ chromium could not resolve a single font. That is the font wedge: the editor page's"
    );
    println!(
        "    first per-character fallback hits SkFontMgr_FontConfigInterface.cpp:163 SK_ABORT and"
    );
    println!("    kills the BROWSER process, which the harness can only see as a CDP timeout.");
    println!(
        "    • the usual cause is a cross-distro fontconfig cache (a container that shares the home"
    );
    println!(
        "      dir cached ITS font set under the same directory hashes; chromium accepts those"
    );
    println!("      entries and never rescans).");
    println!(
        "    • the gate sidesteps that by OWNING its cache — `ensure_gate_font_cache` always points"
    );
    println!("      chromium at $TMPDIR/tbd-gate-cache-<distro>, whatever XDG_CACHE_HOME says (");
    println!(
        "      it used to stand down when XDG_CACHE_HOME was set, which this container exports as"
    );
    println!("      ~/.cache — the very path the fix exists to avoid).");
    println!(
        "    • so if you are reading this, the gate's OWN cache is bad. Remedy: delete the directory"
    );
    println!(
        "      named on the `cache:` line above and re-run; it is a cache and costs ~1 s to rebuild."
    );
    println!(
        "    • to put it elsewhere deliberately, set TBD_GATE_FONT_CACHE (gate-scoped, so it cannot"
    );
    println!("      silently mean 'share a font cache with another distro').");
    println!(
        "    • verify the machine really has fonts:  fc-list | wc -l   (0 = none installed). Note a"
    );
    println!(
        "      healthy fc-list proves nothing about chromium — one wedge had fc-list at 783 and chromium"
    );
    println!("      at zero, because they were reading different caches.");
}

pub(super) fn print_wedge_hint() {
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
        "    • see {} (known wedge modes + the P0–P6 recipe)",
        crate::repository_layout::documentation::EDITOR_GATE_RUNBOOK
    );
}
