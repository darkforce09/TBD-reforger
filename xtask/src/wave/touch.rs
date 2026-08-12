//! Fingerprint invalidation, and the clippy step that depends on it.
//!
//! Force cargo to actually recompile what this slice changed.
//!
//! The shared `CARGO_TARGET_DIR` is necessary (a per-worktree target is ~44 GB) but it lets cargo
//! hand one worktree an artifact built from ANOTHER worktree's source. OBSERVED by T-193: `cargo
//! test` reported 113 passing from a binary that did not contain its new tests, and `--list` showed
//! main's 15 eden_chrome tests rather than its own 18. Touching the source forced a real rebuild
//! and the true numbers appeared.
//!
//! That means a slice gate can print PASS on source it never compiled — which makes every other
//! check in this file advisory. Bumping mtime on the changed files invalidates the fingerprint.

use std::path::{Path, PathBuf};

use super::changed::{
    changed_rs, compiled_include_input_paths, include_consumer_package_dirs, owning_package_dir,
    workspace_members,
};
use super::{Ctx, host};
use crate::{wprint, wprintln};

/// `touch` the given paths. Batched, because the bash's `-exec touch {} +` is one process for 289
/// files rather than 289 processes, and that difference is measurable at wave-gate scale.
///
/// `touch` CREATES a missing file, so every caller here must have proved the path exists first —
/// the bash relied on `[ -f ]` and on `find` for exactly that.
fn touch_paths(paths: &[PathBuf]) -> usize {
    let mut done = 0usize;
    for chunk in paths.chunks(2000) {
        let ok = std::process::Command::new("touch")
            .args(chunk)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            done += chunk.len();
        }
    }
    done
}

/// Bump mtime on everything this slice changed, so cargo cannot reuse a foreign artifact.
pub fn touch_changed(base: &str) -> i32 {
    // T-536: empty→listed=0→return 0 must not mask a failed changed_rs (e.g. git_porcelain_paths
    // rc≠0). Same class as T-492 for fmt_changed/clippy_changed — `for f in $(changed_rs …)`
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
        // still contains the deleted code. T-409: deletion-only used to hard-fail here while
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

/// T-421. The other half of the cure, and the half that actually makes the two repros red.
///
/// WHAT WAS WRONG WITH THE OLD REASONING. The comment on `gate_test_api` used to say `cargo
/// check`/`clippy` need no private dir "because they emit no binary to run". The exposure was never
/// about running a binary. Cargo's freshness test is MTIME-BASED: a unit is fresh when no source
/// file is newer than its recorded output. So a check step can return a verdict about a file it
/// never opened, and both of the ticket's repros are that one sentence:
///
///   A. MEASURED 2026-07-26. Append `THIS IS NOT RUST AND CANNOT COMPILE ###` to
///      `crates/map-engine-core/src/slot_line.rs`, then `touch -r` it back to its ORIGINAL mtime.
///      `cargo check --workspace --quiet` -> rc 0. `touch` it (identical bytes) -> rc 101,
///      "reserved multi-hash token is forbidden". The gate's own clippy line: same, 0 then 101.
///   B. MEASURED 2026-07-26. A sibling worktree added a const and built into the shared dir. From a
///      tree that does not contain that symbol, `cargo check -p map-engine-core --features
///      doc,mission,world` reported `Finished in 0.06s`, and `--message-format=json` named
///      `libmap_engine_core-<hash>.rmeta` as its own artifact — an rmeta that greps 1 for the
///      foreign symbol while the tree greps 0. The check stood on another tree's work and said
///      PASS.
///
/// WHY THE PRIVATE DIR IS NOT ENOUGH, which is the thing to not re-derive wrongly. MEASURED
/// 2026-07-26 against a freshly built `target-gate-check`: repro A run in the PRIVATE dir still
/// returned rc 0. Of course it does — the mechanism is mtime, and a private dir changes only whose
/// artifacts are there, not how freshness is decided. A private dir alone cures neither repro; it
/// is the touch that does, and the private dir is what keeps the touch sufficient (it bounds the
/// writers to serialised gates, so nothing can re-freshen a fingerprint against another tree's
/// source between our touch and our last step).
///
/// WHY THE WHOLE WORKSPACE AND NOT JUST THE DIFF. [`touch_changed`] already covers `$base..HEAD`
/// union `git status --porcelain`, and that defence is real — keep it. What it cannot cover is a
/// crate this slice did not touch but some OTHER tree did: wave 5's own 12/12 run touched only
/// map-engine-core, website-frontend and xtask, so website-api and every other member's verdict
/// rested on artifacts of unidentified provenance. Provenance is not a property of the diff, so the
/// invalidation cannot be scoped to the diff.
///
/// THE COST, and why it is not the "full recheck every run" it sounds like. MEASURED 2026-07-26:
/// the touch invalidates 14 of 14 workspace units and 0 of 696 dependency units — the 609-crate dep
/// graph is what makes a cold build expensive and NONE of it is touched. `cargo check --workspace`
/// goes 0.19 s warm -> 1.09 s touched. Nine tenths of a second buys a verdict about this tree.
pub fn touch_workspace(ctx: &Ctx) -> i32 {
    let dirs = workspace_members();
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
    let incl_paths = compiled_include_input_paths();
    let existing: Vec<PathBuf> = incl_paths.into_iter().filter(|p| p.is_file()).collect();
    let incl_n = existing.len();
    touch_paths(&existing);
    wprintln!(
        "touch_workspace: invalidated {n} workspace .rs file(s) and {incl_n} include_str!/include_bytes! input(s) across {} member(s)",
        dirs.len()
    );
    0
}

/// The `[package] name` of a crate directory.
fn package_name(dir: &str) -> Option<String> {
    let body = std::fs::read_to_string(Path::new(dir).join("Cargo.toml")).ok()?;
    // `sed -n '/^\[package\]/,/^\[/p'` — from `[package]` to the NEXT line starting with `[`.
    let mut in_pkg = false;
    for line in body.lines() {
        if !in_pkg {
            if line.starts_with("[package]") {
                in_pkg = true;
            }
            continue;
        }
        if line.starts_with('[') {
            break;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('"') {
                    if let Some(end) = rest.find('"') {
                        return Some(rest[..end].to_string());
                    }
                }
            }
        }
    }
    None
}

