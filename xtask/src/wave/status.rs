//! `status`, `prep` and `wave` — the read-only half of the driver.
//!
//! Nothing here mutates anything, which is why the port starts here: it establishes the
//! ledger-reading code every other command reuses, at zero blast radius.

use super::{COLLIDE, Ctx, host, ledger};
use crate::{wprint, wprintln};

/// `wave.sh status` — where are we, and what is blocking?
pub fn cmd_status(ctx: &Ctx) -> u8 {
    let w = ledger::current_wave(ctx);
    wprintln!("═══ platform program ═══");
    wprintln!("plan:  {}", ctx.plan);

    let vd = ledger::verify_debt(ctx);
    wprint!("verify: {vd}");
    if vd.starts_with("unknown") {
        wprintln!("  <- run an adversarial verifier and record the sha");
    } else {
        // `c="${vd%% *}"` — the count, up to the first space. `[ "$c" -ge N ] 2>/dev/null`
        // swallows the "integer expression expected" a non-numeric prefix would produce, so a
        // malformed marker prints a bare newline rather than nagging.
        let c: i64 = vd.split(' ').next().unwrap_or("0").parse().unwrap_or(-1);
        if c >= ctx.verify_debt_nag {
            wprintln!(
                "  <- OVERDUE, {}+ landings unverified (rule 4)",
                ctx.verify_debt_nag
            );
        } else {
            wprintln!();
        }
    }

    // `plan_rows | awk -F'\t' '$1!="0"'` — awk compares against the STRING "0" here, so a row
    // spelled `0.0` would be counted. Preserved rather than tidied into a numeric test.
    let counted: Vec<String> = ledger::plan_rows(ctx)
        .into_iter()
        .filter(|r| r.split('\t').next().unwrap_or("") != "0")
        .collect();
    let total = counted.len();
    let mut open = 0usize;
    for r in &counted {
        let t = r.split('\t').nth(1).unwrap_or("");
        if !ctx.registry_view.is_shipped(t) {
            open += 1;
        }
    }
    wprintln!("open:  {open} / {total} tickets");

    if w == "done" {
        wprintln!("ALL WAVES COMPLETE");
        return 0;
    }
    wprintln!("wave:  {w}");
    wprintln!();

    let mut ready = 0usize;
    for t in ledger::wave_tickets(ctx, &w) {
        if ctx.registry_view.is_shipped(&t) {
            wprintln!("  {:<9} SHIPPED", t);
            continue;
        }
        let st = ledger::tree_state(ctx, &t);
        if st == "committed" && ledger::has_work(&t) {
            wprintln!(
                "  {:<9} READY TO LAND  {}",
                t,
                ledger::ticket_title(ctx, &t)
            );
            ready += 1;
        } else if st == "committed" {
            wprintln!("  {:<9} tree clean, no commits yet", t);
        } else if st == "dirty" {
            wprintln!("  {:<9} IN PROGRESS (uncommitted)", t);
        } else if st == "unknown" {
            wprintln!("  {:<9} ⚠ STATUS UNREADABLE — will not land", t);
        } else {
            wprintln!("  {:<9} not started", t);
        }
    }
    wprintln!();
    if ready > 0 {
        wprintln!("→ {ready} slice(s) ready: cargo xtask platform wave land");
    }
    wprintln!("→ dispatch set: {COLLIDE}");
    0
}

/// `wave.sh prep` — print the next disjoint dispatch set.
///
/// cargo is a HOST binary inside the dev container, so this goes through the bridge — unlike the
/// `python3` it replaced, which was present on both sides. `hostrun` degrades to a plain exec on
/// the host, so the same line is correct from either shell.
pub fn cmd_prep(ctx: &Ctx) -> u8 {
    wprintln!("next disjoint dispatch set:");
    // `hostrun $COLLIDE`, UNQUOTED in the bash, so the shell word-splits it into argv.
    let cmd: Vec<String> = COLLIDE.split_whitespace().map(str::to_string).collect();
    host::inherit(&ctx.host.hostrun_argv(&cmd));
    wprintln!();
    wprintln!(
        "create trees with:  cargo run -q -p xtask -- platform slice-worktree -- new <TICKET>"
    );
    wprintln!("(slice-worktree is program-agnostic; it keys off the branch name only)");
    0
}

/// ── WAVE DISCIPLINE ─────────────────────────────────────────────────────────────────────────
///
/// Restored on operator instruction 2026-07-26 after the run drifted into a continuous stream of
/// individual agents. The drift was not merely cosmetic: the wave boundary is the EVENT that fires
/// the adversarial verifier (rule 4), so dissolving waves silently deleted the verifier and 27
/// tickets landed unreviewed. The operator noticed; the tooling did not.
///
/// Note this does NOT reintroduce the T-181 land barrier that cost 89% of that program's wall
/// clock. Slices still land the moment they are green (note 2). What a wave gates is DISPATCH: you
/// may not open wave N+1 until wave N is shipped, gated and VERIFIED. Landing stays eager; starting
/// is paced.
pub fn cmd_wave(ctx: &Ctx) -> u8 {
    let w = ledger::current_wave(ctx);
    if w == "done" {
        wprintln!("all waves shipped");
        return 0;
    }
    let mut total = 0usize;
    let mut shipped = 0usize;
    let mut open: Vec<String> = Vec::new();
    for t in ledger::wave_tickets(ctx, &w) {
        total += 1;
        if ctx.registry_view.is_shipped(&t) {
            shipped += 1;
        } else {
            open.push(t);
        }
    }
    wprintln!("═══ wave {w} — {shipped}/{total} shipped ═══");
    if !open.is_empty() {
        wprintln!("open:");
        for t in &open {
            wprintln!("  {:<8} {}", t, ledger::ticket_title(ctx, t));
        }
    }
    wprintln!();
    wprintln!("verify debt: {}", ledger::verify_debt(ctx));
    if !open.is_empty() {
        // `$((w+1))` — w is numeric here by construction (current_wave only returns digits or
        // "done", and "done" returned above).
        wprintln!(
            "STATUS: wave {w} is OPEN — finish it before dispatching wave {}.",
            w.parse::<i64>().unwrap_or(0) + 1
        );
    } else {
        wprintln!(
            "STATUS: wave {w} tickets are all shipped. Run 'cargo xtask platform wave wave --close' to gate and advance."
        );
    }
    0
}
