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
//! free RAM + orphaned chrome processes, checks that chromium can actually resolve a font
//! ([`check_fonts`] — T-320; a zero-font chromium is a hard fail as of T-362), and runs a
//! **short-timeout editor liveness probe** that FAILS with an actionable message + a native-stack
//! hint instead of the 130 s hang. See `docs/website/EDITOR_GATE_RUNBOOK.md`.
//!
//! The font cache the whole harness runs against is decided here too —
//! [`ensure_gate_font_cache`], which every browser gate calls before launching chromium.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use crate::cdp;
use crate::serve::{ServeConfig, start_server};

const EDIT_PATH: &str = "/missions/smoke/edit?force=webgl&sat=preview";
const DEFAULT_DIST: &str = "apps/website/frontend/dist";

/// The stderr line chromium prints when fontconfig hands it an empty font set (T-320).
const NO_FONT_MARKER: &str = "Could not find any font";

/// How long [`check_fonts`] watches chromium's log before calling silence a pass. The errors it looks
/// for are emitted ~250–400 ms after launch (measured), so this is ~12× headroom; a broken
/// environment is reported in well under a second because the loop breaks on the first match.
const FONT_PROBE_WINDOW_MS: u64 = 5_000;

/* ──────────────── T-320 / T-362 — the gate owns its fontconfig cache ──────────────── */

/// Deliberate, **gate-scoped** override for [`gate_font_cache_dir`] (T-362).
///
/// This is the escape hatch that replaced "respect `XDG_CACHE_HOME`" — see
/// [`ensure_gate_font_cache`] for why the general variable could not keep that job.
const FONT_CACHE_ENV: &str = "TBD_GATE_FONT_CACHE";

/// Why the gate is using the cache directory it is using — reported by [`check_fonts`], because a
/// font cache the operator cannot see is a font cache nobody can debug.
enum CacheOrigin {
    /// The gate picked it (the normal case): `$TMPDIR/tbd-gate-cache-<distro>`.
    Owned,
    /// The operator pinned it explicitly via [`FONT_CACHE_ENV`].
    Pinned,
}

