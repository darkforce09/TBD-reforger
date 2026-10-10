//! The wave close ceremony: the marker commit and the close record as one motion.
//!
//! **Role:** `close_ceremony` writes the `wave <N> CLOSED` marker commit, records the close in the
//! central ticket manager (`ttm wave close <N> --sha <marker>`, which also repacks the plan), and
//! proves the end state (`ttm wave check`, and the plan's wave base agreeing with the marker
//! ledger), or prints the would-be subject under `--dry-run`.
//!
//! **Position:** called by `cmd_wave_close` in the sibling `wave_close.rs` only after every
//! validation passed.
//!
//! **Signals & state:** none held; commits into the repository at the context's root and records
//! the close in the ticket manager.
//!
//! **Invariants:** the process working directory is the root (the marker oracles read relative to
//! it), and every write names the root explicitly, so a misdirected caller can misread but never
//! commit elsewhere; a wave number that does not parse refuses and writes no marker; the subject
//! the ceremony writes passes the same anchored marker check the base derivation reads; recording
//! the close is idempotent for the same wave and sha, so a failed record is repaired by re-running
//! it.

use super::*;

/// The marker commit and the close record, as ONE motion.
///
/// `cmd_wave_close`'s validations (all-shipped, verifier recorded AND at HEAD, the full wave gate)
/// run first; the ceremony is this separate function, called only after every validation has
/// passed. `vouched` says the operator named the members (`--tickets`), which the close record
/// then stores instead of the plan's frozen set.
///
/// CWD CONTRACT: the marker authority and oracle are cwd-bound by design ([`Ctx::enter`] chdirs
/// to the root once, and the whole gate stack rides that), so the caller guarantees the process
/// cwd is `root`. Every WRITE this function performs is root-explicit anyway ([`git_at`]): a
/// misdirected caller can misread, but it can never commit into a repo it was not handed — and
/// the misread ends in refusal, because the candidate object cannot resolve outside `root`.
pub(super) fn close_ceremony(
    ctx: &Ctx,
    w: &str,
    wave_ids: &[String],
    vouched: bool,
    summary: Option<&str>,
    dry_run: bool,
) -> u8 {
    let root: &Path = &ctx.root;
    // Fail-closed parse. The old print did `w.parse().unwrap_or(0)` — fine for prose, lethal for
    // a ledger: a marker claiming wave 0 must be impossible to write, not merely unlikely.
    let n: i64 = match w.parse() {
        Ok(n) => n,
        Err(_) => {
            wprintln!("REFUSED: current wave '{w}' is not a number — no marker written.");
            return 1;
        }
    };
    let Ok(label) = u32::try_from(n) else {
        wprintln!(
            "REFUSED: wave {n} is not a wave label the ticket manager can record — no marker written."
        );
        return 1;
    };
    let subject = match close_subject(n, summary, wave_ids) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: {e}");
            wprintln!("         No marker written.");
            return 1;
        }
    };

    if dry_run {
        // The one mode that prints without committing. String-level self-checks have passed;
        // the object-level oracle run needs a candidate commit object, and --dry-run writes
        // NOTHING — not even to the object store — so it stops here.
        wprintln!("--dry-run: would commit wave-close marker subject:");
        wprintln!("  {subject}");
        wprintln!("(nothing written; working tree, marker ledger and ticket manager untouched)");
        return 0;
    }

    // DIRTY TREE = REFUSAL, before anything is created. The ceremony moves HEAD to a commit of
    // HEAD's own tree; starting it on top of unrelated changes strands them behind a marker. Same
    // LFS-neutral, fail-closed porcelain read as tree_state/git_porcelain_paths: a status that
    // CANNOT run is never an empty status.
    let mut porcelain: Vec<&str> = ledger::LFS_NEUTRAL.to_vec();
    porcelain.extend_from_slice(&["status", "--porcelain"]);
    let dirty = match git_at(root, &porcelain) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: cannot read the working tree state — no marker written.");
            wprintln!("         {e}");
            return 1;
        }
    };
    let dirty_paths: Vec<&str> = dirty.lines().filter(|l| !l.trim().is_empty()).collect();
    if !dirty_paths.is_empty() {
        wprintln!(
            "REFUSED: the working tree is dirty — the close ceremony writes commits, and it must"
        );
        wprintln!("         not sweep up or sit on top of unrelated changes. Clean these first:");
        for p in dirty_paths.iter().take(10) {
            wprintln!("           {p}");
        }
        if dirty_paths.len() > 10 {
            wprintln!("           … and {} more", dirty_paths.len() - 10);
        }
        wprintln!("         No marker written.");
        return 1;
    }

    let head = match git_at(root, &["rev-parse", "HEAD"]) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: cannot resolve HEAD — no marker written. {e}");
            return 1;
        }
    };

    // THE CANDIDATE. `commit-tree` writes a commit OBJECT and moves no ref: unreachable from
    // everything, absent from `git log`, invisible to every `rev-list … HEAD` scan the ledger
    // runs. Marker subjects are the ledger and the marker carries no diff, so the tree is
    // HEAD's own — the `--allow-empty` shape, made first-class.
    let cand = match git_at(
        root,
        &["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", &subject],
    ) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: could not create the candidate marker object — no marker written.");
            wprintln!("         {e}");
            return 1;
        }
    };

    // SELF-CHECK, against the exact object, with the SAME functions the gate derives from —
    // never a re-implementation. wave_close_number re-runs wave_close_subject_ok on the object's
    // subject and must parse to exactly the wave being closed; wave_close_is_newest_wave is
    // oracle 1's acceptance window verbatim (strictly above every non-disavowed marker, at most
    // one above the highest claim any marker makes). The candidate is not reachable from HEAD,
    // so the oracle compares it against the ledger without it — exactly the question being asked.
    match super::super::base::wave_close_number(&cand) {
        Some(got) if got == n => {}
        got => {
            wprintln!("REFUSED: candidate marker failed the authority self-check — no ref moved.");
            wprintln!("         built subject: {subject:?}");
            wprintln!("         wave_close_number parsed {got:?}, expected Some({n})");
            return 1;
        }
    }
    if super::super::base::wave_close_is_newest_wave(&cand) != 0 {
        wprintln!(
            "REFUSED: the marker-ledger oracle rejected the candidate subject (details above) —"
        );
        wprintln!("         no ref moved; the candidate object is unreachable garbage.");
        return 1;
    }
    wprintln!("close subject self-check ✓ the authority parses {subject:?} as wave {n} and the");
    wprintln!("                          marker-ledger oracle accepts it");

    // PROMOTE. The validated object becomes the marker — same sha, so what the oracle approved
    // is byte-for-byte what the ledger gains. The old-value guard makes this a compare-and-swap:
    // a HEAD that moved since the dirty check refuses instead of overwriting.
    if let Err(e) = git_at(
        root,
        &[
            "update-ref",
            "-m",
            &format!("wave --close: {subject}"),
            "HEAD",
            &cand,
            &head,
        ],
    ) {
        wprintln!("REFUSED: could not advance HEAD to the validated marker — no ref moved.");
        wprintln!("         {e}");
        return 1;
    }
    wprintln!("marker committed: {} {subject}", short(&cand));

    // RECORD — the ticket manager stores the close against the exact marker sha and repacks the
    // plan, so the recompiled base becomes {n} and open waves renumber {n}+1 onward. The marker is
    // already committed, so every failure from here on says so and names the safe re-run.
    let members: &[String] = if vouched { wave_ids } else { &[] };
    let mut rerun = vec![label.to_string(), "--sha".to_string(), cand.clone()];
    if vouched {
        rerun.push("--members".to_string());
        rerun.extend(wave_ids.iter().cloned());
    }
    let mut rerun_args = vec!["wave", "close"];
    rerun_args.extend(rerun.iter().map(String::as_str));
    let rerun_command = ctx.ticket_manager.display_command(&rerun_args);
    let recorded = ctx.ticket_manager.wave_close(label, &cand, members);
    ctx.forget_wave_plan();
    match recorded {
        Ok(close) => wprintln!(
            "close recorded: wave {} at {} ({} member(s))",
            close.n,
            short(&cand),
            close.members.len()
        ),
        Err(e) => {
            wprintln!(
                "recording the close FAILED: {}",
                crate::error::error_chain_text(&e)
            );
            wprintln!(
                "  The marker IS committed. Fix the cause, then re-run — it is idempotent for"
            );
            wprintln!("  this wave and sha, so running it again is safe:");
            wprintln!("    {rerun_command}");
            return 1;
        }
    }

    // END-STATE PROOF, not a hope: the promise is "the plan ends check-green with no manual
    // step", so run the check that would have been red and say so.
    match ctx.ticket_manager.wave_check() {
        Ok(check) if check.ok => {}
        Ok(check) => {
            wprintln!("ttm wave check is RED after the close ceremony — fix before pushing:");
            for finding in &check.findings {
                wprintln!("  ERROR: {finding}");
            }
            wprintln!("  The marker IS committed and the close recorded; re-running");
            wprintln!("    {rerun_command}");
            wprintln!("  is safe once the tickets are fixed.");
            return 1;
        }
        Err(e) => {
            wprintln!(
                "ttm wave check could not run after the close ceremony: {}",
                crate::error::error_chain_text(&e)
            );
            wprintln!("  The marker IS committed; re-running `{rerun_command}` is safe.");
            return 1;
        }
    }

    // LEDGER AGREEMENT: the ticket manager's wave base and the marker ledger in git must name the
    // same newest close, or the next gate and the next repack disagree about where waves start.
    let stored_base = match ctx.wave_plan() {
        Ok(plan) => i64::from(plan.wave_base),
        Err(e) => {
            wprintln!(
                "could not read the wave plan after the close: {}",
                crate::error::error_chain_text(&e)
            );
            return 1;
        }
    };
    match super::super::base::newest_close_base(root) {
        Ok(Some(git_base)) if git_base == stored_base && git_base == n => {}
        Ok(git_base) => {
            wprintln!(
                "LEDGERS DISAGREE after the close: the ticket manager's wave base is {stored_base},"
            );
            wprintln!(
                "  the newest standing marker in git claims {}, and this close was wave {n}.",
                git_base.map_or_else(|| "none".to_string(), |b| b.to_string())
            );
            wprintln!("  The marker IS committed; re-running `{rerun_command}` is safe.");
            return 1;
        }
        Err(e) => {
            wprintln!("could not read the marker ledger after the close: {e}");
            return 1;
        }
    }
    wprintln!("ttm wave check green (wave base is now {n} in both ledgers)");

    wprintln!();
    wprintln!("WAVE {n} CLOSED. Wave {} may be dispatched.", n + 1);
    0
}
