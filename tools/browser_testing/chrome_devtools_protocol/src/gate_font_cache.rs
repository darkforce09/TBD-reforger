//! The fontconfig cache folder every browser the harness launches reads.
//!
//! **Role:** decides, once per process, the folder chromium's fontconfig cache lives in: the
//! folder `TBD_GATE_FONT_CACHE` names, or a gate-owned folder under the system temporary folder
//! keyed by the distribution.
//! **Position:** part of the DevTools protocol client; [`super::launch_with_gpu`] hands the folder
//! to every browser it spawns, and the `gate doctor` diagnostics report it and install it
//! process-wide.
//! **Signals & state:** one process-wide `OnceLock` holding the decision.
//! **Invariants:** an empty `TBD_GATE_FONT_CACHE` counts as unset; the decision never reads
//! `XDG_CACHE_HOME`, because a cache shared across distributions wedges the browser.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Deliberate, **gate-scoped** override for [`gate_font_cache_dir`].
///
/// This is the escape hatch that replaced "respect `XDG_CACHE_HOME`" — see
/// `diagnostics::ensure_gate_font_cache` for why the general variable could not keep that job.
pub const GATE_FONT_CACHE_ENV: &str = "TBD_GATE_FONT_CACHE";

/// Why the gate is using the cache directory it is using — reported by `check_fonts`, because a
/// font cache the operator cannot see is a font cache nobody can debug.
pub enum CacheOrigin {
    /// The gate picked it (the normal case): `$TMPDIR/tbd-gate-cache-<distro>`.
    Owned,
    /// The operator pinned it explicitly via [`GATE_FONT_CACHE_ENV`].
    Pinned,
}

/// Where the gate keeps the fontconfig cache it owns.
///
/// [`super::launch_with_gpu`] sets `XDG_CACHE_HOME` to it on every browser it spawns, and the
/// `gate` prologue (`diagnostics::ensure_gate_font_cache`) installs it process-wide before any
/// browser starts.
pub fn gate_font_cache_dir() -> &'static Path {
    &resolved_font_cache().0
}

/// The cache folder and why it was chosen, decided once per process.
pub fn resolved_font_cache() -> &'static (PathBuf, CacheOrigin) {
    static RESOLVED: OnceLock<(PathBuf, CacheOrigin)> = OnceLock::new();
    RESOLVED.get_or_init(|| match std::env::var_os(GATE_FONT_CACHE_ENV) {
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
        // Unreadable `/etc/os-release` → the undiscriminated name. Losing the discriminator is worse than
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
