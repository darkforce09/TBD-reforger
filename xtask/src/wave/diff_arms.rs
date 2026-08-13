//! The verdict-diff ARMS — one scenario each.
//!
//! Split out of [`super::diff`] purely for size (SIZE-3); the driver, the comparator and the
//! anti-vacuity contract all live there and are documented there. Every function here builds a
//! state, runs both implementations over it, and hands the pair to `compare` together with the
//! expectation that proves the BASH side actually reached the state under test.

use std::path::Path;
use std::process::Command;

use super::diff::{
    ArmResult, Run, bash_side, compare, make_clone, normalise, rust_side, scratch,
    strip_parent_target_dir,
};
use super::{Ctx, host};

/// THE NOISE FLOOR. Run BASH TWICE and diff it against itself.
///
/// Without this, "bash and rust agree" is unfalsifiable: an arm whose output is nondeterministic
/// would disagree with everything including itself, and an arm that is trivially empty agrees with
/// everything. This reports what disagreement looks like when NOTHING changed.
pub fn arm_noise_floor(ctx: &Ctx) -> ArmResult {
    let Some(dir) = make_clone(ctx, "noise") else {
        return ArmResult {
            name: "noise-floor".into(),
            ok: false,
            note: "could not clone".into(),
        };
    };
    let a = bash_side(&dir, &["status"]);
    let b = bash_side(&dir, &["status"]);
    let ok = normalise(&a.out) == normalise(&b.out) && a.rc == b.rc;
    ArmResult {
        name: "noise-floor (bash vs bash)".into(),
        ok,
        note: if ok {
            format!(
                "bash agrees with itself — floor is 0 differing lines (rc={})",
                a.rc
            )
        } else {
            "bash DISAGREES WITH ITSELF — no arm below can be believed".into()
        },
    }
}

