use super::*;

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

    let all =
        super::super::git_stdout_lossy(&["-C", &dir.display().to_string(), "rev-list", "--all"]);
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
        super::super::git_stdout_lossy(&["-C", &dir.display().to_string(), "rev-parse", "HEAD"]);

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
