use super::*;

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
    let _ = std::fs::write(
        repo.join(".ai/tickets/ROOT"),
        "# ticket-registry root marker\n",
    );
    // T-912.2: the scratch repo carries a stub of the modern plan (the lock), not the dead TSV —
    // this arm exercises the push guard, which never reads the plan at all.
    let _ = std::fs::write(
        repo.join(".ai/tickets/wave.lock"),
        "version = 1\nmax_concurrent = 8\npack_last = []\nwaves = []\n\n[owns]\n\n[depends_on]\n",
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
