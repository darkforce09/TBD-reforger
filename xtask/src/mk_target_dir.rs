//! `mk_target_dir` — custody of `CARGO_TARGET_DIR`, and the two `make` targets that police it.
//!
//! T-853 Phase 3, slice T-895. Split out of [`crate::mk_build`] at a seam with no shared state
//! (SIZE-1: keep a module under 600 lines): that module owns the *recipes*, this one owns *which
//! directory they are allowed to write into* — `print-cargo-target-dir`, `verify-cargo-target` and
//! `reclaim-target-ci`, plus the pin they all read.
//!
//! ── THE PIN THIS SLICE EXISTS TO PROTECT ─────────────────────────────────────────────────────
//!
//! `Makefile:15-17` derived it like this:
//!
//! ```make
//! TBD_GIT_COMMON := $(shell git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
//! TBD_REPO_ROOT  := $(patsubst %/.git,%,$(TBD_GIT_COMMON))
//! export CARGO_TARGET_DIR ?= $(TBD_REPO_ROOT)/target
//! ```
//!
//! `--git-common-dir` is the **primary** checkout's `.git`, shared by every linked worktree. So a
//! run from `.ai/artifacts/worktrees/T-XXX` still points at the primary repo's warm 52 GB
//! `target/`. That is T-253/T-322, and it is why eight parallel slice agents do not each
//! cold-build a 609-crate workspace.
//!
//! **This is why the pin is computed in Rust and NOT moved into `.cargo/config.toml`.** A `[env]`
//! entry with `relative = true` resolves against the *config file's own directory* — which inside a
//! linked worktree is THAT WORKTREE. It looks like the same pin written more declaratively, and it
//! silently reverses the invariant: every worktree gets its own cold `target/` and nothing reports
//! an error. [`verify_cargo_target`] §4 asserts the negation directly, so the reversal is caught
//! rather than merely warned about in prose.
//!
//! ── TWO ROOTS, NEVER ONE ─────────────────────────────────────────────────────────────────────
//!
//! [`primary_root`] (`$(TBD_REPO_ROOT)`) and [`cwd_root`] (`$(CURDIR)`) are the SAME directory in
//! the primary checkout and DIFFERENT inside a worktree. The Makefile used both, and the difference
//! is load-bearing:
//!
//! * the shared warm cache is `primary_root/target` — deliberately cross-worktree;
//! * `rust-api`'s private dir is `cwd_root/target-dev-api` — deliberately per-worktree, because it
//!   starts a long-lived server that must not sit in the shared build-lock queue (T-322).
//!
//! Collapsing them either way is a silent regression, so they are two functions with two names.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use tbd_gate::proc::Run;
use tbd_gate::{Finding, Kind, NotRun, Verdict};

use crate::mk_build::{Step, rust_api, rust_build};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The source text that must be present in **this file** for the shared pin to exist at all.
///
/// ── WHY A GATE READS ITS OWN SOURCE ──────────────────────────────────────────────────────────
///
/// `make verify-cargo-target` grepped the *Makefile* for `export CARGO_TARGET_DIR ?=
/// $(TBD_REPO_ROOT)/target`. That self-reference is the half of the check that survives a
/// refactor: the behavioural probe below can be satisfied by an accident (a stray environment
/// variable, a `.cargo/config.toml` that happens to agree today), while the source pin says the
/// formula is still written down where it belongs. Deleting the self-reference when the pin moved
/// out of the Makefile would have evaporated the check — the exact shape of `verify-t440` reading
/// its own `wave.sh` call sites, which cost 7 test failures in this program when the const and the
/// call site were changed apart.
///
/// So it moves WITH the pin, and [`tests::pin_marker_is_present_in_this_file`] derives its fixture
/// FROM this const (via `include_str!`) so the two cannot drift.
pub(crate) const PIN_SOURCE_MARKER: &str = "primary_root().join(\"target\")";

/// This module's own path, repo-relative — the file [`PIN_SOURCE_MARKER`] must appear in.
pub(crate) const PIN_SOURCE_REL: &str = "xtask/src/mk_target_dir.rs";

/// The private target dir `api` / `rust-api` keep (T-322), relative to [`cwd_root`].
pub(crate) const DEV_API_TARGET: &str = "target-dev-api";

/// `$(CURDIR)` — the checkout `make` would have been invoked from. **Inside a worktree this IS the
/// worktree.** Used only for [`DEV_API_TARGET`]; never for the shared cache.
pub(crate) fn cwd_root() -> PathBuf {
    crate::root::find_repo_root().unwrap_or_else(|_| PathBuf::from("."))
}