/// Point chromium at a **gate-owned** fontconfig cache, unconditionally.
///
/// # Why this exists (T-320 — the editor-gate wedge)
///
/// `~/.cache` is shared by every distro that shares the home directory (a distrobox/toolbox
/// container and the host both write `~/.cache/fontconfig`). Fontconfig keys each cache file by a
/// hash of the font directory, and the caches written from a container describe *that* container's
/// font set. When chromium's bundled fontconfig finds those entries it accepts them and **never
/// rescans**, so it comes up with **zero fonts** — the browser process logs
/// `Could not find any font: , sans` at startup and every UI text run shapes to `glyph_count: 0`.
///
/// That state is survivable right up until something asks for a **per-character** fallback, which
/// lands in `SkFontMgr_FontConfigInterface::onMatchFamilyStyleCharacter` — an unconditional
/// `SK_ABORT("Not implemented")` (`SkFontMgr_FontConfigInterface.cpp:163`). The whole **browser
/// process** takes SIGABRT, the CDP websocket goes silent mid-call, and the harness reports
/// `cdp: ws call timed out (Runtime.evaluate)` / `timeout waiting for Page.loadEventFired` — a
/// "wedge" that is really a corpse. The editor route is the one that reaches that fallback; `/`
/// and every other SPA route render inside the fonts they already matched and survive, which is
/// exactly why the failure looked editor-specific and unfixable from the app side.
///
/// This is the same class of defect T-177 was created to kill: the gate depending on **unpinned
/// external state**. A gate-owned cache dir makes the font set a function of the machine's
/// installed fonts only. The first run pays one fontconfig scan (~1 s); after that it is cached.
///
/// # Why it is now unconditional (T-362)
///
/// It used to return early whenever `XDG_CACHE_HOME` was set, on the reasoning that an operator who
/// sets it means it. The reasoning does not survive contact with this machine: the Debian container
/// **exports `XDG_CACHE_HOME=~/.cache`**, which is precisely the shared host/container path whose
/// poisoned cache caused T-320. So the fix documented itself as unconditional and was, in the one
/// environment it was written for, disabled.
///
/// MEASURED 2026-07-26, end to end, with a cache warmed by a container-side chromium at a scratch
/// `XDG_CACHE_HOME` and then read by a host-side one:
///   * container chromium warms 35 `le64.cache-11` files and resolves fonts fine;
///   * **host** chromium on that same directory logs
///     `ERROR:ui/gfx/platform_font_skia.cc:258] Could not find any font: , sans`;
///   * `gate doctor` pointed at it reported `✗ fonts` and then `✗ liveness … the headless browser
///     process DIED`, whose underlying error was `cdp: ws call timed out (Runtime.evaluate)` — the
///     T-320 signature that cost five sessions.
///
/// Controls (the gate's own cache, a virgin directory, and the operator's real `~/.cache`) all
/// resolved fonts, so the variable is isolated: a cache written from the *other* distro at a shared
/// path. The real `~/.cache` passing is why the hazard was latent rather than active — the two font
/// trees here differ in directory mtime, and fontconfig's validity check rejects a cache whose
/// recorded mtime does not match, so today each side quietly rescans. That is a coincidence of this
/// machine's layout, not a property of the design, and it is one `fc-cache` away from ending.
///
/// Two further reasons owning beats verifying, which is why this does **not** merely check the
/// operator's cache and keep using it:
///   * **A shared cache is a TOCTOU.** Verification samples a directory a *third party* may rewrite;
///     the poisoning write above took ~4 s from the container. A gate that verified at t=0 and runs
///     smokes at t=60 has proved nothing. Owning the directory removes the race instead of sampling
///     it.
///   * **Falling back late would be unsafe.** Switching caches *after* discovering a bad one means a
///     second `set_var` once the browser is already up — i.e. inside the multi-threaded window
///     T-354 warned about, after `start_server` spawned tokio tasks and `reqwest::Client::new()`
///     started reading proxy env vars. The only sound place to decide is before any of that.
///
/// A deliberate setting is still honoured — through [`FONT_CACHE_ENV`], which is *about the gate*.
/// That distinction is the whole point: `XDG_CACHE_HOME` says "put caches here" and says nothing
/// about whether sharing a fontconfig cache across distros is acceptable, whereas
/// `TBD_GATE_FONT_CACHE` can only mean "the gate's font cache goes here".
///
/// # Thread safety
///
/// Still **not** safe to call from a multi-threaded process: `std::env::set_var` races every
/// concurrent `getenv` in the process, including ones inside libc and reqwest, and no restructuring
/// here changes that. What T-362 does change is that the *decision* no longer depends on the
/// ambient environment, so moving the call is now behaviour-preserving — see
/// [`gate_font_cache_dir`] for the hand-off that lets the `unsafe` disappear entirely.
pub fn ensure_gate_font_cache() {
    let _ = font_cache_install();
}

/// The one-shot install, memoised so the decision and its outcome are both stable.
///
/// `Err` is kept rather than swallowed: a cache the gate *failed* to install is a cache the smokes
/// will not use, and [`check_fonts`] has to be able to say so instead of verifying an intention.
fn font_cache_install() -> &'static Result<(), String> {
    static ONCE: OnceLock<Result<(), String>> = OnceLock::new();
    ONCE.get_or_init(|| {
        let dir = gate_font_cache_dir();
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        // SAFETY: called from the single-threaded prologue of a `gate` subcommand, before any
        // browser/subprocess is spawned and before any tokio task exists. `Command` snapshots the
        // parent env at spawn, so every chromium launched afterwards inherits this.
        unsafe { std::env::set_var("XDG_CACHE_HOME", dir) };
        Ok(())
    })
}

/// Where the gate keeps the fontconfig cache it owns.
///
/// Public so the eventual per-child fix can use it: once `cdp::launch` sets
/// `XDG_CACHE_HOME` on the `Command` itself (`.env("XDG_CACHE_HOME", gate_font_cache_dir())`)
/// rather than inheriting it from the process, [`ensure_gate_font_cache`]'s `unsafe set_var` — and
/// with it every question about when it is safe to call — can be deleted outright. That is the
/// version of the T-354 hand-off that needs no single-threaded window at all.
pub fn gate_font_cache_dir() -> &'static Path {
    &resolved_font_cache().0
}

