//! Landing slices on main, and the bounded rollback and verifier record.
//!
//! **Role:** `cmd_land` merges every slice that is ready (gate receipt green at its HEAD, clean
//! tree, work on its branch) to main, records each landing in the central ticket manager (`ttm
//! land`), runs the full wave gate on the merged result, then drops the landed worktrees, repacks
//! the wave plan (`ttm wave repack`) and pushes; `cmd_revert` reverts a wave back to its base and
//! names the landings to clear; `cmd_verified` records the sha an adversarial verifier examined.
//!
//! **Position:** re-exported by the parent `land` module; the slice worktrees are merged and
//! dropped through `crate::slice_worktree::run_at`, the receipts read through `verdict`.
//!
//! **Signals & state:** none held; mutates main (merges, the push), the slice worktrees and the
//! ticket manager's landing records and wave plan.
//!
//! **Invariants:** a slice without a green gate receipt at its current HEAD is refused with the
//! exact re-gate command; landing has no wave barrier; a red gate after the merge keeps every
//! worktree for inspection; a landing the ticket manager refuses to record stops before the gate,
//! and a failed repack stops before the push; arguments are an allowlist (known flags, and
//! tickets of the current wave named by slug or legacy number).

use super::*;

/// Land every slice that is ready. No barrier — see correction 2.
pub(crate) fn cmd_land(ctx: &Ctx, args: &[String]) -> u8 {
    // ARGUMENTS ARE AN ALLOWLIST, and unknown ones are REFUSED.
    //
    // A bare `[ "${1:-}" = "--wave" ] && barrier=1` test admits any other
    // A discarded argument makes `land <id>` byte-for-byte `land`, landing every
    // committed slice in the wave. OBSERVED 2026-07-26 — it merged two slices whose agents had
    // not yet REPORTED, defeating rule 11 from inside the tool that rule depends on, and dropped
    // their worktrees out from under two live agents. Nothing was lost only because the gate
    // happened to pass.
    //
    // That is this run's signature defect one more time: an interface that reads narrow and acts
    // wide. A filter-shaped argument MUST filter or MUST refuse — silently ignoring it is the one
    // option that cannot be discovered before it does damage.
    let mut barrier = false;
    let mut bookkeeping = false;
    let mut only: Vec<String> = Vec::new();
    for a in args {
        if a == "--wave" {
            barrier = true;
        } else if a == "--bookkeeping" {
            // Escape hatch: a command-center/manual bookkeeping land may proceed
            // without slice-run receipts. It stamps only receipts that already exist and
            // NEVER fabricates a run file or token counts. Default is strict.
            bookkeeping = true;
        } else if a.is_empty() {
            // `'')` — an empty positional is dropped, not refused.
        } else if ticket_manager_client::is_ticket_reference(a) {
            only.push(a.clone());
        } else {
            werr!(
                "land: refusing unknown argument '{a}' (expected --wave, --bookkeeping and/or tickets of the current wave)"
            );
            return 2;
        }
    }

    let w = lock_or_refuse!(ledger::current_wave(ctx));
    if w == "done" {
        wprintln!("nothing to land");
        return 0;
    }

    let wave_ids = lock_or_refuse!(ledger::wave_tickets(ctx, &w));
    let plan = lock_or_refuse!(ledger::load_plan(ctx));

    // A named ticket that is not in the current wave would otherwise land NOTHING and say
    // "no slice is ready" — indistinguishable from "your slice is not finished". Each name is
    // matched to its wave row by slug or legacy number, and the slug is what lands.
    let mut named: Vec<String> = Vec::new();
    let mut miss: Vec<&str> = Vec::new();
    for want in &only {
        match plan
            .row(want)
            .filter(|row| wave_ids.iter().any(|t| t == row.slug.as_str()))
        {
            Some(row) => named.push(row.slug.to_string()),
            None => miss.push(want.as_str()),
        }
    }
    if !miss.is_empty() {
        werr!(
            "land: {} not in wave {w} — nothing named was landed",
            miss.join(" ")
        );
        return 2;
    }

    let mut ready: Vec<String> = Vec::new();
    let mut blocked: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for t in &wave_ids {
        if ledger::is_complete(ctx, t) {
            continue;
        }
        if !named.is_empty() && !named.iter().any(|o| o == t) {
            skipped.push(t.clone());
            continue;
        }
        if ledger::tree_state(ctx, t) == "committed" && ledger::has_work(t) {
            ready.push(t.clone());
        } else {
            blocked.push(t.clone());
        }
    }

    if !named.is_empty() {
        // "other unshipped", NOT "other ready" — these were filtered out before tree_state ran, so
        // their readiness is unknown and claiming it would be the same overclaim this script exists
        // to catch.
        let tail = if skipped.is_empty() {
            String::new()
        } else {
            format!("  (holding {} other unshipped slice(s))", skipped.len())
        };
        wprintln!("landing ONLY: {}{tail}", named.join(" "));
    }

    if ready.is_empty() {
        wprintln!("no slice is ready to land");
        return 0;
    }
    if barrier && !blocked.is_empty() {
        wprintln!(
            "--wave: holding {} ready slice(s) for {} unfinished: {}",
            ready.len(),
            blocked.len(),
            blocked.join(" ")
        );
        wprintln!(
            "(this is the wave barrier that cost 89% of wall clock — omit --wave to land now)"
        );
        return 0;
    }
    // A factory land is STRICT about run receipts — every landing ticket must have a slice-run
    // receipt in the ticket manager or the land refuses before touching main. `--bookkeeping`
    // waives the requirement for manual/command-center lands; land still never invents a receipt
    // it does not have.
    let missing: Vec<&str> = ready
        .iter()
        .filter(|t| plan.row(t).is_none_or(|row| row.receipt_count == 0))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() && !bookkeeping {
        werr!(
            "land: no slice-run receipt in the ticket manager for: {}\n      \
             a factory land requires the harness receipt — produce one with \
             `cargo xtask platform slice-run <id>`;\n      \
             for command-center/manual bookkeeping lands pass --bookkeeping \
             (waives the requirement; stamps nothing, invents nothing)",
            missing.join(" ")
        );
        return 2;
    }
    if bookkeeping && !missing.is_empty() {
        wprintln!(
            "--bookkeeping: landing WITHOUT run receipts for: {} (nothing will be stamped for these)",
            missing.join(" ")
        );
    }

    // NOTHING MECHANICAL USED TO STOP AN UNGATED SLICE LANDING.
    //
    // `land` ran the WAVE gate after merging (below) and never asked whether a SLICE gate had run
    // before. On 2026-08-14 one had not: it refused from the wrong cwd, the exit was masked by a
    // pipe, and the slice merged anyway — the miss was caught by hand, afterwards.
    //
    // The check is here, in its own pass, for two reasons. It is BEFORE the merge loop, because a
    // refusal after `git merge` is a report, not a gate. And it is ALL-OR-NOTHING: one ungated
    // slice stops the whole land rather than landing its siblings and leaving a partial wave for
    // whoever reads the scrollback. The sha compared is the tip of `slice/<id>` — the commit the
    // merge below will bring in — against the sha the gate stamped from inside that worktree.
    //
    // NO `--bookkeeping` WAIVER, deliberately. That flag waives TOKEN receipts for manual
    // lands; a bookkeeping land still merges real code to main, so waiving the gate receipt would
    // reopen this exact hole behind a flag. Re-gating a finished slice is seconds.
    let mut ungated: Vec<String> = Vec::new();
    let mut gated: Vec<String> = Vec::new();
    for t in &ready {
        let tip = git_stdout_lossy(&["rev-parse", &format!("slice/{t}")]);
        match verdict::land_refusal(&ctx.main_root, t, &tip) {
            Some(refusal) => ungated.push(refusal),
            None => gated.push(format!("{t}@{}", short(&tip))),
        }
    }
    if !ungated.is_empty() {
        for refusal in &ungated {
            werr!("{refusal}");
        }
        werr!("land: nothing was landed — main is untouched.");
        return 2;
    }
    // Say so on the HAPPY path too. A check that speaks only when it refuses is a check nobody can
    // confirm is running — which is how the 2026-08-14 gate went unnoticed in the first place. The
    // sha printed here is the one the gate stamped AND the one about to be merged; they are equal
    // by the arm above, so this line is the operator's proof that both halves agree.
    wprintln!("gate verdict PASS: {}", gated.join(" "));

    // The last known-GREEN main. THE REVERT TARGET ONLY — not the gate's diff anchor.
    //
    // It is not the gate's anchor: that would be wrong the moment main moves
    // after the wave's close marker, which it always does (the command centre commits briefs, a
    // ledger row and its own fixes between the close and the first land). Measured 2026-09-06
    // landing one slice into wave 241:
    //
    //   gate: base edb4e4e4d starts AFTER this wave opened — refusing to run.
    //           this wave opened at 52a038a77
    //           7 commit(s) of this wave sit OUTSIDE edb4e4e4d..HEAD. touch_changed, wasm32,
    //           fmt and the trunk build would each report PASS/SKIP without reading one of them
    //
    // That refusal is the base derivation working exactly as designed — a gate anchored at pre-merge HEAD reads
    // only the merge and calls the whole wave green — so the anchor is what has to change, not the
    // check. The gate derives its own base from the close-marker ledger when given none, which is
    // the same base the end-of-wave gate and the close ceremony use. The revert target stays
    // pre-merge HEAD, because that IS the commit to roll back to.
    let base = git_stdout_lossy(&["rev-parse", "HEAD"]);
    wprintln!("revert target: {base}");

    let mut landed: Vec<String> = Vec::new();
    for t in &ready {
        let title = ledger::ticket_title(ctx, t);
        wprintln!("── landing {t}: {title}");
        super::super::flush();
        let ok = process_runner::Run::new("git")
            .args([
                "merge",
                "--no-ff",
                &format!("slice/{t}"),
                "-m",
                &format!("{t}: {title}"),
            ])
            .terminal()
            .map(|code| code == 0)
            .unwrap_or(false);
        if !ok {
            wprintln!("  MERGE FAILED — resolve by hand, then re-run land");
            wprintln!("  (nothing dropped; every worktree is intact)");
            return 1;
        }
        // The merge succeeded — record the landing NOW, before the gate and the repack: the
        // ticket manager stamps the newest receipt `landed` at this sha. Land never invents token
        // counts: a bookkeeping land does not require a receipt, and a landing the ticket
        // manager refuses to record is a hard stop, not a silent shrug.
        let land_sha = git_stdout_lossy(&["rev-parse", "HEAD"]);
        let require_receipt = !bookkeeping;
        match ctx.ticket_manager.land(t, land_sha.trim(), require_receipt) {
            Ok(recorded) => {
                let stamp = match recorded.stamped_receipt {
                    Some(_) => "receipt stamped landed",
                    None => "no receipt to stamp",
                };
                wprintln!("  landing recorded @ {} ({stamp})", short(&land_sha));
            }
            Err(e) => {
                werr!(
                    "  landing record FAILED for {t}: {}",
                    crate::error::error_chain_text(&e)
                );
                werr!(
                    "  (the merge is on main; fix the cause, then `{}`, and re-run land)",
                    ctx.ticket_manager
                        .display_command(&["land", t, "--sha", land_sha.trim()])
                );
                return 1;
            }
        }
        landed.push(t.clone());
    }
    ctx.forget_wave_plan();

    wprintln!();
    wprintln!(
        "landed {} slice(s). Running the wave gate on merged main:",
        landed.len()
    );
    if gate::cmd_gate(ctx, "") != 0 {
        // DO NOT DROP. `slice-worktree drop` is `worktree remove --force` + `branch -D`, so
        // dropping here would destroy the tree and branch of every slice in the wave BEFORE anyone
        // can see which one broke it — the exact failure the reap incident (643c5233) was
        // fixed to prevent, and which this script originally reproduced by dropping inside the
        // merge loop.
        wprintln!(
            "GATE RED AFTER MERGE — all {} worktree(s) KEPT for inspection: {}",
            landed.len(),
            landed.join(" ")
        );
        wprintln!("  fix on main and re-run:  cargo xtask platform wave gate");
        wprintln!("  or roll back the wave :  cargo xtask platform wave revert {base}");
        return 1;
    }

    // Green. Only now is it safe to destroy the evidence.
    for t2 in &landed {
        // The bash shelled out to `cargo run -q -p xtask -- platform slice-worktree -- drop`.
        // Called in-process instead: same code, same output, same rc, minus a cargo invocation
        // that could print `Compiling` lines into the middle of a land.
        let rc = crate::slice_worktree::run_at(&ctx.root, &["drop".to_string(), t2.clone()])
            .unwrap_or(1);
        if rc != 0 {
            wprintln!("  (drop failed for {t2} — remove by hand)");
        }
    }

    // `ttm wave repack` is land's final mutation, BEFORE the push, so the plan the next agent
    // reads already reflects this land. The plan lives in the ticket manager, so nothing is
    // committed here.
    if repack_after_land(ctx) != 0 {
        return 1;
    }

    // Rule 5: work must not be trapped on one machine. This was missing entirely.
    if push::cmd_push(ctx) != 0 {
        wprintln!("PUSH FAILED — work is landed on local main but not on origin");
    }

    if !blocked.is_empty() {
        wprintln!("still in flight: {}", blocked.join(" "));
    }
    0
}

