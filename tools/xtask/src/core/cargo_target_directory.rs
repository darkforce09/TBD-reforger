//! Custody of `CARGO_TARGET_DIR`: the shared pin, the layout of the build output folder, the
//! glibc stamp guard, and the `mk` targets that print, verify and reclaim them.
//!
//! **Role:** answers where every build writes. [`resolve_target_dir`] is the shared cache pin
//! (`CARGO_TARGET_DIR` when set, else [`primary_root`]`/target`); [`build_output_subfolder`] names
//! each tool's purpose subfolder inside `target/`; [`abi_guard`] refuses a target directory that
//! another glibc built; [`verify_cargo_target`] and [`reclaim_target_ci`] are the bodies of
//! `cargo xtask mk verify-cargo-target` and `cargo xtask mk reclaim-target-ci`.
//!
//! **Position:** read by the `mk` recipes (`crate::commands::build::recipes`), the wave gate and
//! its reclaim sweep (`crate::commands::platform::wave_execution`); reads `git` for the primary
//! checkout and the recipe steps for the private-directory checks.
//!
//! **Signals & state:** none held; [`abi_guard`] writes one stamp file per target directory and
//! [`reclaim_target_ci`] deletes folders under the root it is given.
//!
//! **Invariants:**
//! - Two roots, never one. [`primary_root`] (the primary checkout, from `git rev-parse
//!   --git-common-dir`) and [`cwd_root`] (this checkout) are the same folder in the primary
//!   checkout and different inside a linked worktree. The shared warm cache is
//!   `primary_root/target`, shared by every worktree so parallel slices do not each cold-build the
//!   workspace; the development API's private directory is `cwd_root/target/dev-api`, per checkout,
//!   because it starts a long-lived server that must not wait in the shared build-lock queue.
//!   Collapsing them either way is a silent regression, so they are two functions with two names.
//! - The pin is computed here and never moved into `.cargo/config.toml`: an `[env]` entry with
//!   `relative = true` resolves against the config file's own folder, which inside a linked
//!   worktree is that worktree, so every worktree would get its own cold `target/` and nothing would
//!   report it. [`verify_cargo_target`] §4 asserts the negation, and §1 reads this file for
//!   [`PIN_SOURCE_MARKER`].
//! - All build output lives under one `target/` folder. Each purpose subfolder is its own
//!   `CARGO_TARGET_DIR` (or trunk dist folder, or compose project) and so holds its own cargo
//!   lock, and no subfolder name is an entry cargo writes inside a target directory (profile
//!   folders, `build`, `doc`, `package`, `tmp`, target triples, its bookkeeping files), so nesting
//!   shares no file with the shared cache.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use process_runner::Run;
use verification_core::{Finding, Kind, NotRun, Verdict};

use crate::commands::build::recipes::{Step, rust_api, rust_build};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The source text that must be present in **this file** for the shared pin to exist at all.
///
/// The behavioural probe of [`verify_cargo_target`] can be satisfied by an accident (a stray
/// environment variable, a `.cargo/config.toml` that happens to agree today); the source pin says
/// the formula is still written down where it belongs. It moves with the pin, and
/// `tests::pin_marker_is_present_in_this_file` derives its fixture from this const (through
/// `include_str!`) so the two cannot drift.
pub(crate) const PIN_SOURCE_MARKER: &str = "primary_root().join(\"target\")";

/// This module's own path, repository-relative: the file [`PIN_SOURCE_MARKER`] must appear in.
pub(crate) const PIN_SOURCE_REL: &str = "tools/xtask/src/core/cargo_target_directory.rs";

/// The checkout this process runs in. **Inside a worktree this is the worktree.** Used only for
/// the development API's private directory ([`dev_api_target_dir`]); never for the shared cache.
pub(crate) fn cwd_root() -> PathBuf {
    repository_layout::find_repository_root().unwrap_or_else(|_| PathBuf::from("."))
}

