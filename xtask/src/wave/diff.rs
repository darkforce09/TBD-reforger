//! The verdict-diff harness — `cargo xtask platform wave diff <arm>`.
//!
//! Not in the bash. It exists because a port of THIS file cannot be accepted on inspection: the
//! acceptance criterion is byte-for-byte stdout + stderr + rc against `scripts/platform/wave.sh`,
//! and for mutating commands the post-state too.
//!
//! ── ANTI-VACUITY (T-556) IS THE POINT ───────────────────────────────────────────────────────
//!
//! Two agents in this program produced a fake pass because an arm agreed at rc=0 on both sides
//! while testing nothing. So every arm here carries an EXPECTATION about the bash side — a
//! required rc, or a substring that must appear — and the arm FAILS if the bash did not do what
//! the arm claims to be comparing. An arm that cannot prove the bash went where it meant to send
//! it is reported as VACUOUS and counts as a failure, not a pass.
//!
//! ── NEVER THE LIVE TREE ─────────────────────────────────────────────────────────────────────
//!
//! Mutating arms run against a throwaway `git clone --shared`, with `origin` re-pointed at a bare
//! repo under the scratch directory so a `push` arm can complete without touching anything real.
//!
//! ── ON REPRODUCIBILITY ──────────────────────────────────────────────────────────────────────
//!
//! Some output is not reproducible run-to-run: cargo wall clocks, `Compiling` lines on a cold
//! cache, `$CARGO_TARGET_DIR` in paths. [`normalise`] removes exactly those, and the `noise-floor`
//! arm runs BASH TWICE and diffs it against itself so the claim "these two agree" is reported next
//! to "and here is what disagreement looks like when nothing changed".

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{Ctx, base, host};
use crate::wprintln;

/// One side's result.
pub struct Run {
    pub out: String,
    pub rc: i32,
}

pub fn scratch() -> PathBuf {
    let p = std::env::var("TBD_WAVE_DIFF_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::temp_dir().join(format!("t853-wavediff-{}", std::process::id()))
        });
    let _ = std::fs::create_dir_all(&p);
    p
}

/// Run `bash scripts/platform/wave.sh <args>` in `cwd`, merged output + rc.
pub fn bash_side(cwd: &Path, args: &[&str]) -> Run {
    let mut c = Command::new("bash");
    c.arg("scripts/platform/wave.sh")
        .args(args)
        .current_dir(cwd);
    let out = c.output().expect("bash");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        out: s,
        rc: host::status_code(&out.status),
    }
}

/// Run THIS binary's `platform wave <args>` in `cwd`, merged output + rc.
pub fn rust_side(cwd: &Path, args: &[&str]) -> Run {
    let exe = std::env::current_exe().expect("current_exe");
    let mut c = Command::new(exe);
    c.args(["platform", "wave"]).args(args).current_dir(cwd);
    let out = c.output().expect("xtask");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        out: s,
        rc: host::status_code(&out.status),
    }
}

/// Strip the parts that are genuinely not reproducible run-to-run.
///
/// `sed -E 's/ in [0-9]+\.[0-9]+s/ in Xs/g'` is the cargo wall clock; the rest are cargo's
/// progress lines and absolute scratch paths. Nothing here touches a verdict, a refusal message or
/// a rc — if an arm needs more normalisation than this to agree, it does not agree.
pub fn normalise(s: &str) -> String {
    let time = regex::Regex::new(r" in [0-9]+\.[0-9]+s").expect("static regex");
    let mut out = String::new();
    for line in s.lines() {
        let t = line.trim_start();
        if t.starts_with("Compiling ") || t.starts_with("Finished ") || t.starts_with("Blocking ") {
            continue;
        }
        out.push_str(&time.replace_all(line, " in Xs"));
        out.push('\n');
    }
    out
}

/// The result of one arm.
pub struct ArmResult {
    pub name: String,
    pub ok: bool,
    pub note: String,
}

pub fn compare(name: &str, b: &Run, r: &Run, expect: impl Fn(&Run) -> Option<String>) -> ArmResult {
    // ANTI-VACUITY FIRST. If the bash did not go where this arm claims, nothing below is evidence.
    if let Some(why) = expect(b) {
        return ArmResult {
            name: name.into(),
            ok: false,
            note: format!("VACUOUS — the bash side did not reach the state under test: {why}"),
        };
    }
    let bn = normalise(&b.out);
    let rn = normalise(&r.out);
    if b.rc != r.rc {
        return ArmResult {
            name: name.into(),
            ok: false,
            note: format!("rc differs: bash={} rust={}", b.rc, r.rc),
        };
    }
    if bn != rn {
        let mut note = String::from("stdout+stderr differ:\n");
        for (i, (x, y)) in bn.lines().zip(rn.lines()).enumerate() {
            if x != y {
                note.push_str(&format!(
                    "  line {}:\n    bash: {x:?}\n    rust: {y:?}\n",
                    i + 1
                ));
            }
        }
        let (bl, rl) = (bn.lines().count(), rn.lines().count());
        if bl != rl {
            note.push_str(&format!("  line count: bash={bl} rust={rl}\n"));
        }
        return ArmResult {
            name: name.into(),
            ok: false,
            note,
        };
    }
    ArmResult {
        name: name.into(),
        ok: true,
        note: format!("identical (rc={}, {} lines)", b.rc, bn.lines().count()),
    }
}