/// GATE-LOCK INTEROP, BOTH DIRECTIONS, WITH THE REAL DRIVERS.
///
/// A half-ported factory WILL run `bash scripts/platform/wave.sh gate --slice` and
/// `cargo xtask platform wave gate --slice` at the same time. They must contend on the SAME
/// `$MAIN_ROOT/target/.tbd-gate.lock`, and the loser must REFUSE rather than run unserialised.
///
/// The lock path is overridden to a scratch file and the poll/max shortened, so this arm never
/// touches the real gate lock and never waits an hour to prove a refusal.
pub fn arm_lock(ctx: &Ctx) -> Vec<ArmResult> {
    let Some(dir) = make_clone(ctx, "lock") else {
        return vec![ArmResult {
            name: "gate lock".into(),
            ok: false,
            note: "could not clone".into(),
        }];
    };
    // `gate --slice` refuses at `refuse_empty_range` BEFORE it ever reaches the lock, and a fresh
    // clone is clean, so without this the arm reports VACUOUS — which is exactly what it did on its
    // first run. One untracked file makes the porcelain non-empty and lets both drivers get as far
    // as `take_gate_lock`, which is the only thing this arm is about.
    let _ = std::fs::write(
        dir.join("t853-lock-arm-dirty.txt"),
        b"make the change set non-empty\n",
    );

    let lockfile = scratch().join("interop.lock");
    let _ = std::fs::remove_file(&lockfile);
    let _ = std::fs::remove_file(scratch().join("interop.lock.holder"));
    let _ = std::fs::write(&lockfile, b"");
    let exe = std::env::current_exe().expect("current_exe");

    let with_env = |c: &mut Command| {
        c.env("TBD_GATE_LOCK", &lockfile)
            .env("TBD_GATE_LOCK_POLL", "1")
            .env("TBD_GATE_LOCK_MAX", "3");
    };
    let side = |prog: &str, args: &[&str], cwd: &Path| -> Run {
        let mut c = Command::new(prog);
        c.args(args).current_dir(cwd);
        with_env(&mut c);
        strip_parent_target_dir(&mut c);
        let o = c.output().expect("side");
        let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&o.stderr));
        Run {
            out: s,
            rc: host::status_code(&o.status),
        }
    };

    // Is the lock currently free? One non-blocking probe, which is also the only direct evidence
    // either way.
    let is_free = |p: &Path| -> bool {
        Command::new("flock")
            .args(["-n", "-x"])
            .arg(p)
            .args(["-c", "true"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    let settle = |p: &Path, want_free: bool| -> bool {
        for _ in 0..100 {
            if is_free(p) == want_free {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        false
    };

    let mut out = Vec::new();

    // DIRECTION 1 — bash's `flock(1)` holds; both drivers must refuse identically.
    for (label, holder_kind) in [
        ("gate lock: bash flock holds", "flock"),
        ("gate lock: this port holds", "rust"),
    ] {
        // THE PREVIOUS DIRECTION'S HOLDER MUST BE FULLY GONE, and killing it is not enough.
        //
        // MEASURED here: `flock -x FILE -c 'sleep 12'` forks a shell that INHERITS the lock
        // descriptor, so SIGKILLing `flock` leaves `sleep` holding the lock for the rest of its
        // run. Direction 2 then spawned its holder into a lock the corpse still owned, the driver
        // under test waited, the corpse exited, and the driver acquired — which the arm reported as
        // "bash waited but did not refuse". That is the SAME fd-inheritance mechanism `wave.sh`'s
        // LOCK RELEASE note records for `exec 9>>`, met here in the harness. Wait for the lock to
        // actually be free instead of assuming a kill freed it.
        if !settle(&lockfile, true) {
            out.push(ArmResult {
                name: label.into(),
                ok: false,
                note:
                    "VACUOUS — the lock was still held by a previous holder; nothing was measured"
                        .into(),
            });
            continue;
        }
        let mut holder = match holder_kind {
            "flock" => {
                let mut c = Command::new("flock");
                c.arg("-x").arg(&lockfile).args(["-c", "sleep 12"]);
                c.stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                c.spawn().expect("flock holder")
            }
            _ => {
                let mut c = Command::new(&exe);
                c.args(["platform", "wave", "diff", "hold-lock", "20"])
                    .current_dir(&dir);
                // A GENEROUS max for the HOLDER only: it must not give up while a previous
                // direction's killed holder is still releasing. The drivers under test keep the
                // short max — that is the number this arm is measuring.
                c.env("TBD_GATE_LOCK", &lockfile)
                    .env("TBD_GATE_LOCK_POLL", "1")
                    .env("TBD_GATE_LOCK_MAX", "30");
                c.stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                c.spawn().expect("rust holder")
            }
        };
        // WAIT FOR THE HOLDER TO ACTUALLY HOLD, rather than sleeping and hoping. A fixed sleep let
        // direction 2 run against a lock nobody had taken — the arm reported VACUOUS, correctly,
        // and this is the fix. Poll until a non-blocking `flock -n` is REFUSED, which is the only
        // direct evidence that the lock is held.
        if !settle(&lockfile, false) {
            let _ = holder.kill();
            let _ = holder.wait();
            out.push(ArmResult {
                name: label.into(),
                ok: false,
                note: format!(
                    "VACUOUS — the {holder_kind} holder never took the lock, so nothing contended"
                ),
            });
            continue;
        }

        let b = side(
            "bash",
            &["scripts/platform/wave.sh", "gate", "--slice", "T-853"],
            &dir,
        );
        let r = side(
            exe.to_str().unwrap_or("xtask"),
            &["platform", "wave", "gate", "--slice", "T-853"],
            &dir,
        );
        let _ = holder.kill();
        let _ = holder.wait();

        out.push(compare(label, &b, &r, |b| {
            // ANTI-VACUITY: the bash must have BLOCKED and then REFUSED. An arm where the bash sailed
            // through would have both sides "agreeing" about a lock nobody was holding.
            if !b.out.contains("gate: WAITING for the gate lock") {
                Some(format!(
                    "bash never waited — it took the lock. First line: {:?}",
                    first_line(&b.out)
                ))
            } else if !b.out.contains("gate: REFUSING — no lock after") {
                Some("bash waited but did not refuse".into())
            } else if b.rc != 2 {
                Some(format!(
                    "bash refused but returned rc {} instead of 2",
                    b.rc
                ))
            } else {
                None
            }
        }));
    }
    let _ = std::fs::remove_file(&lockfile);
    out
}

pub fn arm_status(ctx: &Ctx) -> ArmResult {
    let Some(dir) = make_clone(ctx, "status") else {
        return ArmResult {
            name: "status".into(),
            ok: false,
            note: "could not clone".into(),
        };
    };
    let b = bash_side(&dir, &["status"]);
    let r = rust_side(&dir, &["status"]);
    compare("status", &b, &r, |b| {
        // The bash must have produced a real report, not an empty tree's "ALL WAVES COMPLETE".
        if !b.out.contains("═══ platform program ═══") {
            return Some("no program banner".into());
        }
        if !b.out.contains("open:") {
            return Some("no open-ticket census".into());
        }
        None
    })
}

/// THE BASE DERIVATION, DIFFERENTIALLY, OVER THE ENTIRE HISTORY.
///
/// The derivation is a pure function of `git log`, so it can be probed at EVERY commit without a
/// checkout: `git update-ref --no-deref HEAD <sha>` moves HEAD in the clone and every `rev-list` /
/// `log` / `rev-parse` answer follows, while the working tree (which the derivation never reads)
/// stays put. That turns a multi-hour checkout sweep into a metadata-only walk.
///
/// The bash side is the REAL TEXT, extracted from `wave.sh` by line range rather than retyped:
/// `WAVE_CLOSE_MARKER_RE` through `prev_wave_close`. Retyping it would test this harness's copy of
/// the algorithm instead of the algorithm.
pub fn arm_base(ctx: &Ctx) -> ArmResult {
    let Some(dir) = make_clone(ctx, "base") else {
        return ArmResult {
            name: "base derivation".into(),
            ok: false,
            note: "could not clone".into(),
        };
    };
    // Extract the pure block from the bash. Anchored on the marker constant and the end of
    // prev_wave_close so a future edit that moves the block does not silently extract prose.
    let src = match std::fs::read_to_string(ctx.root.join("scripts/platform/wave.sh")) {
        Ok(s) => s,
        Err(_) => {
            return ArmResult {
                name: "base derivation".into(),
                ok: false,
                note: "bash driver deleted at T-902 — refusing to extract a missing wave.sh".into(),
            };
        }
    };
    let lines: Vec<&str> = src.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("WAVE_CLOSE_MARKER_RE="));
    let end = lines
        .iter()
        .position(|l| l.starts_with("# ── T-613: DOES ANYTHING OTHER THAN THE MARKER AGREE?"));
    let (Some(start), Some(end)) = (start, end) else {
        return ArmResult {
            name: "base derivation".into(),
            ok: false,
            note: "could not locate the base block in wave.sh — refusing to test a guess".into(),
        };
    };
    let block = lines[start..end].join("\n");
    // Non-vacuity on the EXTRACTION itself: the block must actually contain the three functions.
    for needle in [
        "wave_close_subject_ok()",
        "wave_close_disavowed()",
        "prev_wave_close()",
    ] {
        if !block.contains(needle) {
            return ArmResult {
                name: "base derivation".into(),
                ok: false,
                note: format!(
                    "extracted block is missing {needle} — refusing to claim a comparison"
                ),
            };
        }
    }
    let probe = scratch().join("base_fns.sh");
    let _ = std::fs::write(
        &probe,
        format!("set -uo pipefail\n{block}\nprev_wave_close\n"),
    );

    let all = super::git_stdout_lossy(&["-C", &dir.display().to_string(), "rev-list", "--all"]);
    let commits: Vec<&str> = all.lines().filter(|l| !l.is_empty()).collect();
    if commits.len() < 100 {
        return ArmResult {
            name: "base derivation".into(),
            ok: false,
            note: format!(
                "only {} commits reachable — that is not 'the entire history'",
                commits.len()
            ),
        };
    }
    let orig_head =
        super::git_stdout_lossy(&["-C", &dir.display().to_string(), "rev-parse", "HEAD"]);

    let mut checked = 0usize;
    let mut derived_some = 0usize;
    let mut unprobed: Vec<String> = Vec::new();
    let mut mismatches: Vec<String> = Vec::new();
    let exe = std::env::current_exe().expect("current_exe");
    for sha in &commits {
        // Move HEAD without touching the working tree.
        let ok = Command::new("git")
            .args([
                "-C",
                &dir.display().to_string(),
                "update-ref",
                "--no-deref",
                "HEAD",
                sha,
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            // A commit this walk could not point HEAD at was NOT compared. Counting it as passed
            // would be the exact overclaim this harness exists to prevent, so it is collected and
            // named in the verdict instead.
            unprobed.push((*sha).to_string());
            continue;
        }
        let mut bcmd = Command::new("bash");
        bcmd.arg(&probe).current_dir(&dir);
        strip_parent_target_dir(&mut bcmd);
        let bo = bcmd.output().expect("bash probe");
        // stdout AND stderr: the disavowal skip ("gate: skipping wave-close … reverted by …") is
        // written to stderr and NAMES TWO SHAS, so it is part of the derivation's observable
        // behaviour, not decoration. Comparing stdout alone would let a port that never noticed a
        // revert still agree on the answer whenever the fallthrough landed in the same place.
        let bash_out = format!(
            "{}{}",
            String::from_utf8_lossy(&bo.stdout),
            String::from_utf8_lossy(&bo.stderr)
        )
        .trim()
        .to_string();
        let bash_rc = host::status_code(&bo.status);

        let mut rcmd = Command::new(&exe);
        rcmd.args(["platform", "wave", "diff", "base-probe"])
            .current_dir(&dir);
        strip_parent_target_dir(&mut rcmd);
        let ro = rcmd.output().expect("rust probe");
        let rust_out = format!(
            "{}{}",
            String::from_utf8_lossy(&ro.stdout),
            String::from_utf8_lossy(&ro.stderr)
        )
        .trim()
        .to_string();
        let rust_rc = host::status_code(&ro.status);

        checked += 1;
        if !bash_out.is_empty() {
            derived_some += 1;
        }
        // rc: bash `prev_wave_close` returns 1 when nothing is reachable, 0 otherwise. The Rust
        // probe mirrors that.
        let bash_found = bash_rc == 0;
        let rust_found = rust_rc == 0;
        if bash_out != rust_out || bash_found != rust_found {
            mismatches.push(format!(
                "{sha}: bash={bash_out:?} (rc {bash_rc}) rust={rust_out:?} (rc {rust_rc})"
            ));
            if mismatches.len() > 10 {
                break;
            }
        }
    }
    // Restore the clone's HEAD so a later arm reusing the directory is not surprised.
    let _ = Command::new("git")
        .args([
            "-C",
            &dir.display().to_string(),
            "update-ref",
            "--no-deref",
            "HEAD",
            &orig_head,
        ])
        .status();

    // Mismatches first. T-902: an inherited CARGO_TARGET_DIR made every rust probe print a
    // three-line ignore banner; the walk broke at 11 mismatches, then this function reported
    // VACUOUS because derived_some was still < 50 — hiding the real DIFF behind a vacuity
    // claim. If they disagreed, that is the result; "both sides empty" can only be claimed
    // when they agreed.
    if !mismatches.is_empty() {
        return ArmResult {
            name: "base derivation".into(),
            ok: false,
            note: format!(
                "{} mismatch(es) over {checked} commits:\n    {}",
                mismatches.len(),
                mismatches.join("\n    ")
            ),
        };
    }
    // ANTI-VACUITY: a walk in which the derivation NEVER found a marker proves nothing — that is
    // "both sides returned empty", the exact fake-pass shape T-556 names.
    if derived_some < 50 {
        return ArmResult {
            name: "base derivation".into(),
            ok: false,
            note: format!(
                "VACUOUS — only {derived_some} of {checked} commits derived a base at all; both sides agreeing on 'nothing' is not evidence"
            ),
        };
    }
    let unprobed_note = if unprobed.is_empty() {
        String::new()
    } else {
        format!(
            "; {} of {} NOT PROBED (HEAD could not be pointed at them): {}",
            unprobed.len(),
            commits.len(),
            unprobed
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    ArmResult {
        name: "base derivation".into(),
        ok: true,
        note: format!(
            "identical over ALL {checked} of {} commits ({derived_some} derived a marker){unprobed_note}",
            commits.len()
        ),
    }
}

/// Every refusal path that can be exercised without building anything.
///
/// These are the arms where the two implementations must agree on a REFUSAL — the direction where
/// a port silently widening the tool does its damage.
pub fn arm_refusals(ctx: &Ctx) -> Vec<ArmResult> {
    let Some(dir) = make_clone(ctx, "refuse") else {
        return vec![ArmResult {
            name: "refusals".into(),
            ok: false,
            note: "could not clone".into(),
        }];
    };
    let mut out = Vec::new();

    let cases: &[(&str, &[&str], &str)] = &[
        // reclaim's argument allowlist — the provided baseline (rc 2, stderr).
        (
            "reclaim --dry-run",
            &["reclaim", "--dry-run"],
            "refusing unknown argument",
        ),
        // land's allowlist. `land T-204` was byte-for-byte `land` before T-?; a non-ticket
        // argument must refuse rather than land the whole wave.
        (
            "land --force",
            &["land", "--force"],
            "refusing unknown argument",
        ),
        ("land T-999999", &["land", "T-999999"], "not in wave"),
        // A ticket id where a rev belongs — the T-394 incident, three slices deep.
        (
            "gate T-394",
            &["gate", "T-394"],
            "is a ticket id, not a git base",
        ),
        (
            "gate nonsense",
            &["gate", "zzzznotarev"],
            "is not a resolvable commit",
        ),
        // `gate HEAD` resolves and is an ancestor and is still vacuous.
        ("gate HEAD", &["gate", "HEAD"], "refusing to run"),
        // The T-742 ad-hoc test refusals.
        (
            "test (no --slice)",
            &["test", "-p", "website-frontend"],
            "--slice T-nnn is required",
        ),
        (
            "test (bad slice id)",
            &["test", "--slice", "nope", "-p", "x"],
            "expected T-nnn",
        ),
        (
            "test (no args)",
            &["test", "--slice", "T-742"],
            "pass cargo test args",
        ),
        (
            "test (no -p)",
            &["test", "--slice", "T-742", "--lib"],
            "must include -p / --package",
        ),
        (
            "verified (no sha)",
            &["verified"],
            "usage: wave.sh verified",
        ),
        ("verified (bad sha)", &["verified", "zzzz"], "not a sha"),
        ("revert (no sha)", &["revert"], "usage: wave.sh revert"),
        ("revert (bad sha)", &["revert", "zzzz"], "no such commit"),
        (
            "unknown command",
            &["frobnicate"],
            "Platform wave lifecycle",
        ),
        // Not refusals, but the same shape of read-only comparison and free to run here.
        // `wave` reports the current wave's census; `wave --close` refuses while it is open, which
        // is the interesting branch (the other branch runs the full gate).
        ("wave", &["wave"], "═══ wave "),
        ("wave --close", &["wave", "--close"], "REFUSED: wave "),
    ];
    for (name, args, needle) in cases {
        let b = bash_side(&dir, args);
        let r = rust_side(&dir, args);
        out.push(compare(name, &b, &r, |b| {
            if !b.out.contains(needle) {
                Some(format!(
                    "bash output does not contain {needle:?}; got {:?}",
                    first_line(&b.out)
                ))
            } else if b.rc == 0
                && *name != "unknown command"
                && !name.starts_with("gate HEAD")
                // `wave` is a report, not a refusal — rc 0 is its correct outcome. The needle above
                // is what proves it did something.
                && *name != "wave"
            {
                Some("bash returned rc 0 — this arm is supposed to be a refusal".to_string())
            } else {
                None
            }
        }));
    }
    out
}

/// The T-599/T-600 push guard, on a purpose-built repo.
///
/// Four cases, and the interesting one is case 2: an LFS file that exists ONLY in an intermediate
/// commit. Before T-600 the guard returned rc 0 and empty output over it — it ALLOWED the push.
pub fn arm_push_guard(ctx: &Ctx) -> Vec<ArmResult> {
    let root = scratch().join("push");
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("work");
    let origin = root.join("origin.git");
    let _ = std::fs::create_dir_all(&repo);
    // A REAL bare origin under the scratch dir, so even the ALLOW case pushes somewhere harmless.
    let _ = Command::new("git")
        .args(["init", "-q", "--bare", "-b", "main"])
        .arg(&origin)
        .status();
    let g = |args: &[&str]| {
        // Captured, not inherited: `git add` on an lfs-attributed path prints the missing-filter
        // diagnostic, and letting it leak into this command's stdout would put fixture noise in the
        // middle of the verdict table.
        let _ = Command::new("git")
            .args(["-C", &repo.display().to_string()])
            .args(args)
            .output();
    };
    let _ = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .arg(&repo)
        .output();
    g(&["config", "user.email", "t853@example.invalid"]);
    g(&["config", "user.name", "t853"]);
    // THE FIXTURE MUST NEUTRALISE LFS, AND THAT IS THE WHOLE POINT OF THE SCENARIO. git-lfs is
    // absent on both sides here, so a `git add` of an lfs-attributed path fails and leaves the file
    // UNTRACKED — which is what made this arm report VACUOUS the first time it ran. With the
    // filters neutralised the blob is stored as ordinary bytes while `.gitattributes` still says
    // `filter=lfs`, and that is exactly the state the guard exists to refuse: a commit whose LFS
    // object was never uploaded because the clean filter never ran.
    for (k, v) in [
        ("filter.lfs.process", ""),
        ("filter.lfs.clean", "cat"),
        ("filter.lfs.smudge", "cat"),
        ("filter.lfs.required", "false"),
    ] {
        g(&["config", k, v]);
    }
    // The harness must run wave.sh from a tree that HAS wave.sh, so copy the script in.
    let _ = std::fs::create_dir_all(repo.join("scripts/platform"));
    let _ = std::fs::create_dir_all(repo.join(".ai/tickets"));
    let _ = std::fs::create_dir_all(repo.join("docs/platform"));
    let _ = std::fs::copy(
        ctx.root.join("scripts/platform/wave.sh"),
        repo.join("scripts/platform/wave.sh"),
    );
    let _ = std::fs::write(repo.join(".ai/tickets/registry.json"), r#"{"tickets":[]}"#);
    let _ = std::fs::write(
        repo.join("docs/platform/wave_plan.tsv"),
        "wave\tticket\ttitle\towns\n",
    );
    let _ = std::fs::write(
        repo.join(".gitattributes"),
        "*.tbd-sat filter=lfs diff=lfs merge=lfs -text\n",
    );
    let _ = std::fs::write(repo.join("ordinary.txt"), "hello\n");
    g(&["add", "-A"]);
    g(&["commit", "-qm", "base"]);
    g(&["remote", "add", "origin", &origin.display().to_string()]);
    g(&["push", "-q", "--no-verify", "origin", "main"]);

    let mut out = Vec::new();

    // CASE 1 — ordinary bytes under a tree the old guard matched by PATH. Must ALLOW (T-599).
    let _ = std::fs::create_dir_all(repo.join("packages/map-assets/everon/objects"));
    let _ = std::fs::write(
        repo.join("packages/map-assets/everon/objects/type-inventory.json"),
        "{}\n",
    );
    g(&["add", "-A"]);
    g(&["commit", "-qm", "ordinary content under map-assets"]);
    {
        let b = bash_side(&repo, &["push"]);
        // Undo the push the bash just performed so the Rust side sees the same range.
        let _ = Command::new("git")
            .args([
                "-C",
                &origin.display().to_string(),
                "update-ref",
                "refs/heads/main",
                "HEAD~1",
            ])
            .status();
        let _ = Command::new("git")
            .args(["-C", &repo.display().to_string(), "fetch", "-q", "origin"])
            .status();
        let r = rust_side(&repo, &["push"]);
        out.push(compare("push: ordinary bytes ALLOW", &b, &r, |b| {
            if b.out.contains("REFUSING") {
                Some("bash refused a legitimate push — the T-599 false positive is back".into())
            } else if b.rc != 0 {
                Some(format!(
                    "bash push failed (rc {}): {:?}",
                    b.rc,
                    first_line(&b.out)
                ))
            } else {
                None
            }
        }));
    }

    // CASE 2 — T-600: an LFS file present ONLY in an intermediate commit. Must REFUSE.
    let _ = std::fs::write(repo.join("everon.tbd-sat"), "pointerish\n");
    g(&["add", "-A"]);
    g(&["commit", "-qm", "add sat"]);
    let _ = std::fs::remove_file(repo.join("everon.tbd-sat"));
    g(&["add", "-A"]);
    g(&["commit", "-qm", "remove sat"]);
    {
        let b = bash_side(&repo, &["push"]);
        let r = rust_side(&repo, &["push"]);
        out.push(compare("push: T-600 intermediate LFS", &b, &r, |b| {
            if !b.out.contains("resolve to `filter: lfs`") {
                Some(format!("bash did not refuse; got {:?}", first_line(&b.out)))
            } else if b.rc == 0 {
                Some("bash returned rc 0 on a refusal".into())
            } else {
                None
            }
        }));
    }

    // CASE 3 — T-600 second disguise: the attribute rule is REMOVED by HEAD, so `check-attr` at
    // HEAD answers `unspecified`. Each commit's OWN .gitattributes must still refuse.
    let _ = std::fs::write(repo.join(".gitattributes"), "# lfs rule removed\n");
    g(&["add", "-A"]);
    g(&["commit", "-qm", "drop the lfs rule"]);
    {
        let b = bash_side(&repo, &["push"]);
        let r = rust_side(&repo, &["push"]);
        out.push(compare("push: rule gone by HEAD", &b, &r, |b| {
            if !b.out.contains("resolve to `filter: lfs`") {
                Some(format!("bash did not refuse; got {:?}", first_line(&b.out)))
            } else {
                None
            }
        }));
    }

    // CASE 4 — the cannot-tell direction. A range git cannot resolve must REFUSE, never allow.
    {
        let _ = Command::new("git")
            .args([
                "-C",
                &repo.display().to_string(),
                "remote",
                "set-url",
                "origin",
                "/nonexistent/nope.git",
            ])
            .status();
        let _ = Command::new("git")
            .args([
                "-C",
                &repo.display().to_string(),
                "update-ref",
                "-d",
                "refs/remotes/origin/main",
            ])
            .status();
        let b = bash_side(&repo, &["push"]);
        let r = rust_side(&repo, &["push"]);
        out.push(compare("push: cannot determine", &b, &r, |b| {
            if !b.out.contains("could not determine LFS status") {
                Some(format!(
                    "bash did not take the cannot-tell branch; got {:?}",
                    first_line(&b.out)
                ))
            } else {
                None
            }
        }));
    }

    out
}

pub fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

/// `gate --migrate-persist audit` — a REAL gate step, end to end, on both implementations.
///
/// This is the only gate step with its own CLI entry point on both sides, which makes it the one
/// step comparable without paying for a whole gate. It is worth paying for: T-555's persist DB is
/// the check that would have caught a843905f (an edit to an already-applied migration that killed
/// every existing database) and 0017's unique index over a duplicate seat.
///
/// AUDIT mode only. `advance` COMMITS to the shared `tbd_gate_migrate_persist` and takes the gate
/// lock; running it twice from a harness would move a database the real factory depends on.
/// `audit` re-hashes every applied migration and dry-runs the pending ones inside a transaction it
/// rolls back, so both runs see the same state and neither changes it.
pub fn arm_migrate_persist(ctx: &Ctx) -> Vec<ArmResult> {
    let Some(dir) = make_clone(ctx, "migrate") else {
        return vec![ArmResult {
            name: "gate --migrate-persist".into(),
            ok: false,
            note: "could not clone".into(),
        }];
    };
    let args = ["gate", "--migrate-persist", "audit"];
    let b = bash_side(&dir, &args);
    let r = rust_side(&dir, &args);
    vec![compare("gate --migrate-persist audit", &b, &r, |b| {
        // ANTI-VACUITY. The step must have reached a real database and reported a real census.
        // Its own header calls an unreachable database a FAIL and not a skip, for exactly this
        // reason — so an arm that accepted "cannot reach Postgres" from both sides would be
        // comparing two identical apologies.
        if b.out.contains("cannot reach Postgres") {
            return Some(
                "Postgres is down — the step never examined a database (`cargo xtask db up`)"
                    .into(),
            );
        }
        if b.out.contains("bootstrapping ") {
            return Some(
                "the persist DB was empty and the bash run BOOTSTRAPPED it, so the two runs saw \
                 different databases; re-run now that it is populated"
                    .into(),
            );
        }
        if !b.out.contains("audited ") {
            return Some(format!(
                "bash printed no audit census: {:?}",
                first_line(&b.out)
            ));
        }
        None
    })]
}
