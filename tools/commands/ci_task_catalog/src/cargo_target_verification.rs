//! The `mk` targets that police the shared `CARGO_TARGET_DIR` pin and reclaim the CI scratch
//! folder.
//!
//! **Role:** [`verify_cargo_target`] and [`reclaim_target_ci`] are the bodies of
//! `cargo xtask mk verify-cargo-target` and `cargo xtask mk reclaim-target-ci`.
//! **Position:** dispatched by the `mk` recipes (`crate::build_lane::recipes`); checks the
//! pin of [`super::cargo_target_pin`] (its source text, its behaviour with the variable unset, the
//! worktree-local reversal) and the recipe steps of `rust-build` and `rust-api`; reclaims under
//! the build output folder of `repository_layout::build_output`.
//! **Signals & state:** none held; [`reclaim_target_ci`] deletes folders under the root it is
//! given.
//! **Invariants:** a deleted or unreadable pin source never reads as a held pin; `rust-build`
//! inherits the shared pin while `rust-api` keeps its private per-checkout folder; a reclaim path
//! never equals the shared cache and always ends in its own name.

use std::path::Path;

use crate::Result;
use process_runner::Run;
use repository_layout::build_output::{
    BUILD_OUTPUT_FOLDER, CONTINUOUS_INTEGRATION_SUBFOLDER, build_output_subfolder,
};
use verification_core::{Finding, Kind, NotRun, Verdict};

use super::cargo_target_pin::{cwd_root, dev_api_target_dir, primary_root, resolve_target_dir};
use crate::build_lane::recipes::{Step, rust_api, rust_build};

/// The source text that must be present in the pin's file ([`PIN_SOURCE_REL`]) for the shared
/// pin to exist at all.
///
/// The behavioural probe of [`verify_cargo_target`] can be satisfied by an accident (a stray
/// environment variable, a `.cargo/config.toml` that happens to agree today); the source pin says
/// the formula is still written down where it belongs. It moves with the pin, and
/// `tests::pin_marker_is_present_in_this_file` reads the pin's file through `include_str!` and
/// looks for this const, so the two cannot drift.
pub(crate) const PIN_SOURCE_MARKER: &str = "primary_root().join(\"target\")";

/// The pin's file, repository-relative: the file [`PIN_SOURCE_MARKER`] must appear in.
pub(crate) const PIN_SOURCE_REL: &str = "tools/commands/ci_task_catalog/src/cargo_target_pin.rs";

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
        // `du -sh` on the inherited terminal: its output (size TAB path) is part of the target's
        // contract.
        let _ = Run::new("du").arg("-sh").arg(folder).terminal();
        std::fs::remove_dir_all(folder)?;
        println!(
            "removed {} (shared target/ left intact at {warm})",
            folder.display()
        );
    }
    Ok(0)
}

#[cfg(test)]
#[path = "tests/cargo_target_verification.rs"]
mod tests;