/// `$(TBD_REPO_ROOT)` — the **primary** checkout, from `git rev-parse --path-format=absolute
/// --git-common-dir` with a trailing `/.git` stripped (`$(patsubst %/.git,%,…)`).
///
/// Same derivation as `wave::Ctx::enter`'s `main_root`, deliberately: the mandate is one formula,
/// not two. The fallback mirrors the Makefile's `2>/dev/null` — an empty `$(TBD_GIT_COMMON)` made
/// `$(patsubst …)` empty and the pin `/target`, which is nonsense; here a git that cannot answer
/// falls back to this checkout, which is at worst a cold build and never a write to `/`.
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

/// `CARGO_TARGET_DIR ?= $(TBD_REPO_ROOT)/target` — the whole formula, in one expression.
///
/// `env` is the caller's `$CARGO_TARGET_DIR`, threaded as a **parameter** rather than read from the
/// process environment, so [`verify_cargo_target`] can ask "what would this be with the variable
/// unset?" without a `remove_var` (unsafe, global, and racy with any thread) and without the
/// `env -u CARGO_TARGET_DIR $(MAKE) -s print-cargo-target-dir` sub-process the Makefile needed.
/// One function answers both questions, so the probe cannot test a different formula than the one
/// that ships.
pub(crate) fn resolve_target_dir(env: Option<&str>) -> String {
    match env {
        // `?=` — an operator/wave.sh export wins, as it must: `wave.sh` hands its gate steps a
        // private dir, and a pin that overrode that would put every gate back in the shared cache.
        Some(v) if !v.is_empty() => v.to_string(),
        _ => primary_root().join("target").display().to_string(),
    }
}

/// `$CARGO_TARGET_DIR` from the environment, empty treated as unset (make's `?=` semantics).
pub(crate) fn env_pin() -> Option<String> {
    std::env::var("CARGO_TARGET_DIR")
        .ok()
        .filter(|s| !s.is_empty())
}

// ── THE TWO-GLIBC GUARD ──────────────────────────────────────────────────────────────────────