fn resolved_font_cache() -> &'static (PathBuf, CacheOrigin) {
    static RESOLVED: OnceLock<(PathBuf, CacheOrigin)> = OnceLock::new();
    RESOLVED.get_or_init(|| match std::env::var_os(FONT_CACHE_ENV) {
        Some(v) if !v.is_empty() => (PathBuf::from(v), CacheOrigin::Pinned),
        // An empty value is treated as unset, so `TBD_GATE_FONT_CACHE=` cannot accidentally point
        // the cache at the process's cwd.
        _ => (default_font_cache_dir(), CacheOrigin::Owned),
    })
}

/// `$TMPDIR/tbd-gate-cache-<distro>` — keyed by the distro whose font tree the cache describes.
///
/// The suffix is not cosmetic. `$TMPDIR` is `/tmp`, and on this box `/tmp` is **shared** between the
/// host and the Debian container (measured), so a bare `tbd-gate-cache` is itself a cross-distro
/// shared path — the exact shape of the bug this function exists to prevent, just one directory
/// over. It only fails to bite today because the gate binary is host-built and glibc keeps it from
/// running in the container; the container's chromium runs fine there, which is all the poisoning
/// ever needed. Keying on `/etc/os-release` `ID`+`VERSION_ID` means each distro warms and reads its
/// own cache, so the guarantee stops depending on who happens to be able to launch the gate.
fn default_font_cache_dir() -> PathBuf {
    let name = match distro_slug() {
        Some(slug) => format!("tbd-gate-cache-{slug}"),
        // Unreadable `/etc/os-release` → the pre-T-362 name. Losing the discriminator is worse than
        // the old behaviour in no way, and inventing an unstable one (pid, time) would defeat the
        // caching this directory exists for.
        None => "tbd-gate-cache".to_string(),
    };
    std::env::temp_dir().join(name)
}

/// `ID`+`VERSION_ID` from `/etc/os-release`, reduced to a filename-safe slug (`debian-12`).
fn distro_slug() -> Option<String> {
    let os_release = std::fs::read_to_string("/etc/os-release").ok()?;
    let field = |key: &str| {
        os_release
            .lines()
            .find_map(|l| l.strip_prefix(key))
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|v| !v.is_empty())
    };
    let id = field("ID=")?;
    let slug: String = match field("VERSION_ID=") {
        Some(version) => format!("{id}-{version}"),
        None => id,
    }
    .chars()
    .map(|c| {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
            c
        } else {
            '_'
        }
    })
    .collect();
    Some(slug)
}

