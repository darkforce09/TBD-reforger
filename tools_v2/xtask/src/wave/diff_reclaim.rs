//! The `reclaim` arm — the one arm whose subject is `rm -rf`.
//!
//! ── WHY THIS IS ITS OWN FIXTURE AND NOT A CLONE ─────────────────────────────────────────────
//!
//! `cmd_reclaim` sweeps three roots: `/var/tmp/*target*` &co, `$MAIN_ROOT/target-*`, and
//! `$HOME/.cache/tbd-target-T-*`. Two of those are the OPERATOR'S: running the real sweep to test a
//! port would delete live agent caches, which is precisely the class of damage this whole port is
//! written to avoid. So:
//!
//!   * `$MAIN_ROOT` is the throwaway clone (it comes from `git rev-parse --git-common-dir`), so the
//!     `target-*` sweep lands inside the scratch directory by construction.
//!   * `$HOME` is REPOINTED at a fabricated home under the scratch directory. This is the only way
//!     to exercise the `~/.cache/tbd-target-T-*` sweep at all without touching the real one —
//!     measured on this machine at the time of writing: 10 real `tbd-target-T-*` dirs, several
//!     belonging to slices that are still live.
//!   * `/var/tmp` is left alone and verified EMPTY of matches first; if it is not, the arm reports
//!     that rather than deleting somebody's cache.
//!
//! ── WHY THE FIXTURE IS BUILT TWICE ──────────────────────────────────────────────────────────
//!
//! The bash run DELETES the dirs, so the Rust run would otherwise see an empty tree and "agree"
//! about nothing — the fake-pass shape again. The fixture is therefore rebuilt byte-identically
//! between the two runs, and the arm asserts the bash actually removed something.
//!
//! ── THE DISTINCTION UNDER TEST ──────────────────────────────────────────────────────────────
//!
//! A live slice's cache must be SPARED and a dead slice's REMOVED, and the two names differ only in
//! their digits. The fixture creates a real `git worktree` for `T-999` so the spared set is
//! non-empty — without it the arm would only ever prove that everything gets deleted.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::Ctx;
use super::diff::{ArmResult, bash_side, compare, make_clone, rust_side, scratch};

/// The dirs the fixture plants, and what each one proves.
const MAIN_DIRS: &[(&str, &str)] = &[
    ("target-T-888", "orphan slice dir — MUST be removed"),
    ("target-T-999", "live slice dir — MUST be spared"),
    ("target-T-999-api", "live slice's api dir — MUST be spared"),
    (
        "target-dev-api",
        "operator's cargo xtask mk rust-api cache — MUST be spared, by name",
    ),
    (
        "target-ci",
        "unparseable name — MUST be spared, not guessed at",
    ),
    (
        "target-gate-check",
        "gate dir — opt-in only, MUST be spared by default",
    ),
    ("dist-gate-frontend", "gate dist — opt-in only"),
];

const HOME_DIRS: &[(&str, &str)] = &[
    ("tbd-target", "the SHARED agent cache — never reclaimed"),
    ("tbd-target-T-888", "orphan ad-hoc dir — MUST be removed"),
    ("tbd-target-T-999", "live slice ad-hoc dir — MUST be spared"),
    (
        "tbd-target-wave138-verify",
        "non-T-* verifier path — MUST be spared",
    ),
];

fn plant(dir: &Path, entries: &[(&str, &str)]) {
    for (name, why) in entries {
        let d = dir.join(name);
        let _ = std::fs::create_dir_all(&d);
        // Identical bytes on both runs so `du -sm` reports the same size for each.
        let _ = std::fs::write(d.join("filler.txt"), format!("{why}\n"));
    }
}

/// Build (or rebuild) the whole fixture. Returns the fake `$HOME`.
fn build_fixture(clone: &Path) -> PathBuf {
    let home = scratch().join("fakehome");
    let cache = home.join(".cache");
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::create_dir_all(&cache);
    plant(&cache, HOME_DIRS);
    for (name, _) in MAIN_DIRS {
        let _ = std::fs::remove_dir_all(clone.join(name));
    }
    plant(clone, MAIN_DIRS);
    home
}

