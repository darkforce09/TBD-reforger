use super::*;

/// T-923 — the marker commit, the repack and the lock-refresh commit, as ONE motion.
///
/// TESTABILITY CUT, stated plainly: `cmd_wave_close`'s validations (all-shipped, verifier
/// recorded AND at HEAD, the full wave gate) need a live registry, a verifier marker file and a
/// gateable tree — none of which a unit test can fabricate honestly. The ceremony is therefore
/// this separate function, called by `cmd_wave_close` only after every validation has passed,
/// and the fabricated-repo tests drive it directly.
///
/// CWD CONTRACT: the marker authority and oracle are cwd-bound by design ([`Ctx::enter`] chdirs
/// to the root once, and the whole gate stack rides that), so the caller guarantees the process
/// cwd is `root`. Every WRITE this function performs is root-explicit anyway ([`git_at`]): a
/// misdirected caller can misread, but it can never commit into a repo it was not handed — and
/// the misread ends in refusal, because the candidate object cannot resolve outside `root`.
pub(super) fn close_ceremony(
    root: &Path,
    w: &str,
    wave_ids: &[String],
    summary: Option<&str>,
    dry_run: bool,
) -> u8 {
    // Fail-closed parse. The old print did `w.parse().unwrap_or(0)` — fine for prose, lethal for
    // a ledger: a marker claiming wave 0 must be impossible to write, not merely unlikely.
    let n: i64 = match w.parse() {
        Ok(n) => n,
        Err(_) => {
            wprintln!("REFUSED: current wave '{w}' is not a number — no marker written.");
            return 1;
        }
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
        wprintln!("(nothing written; working tree, ledger and lock untouched)");
        return 0;
    }

    // DIRTY TREE = REFUSAL, before anything is created. The ceremony commits twice; starting it
    // on top of unrelated changes either sweeps them into the lock commit or strands them behind
    // a marker. Same LFS-neutral, fail-closed porcelain read as tree_state/git_porcelain_paths
    // (T-401): a status that CANNOT run is never an empty status.
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

    // REPACK — T-914's include-HEAD derivation exists exactly for this moment: the fresh marker
    // sits AT HEAD, so the recompiled base becomes {n} and open waves renumber {n}+1 onward.
    if let Err(e) = ticket_engine::wave_lock::repack_quiet(root) {
        wprintln!("wave repack FAILED after the close marker: {e:#}");
        wprintln!("  The marker IS committed. Fix the ticket tree, run `cargo xtask wave repack`,");
        wprintln!("  commit the lock — the documented close → check-red → repack recovery loop.");
        return 1;
    }

    // The lock-refresh commit, in the shape repack_after_land uses: explicit path, never -A. A
    // byte-identical lock skips the commit (not a reachable state right after a fresh marker —
    // the base just changed — but the guard costs nothing and lies about nothing).
    let lock_dirty = git_at(
        root,
        &[
            "status",
            "--porcelain",
            "--",
            ticket_engine::wave_lock::LOCK_REL,
        ],
    )
    .unwrap_or_default();
    if !lock_dirty.trim().is_empty() {
        let committed = git_at(root, &["add", "--", ticket_engine::wave_lock::LOCK_REL]).is_ok()
            && git_at(root, &["commit", "-m", "wave.lock: repack after close"]).is_ok();
        if !committed {
            wprintln!("could not commit the wave.lock refresh — commit it by hand before pushing");
            return 1;
        }
        wprintln!("wave.lock refreshed and committed (rides this close)");
    }

    // END-STATE PROOF, not a hope: the promise is "tree ends check-green with no manual step",
    // so run the check that would have been red and say so.
    let errs = ticket_engine::wave_lock::check_as_errors(root);
    if !errs.is_empty() {
        wprintln!("wave check is RED after the close ceremony — fix before pushing:");
        for e in &errs {
            wprintln!("  ERROR: {e}");
        }
        return 1;
    }
    wprintln!("wave check green (ledger base is now {n})");

    wprintln!();
    wprintln!("WAVE {n} CLOSED. Wave {} may be dispatched.", n + 1);
    0
}