/// How the doctor names the cache chromium will actually inherit.
///
/// Deliberately reports the **effective** `XDG_CACHE_HOME` rather than the one the gate meant to
/// install: if the install failed, the font probe below is measuring a different directory from the
/// one the smokes will use, and that divergence is the single most useful thing to print.
fn font_cache_report() -> String {
    let (dir, origin) = resolved_font_cache();
    let effective = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| "<unset>".into());
    let tag = match origin {
        CacheOrigin::Owned => "gate-owned",
        CacheOrigin::Pinned => "pinned by TBD_GATE_FONT_CACHE",
    };
    match font_cache_install() {
        Ok(()) => format!("{effective} ({tag})"),
        Err(e) => format!(
            "{effective} — NOT the gate's cache ({}); {e}",
            dir.display()
        ),
    }
}

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
    ensure_gate_font_cache();
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
    let fonts = check_fonts().await;
    if matches!(fonts, FontProbe::Inconclusive) {
        warnings += 1;
    }
    warnings += check_dist(dist.as_deref().unwrap_or(DEFAULT_DIST));

    // T-362 — a zero-font chromium is a HARD fail, and it short-circuits the liveness probe.
    //
    // Two changes from the T-320 shape, both measured against the poisoned-cache repro. It used to
    // count as one *warning*, so a fonts failure that the liveness probe happened to survive exited
    // **0** and handed the wedge to the suite — the gate reporting OK on an environment it had just
    // proved was fatal. And running liveness anyway costs ~15 s to produce `the headless browser
    // process DIED`, which is a true statement about a browser we already knew would die and reads
    // as a renderer bug rather than a font cache. Failing here names the cause instead.
    if matches!(fonts, FontProbe::NoFonts) {
        print_font_wedge_hint();
        println!(
            "== gate doctor: FAIL — chromium cannot resolve a single font; every editor smoke would \
             SIGABRT the browser (T-320)"
        );
        return Ok(1);
    }

    let dist = dist.unwrap_or_else(|| DEFAULT_DIST.to_string());
    let live = liveness_probe(&dist, env).await;
    let live_ok = match live {
        Ok(Liveness::Ready) => {
            println!("  ✓ liveness    editor page booted; evaluate responsive");
            true
        }
        Ok(Liveness::BrowserDied) => {
            println!(
                "  ✗ liveness    the headless browser process DIED during the probe (it is not a \
                 slow page — the CDP endpoint stopped answering entirely)"
            );
            false
        }
        Ok(Liveness::NotReady) => {
            println!("  ✗ liveness    editor page did not become ready within the budget");
            false
        }
        Err(e) => {
            println!("  ✗ liveness    {e}");
            false
        }
    };

    if !live_ok {
        // A confirmed zero-font environment already returned above, so the only fonts state left
        // that could explain a dead browser is the one where the probe never got a verdict.
        if matches!(fonts, FontProbe::Inconclusive) {
            print_font_wedge_hint();
        }
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

/// What [`check_fonts`] learned. The three states are not decoration (T-362): "chromium told us it
/// has no fonts" is a **guaranteed** browser SIGABRT on the editor route and must stop the gate,
/// whereas "the probe could not run" taught us nothing and must not — collapsing both into `false`
/// meant the only actionable one was reported at the same severity as a shrug.
enum FontProbe {
    Ok,
    /// The marker fired: chromium resolved zero fonts. The T-320 wedge, pre-crash.
    NoFonts,
    /// The probe itself failed (no log, no spawn, chromium exited early) — verdict unknown.
    Inconclusive,
}

/// **T-320 — can chromium resolve a font at all?**
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
/// T-362 — this probe deliberately **inherits** `XDG_CACHE_HOME` instead of passing its own via
/// `Command::env`. It has to measure what every other browser launch in the run will get, not what
/// [`ensure_gate_font_cache`] intended; a probe that forces its own cache would pass while the
/// smokes inherited a poisoned one, which is the exact failure this check exists to catch.
async fn check_fonts() -> FontProbe {
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
        // T-362 — name the cache on the FAILURE path too. Which directory chromium was reading is
        // the first thing anyone needs and the one thing the old message omitted.
        println!(
            "  ✗ fonts       chromium resolves NO font ('{NO_FONT_MARKER}') — the editor page will \
             SIGABRT the browser on its first fallback glyph (T-320)"
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
/// (T-320: this is what a `gate doctor` run inside a worktree with no build looks like).
fn check_dist(dist: &str) -> u32 {
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

/// What the liveness probe found. `BrowserDied` is the T-320 case: a CDP call never answers because
/// the browser process is **gone**, not because the page is slow — and the two want different
/// diagnoses.
enum Liveness {
    Ready,
    NotReady,
    BrowserDied,
}

/// The ~15 s editor liveness probe: serve the dist, launch chromium, navigate the editor, and run a
/// short-timeout `1+1` then a bounded readiness poll. A pegged/dead main thread fails here in seconds
/// (via [`cdp::Page::evaluate_with_timeout`]) instead of the suite's 130 s hang. The whole probe is
/// wrapped in an overall timeout so it can never inherit the wedge it exists to catch.
///
/// T-320: whatever the outcome, before reporting we ask the browser's own `/json/version` whether it
/// is still alive. A dead endpoint turns "timed out" into "crashed", which is the difference between
/// hunting the app and hunting the environment.
async fn liveness_probe(dist: &str, env: Option<&Value>) -> Result<Liveness> {
    ensure_gate_font_cache();
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
    // T-320 — ask the browser itself. `Runtime.evaluate` "timing out" is what a SIGABRT'd browser
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

/// The T-320 remedy, printed when [`check_fonts`] says chromium has no fonts.
fn print_font_wedge_hint() {
    println!(
        "  ─ chromium could not resolve a single font. That is the T-320 wedge: the editor page's"
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
    println!(
        "      chromium at $TMPDIR/tbd-gate-cache-<distro>, whatever XDG_CACHE_HOME says (T-362:"
    );
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
        "      healthy fc-list proves nothing about chromium — T-320 had fc-list at 783 and chromium"
    );
    println!("      at zero, because they were reading different caches.");
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
