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
    // exactly the dir in which one worktree's artifacts turn up in another's build.
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
    // touch_changed's rc is honoured: its whole job is to invalidate the cargo fingerprints the
    // steps below depend on, so "it invalidated nothing" has to be a red, not a line of output
    // nobody is looking at.
    if touch::touch_changed("") != 0 {
        r.fail = true;
    }
    // Inside the lock and before every cargo step, for the same reason touch_changed is: it
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
    // The first step in this gate that RUNS anything rather than compiling it. See
    // `changed::frontend_tests_changed` for the scope it derives: deterministic frontend test
    // failures are otherwise in this gate's blind spot until after a merge.
    r.run("test (frontend, changed)", || {
        changed::frontend_tests_changed(ctx, "", tid)
    });
    // NOT change-scoped, and it is in the CHEAP gate on purpose: a slice whose diff is 0 `.rs`
    // files scopes every step above it down to nothing, so without this one its gate goes green
    // over a red `cargo xtask ci schema-validate`. ~1.4 s warm.
    r.run("schema", || schema::gate_schema(ctx));
    // The half `schema` cannot reach.
    //
    // `gate_schema` validates the catalogue AS COMMITTED. It cannot tell you the committed
    // catalogue disagrees with `contracts_v2/rules/prefab-classify.json`, because a rule edit
    // changes nothing until the catalogue is rebuilt. This step re-derives the classification lane
    // from committed artifacts alone and exits 1 on disagreement. ~12 s.
    //
    // `checkrun`, NOT `hostrun`: `hostrun` bakes in the SHARED CARGO_TARGET_DIR, and
    // `tools_v2/developer-tools/src/browser_testing/server.rs` `repo_root()` is
    // `env!("CARGO_MANIFEST_DIR")` — a COMPILE-TIME constant. A shared dir can therefore hand this
    // step a `world` binary that reads a DIFFERENT WORKTREE'S rules and catalogue while reporting
    // on yours, which is exactly the two inputs the verdict is about.
    //
    // And NOT folded into `xtask ci schema-validate`: gate_schema's drift tripwire reads that
    // task's `xtask schema <name>` steps, and this is a `developer-tools --bin world` call — it
    // would either trip the tripwire or be silently skipped by it.
    r.run("catalogue drift", || {
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
    // Class-R on migration 0016's claim UPDATE body — the migrate step is schema-count-only, so a
    // hollow claim migration stays green without this. Unconditional: every slice must hit it.
    r.run("db_migrate claim body", || {
        migrate::gate_db_migrate_claim_body(ctx)
    });
    // The populated-database step, in AUDIT mode: checksum-audits every already-applied migration
    // and dry-runs the pending ones against real rows, without advancing the shared DB. It belongs
    // in the CHEAP gate because an edit to an already-applied migration breaks every existing
    // database and reaches main through a slice gate. Unconditional and not change-scoped: a slice
    // that touches no migration can still be the one that has to notice a sibling's drift, and
    // this step is psql-only (~1 s), not a cargo step.
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
    // Hot-path twin of the cmd_gate run — see the note there for why the language gates run in a
    // driver rather than only in a composite. `verify no-python` and `verify no-shell` share one
    // TrackedLanguageBan table (hard zero). Catching a planted shell / Make / python3 path at
    // SLICE time is the cheapest place to catch it; the no-node twin stays wave-level.
    r.run("no-python", || {
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
    // RECORD THE VERDICT WHERE `land` CAN READ IT, ON EVERY RUN.
    //
    // A verdict that exists only in the terminal is one a swallowed exit code loses: the gate
    // refuses, the pipe eats the status, and the slice is merged by hand with nothing having
    // examined it. `land` cannot catch that unless there is a receipt to ask about.
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