/// Recompile the wave plan in the ticket manager after a land. Refusing to continue on a
/// repack error is deliberate: pushing a main whose plan cannot be recompiled would hand the next
/// agent a red `ttm wave check` with this command's name on it.
pub(super) fn repack_after_land(ctx: &Ctx) -> u8 {
    let outcome = ctx.ticket_manager.wave_repack(&[]);
    ctx.forget_wave_plan();
    match outcome {
        Ok(repacked) => {
            wprintln!("wave plan repacked: {}", repacked.summary);
            0
        }
        Err(e) => {
            wprintln!(
                "wave repack FAILED after land: {}",
                crate::error::error_chain_text(&e)
            );
            wprintln!(
                "  fix the tickets, run `{}`, then push.",
                ctx.ticket_manager.display_command(&["wave", "repack"])
            );
            1
        }
    }
}

/// Roll main back to a known-green commit, keeping the slice branches alive.
///
/// The bounded-rollback half of self-healing: when a wave cannot be fixed within its retry budget,
/// main returns to green and the offending slices are quarantined rather than left broken. Uses
/// `revert`, never `reset --hard` — main is pushed, so history must not be rewritten.
pub(crate) fn cmd_revert(ctx: &Ctx, base: &str) -> u8 {
    if base.is_empty() {
        wprintln!("usage: cargo xtask platform wave revert <known-green-sha>");
        return 1;
    }
    if git_stdout(&["rev-parse", "--verify", &format!("{base}^{{commit}}")]).is_none() {
        wprintln!("no such commit: {base}");
        return 1;
    }
    let n: i64 = git_stdout_lossy(&["rev-list", "--count", &format!("{base}..HEAD")])
        .trim()
        .parse()
        .unwrap_or(0);
    if n == 0 {
        wprintln!("already at {base}");
        return 0;
    }
    wprintln!("reverting {n} commit(s) back to {base}");
    let list = git_stdout_lossy(&["rev-list", &format!("{base}..HEAD")]);
    let reverted: Vec<String> = list
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    for c in list.lines().filter(|l| !l.is_empty()) {
        // `git rev-list --parents -n1 $c | wc -w` > 2 means "sha + two or more parents" = a merge.
        let parents = git_stdout_lossy(&["rev-list", "--parents", "-n1", c]);
        let is_merge = parents.split_whitespace().count() > 2;
        super::super::flush();
        let ok = if is_merge {
            process_runner::Run::new("git")
                .args(["revert", "--no-edit", "-m", "1", c])
                .terminal()
                .map(|code| code == 0)
                .unwrap_or(false)
        } else {
            process_runner::Run::new("git")
                .args(["revert", "--no-edit", c])
                .terminal()
                .map(|code| code == 0)
                .unwrap_or(false)
        };
        if !ok {
            if is_merge {
                wprintln!("revert of merge {c} failed — resolve by hand");
            } else {
                wprintln!("revert of {c} failed — resolve by hand");
            }
            return 1;
        }
    }
    wprintln!("main is back at the {base} tree. Slice branches were NOT deleted.");
    name_reverted_landings(ctx, &reverted);
    0
}