/// `git clone --shared` of the live repo into the scratch dir. NEVER the live tree.
///
/// The LFS filter neutralisation is not optional here and it is the same set `tree_state` uses:
/// git-lfs is installed neither in the container nor on the host, so a plain clone dies mid-checkout
/// with `git-lfs filter-process: not found` / `fatal: the remote end hung up unexpectedly` and
/// leaves a PARTIAL working tree. A harness that compared two implementations over a half-checked-out
/// repo would be measuring the checkout, not the port.
pub fn make_clone(ctx: &Ctx, name: &str) -> Option<PathBuf> {
    let dst = scratch().join(name);
    let _ = std::fs::remove_dir_all(&dst);
    let ok = Command::new("git")
        .args(super::ledger::LFS_NEUTRAL)
        .args(["clone", "--shared", "--no-hardlinks", "-q"])
        .arg(&ctx.main_root)
        .arg(&dst)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return None;
    }
    // The clone's own config must carry the neutralisation too — every later `git` in this
    // directory (including the ones wave.sh runs) would otherwise hit the missing filter.
    for (k, v) in [
        ("filter.lfs.process", ""),
        ("filter.lfs.clean", "cat"),
        ("filter.lfs.smudge", "cat"),
        ("filter.lfs.required", "false"),
    ] {
        let _ = Command::new("git")
            .args(["-C", &dst.display().to_string(), "config", k, v])
            .status();
    }
    Some(dst)
}

pub fn cmd_diff(ctx: &Ctx, args: &[String]) -> u8 {
    let arm = args.first().map(String::as_str).unwrap_or("all");
    match arm {
        // Internal probe: print the derived wave base and nothing else. Used by the base arm.
        "base-probe" => match base::prev_wave_close() {
            Some(s) => {
                wprintln!("{s}");
                0
            }
            None => 1,
        },
        // Internal: hold the gate lock for N seconds so the `lock` arm can put THIS
        // implementation on the holding side. There is no other way to script that direction —
        // `GateState` has no public way to hold a lock without running a gate, which is the
        // T-406 property.
        "hold-lock" => {
            let secs: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);
            let mut st = super::lock::GateState::new();
            let rc = st.take(ctx, "diff hold-lock");
            if rc != 0 {
                return rc;
            }
            wprintln!("held");
            super::flush();
            std::thread::sleep(std::time::Duration::from_secs(secs));
            0
        }
        "lock" => report(super::diff_arms::arm_lock(ctx)),
        "reclaim" => report(super::diff_reclaim::arm_reclaim(ctx)),
        "migrate" => report(super::diff_arms::arm_migrate_persist(ctx)),
        "status" => report(vec![super::diff_arms::arm_status(ctx)]),
        "base" => report(vec![super::diff_arms::arm_base(ctx)]),
        "refusals" => report(super::diff_arms::arm_refusals(ctx)),
        "push-guard" => report(super::diff_arms::arm_push_guard(ctx)),
        "noise-floor" => report(vec![super::diff_arms::arm_noise_floor(ctx)]),
        "all" => {
            use super::diff_arms::*;
            let mut v = vec![arm_noise_floor(ctx), arm_status(ctx), arm_base(ctx)];
            v.extend(arm_refusals(ctx));
            v.extend(arm_push_guard(ctx));
            v.extend(arm_lock(ctx));
            v.extend(super::diff_reclaim::arm_reclaim(ctx));
            v.extend(arm_migrate_persist(ctx));
            report(v)
        }
        other => {
            wprintln!("wave diff: unknown arm '{other}'");
            wprintln!("  arms: status | base | refusals | push-guard | noise-floor | all");
            2
        }
    }
}

fn report(rs: Vec<ArmResult>) -> u8 {
    let mut bad = 0;
    for r in &rs {
        wprintln!(
            "  {:<28} {}  {}",
            r.name,
            if r.ok { "OK  " } else { "DIFF" },
            r.note
        );
        if !r.ok {
            bad += 1;
        }
    }
    wprintln!();
    wprintln!(
        "verdict-diff: {}/{} arms identical",
        rs.len() - bad,
        rs.len()
    );
    if bad > 0 { 1 } else { 0 }
}
