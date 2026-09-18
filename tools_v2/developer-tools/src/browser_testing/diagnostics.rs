//! `gate doctor` — a fail-fast preflight for the editor CDP smokes (T-177).
//!
//! Why this exists: the editor gate used to depend on **unpinned external state** (a floating
//! playwright chromium, a floating toolchain, no committed runner — `t151_10` D-06), and when the
//! environment drifted it **hung 130 s** with a cryptic `cdp: ws call timed out (Runtime.evaluate)`
//! instead of failing fast — turning a routine ticket into a multi-hour reverse-engineering session.
//! (The actual T-177 root cause: `chrome-headless-shell`'s stubbed Skia font manager FATAL-crashes on
//! per-character font fallback; fixed in [`crate::browser_testing::cdp::find_chromium`].)
//!
//! `gate doctor` runs before the suite (a prerequisite of `cargo xtask mk leptos-gates`) and, in ~15 s:
//! validates the resolved chromium + toolchain against the committed pins (`gate-env.json`), checks
//! free RAM + orphaned chrome processes, checks that chromium can actually resolve a font
//! (`check_fonts` — T-320; a zero-font chromium is a hard fail as of T-362), and runs a
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

use crate::browser_testing::cdp;
use crate::browser_testing::server::{ServeConfig, start_server};

const EDIT_PATH: &str = "/missions/smoke/edit?force=webgl&sat=preview";
const DEFAULT_DIST: &str = "apps/website/frontend/dist";

/// The stderr line chromium prints when fontconfig hands it an empty font set (T-320).
const NO_FONT_MARKER: &str = "Could not find any font";

/// How long `check_fonts` watches chromium's log before calling silence a pass. The errors it looks
/// for are emitted ~250–400 ms after launch (measured), so this is ~12× headroom; a broken
/// environment is reported in well under a second because the loop breaks on the first match.
const FONT_PROBE_WINDOW_MS: u64 = 5_000;

/* ──────────────── T-320 / T-362 — the gate owns its fontconfig cache ──────────────── */

/// Deliberate, **gate-scoped** override for [`gate_font_cache_dir`] (T-362).
///
/// This is the escape hatch that replaced "respect `XDG_CACHE_HOME`" — see
/// [`ensure_gate_font_cache`] for why the general variable could not keep that job.
const FONT_CACHE_ENV: &str = "TBD_GATE_FONT_CACHE";

/// Why the gate is using the cache directory it is using — reported by `check_fonts`, because a
/// font cache the operator cannot see is a font cache nobody can debug.
enum CacheOrigin {
    /// The gate picked it (the normal case): `$TMPDIR/tbd-gate-cache-<distro>`.
    Owned,
    /// The operator pinned it explicitly via `FONT_CACHE_ENV`.
    Pinned,
}

/// What `check_fonts` learned. The three states are not decoration (T-362): "chromium told us it
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

/// What the liveness probe found. `BrowserDied` is the T-320 case: a CDP call never answers because
/// the browser process is **gone**, not because the page is slow — and the two want different
/// diagnoses.
enum Liveness {
    Ready,
    NotReady,
    BrowserDied,
}

#[path = "diagnostics/ensure_gate_font_cache.rs"]
mod ensure_gate_font_cache;
pub use ensure_gate_font_cache::ensure_gate_font_cache;
use ensure_gate_font_cache::font_cache_report;
pub use ensure_gate_font_cache::gate_font_cache_dir;
pub use ensure_gate_font_cache::run;

#[path = "diagnostics/check_fonts.rs"]
mod check_fonts;
use check_fonts::check_dist;
use check_fonts::check_fonts;
use check_fonts::count_chrome_processes;
use check_fonts::liveness_probe;
use check_fonts::print_font_wedge_hint;
use check_fonts::print_wedge_hint;
