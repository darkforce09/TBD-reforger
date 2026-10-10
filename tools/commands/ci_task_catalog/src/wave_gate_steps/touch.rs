//! Fingerprint invalidation before a gate's cargo steps.
//!
//! **Role:** `touch_changed` bumps the modification time of every file a range changed;
//! `touch_workspace` does the same for every workspace member's sources.
//!
//! **Position:** called by both gates inside the gate lock, before their first cargo step, and by
//! `test_cmd`.
//!
//! **Signals & state:** none held; writes file modification times.
//!
//! **Invariants:** the shared target folder lets cargo hand one worktree an artifact built from
//! another's source, so a gate's cargo steps are trustworthy only after the touch; only paths
//! proved to exist are touched (`touch` would create a missing one); a workspace that parses to
//! zero members, or cannot be read, refuses rather than touching nothing.

use std::path::{Path, PathBuf};

use super::changed::{
    changed_rs, compiled_include_input_paths, include_consumer_package_dirs, owning_package_dir,
    workspace_members,
};
use super::step_context::Ctx;
use super::wprintln;

/// `touch` the given paths. Batched, because the bash's `-exec touch {} +` is one process for 289
/// files rather than 289 processes, and that difference is measurable at wave-gate scale.
///
/// `touch` CREATES a missing file, so every caller here must have proved the path exists first —
/// the bash relied on `[ -f ]` and on `find` for exactly that.
fn touch_paths(paths: &[PathBuf]) -> usize {
    let mut done = 0usize;
    for chunk in paths.chunks(2000) {
        let ok = process_runner::Run::new("touch")
            .args(chunk)
            .terminal()
            .map(|code| code == 0)
            .unwrap_or(false);
        if ok {
            done += chunk.len();
        }
    }
    done
}

/// Bump mtime on everything this slice changed, so cargo cannot reuse a foreign artifact.
pub(crate) fn touch_changed(base: &str) -> i32 {
    // empty→listed=0→return 0 must not mask a failed changed_rs (e.g. git_porcelain_paths
    // rc≠0). Same class as fmt_changed/clippy_changed — `for f in $(changed_rs …)`
    // discarded the rc and treated porcelain failure as an empty change list.
    let files = match changed_rs(base) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let mut listed = 0usize;
    let mut touched = 0usize;
    for f in &files {
        listed += 1;
        if Path::new(f).is_file() {
            touched += touch_paths(&[PathBuf::from(f)]);
            continue;
        }
        // Deleted/renamed-away: the file cannot be touched, but its crate (or include! consumers)
        // still needs a fingerprint bump — otherwise cargo is free to reuse a stale artifact that
        // still contains the deleted code. Deletion-only must not hard-fail here while
        // clippy_changed correctly stayed green.
        if let Some(d) = owning_package_dir(f) {
            let manifest = Path::new(&d).join("Cargo.toml");
            if manifest.is_file() {
                touched += touch_paths(&[manifest]);
                continue;
            }
        }
        for d in include_consumer_package_dirs(f) {
            let manifest = Path::new(&d).join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            touched += touch_paths(&[manifest]);
        }
    }
    // Non-vacuity, load-bearing for every step after it: listed Rust changes but NOTHING's
    // fingerprint was invalidated → cargo may hand this gate a foreign/stale artifact.
    // Deletion-only that resolved to an owning crate (or include! consumers) is green above;
    // this refuse is the residual "wrong reason" case (orphan path, no package, no include!).
    if listed > 0 && touched == 0 {
        wprintln!(
            "  touch_changed: REFUSING — {listed} changed Rust file(s) listed, but no source and no"
        );
        wprintln!(
            "                 owning crate Cargo.toml could be touched, so no cargo fingerprint was"
        );
        wprintln!(
            "                 invalidated. Every step below could run on a stale or foreign artifact."
        );
        return 1;
    }
    0
}