/// Clippy, scoped to the crates the slice actually touched, WITH `--all-targets`.
///
/// WHY THIS EXISTS: the slice gate ran check + wasm32 + fmt and no clippy at all, so a lint in a
/// slice's own code could not surface until the wave gate ran `clippy --all-targets` on merged main
/// — where it reads as somebody else's problem and blocks every other slice in the group. Hit for
/// real on T-329, which added a large test file: `doc_list_item_without_indentation` and an
/// unnecessary `to_string`, both in code it wrote, neither visible to the gate it was told to pass.
///
/// `--all-targets` is the load-bearing flag: the wave gate uses it, so tests and benches are gated
/// there. Without it here, a test-only lint is invisible to the agent and certain to land red. That
/// is exactly the T-329 case.
///
/// Scoped to changed crates rather than the workspace because `clippy --workspace -D warnings` is
/// red on clean main — a gate nothing can pass teaches agents that gate failures are noise. T-603
/// re-measured 2026-07-31: 60 errors, ALL of them website-frontend linted natively, none in
/// tools/tbd-tools or xtask (this note used to blame those two; they are clean and the wave gate
/// now lints them by name). Frontend goes through wasm32 with NO `-D`, matching ci.yml:113;
/// everything else takes `-D warnings`, matching the wave gate.
pub fn clippy_changed(ctx: &Ctx, base: &str) -> i32 {
    // T-492: propagate changed_rs failure — empty stdout + rc≠0 must not become SKIP.
    let files = match changed_rs(base) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    if files.is_empty() {
        wprintln!("no rust changes");
        return 0;
    }
    // Map each file to its owning crate by walking up to the nearest Cargo.toml with a [package]
    // name. Orphan fragments (apps/website/shared/*.rs) have no package ancestor — the walk stops
    // at '.' — but they are include!'d into real crates (T-405 is_http_url_cases.rs → website-api +
    // website-frontend). T-406's empty-crates refuse false-red'd that shape; resolve via include!
    // consumers before refusing.
    let mut crates: Vec<String> = Vec::new();
    let push_unique = |crates: &mut Vec<String>, c: String| {
        if !crates.contains(&c) {
            crates.push(c);
        }
    };
    for f in &files {
        if let Some(d) = owning_package_dir(f) {
            if let Some(c) = package_name(&d) {
                push_unique(&mut crates, c);
            }
            continue;
        }
        for pkg in include_consumer_package_dirs(f) {
            if !Path::new(&pkg).join("Cargo.toml").is_file() {
                continue;
            }
            if let Some(c) = package_name(&pkg) {
                push_unique(&mut crates, c);
            }
        }
    }
    // Non-vacuity. This branch used to print "no crate resolved" and return 0, i.e.
    // `clippy (changed crates) PASS` having compiled nothing. Printing a reason is not the same
    // as reporting a result: the verdict still read as clean.
    //
    // Deliberately NOT keyed on the files existing. A slice that DELETES a file leaves its crate's
    // Cargo.toml in place, the crate resolves, and clippy genuinely lints the crate the file was
    // removed from — that is real examination and must stay green. The vacuous case is exactly
    // this one: nothing to lint at all (no package ancestor AND no include! consumer).
    if crates.is_empty() {
        wprintln!(
            "clippy: REFUSING to pass — the changed Rust file(s) resolved to NO crate, so clippy was"
        );
        wprintln!(
            "        invoked ZERO times. 'examined nothing' is not 'examined everything and it was"
        );
        wprintln!("        fine'. (Files listed: {}.)", files.len());
        return 1;
    }
    for c in &crates {
        let argv: Vec<String> = match c.as_str() {
            // T-742: --all-targets is load-bearing (see function header) — without it, #[cfg(test)]
            // lints are invisible here and certain to land red once T-752 teaches CI/Makefile the
            // same flag. NO -D warnings: ci.yml website-frontend clippy is advisory (no -D),
            // matching the wave-gate `clippy frontend` step. Align -D with CI intent, not with the
            // other crates.
            "website-frontend" => host::v(&[
                "cargo",
                "clippy",
                "-p",
                "website-frontend",
                "--target",
                "wasm32-unknown-unknown",
                "--all-targets",
                "--quiet",
            ]),
            // T-614 — tbd-tools AND xtask USED TO BE SKIPPED HERE, reason "red on main, ungated by
            // CI". The first half was FALSE and contradicted by the header of this same function:
            // T-603's re-measure found the 60 workspace errors are ALL website-frontend and called
            // these two clean, and the wave gate has linted them by name since then. Re-verified
            // 2026-08-01 through this very function, both directions: with the arm removed, a
            // `format!("{}", "verify")` injected into tools/tbd-tools/src/enf/apidoc.rs and into
            // xtask/src/sync.rs made clippy_changed return 1 naming each file and line in turn, and
            // with the injections removed it returned 0 having actually compiled both crates. The
            // old arm returned 0 with BOTH injections in place, printing `(skipped tbd-tools: …)
            // (skipped xtask: …)` and compiling nothing. The second half is still true — the
            // ci.yml change was comment-only — which is exactly why the skip had to go: nothing
            // else lints them before merged main, so a slice editing ONLY these crates had its own
            // gate examine none of its code and landed its lint at the wave gate, where it reads as
            // somebody else's problem and blocks the whole group. They now fall through to the
            // default arm below, like every other crate.
            //
            // --features doc,mission,world is REQUIRED (same floor as --all-features / the gate
            // test step). lib.rs gates doc/mission/world behind features, so a featureless clippy
            // COMPILES NONE OF THEM and reports success on code it never read. PROVED by
            // perturbation 2026-07-26: a `format!("{}", "verify")` injected into flatten.rs:767 —
            // the file this script's own comment calls the most contended in the backlog — gave
            // `clippy (changed crates) PASS` / `SLICE GATE: PASS` without features, and
            // `error: useless use of format!` with them. The adversarial verifier found this; the
            // gate did not.
            "map-engine-core" => host::v(&[
                "cargo",
                "clippy",
                "-p",
                "map-engine-core",
                "--features",
                "doc,mission,world",
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ]),
            other => host::v(&[
                "cargo",
                "clippy",
                "-p",
                other,
                "--all-targets",
                "--quiet",
                "--",
                "-D",
                "warnings",
            ]),
        };
        let (out, rc) = host::capture(&ctx.host.checkrun_argv(&ctx.gate_check_target, &argv));
        wprint!("{out}");
        if rc != 0 {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_name_reads_only_the_package_table() {
        let dir = std::env::temp_dir().join(format!("t853-pkg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"website-api\"\nedition = \"2024\"\n\n[dependencies]\nname = \"wrong\"\n",
        )
        .unwrap();
        assert_eq!(
            package_name(&dir.display().to_string()),
            Some("website-api".into())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