/// The **primary** checkout: `git rev-parse --path-format=absolute --git-common-dir` with its
/// trailing `/.git` removed.
///
/// The same derivation as `crate::commands::platform::wave_execution::Ctx::enter`'s `main_root`:
/// one formula, not two. A git that cannot answer falls back to this checkout, which is at worst a
/// cold build and never a write to `/`.
pub(crate) fn primary_root() -> PathBuf {
    let common = Run::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()
        .filter(|o| o.code == 0)
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());
    match common {
        Some(g) => Path::new(&g)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(cwd_root),
        None => cwd_root(),
    }
}

/// `CARGO_TARGET_DIR` when set and non-empty, else the primary checkout's `target/`.
///
/// `env` is the caller's `$CARGO_TARGET_DIR`, threaded as a **parameter** rather than read from the
/// process environment, so [`verify_cargo_target`] can ask "what would this be with the variable
/// unset?" without a `remove_var` (unsafe, global, and racy with any thread). One function answers
/// both questions, so the probe cannot test a different formula than the one that ships.
pub(crate) fn resolve_target_dir(env: Option<&str>) -> String {
    match env {
        // An operator or driver export wins: the wave driver hands its gate steps a private
        // directory, and a pin that overrode it would put every gate back in the shared cache.
        Some(v) if !v.is_empty() => v.to_string(),
        _ => primary_root().join("target").display().to_string(),
    }
}

/// `$CARGO_TARGET_DIR` from the environment, empty treated as unset.
pub(crate) fn env_pin() -> Option<String> {
    std::env::var("CARGO_TARGET_DIR")
        .ok()
        .filter(|s| !s.is_empty())
}

// ── THE BUILD OUTPUT FOLDER AND ITS PURPOSE SUBFOLDERS ───────────────────────────────────────

/// The one gitignored folder that holds all build output, relative to a checkout root. Under
/// [`primary_root`] it is also the shared cache that [`resolve_target_dir`] pins.
pub(crate) const BUILD_OUTPUT_FOLDER: &str = "target";

/// The development API's private `CARGO_TARGET_DIR` (`cargo xtask mk rust-api`), under
/// [`cwd_root`]; see [`dev_api_target_dir`].
pub(crate) const DEV_API_SUBFOLDER: &str = "dev-api";
/// The wave gate's trunk `CARGO_TARGET_DIR`; `TBD_GATE_TRUNK_TARGET` overrides it.
pub(crate) const GATE_TRUNK_SUBFOLDER: &str = "gate-trunk";
/// The wave gate's trunk dist folder; `TBD_GATE_TRUNK_DIST` overrides it.
pub(crate) const GATE_FRONTEND_DIST_SUBFOLDER: &str = "gate-dist-frontend";
/// The wave gate's `cargo check` and clippy `CARGO_TARGET_DIR`; `TBD_GATE_CHECK_TARGET`
/// overrides it.
pub(crate) const GATE_CHECK_SUBFOLDER: &str = "gate-check";
/// The wave gate's schema step `CARGO_TARGET_DIR`; `TBD_GATE_SCHEMA_TARGET` overrides it.
pub(crate) const GATE_SCHEMA_SUBFOLDER: &str = "gate-schema";
/// The wave gate's `api` test `CARGO_TARGET_DIR`.
pub(crate) const GATE_API_SUBFOLDER: &str = "gate-api";
/// The wave gate's `map_engine` test `CARGO_TARGET_DIR`.
pub(crate) const GATE_MAP_ENGINE_SUBFOLDER: &str = "gate-map-engine";
/// The wave gate's `frontend` test `CARGO_TARGET_DIR`.
pub(crate) const GATE_FRONTEND_SUBFOLDER: &str = "gate-frontend";
/// The wave gate's `xtask` and `developer_tools` test `CARGO_TARGET_DIR`.
pub(crate) const GATE_TOOLS_SUBFOLDER: &str = "gate-tools";
/// A slice gate's private frontend test `CARGO_TARGET_DIR` is this prefix followed by the slice id.
pub(crate) const GATE_SLICE_FRONTEND_PREFIX: &str = "gate-slice-frontend-";
/// The continuous-integration scratch `CARGO_TARGET_DIR` that `mk reclaim-target-ci` deletes.
pub(crate) const CONTINUOUS_INTEGRATION_SUBFOLDER: &str = "ci";
/// The `mcpd` broker's private `CARGO_TARGET_DIR` (`cargo xtask mcp daemon start`), so a wave
/// gate never rewrites the running daemon's binary; `MCPD_CARGO_TARGET_DIR` overrides it.
pub(crate) const MCP_DAEMON_SUBFOLDER: &str = "dev-mcpd";
/// The throwaway compose project of `cargo xtask db selftest`'s compose-parity arm.
pub(crate) const DATABASE_SELFTEST_SUBFOLDER: &str = "db-selftest";
/// The prefix every wave-gate subfolder shares; `platform wave reclaim --gate-dirs` sweeps the
/// subfolders that carry it.
pub(crate) const GATE_SUBFOLDER_PREFIX: &str = "gate-";