/// The ABI that is allowed to write into a given target dir — stamped on first use, enforced after.
///
/// See the module header. This is the difference between `GLIBC_2.39 not found` (a link error that
/// reads as a broken checkout) and a named refusal at the boundary. An unreadable or unwritable
/// stamp is **not** a failure: this guard exists to catch a specific measured collision, and a
/// guard that blocks builds over a permissions quirk would simply be disabled by the next person.
pub(crate) fn abi_guard(dir: &Path) -> std::result::Result<(), String> {
    let want = abi_id();
    let stamp = dir.join(".tbd-build-abi");
    if let Ok(found) = std::fs::read_to_string(&stamp) {
        let found = found.trim();
        if !found.is_empty() && found != want {
            return Err(format!(
                "REFUSING: {} was built by '{found}', this is '{want}'.\n      \
                 Two glibcs sharing one CARGO_TARGET_DIR produce `GLIBC_2.xx not found` at run \
                 time, which reads like a broken checkout (measured 2026-08-12, T-853).\n      \
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
/// or `/.dockerenv`) — the same one `hostrun.rs` uses, and NOT `command -v distrobox-host-exec`,
/// which is true on both sides of the bridge (the 126 trap).
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

/// T-253: assert the shared `CARGO_TARGET_DIR` pin is intact. Port of `Makefile:27-48`.
///
/// Five checks; the first three are the Makefile's, §4 is new, §5 is the Makefile's `make -n
/// rust-build` grep expressed against the data instead of against dry-run text.
pub(crate) fn verify_cargo_target(root: &Path) -> Result<u8> {
    let expected = primary_root().join("target").display().to_string();

    // §1 — the moved self-reference. See PIN_SOURCE_MARKER.
    match pin_marker_verdict(root) {
        Verdict::Held => {}
        _ => {
            println!("FAIL: {PIN_SOURCE_REL} missing `{PIN_SOURCE_MARKER}` (T-253 shared pin)");
            return Ok(1);
        }
    }

    // §2/§3 — the behavioural probe, with CARGO_TARGET_DIR unset. `resolve_target_dir(None)` is
    // literally the function the CLI calls, so this cannot drift from what ships.
    let got = resolve_target_dir(None);
    if got.is_empty() {
        println!("FAIL: with CARGO_TARGET_DIR unset, make resolved an empty target dir");
        return Ok(1);
    }
    if got != expected {
        println!(
            "FAIL: with CARGO_TARGET_DIR unset, make resolves to '{got}' (expected {expected} — \
             primary-repo shared target, not a worktree-local dir)"
        );
        return Ok(1);
    }

    // §4 — NEW, and the reason this gate now bites harder than the Makefile's.
    if pin_is_worktree_local(&got, &cwd_root(), &primary_root()) {
        println!(
            "FAIL: in a linked worktree the pin resolved to '{got}' — that is this worktree's \
             own target/, not the primary repo's shared one (T-253/T-322)"
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
    // The parenthetical is not decoration: `rust-api` keeping its private dir is the other half of
    // the invariant, so it is asserted rather than merely claimed.
    if !rust_api()
        .iter()
        .any(|s| s.recipe_env("CARGO_TARGET_DIR").is_some())
    {
        println!("FAIL: rust-api lost its private target-dev-api dir (T-322)");
        return Ok(1);
    }

    println!(
        "OK: CARGO_TARGET_DIR pin={expected} (rust-build inherits; api/rust-api keep private \
         target-dev-api)"
    );
    Ok(0)
}

/// §4 — has the pin been reversed into a per-worktree one?
///
/// A `.cargo/config.toml` `[env]` with `relative = true` resolves against the config file's own
/// directory, so inside a linked worktree it yields THAT worktree's `target/` while still looking
/// like "having a pin". §1–§3 all pass under that reversal when the probe happens to run in the
/// primary checkout, which is why this is a separate assertion and why it takes its three inputs as
/// parameters: the RED arm is only reachable from a test if the roots can be supplied.
pub(crate) fn pin_is_worktree_local(got: &str, here: &Path, primary: &Path) -> bool {
    here != primary && got == here.join("target").display().to_string()
}

/// §5 — the Makefile grepped `make -n rust-build` for `CARGO_TARGET_DIR=`. Here the recipe IS a
/// value, so the check reads the data directly and cannot be fooled by dry-run formatting.
/// Returns the offending echoed line, as the Makefile printed the offending recipe.
pub(crate) fn private_target_dir_violation(steps: &[Step]) -> Option<String> {
    steps
        .iter()
        .find(|s| s.recipe_env("CARGO_TARGET_DIR").is_some())
        .map(Step::echo)
}

/// §1 as a [`Verdict`] — `Held`, `Failed`, or `DidNotRun` when the source file cannot be read.
///
/// A deleted or unreadable `mk_build.rs` must never read as "the pin is fine"; that is the
/// fail-open `verify-no-python` sat on for four waves.
pub(crate) fn pin_marker_verdict(root: &Path) -> Verdict {
    let path = root.join(PIN_SOURCE_REL);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(source) => {
            return Verdict::did_not_run(
                "T-253 pin source",
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

/// T-253: delete the obsolete primary-repo `target-ci/` (~13G). Port of `Makefile:50-67`.
///
/// `root` is a parameter so the destructive path is testable against a scratch tree — the T-853
/// rule is "never perturb the real tree", and a function that can only read the real repo cannot
/// be proved to leave a live slice's dir alone.
///
/// Two refusals guard it and both are kept: the collision test (never the warm shared `target/`)
/// and the shape test (the path must end in `target-ci`). A slice's own dir is unreachable by
/// construction — this function never derives a path from anything but `root`.
///
/// ── PRESERVED ODDITY: IN THE MAKEFILE, BOTH REFUSALS ARE DEAD CODE ───────────────────────────
///
/// MEASURED 2026-08-12 while writing the RED arms. `ci` is `$(TBD_REPO_ROOT)/target-ci` and `warm`
/// is `$(TBD_REPO_ROOT)/target`, so:
///
/// * the collision test asks whether `X/target-ci` equals `X/target` — never, for any `X`;
/// * the shape test asks whether `X/target-ci` ends in `/target-ci` — always, for any non-empty
///   `X`, and an EMPTY `$(TBD_REPO_ROOT)` yields the absolute `/target-ci`, which also passes.
///
/// So neither `REFUSING:` line was reachable from `make`, and the RED arm that went looking for
/// them aborted itself rather than pretend (T-556). They are reproduced here verbatim anyway,
/// because deleting a guard is a behaviour change and this is a port — but one of them DOES become
/// reachable in Rust: `Path::new("").join("target-ci")` is the *relative* `target-ci`, which fails
/// the shape test. `tests::reclaim_refusals_are_preserved` pins that, so the text is not merely
/// carried along untested.
pub(crate) fn reclaim_target_ci(root: &Path) -> Result<u8> {
    let ci = root.join("target-ci").display().to_string();
    let warm = root.join("target").display().to_string();

    if ci == warm || ci == format!("{warm}/") {
        println!("REFUSING: reclaim path collides with shared target/ ({warm})");
        return Ok(1);
    }
    if !(ci.ends_with("/target-ci") || ci.ends_with("/target-ci/")) {
        println!("REFUSING: path '{ci}' is not …/target-ci");
        return Ok(1);
    }
    if !Path::new(&ci).exists() {
        println!("target-ci already absent at {ci}");
        return Ok(0);
    }
    // `du -sh` with inherited stdio: its output (size TAB path) is part of the target's contract.
    let _ = Command::new("du")
        .args(["-sh", &ci])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    std::fs::remove_dir_all(&ci)?;
    println!("removed {ci} (shared target/ left intact at {warm})");
    Ok(0)
}

#[cfg(test)]
#[path = "mk_build_tests.rs"]
mod tests;
