//! The gate step helpers and the per-slice gate.
//!
//! **Role:** `checkrun` runs a check-class command into the gate's private analysis folder,
//! `hostrun` runs a command through the host bridge, and `gate_slice` is the cheap gate a slice
//! agent runs in its worktree before reporting done: `cargo check`, the format of the changed
//! files and the tests of the changed frontend crates.
//!
//! **Position:** re-exported by the parent `gate` module; `gate_dispatch.rs` uses both helpers for
//! the wave gate.
//!
//! **Signals & state:** the gate lock (`lock::GateState`) for one slice gate; the verdict receipt
//! it writes through `verdict::record_slice_gate`.
//!
//! **Invariants:** the slice gate refuses an empty `main...HEAD` range (run from main) with exit 2
//! and writes no receipt for a run that examined nothing; it takes the gate lock and invalidates
//! cargo fingerprints before its first cargo step; every analysis step builds into the private
//! check folder with `CARGO_INCREMENTAL=0`.

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
pub(crate) fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {
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
    r.run("fmt (changed)", || changed::fmt_changed(ctx, ""));
    // The one step in this gate that RUNS anything rather than compiling it. See
    // `changed::frontend_tests_changed` for the scope it derives: deterministic frontend test
    // failures are otherwise in this gate's blind spot until after a merge.
    r.run("test (frontend, changed)", || {
        changed::frontend_tests_changed(ctx, "", tid)
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
