use super::*;

/// Land every slice that is ready. No barrier — see correction 2.
pub fn cmd_land(ctx: &Ctx, args: &[String]) -> u8 {
    // ARGUMENTS ARE AN ALLOWLIST, and unknown ones are REFUSED.
    //
    // This used to be `[ "${1:-}" = "--wave" ] && barrier=1` and nothing else, so any other
    // argument was silently discarded: `land T-204` was byte-for-byte `land`, and landed every
    // committed slice in the wave. OBSERVED 2026-07-26 — it merged T-389 and T-229 whose agents had
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
            // T-913.2 escape hatch: a command-center/manual bookkeeping land may proceed
            // without slice-run receipts. It stamps only receipts that already exist and
            // NEVER fabricates a run file or token counts. Default is strict.
            bookkeeping = true;
        } else if a.is_empty() {
            // `'')` — an empty positional is dropped, not refused.
        } else if is_ticket_glob(a) {
            only.push(a.clone());
        } else {
            werr!(
                "land: refusing unknown argument '{a}' (expected --wave, --bookkeeping and/or T-nnn ticket ids)"
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
    let mut ready: Vec<String> = Vec::new();
    let mut blocked: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for t in &wave_ids {
        if ctx.registry_view.is_shipped(t) {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|o| o == t) {
            skipped.push(t.clone());
            continue;
        }
        if ledger::tree_state(ctx, t) == "committed" && ledger::has_work(t) {
            ready.push(t.clone());
        } else {
            blocked.push(t.clone());
        }
    }

    // A named ticket that is not in the current wave would otherwise land NOTHING and say
    // "no slice is ready" — indistinguishable from "your slice is not finished".
    if !only.is_empty() {
        let miss: Vec<&String> = only
            .iter()
            .filter(|want| !wave_ids.iter().any(|t| t == *want))
            .collect();
        if !miss.is_empty() {
            let names: Vec<&str> = miss.iter().map(|s| s.as_str()).collect();
            werr!(
                "land: {} not in wave {w} — nothing named was landed",
                names.join(" ")
            );
            return 2;
        }
        // "other unshipped", NOT "other ready" — these were filtered out before tree_state ran, so
        // their readiness is unknown and claiming it would be the same overclaim this script exists
        // to catch.
        let tail = if skipped.first().map(|s| !s.is_empty()).unwrap_or(false) {
            format!("  (holding {} other unshipped slice(s))", skipped.len())
        } else {
            String::new()
        };
        wprintln!("landing ONLY: {}{tail}", only.join(" "));
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
            "(this is the T-181 barrier that cost 89% of wall clock — omit --wave to land now)"
        );
        return 0;
    }
    // T-913.2: a factory land is STRICT about run receipts — every landing ticket must
    // have a slice-run file under .ai/tickets/metrics/<id>/ or the land refuses before
    // touching main. `--bookkeeping` waives the requirement for manual/command-center
    // lands; land still never invents a receipt it does not have.
    if let Some(refusal) =
        ticket_engine::metrics::land_receipt_refusal(&ctx.root, &ready, bookkeeping)
    {
        werr!("{refusal}");
        return 2;
    }
    if bookkeeping {
        let missing = ticket_engine::metrics::missing_receipts(&ctx.root, &ready);
        if !missing.is_empty() {
            wprintln!(
                "--bookkeeping: landing WITHOUT run receipts for: {} (nothing will be stamped for these)",
                missing.join(" ")
            );
        }
    }

    // T-924 — NOTHING MECHANICAL USED TO STOP AN UNGATED SLICE LANDING.
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
    // NO `--bookkeeping` WAIVER, deliberately. That flag waives T-913.2 TOKEN receipts for manual
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
    // T-946.14: it used to be both, and as the gate's anchor it was wrong the moment main moved
    // after the wave's close marker, which it always does (the command centre commits briefs, a
    // ledger row and its own fixes between the close and the first land). Measured 2026-09-06
    // landing T-675.1 into wave 241:
    //
    //   gate: base edb4e4e4d starts AFTER this wave opened — refusing to run.
    //           this wave opened at 52a038a77
    //           7 commit(s) of this wave sit OUTSIDE edb4e4e4d..HEAD. touch_changed, wasm32,
    //           fmt and the trunk build would each report PASS/SKIP without reading one of them
    //
    // That refusal is T-602 working exactly as designed — a gate anchored at pre-merge HEAD reads
    // only the merge and calls the whole wave green — so the anchor is what has to change, not the
    // check. The gate derives its own base from the close-marker ledger when given none, which is
    // the same base the end-of-wave gate and the close ceremony use. The revert target stays
    // pre-merge HEAD, because that IS the commit to roll back to.
    let base = git_stdout_lossy(&["rev-parse", "HEAD"]);
    wprintln!("revert target: {base}");

    let mut landed: Vec<String> = Vec::new();
    let mut stamped: Vec<String> = Vec::new();
    for t in &ready {
        let title = ledger::ticket_title(ctx, t);
        wprintln!("── landing {t}: {title}");
        super::super::flush();
        let ok = std::process::Command::new("git")
            .args([
                "merge",
                "--no-ff",
                &format!("slice/{t}"),
                "-m",
                &format!("{t}: {title}"),
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            // T-913.2: the merge succeeded — stamp the harness receipt NOW (outcome +
            // land sha + finished), before repack_after_land. Land never invents token
            // counts: a bookkeeping ticket without a receipt is skipped, and a receipt
            // that exists but cannot be stamped is a hard stop, not a silent shrug.
            if ticket_engine::metrics::has_receipt(&ctx.root, t) {
                let land_sha = git_stdout_lossy(&["rev-parse", "HEAD"]);
                match ticket_engine::metrics::stamp_land(&ctx.root, t, &land_sha) {
                    Ok(p) => {
                        let rel = p
                            .strip_prefix(&ctx.root)
                            .unwrap_or(&p)
                            .display()
                            .to_string();
                        wprintln!("  receipt stamped landed @ {}: {rel}", short(&land_sha));
                        stamped.push(rel);
                    }
                    Err(e) => {
                        werr!("  receipt stamp FAILED for {t}: {e:#}");
                        werr!("  (merge is on main; fix the receipt, stamp by hand, re-run land)");
                        return 1;
                    }
                }
            }
            landed.push(t.clone());
        } else {
            wprintln!("  MERGE FAILED — resolve by hand, then re-run land");
            wprintln!("  (nothing dropped; every worktree is intact)");
            return 1;
        }
    }

    if !stamped.is_empty() {
        commit_stamped_receipts(&stamped);
    }

    wprintln!();
    wprintln!(
        "landed {} slice(s). Running the wave gate on merged main:",
        landed.len()
    );
    if gate::cmd_gate(ctx, "") != 0 {
        // DO NOT DROP. `slice-worktree drop` is `worktree remove --force` + `branch -D`, so
        // dropping here would destroy the tree and branch of every slice in the wave BEFORE anyone
        // can see which one broke it — the exact failure the T-181 reap incident (643c5233) was
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
        let rc = crate::commands::platform::slice_worktree::run_at(
            &ctx.root,
            &["drop".to_string(), t2.clone()],
        )
        .unwrap_or(1);
        if rc != 0 {
            wprintln!("  (drop failed for {t2} — remove by hand)");
        }
    }

    // T-912.2 lifecycle (a): `wave repack` is land's final mutation, BEFORE the push, so a lock
    // refresh rides the land rather than sitting dirty behind it. Usually a no-op byte-wise —
    // slice branches do not edit ticket files, and every status writer already runs the same
    // writer — but a merged slice that DID move a ticket must not leave `wave check` red on the
    // main this command just published.
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

/// Run the lock writer and commit the refresh when it changed anything — the land commit
/// carries the lock (lifecycle "a"). Refusing to continue on a writer error is deliberate:
/// pushing a main whose lock cannot be recompiled would hand the next agent a red `ticket
/// check` with this command's name on it.
pub(super) fn repack_after_land(ctx: &Ctx) -> u8 {
    if let Err(e) = ticket_engine::wave_lock::repack_quiet(&ctx.root) {
        wprintln!("wave repack FAILED after land: {e:#}");
        wprintln!("  fix the ticket tree, run `cargo xtask wave repack`, commit, then push.");
        return 1;
    }
    let dirty = git_stdout_lossy(&[
        "status",
        "--porcelain",
        "--",
        ticket_engine::repository::WAVE_LOCK,
    ]);
    if dirty.trim().is_empty() {
        return 0;
    }
    super::super::flush();
    let ok = std::process::Command::new("git")
        .args(["add", "--", ticket_engine::repository::WAVE_LOCK])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("git")
            .args(["commit", "-m", "wave.lock: repack after land"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if !ok {
        wprintln!("could not commit the wave.lock refresh — commit it by hand before pushing");
        return 1;
    }
    wprintln!("wave.lock refreshed and committed (rides this land)");
    0
}

/// T-913.2: commit the land-stamped run receipts so they ride the land — one commit,
/// EXPLICIT paths only (never `-A`), placed before the gate so a later `wave revert` of
/// the merges rolls the stamps back with them.
///
/// Warn-and-continue on failure, deliberately unlike [`repack_after_land`]: a stale lock
/// makes `ticket check` red for everyone, but an uncommitted stamp is still a valid
/// on-disk receipt — blocking the land over its commit would hold real work hostage to
/// bookkeeping.
pub(super) fn commit_stamped_receipts(paths: &[String]) {
    super::super::flush();
    let mut add = std::process::Command::new("git");
    add.args(["add", "--"]);
    for p in paths {
        add.arg(p);
    }
    let ok = add.status().map(|s| s.success()).unwrap_or(false)
        && std::process::Command::new("git")
            .args(["commit", "-m", "metrics: stamp land receipts"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if ok {
        wprintln!("run receipt(s) committed (ride this land)");
    } else {
        wprintln!("could not commit the stamped receipt(s) — commit .ai/tickets/metrics/ by hand");
    }
}

/// The bash `case` glob `T-[0-9]*` — literal `T-`, then a digit, then anything.
pub(super) fn is_ticket_glob(a: &str) -> bool {
    let Some(rest) = a.strip_prefix("T-") else {
        return false;
    };
    rest.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
}

/// Roll main back to a known-green commit, keeping the slice branches alive.
///
/// The bounded-rollback half of self-healing: when a wave cannot be fixed within its retry budget,
/// main returns to green and the offending slices are quarantined rather than left broken. Uses
/// `revert`, never `reset --hard` — main is pushed, so history must not be rewritten.
pub fn cmd_revert(_ctx: &Ctx, base: &str) -> u8 {
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
    for c in list.lines().filter(|l| !l.is_empty()) {
        // `git rev-list --parents -n1 $c | wc -w` > 2 means "sha + two or more parents" = a merge.
        let parents = git_stdout_lossy(&["rev-list", "--parents", "-n1", c]);
        let is_merge = parents.split_whitespace().count() > 2;
        super::super::flush();
        let ok = if is_merge {
            std::process::Command::new("git")
                .args(["revert", "--no-edit", "-m", "1", c])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("git")
                .args(["revert", "--no-edit", c])
                .status()
                .map(|s| s.success())
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
    0
}

/// Record that an adversarial verifier examined `<sha>`.
pub fn cmd_verified(ctx: &Ctx, sha: &str) -> u8 {
    if sha.is_empty() {
        wprintln!("usage: cargo xtask platform wave verified <sha>");
        return 1;
    }
    if git_stdout(&["rev-parse", "--verify", sha]).is_none() {
        wprintln!("not a sha: {sha}");
        return 1;
    }
    let _ = std::fs::create_dir_all(ctx.root.join(ticket_engine::repository::ARTIFACTS_DIR));
    let full = git_stdout_lossy(&["rev-parse", sha]);
    // `git rev-parse "$sha" > file` writes the sha AND its trailing newline.
    let _ = std::fs::write(
        ctx.root
            .join(crate::core::repository_layout::LAST_VERIFIED_MARKER),
        format!("{full}\n"),
    );
    wprintln!("recorded: adversarial verifier examined {}", short(sha));
    0
}