pub fn arm_reclaim(ctx: &Ctx) -> Vec<ArmResult> {
    // NEVER run this arm if the real `/var/tmp` holds anything the sweep would eat. The sweep's
    // first root is not overridable, so the only safe posture is to refuse.
    let var_tmp_hits: Vec<String> = std::fs::read_dir("/var/tmp")
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.contains("target") || n.starts_with("v2-"))
                .collect()
        })
        .unwrap_or_default();
    if !var_tmp_hits.is_empty() {
        return vec![ArmResult {
            name: "reclaim".into(),
            ok: false,
            note: format!(
                "REFUSED to run — /var/tmp holds {} dir(s) this sweep would DELETE for real: {}. \
                 The arm is not worth an operator's cache.",
                var_tmp_hits.len(),
                var_tmp_hits.join(" ")
            ),
        }];
    }

    let Some(dir) = make_clone(ctx, "reclaim") else {
        return vec![ArmResult {
            name: "reclaim".into(),
            ok: false,
            note: "could not clone".into(),
        }];
    };
    // A real linked worktree, so `git worktree list` reports a live slice and the spared set is
    // non-empty. Without this the arm could only ever prove that everything is deleted.
    let _ = Command::new("git")
        .args([
            "-C",
            &dir.display().to_string(),
            "worktree",
            "add",
            "-q",
            "-b",
            "slice/T-999",
        ])
        .arg(dir.join(".ai/artifacts/worktrees/T-999"))
        .output();

    let run = |which: &str, args: &[&str]| -> super::diff::Run {
        let home = build_fixture(&dir);
        // SAFETY OF THE ARM ITSELF: HOME is repointed for the child only. The real
        // ~/.cache/tbd-target-* is never in reach of either implementation here.
        let home_s = home.display().to_string();
        let prev = std::env::var("HOME").unwrap_or_default();
        unsafe { std::env::set_var("HOME", &home_s) };
        let r = if which == "bash" {
            bash_side(&dir, args)
        } else {
            rust_side(&dir, args)
        };
        unsafe { std::env::set_var("HOME", prev) };
        r
    };

    let mut out = Vec::new();
    let mut post_state: Option<PathBuf> = None;
    for (label, args) in [
        (
            "reclaim --no-slice-dirs",
            vec!["reclaim", "--no-slice-dirs"],
        ),
        ("reclaim --gate-dirs", vec!["reclaim", "--gate-dirs"]),
        // LAST on purpose: the post-state assertion below inspects the tree this arm leaves, and
        // `--gate-dirs` legitimately removes the gate dirs the default sweep must spare. Checking
        // after the wrong arm reported the port had deleted something it had not.
        ("reclaim (default sweep)", vec!["reclaim"]),
    ] {
        let b = run("bash", &args);
        let home = build_fixture(&dir);
        let prev = std::env::var("HOME").unwrap_or_default();
        unsafe { std::env::set_var("HOME", home.display().to_string()) };
        let r = rust_side(&dir, &args);
        unsafe { std::env::set_var("HOME", prev) };
        if args.len() == 1 {
            post_state = Some(home);
        }
        out.push(compare(label, &b, &r, |b| {
            // ANTI-VACUITY, tuned per arm. The default sweep MUST have both removed and spared
            // something, or it is not exercising the distinction this function exists for.
            if !b.out.contains("live slices (spared):") {
                return Some(format!(
                    "bash never reported a spared set: {:?}",
                    b.out.lines().next()
                ));
            }
            if args.contains(&"--no-slice-dirs") {
                if !b.out.contains("not swept (--no-slice-dirs)") {
                    return Some("bash did not take the --no-slice-dirs path".into());
                }
            } else if args.contains(&"--gate-dirs") {
                if !b.out.contains("gate dirs (--gate-dirs") {
                    return Some("bash did not take the --gate-dirs path".into());
                }
            } else {
                if !b.out.contains("removed ") {
                    return Some("bash removed NOTHING — the orphan fixture was not swept".into());
                }
                if !b.out.contains("(live slice T-999)") {
                    return Some(
                        "bash spared NOTHING by liveness — the spared set was empty".into(),
                    );
                }
            }
            None
        }));
    }

    // POST-STATE, which is the half a stdout diff cannot see: after the RUST run the orphan must be
    // gone and every spared dir must still be there.
    let Some(home) = post_state else {
        return out;
    };
    let mut wrong: Vec<String> = Vec::new();
    for (name, why) in MAIN_DIRS {
        let present = dir.join(name).exists();
        let want = !why.contains("MUST be removed");
        if present != want {
            wrong.push(format!("{name} present={present} want={want}"));
        }
    }
    for (name, why) in HOME_DIRS {
        let present = home.join(".cache").join(name).exists();
        let want = !why.contains("MUST be removed");
        if present != want {
            wrong.push(format!("~/.cache/{name} present={present} want={want}"));
        }
    }
    out.push(ArmResult {
        name: "reclaim post-state".into(),
        ok: wrong.is_empty(),
        note: if wrong.is_empty() {
            format!(
                "after the port's own sweep: {} spared, orphans gone, shared caches intact",
                MAIN_DIRS.len() + HOME_DIRS.len() - 2
            )
        } else {
            format!("wrong post-state: {}", wrong.join("; "))
        },
    });
    out
}
