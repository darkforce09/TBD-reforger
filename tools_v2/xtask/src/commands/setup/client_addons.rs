//! `cargo xtask setup client-addons`: symlink the framework into the client addon tree.
//!
//! Path pins:
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`. Staging lives at `$HOME/.local/share/tbd-server-addons`.
//!
//! Symlinks `$MOD_ROOT/tbd-framework` into the client addon staging dir and prints Steam launch
//! options. Acceptance is stdout, stderr and exit code on a clean tree plus at least two broken
//! arms, against a throwaway `$HOME` only — never the operator's real addon staging.
//!
//! Preserved oddities:
//! - `ln -sfn` succeeds even when `$MOD_ROOT/tbd-framework` is missing (dangling symlink) —
//!   same as bash; not a red arm.
//! - Success banner (staging path + Steam launch options + Direct Join tip) is byte-identical
//!   to the former script.
//!
//! Fail-opens closed vs bash: none — the script had no `2>/dev/null` / `|| true` on the
//! mkdir/ln path (`set -euo pipefail`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::core::repository_root::find_repo_root;

/// The path pins, for an already-resolved monorepo root.
struct Paths {
    mod_root: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mod_root: root.join("apps/mod"),
        }
    }
}

/// Entry for `xtask setup client-addons`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    run_in(&root)
}

/// `run` with the repo root injected: reads `$HOME` from the process, walks for nothing.
///
/// Split out so the `$HOME` test does not have to `set_current_dir` into a throwaway root to make
/// `find_repo_root` land there. That chdir is process-wide: every other test thread walking from
/// the cwd at that instant (the blueprint fixture and `map_world_los` pins) resolved the
/// throwaway root — which carries a `.ai/tickets/ROOT` marker — and failed with NotFound. Measured
/// 2026-09-05, wave 248 full gate, `test xtask+developer-tools`: reproducible 2/2 in the gate's cold
/// target dir, never in isolation.
pub fn run_in(root: &Path) -> Result<u8> {
    let home = std::env::var("HOME").context("HOME is unset (bash set -u would fail)")?;
    run_with_root(root, Path::new(&home))
}

/// Testable entry that does not walk for the repo root or read `$HOME` from the process.
pub fn run_with_root(root: &Path, home: &Path) -> Result<u8> {
    let paths = Paths::from_root(root);
    let staging = home.join(".local/share/tbd-server-addons");
    let framework = paths.mod_root.join("tbd-framework");
    let link = staging.join("tbd-framework");

    // bash: `mkdir -p "$STAGING"` — shell out so broken-arm stderr matches GNU mkdir.
    let mkdir_status = Command::new("mkdir")
        .arg("-p")
        .arg(&staging)
        .status()
        .context("mkdir -p")?;
    if !mkdir_status.success() {
        return Ok(mkdir_status.code().unwrap_or(1) as u8);
    }

    // bash: `ln -sfn "$MOD_ROOT/tbd-framework" "$STAGING/tbd-framework"`
    // Shell out for GNU ln stderr parity on permission / nesting arms.
    let ln_status = Command::new("ln")
        .arg("-sfn")
        .arg(&framework)
        .arg(&link)
        .status()
        .context("ln -sfn")?;
    if !ln_status.success() {
        return Ok(ln_status.code().unwrap_or(1) as u8);
    }

    println!("Client addon staging: {}", link.display());
    println!();
    println!("Steam → Arma Reforger → Properties → Launch Options:");
    println!(
        "  -addonsDir \"{}\" -addons B2C3D4E5F6A78901",
        staging.display()
    );
    println!();
    println!("Restart the game, then Direct Join → 192.168.0.140 port 2001");

    Ok(0)
}

#[cfg(test)]
#[path = "tests/client_addons/tests.rs"]
mod tests;