/// The tickets whose recorded landing commit is among `reverted`, with the `ttm unland` that
/// clears each record. The ticket manager is told nothing here: clearing a landing is the
/// operator's decision, made after reading which slices the revert took out.
fn name_reverted_landings(ctx: &Ctx, reverted: &[String]) {
    let tickets = match ctx.ticket_manager.list(None) {
        Ok(listing) => listing.tickets,
        Err(e) => {
            werr!(
                "could not list the tickets to find the reverted landings: {}",
                crate::error::error_chain_text(&e)
            );
            return;
        }
    };
    let landed: Vec<_> = tickets
        .iter()
        .filter(|ticket| {
            ticket.landing_sha.as_deref().is_some_and(|sha| {
                reverted
                    .iter()
                    .any(|c| c.starts_with(sha) || sha.starts_with(c.as_str()))
            })
        })
        .collect();
    if landed.is_empty() {
        wprintln!("no recorded landing lies in the reverted range.");
        return;
    }
    wprintln!("these tickets record a landing the revert took out; clear each record with:");
    for ticket in landed {
        wprintln!(
            "  {}",
            ctx.ticket_manager
                .display_command(&["unland", ticket.slug.as_str()])
        );
    }
}

/// Record that an adversarial verifier examined `<sha>`.
pub(crate) fn cmd_verified(ctx: &Ctx, sha: &str) -> u8 {
    if sha.is_empty() {
        wprintln!("usage: cargo xtask platform wave verified <sha>");
        return 1;
    }
    if git_stdout(&["rev-parse", "--verify", sha]).is_none() {
        wprintln!("not a sha: {sha}");
        return 1;
    }
    let _ = std::fs::create_dir_all(ctx.root.join(repository_layout::ARTIFACTS_DIR));
    let full = git_stdout_lossy(&["rev-parse", sha]);
    // `git rev-parse "$sha" > file` writes the sha AND its trailing newline.
    let _ = std::fs::write(
        ctx.root.join(repository_layout::LAST_VERIFIED_MARKER),
        format!("{full}\n"),
    );
    wprintln!("recorded: adversarial verifier examined {}", short(sha));
    0
}