/// Every fixed purpose subfolder name under [`BUILD_OUTPUT_FOLDER`], for the proof that none
/// collides with an entry cargo writes there itself.
pub(crate) const PURPOSE_SUBFOLDERS: &[&str] = &[
    DEV_API_SUBFOLDER,
    GATE_TRUNK_SUBFOLDER,
    GATE_FRONTEND_DIST_SUBFOLDER,
    GATE_CHECK_SUBFOLDER,
    GATE_SCHEMA_SUBFOLDER,
    GATE_API_SUBFOLDER,
    GATE_MAP_ENGINE_SUBFOLDER,
    GATE_FRONTEND_SUBFOLDER,
    GATE_TOOLS_SUBFOLDER,
    CONTINUOUS_INTEGRATION_SUBFOLDER,
    MCP_DAEMON_SUBFOLDER,
    DATABASE_SELFTEST_SUBFOLDER,
];

/// `<checkout_root>/target/<subfolder>`: the one formula every tool names its build output with.
pub(crate) fn build_output_subfolder(checkout_root: &Path, subfolder: &str) -> PathBuf {
    checkout_root.join(BUILD_OUTPUT_FOLDER).join(subfolder)
}

/// The development API's private `CARGO_TARGET_DIR`: `<this checkout>/target/dev-api`.
pub(crate) fn dev_api_target_dir() -> PathBuf {
    build_output_subfolder(&cwd_root(), DEV_API_SUBFOLDER)
}

/// Root-level folder names beside [`BUILD_OUTPUT_FOLDER`] that no tool writes; a machine that ran
/// earlier tooling can still hold them, and the reclaim commands delete them.
pub(crate) const RETIRED_ROOT_LEVEL_FOLDERS: &[&str] = &[
    "target-dev-api",
    "target-ci",
    "target-dev-mcpd",
    "target-mk-db-selftest",
];
/// Prefixes of the retired root-level gate folders (cargo target folders and trunk dist folders).
pub(crate) const RETIRED_ROOT_LEVEL_FOLDER_PREFIXES: &[&str] = &["target-gate-", "dist-gate-"];