/// Invalidate the freshness of every workspace member's sources before the gate's check steps.
///
/// **Why a check step needs it.** Cargo's freshness test is mtime-based: a unit is fresh when no
/// source file is newer than its recorded output. A `cargo check` or `clippy` emits no binary to
/// run, yet it still returns a verdict from recorded output, so it can pass a file it never
/// opened. Two ways that happens:
///
/// * A source file whose bytes change while its mtime is set back (`touch -r`) to the original
///   keeps its unit fresh: `cargo check` answers 0 over a file that cannot compile, and answers
///   101 once the file is touched with identical bytes.
/// * Another worktree building into a shared build folder leaves an `.rmeta` compiled from its
///   tree; a check from a tree without that tree's symbols finishes at once and reports that
///   foreign `.rmeta` as its own artifact.
///
/// **Why a private build folder is not enough on its own.** Freshness is decided by mtime, so a
/// private folder changes only whose artifacts are present, not how freshness is judged: the
/// first case stays green there. The touch is what cures both cases; the private folder keeps the
/// touch sufficient, because it bounds the writers to serialised gates, so nothing can re-freshen
/// a fingerprint against another tree's source between this touch and the last step.
///
/// **Why the whole workspace and not the diff.** [`touch_changed`] covers `$base..HEAD` union
/// `git status --porcelain`, and that defence stays. It cannot cover a member this slice did not
/// change but another tree built: provenance is not a property of the diff, so the invalidation
/// cannot be scoped to the diff.
///
/// **Cost.** The touch invalidates the workspace units only, never a dependency unit; the
/// dependency graph is what makes a cold build expensive and none of it is touched, so a touched
/// `cargo check --workspace` costs about a second over a warm one.
pub(crate) fn touch_workspace(ctx: &Ctx) -> i32 {
    let dirs = match workspace_members() {
        Ok(dirs) => dirs,
        Err(reason) => {
            wprintln!(
                "  touch_workspace: REFUSING — the workspace members could not be read out of Cargo.toml"
            );
            wprintln!(
                "                   ({reason:?}), so no fingerprint was invalidated and every"
            );
            wprintln!(
                "                   cargo step below could report on another tree's artifacts."
            );
            return 1;
        }
    };
    // Non-vacuity, first layer: a manifest reformat that parses to the empty set would "succeed"
    // here and touch nothing, which is the same lie one level up.
    if dirs.is_empty() {
        wprintln!(
            "  touch_workspace: REFUSING — parsed ZERO workspace members out of Cargo.toml, so no"
        );
        wprintln!(
            "                   fingerprint was invalidated and every cargo step below could report on"
        );
        wprintln!("                   another tree's artifacts. Fix the parse, or the manifest.");
        return 1;
    }
    let mut missing = String::new();
    let mut n = 0usize;
    for d in &dirs {
        if !Path::new(d).is_dir() {
            missing.push(' ');
            missing.push_str(d);
            continue;
        }
        let mut files: Vec<PathBuf> = Vec::new();
        for e in walkdir::WalkDir::new(d).into_iter().flatten() {
            if e.file_type().is_file() && e.path().extension().map(|x| x == "rs").unwrap_or(false) {
                files.push(e.path().to_path_buf());
            }
        }
        // `-exec … +` over one find: 289 files in a single touch, not 289 processes.
        touch_paths(&files);
        n += files.len();
    }
    // A member named by the manifest but absent from disk means the parse and the tree disagree,
    // and the crates behind the missing entries are precisely the ones that would keep a stale
    // verdict.
    if !missing.is_empty() {
        wprintln!(
            "  touch_workspace: REFUSING — Cargo.toml names workspace member(s) that are not on disk:"
        );
        wprintln!("                 {missing}");
        wprintln!(
            "                   Their fingerprints were not invalidated, so a cargo step could still be"
        );
        wprintln!("                   handed an artifact built from another worktree's source.");
        return 1;
    }
    // Non-vacuity, second layer. Members parsed, directories present, and still no .rs file found:
    // nothing was invalidated and "examined nothing" is not "examined everything and it was fine".
    if n == 0 {
        wprintln!(
            "  touch_workspace: REFUSING — found ZERO .rs files under the workspace members, so cargo's"
        );
        wprintln!(
            "                   fingerprints are untouched and every check/clippy verdict below would be"
        );
        wprintln!(
            "                   about whatever was last built into {}.",
            ctx.gate_check_target
        );
        return 1;
    }
    let incl_paths = match compiled_include_input_paths() {
        Ok(paths) => paths,
        Err(reason) => {
            wprintln!(
                "  touch_workspace: REFUSING — the include_str!/include_bytes! inputs could not be"
            );
            wprintln!(
                "                   listed ({reason:?}), so their fingerprints stay as they were."
            );
            return 1;
        }
    };
    let existing: Vec<PathBuf> = incl_paths.into_iter().filter(|p| p.is_file()).collect();
    let incl_n = existing.len();
    touch_paths(&existing);
    wprintln!(
        "touch_workspace: invalidated {n} workspace .rs file(s) and {incl_n} include_str!/include_bytes! input(s) across {} member(s)",
        dirs.len()
    );
    0
}
