use super::*;

/// `checkrun <cmd…>` as a step body.
pub(super) fn checkrun(ctx: &Ctx, cmd: &[&str]) -> i32 {
    let (out, rc) = host::capture(
        &ctx.host
            .checkrun_argv(&ctx.gate_check_target, &host::v(cmd)),
    );
    wprint!("{out}");
    rc
}

/// `hostrun <cmd…>` as a step body.
pub(super) fn hostrun(ctx: &Ctx, cmd: &[&str]) -> i32 {
    let (out, rc) = host::capture(&ctx.host.hostrun_argv(&host::v(cmd)));
    wprint!("{out}");
    rc
}

/// Cheap gate — what a slice agent runs before reporting done. Target: ~10 s warm.
pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {
    wprintln!("═══ slice gate {tid} ═══");
    // The helpers all default to `main...HEAD`, which is the slice's own diff when run from its
    // worktree and empty anywhere else. Check the range they will actually use.
    if base::refuse_empty_range(
        "main...HEAD",
        "Run this from the slice's WORKTREE, not from main.",
    ) != 0
    {
        return 2;
    }
    // Even the cheap gate builds into the SHARED CARGO_TARGET_DIR (cargo check, clippy), which is
    // exactly the dir T-193 and T-235 measured one worktree's artifacts appearing in another's.
    let mut state = GateState::new();
    match state.take(
        ctx,
        &format!("slice {}", if tid.is_empty() { "?" } else { tid }),
    ) {
        0 => {}
        n => return n,
    }

    let mut r = Runner {
        fail: false,
        timeout_arm: None,
    };
    // touch_changed's rc was previously DISCARDED, which mattered: its whole job is to invalidate
    // the cargo fingerprints the steps below depend on, so "it invalidated nothing" has to be a
    // red, not a line of output nobody is looking at.
    if touch::touch_changed("") != 0 {
        r.fail = true;
    }
    // T-421. Inside the lock and before every cargo step, for the same reason touch_changed is: it
    // invalidates the fingerprints those steps depend on. rc honoured — a run that invalidated
    // nothing cannot go on to interpret what the steps below report.
    if touch::touch_workspace(ctx) != 0 {
        r.fail = true;
    }

    r.run("cargo check", || {
        checkrun(ctx, &["cargo", "check", "--workspace", "--quiet"])
    });
    r.run("wasm32 (frontend)", || changed::wasm_changed(ctx, ""));
    r.run("fmt (changed)", || changed::fmt_changed(ctx, ""));
    r.run("clippy (changed crates)", || touch::clippy_changed(ctx, ""));
    // T-946.64. The first step in this gate that RUNS anything rather than compiling it. See
    // `changed::frontend_tests_changed` for the two wave-253 failures that bought it: both were
    // deterministic, both were in this gate's blind spot, and both were found only after merge.
    r.run("test (frontend, changed)", || {
        changed::frontend_tests_changed(ctx, "", tid)
    });
    // T-420. NOT change-scoped, and it is in the CHEAP gate on purpose: this is the step that would
    // have stopped T-244, whose diff is 0 .rs files — so every other step above it is change-scoped
    // down to nothing and its slice gate was green over a red `cargo xtask ci schema-validate`. ~1.4 s warm.
    r.run("schema", || schema::gate_schema(ctx));
    // T-583/T-594. The other half of the T-244 lesson above, and the half `schema` cannot reach.
    //
    // `gate_schema` validates the catalogue AS COMMITTED. It cannot tell you the committed
    // catalogue disagrees with `contracts_v2/rules/prefab-classify.json`, because a rule
    // edit changes NOTHING until the catalogue is rebuilt — and until T-278 the only rebuild path
    // needed a Workbench export that is gitignored and absent from every clone. So T-244's
    // `vehicle` rules went in, every gate stayed green, and the shipped artifact was stale for four
    // weeks. This step re-derives the classification lane from committed artifacts alone and exits
    // 1 on disagreement; run on the day T-244 landed it would have gone RED immediately. ~12 s.
    //
    // `checkrun`, NOT `hostrun`: `hostrun` bakes in the SHARED CARGO_TARGET_DIR, and
    // `tools_v2/developer-tools/src/browser_testing/server.rs` `repo_root()` is `env!("CARGO_MANIFEST_DIR")` — a COMPILE-TIME
    // constant. A shared dir can therefore hand this step a `world` binary that reads a DIFFERENT
    // WORKTREE'S rules and catalogue while reporting on yours: the signature defect, with the two
    // inputs the verdict is entirely about.
    //
    // And NOT folded into `xtask ci schema-validate`: gate_schema's drift tripwire reads that
    // task's `xtask schema <name>` steps, and this is a `tbd-tools --bin world` call — it would
    // either trip the tripwire or be silently skipped by it.
    r.run("T-278 catalogue drift", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "developer-tools",
                "--bin",
                "world",
                "--",
                "reclassify",
                "--terrain",
                "everon",
            ],
        )
    });
    // T-515. Class-R on 0016 claim UPDATE body — db_migrate.rs is schema-count-only; a hollow claim
    // migration stays green. Unconditional (wave.sh-only slices must hit it).
    r.run("db_migrate claim body", || {
        migrate::gate_db_migrate_claim_body(ctx)
    });
    // T-555. The populated-database step, in AUDIT mode: checksum-audits every already-applied
    // migration and dry-runs the pending ones against real rows, without advancing the shared DB.
    // It belongs in the CHEAP gate specifically because a843905f — the edit to an already-applied
    // migration that killed every existing database — landed through a slice gate. Unconditional
    // and not change-scoped: a slice that touches no migration can still be the one that has to
    // notice a sibling's drift, and this step is psql-only (~1 s), not a cargo step.
    r.run("db_migrate persist", || {
        migrate::gate_db_migrate_persist(ctx, &state, "audit") as i32
    });
    for (label, name) in VERIFY_STEPS {
        r.run(label, || {
            checkrun(
                ctx,
                &["cargo", "run", "-q", "-p", "xtask", "--", "verify", name],
            )
        });
    }
    // T-620/T-904. Hot-path twin of the cmd_gate run — see the long note there for why this gate
    // spent four waves invoked by nothing. `verify no-python` and `verify no-shell` share one
    // TrackedLanguageBan table (hard zero, no inventory). Catching a planted .sh / Makefile /
    // python3 at SLICE time is the cheapest place to catch it; the no-node twin stays wave-level.
    r.run("no-python (T-620)", || {
        checkrun(
            ctx,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "xtask",
                "--",
                "verify",
                "no-python",
            ],
        )
    });

    wprintln!();
    // T-924 — RECORD THE VERDICT WHERE `land` CAN READ IT, ON EVERY RUN.
    //
    // Before this, the verdict existed only in the terminal, and on 2026-08-14 that was enough to
    // lose one: this gate REFUSED (wrong cwd — the `refuse_empty_range` return above), its exit
    // code was swallowed by a pipe, and the slice was merged by hand with nothing having examined
    // it. `land` could not have caught that, because it had nothing to ask.
    //
    // ONE call site, ahead of the FAIL return, so PASS and FAIL both write. That matters: if only
    // a green gate left a receipt, `land` could not tell a RED gate from a gate that never ran,
    // and those have different fixes. The two early returns above deliberately write NOTHING —
    // they mean no step executed, so there is no verdict to record, and inventing a FAIL there
    // would be this program's signature defect (reporting on an input nothing examined).
    verdict::record_slice_gate(ctx, tid, !r.fail);
    if r.fail {
        state.verdict("FAIL", "SLICE GATE");
        return 1;
    }
    state.verdict("PASS", "SLICE GATE");
    0
}