/// Is `name` (a folder at a checkout root) one of the retired root-level build folders?
pub(crate) fn is_retired_root_level_build_folder(name: &str) -> bool {
    RETIRED_ROOT_LEVEL_FOLDERS.contains(&name)
        || RETIRED_ROOT_LEVEL_FOLDER_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

// ── THE TWO-GLIBC GUARD ──────────────────────────────────────────────────────────────────────

/// The ABI allowed to write into a given target directory: stamped on first use, enforced after.
///
/// Two glibcs sharing one `CARGO_TARGET_DIR` produce `GLIBC_2.xx not found` at run time, a link
/// error that reads like a broken checkout; this turns it into a named refusal at the boundary. An
/// unreadable or unwritable stamp is **not** a failure: the guard catches one specific collision,
/// and a guard that blocked builds over a permissions quirk would simply be disabled.
pub(crate) fn abi_guard(dir: &Path) -> std::result::Result<(), String> {
    let want = abi_id();
    let stamp = dir.join(".tbd-build-abi");
    if let Ok(found) = std::fs::read_to_string(&stamp) {
        let found = found.trim();
        if !found.is_empty() && found != want {
            return Err(format!(
                "REFUSING: {} was built by '{found}', this is '{want}'.\n      \
                 Two glibcs sharing one CARGO_TARGET_DIR produce `GLIBC_2.xx not found` at run \
                 time, which reads like a broken checkout.\n      \
                 Set CARGO_TARGET_DIR to a directory of your own, or delete {}.",
                dir.display(),
                stamp.display()
            ));
        }
        return Ok(());
    }
    if std::fs::create_dir_all(dir).is_ok() {
        let _ = std::fs::write(&stamp, format!("{want}\n"));
    }
    Ok(())
}

/// `glibc<version>-<container|host>`. The container test is distrobox's own (`/run/.containerenv`
/// or `/.dockerenv`), the same one the host bridge uses, and NOT `command -v distrobox-host-exec`,
/// which is true on both sides of the bridge.
pub(crate) fn abi_id() -> String {
    // SAFETY: `gnu_get_libc_version` returns a pointer to a static NUL-terminated string in libc;
    // it takes no arguments, allocates nothing, and the result outlives this call.
    let glibc = unsafe {
        let p = libc::gnu_get_libc_version();
        if p.is_null() {
            "unknown".to_string()
        } else {
            std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    };
    let where_ = if Path::new("/run/.containerenv").exists() || Path::new("/.dockerenv").exists() {
        "container"
    } else {
        "host"
    };
    format!("glibc{glibc}-{where_}")
}

// ── verify-cargo-target ──────────────────────────────────────────────────────────────────────

/// Assert the shared `CARGO_TARGET_DIR` pin is intact; exit 0 when it is, 1 with a `FAIL:` line.
///
/// §1 the source marker; §2/§3 the behavioural probe with the variable unset; §4 the
/// worktree-local reversal; §5 `rust-build` inherits the pin while `rust-api` keeps its private
/// `cwd_root/target/dev-api`.
pub(crate) fn verify_cargo_target(root: &Path) -> Result<u8> {
    let expected = primary_root().join("target").display().to_string();

    // §1 — the self-reference. See PIN_SOURCE_MARKER.
    match pin_marker_verdict(root) {
        Verdict::Held => {}
        _ => {
            println!("FAIL: {PIN_SOURCE_REL} missing `{PIN_SOURCE_MARKER}` (the shared pin)");
            return Ok(1);
        }
    }

    // §2/§3 — the behavioural probe, with CARGO_TARGET_DIR unset. `resolve_target_dir(None)` is
    // the function the CLI calls, so this cannot drift from what ships.
    let got = resolve_target_dir(None);
    if got.is_empty() {
        println!("FAIL: with CARGO_TARGET_DIR unset, the pin resolved to an empty target dir");
        return Ok(1);
    }
    if got != expected {
        println!(
            "FAIL: with CARGO_TARGET_DIR unset, the pin resolves to '{got}' (expected {expected} — \
             primary-repo shared target, not a worktree-local dir)"
        );
        return Ok(1);
    }

    // §4 — the `.cargo/config.toml` reversal.
    if pin_is_worktree_local(&got, &cwd_root(), &primary_root()) {
        println!(
            "FAIL: in a linked worktree the pin resolved to '{got}' — that is this worktree's \
             own target/, not the primary repo's shared one"
        );
        return Ok(1);
    }

    // §5 — `rust-build` must INHERIT the shared export, never set a private dir of its own.
    if let Some(line) = private_target_dir_violation(&rust_build()) {
        println!(
            "FAIL: rust-build must inherit the shared export, not set a private \
             CARGO_TARGET_DIR (got: {line})"
        );
        return Ok(1);
    }
    // `rust-api` keeping its private per-checkout dir is the other half of the invariant, so its
    // exact location is asserted rather than merely claimed.
    let dev_api = dev_api_target_dir().display().to_string();
    let api_dirs: Vec<String> = rust_api()
        .iter()
        .filter_map(|s| s.recipe_env("CARGO_TARGET_DIR").map(str::to_string))
        .collect();
    if api_dirs != [dev_api.clone()] {
        println!("FAIL: rust-api must build into its private {dev_api} (got: {api_dirs:?})");
        return Ok(1);
    }

    println!(
        "OK: CARGO_TARGET_DIR pin={expected} (rust-build inherits; api/rust-api keep private \
         {dev_api})"
    );
    Ok(0)
}

/// §4 — has the pin been reversed into a per-worktree one?
///
/// A `.cargo/config.toml` `[env]` with `relative = true` resolves against the config file's own
/// folder, so inside a linked worktree it yields THAT worktree's `target/` while still looking like
/// "having a pin". §1–§3 all pass under that reversal when the probe runs in the primary checkout,
/// which is why this is a separate assertion and why it takes its three inputs as parameters: the
/// RED arm is only reachable from a test if the roots can be supplied.
pub(crate) fn pin_is_worktree_local(got: &str, here: &Path, primary: &Path) -> bool {
    here != primary && got == here.join("target").display().to_string()
}

/// §5 — the first step that sets its own `CARGO_TARGET_DIR`, as its echoed line. The recipe is a
/// value, so the check reads the data directly and cannot be fooled by dry-run formatting.
pub(crate) fn private_target_dir_violation(steps: &[Step]) -> Option<String> {
    steps
        .iter()
        .find(|s| s.recipe_env("CARGO_TARGET_DIR").is_some())
        .map(Step::echo)
}

/// §1 as a [`Verdict`] — `Held`, `Failed`, or `DidNotRun` when the source file cannot be read.
///
/// A deleted or unreadable source file must never read as "the pin is fine".
pub(crate) fn pin_marker_verdict(root: &Path) -> Verdict {
    let path = root.join(PIN_SOURCE_REL);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(source) => {
            return Verdict::did_not_run(
                "shared target-dir pin source",
                Kind::Pin,
                NotRun::Unreadable { path, source },
            );
        }
    };
    // The const's OWN definition line contains the marker (escaped), and a grep that matches its
    // own needle proves nothing. Excluding lines that mention the const by name is explicit about
    // that, rather than relying on the backslash-escaping to differ by luck.
    let hit = text
        .lines()
        .any(|l| l.contains(PIN_SOURCE_MARKER) && !l.contains("PIN_SOURCE_MARKER"));
    if hit {
        Verdict::Held
    } else {
        Verdict::Failed(Finding {
            headline: format!("{PIN_SOURCE_REL} no longer computes the shared pin"),
            detail: vec![format!("expected source text: {PIN_SOURCE_MARKER}")],
        })
    }
}

// ── reclaim-target-ci ────────────────────────────────────────────────────────────────────────

/// Delete the continuous-integration scratch folder `root/target/ci`, and the retired root-level
/// `root/target-ci` when the machine still holds it. Exit 0 when both are gone, 1 on a refusal.
///
/// `root` is a parameter so the destructive path is testable against a scratch tree, and no path
/// is derived from anything but `root`, so a slice's own folder is unreachable by construction.
/// Two refusals guard every deletion and run before any of them: the path never equals the shared
/// cache `root/target`, and it ends in its own name (`/target/ci`, `/target-ci`). An empty `root`
/// yields a relative path, which fails the second.
pub(crate) fn reclaim_target_ci(root: &Path) -> Result<u8> {
    let warm = root.join(BUILD_OUTPUT_FOLDER).display().to_string();
    let folders = [
        (
            build_output_subfolder(root, CONTINUOUS_INTEGRATION_SUBFOLDER),
            "/target/ci",
        ),
        (root.join("target-ci"), "/target-ci"),
    ];

    for (folder, suffix) in &folders {
        let shown = folder.display().to_string();
        if shown == warm || shown == format!("{warm}/") {
            println!("REFUSING: reclaim path collides with shared target/ ({warm})");
            return Ok(1);
        }
        if !(shown.ends_with(suffix) || shown.ends_with(&format!("{suffix}/"))) {
            println!("REFUSING: path '{shown}' is not …{suffix}");
            return Ok(1);
        }
    }
    for (folder, _) in &folders {
        if !folder.exists() {
            println!("already absent: {}", folder.display());
            continue;
        }
        // `du -sh` with inherited stdio: its output (size TAB path) is part of the target's contract.
        let _ = Command::new("du")
            .arg("-sh")
            .arg(folder)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        std::fs::remove_dir_all(folder)?;
        println!(
            "removed {} (shared target/ left intact at {warm})",
            folder.display()
        );
    }
    Ok(0)
}

#[cfg(test)]
#[path = "../commands/build/tests/recipes.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/cargo_target_directory/tests.rs"]
mod build_output_folder_tests;
