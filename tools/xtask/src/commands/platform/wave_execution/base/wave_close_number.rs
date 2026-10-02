use super::*;

/// The wave NUMBER a marker claims. `None` for anything that is not an anchored marker.
pub fn wave_close_number(rev: &str) -> Option<i64> {
    wave_close_number_in(Path::new("."), rev)
}

/// Has this wave-close been DISAVOWED by a later revert? Returns the reverting commit.
///
/// Verifier F6. A reverted close deriving as the base means a wave the operator had
/// explicitly taken back was never re-gated — narrow and silent, the same shape as everything else
/// on this page. Derivation now SKIPS a disavowed marker and falls through to the one before it,
/// which re-gates the disavowed wave's whole span. That is the over-broad direction, which this
/// file has already established is the safe one.
///
/// The evidence is git's OWN trailer, `This reverts commit <full sha>.`, written by `git revert`
/// and by nothing in this repo. A hand-written revert that omits the trailer is NOT detectable
/// here; that limitation is printed by the caller rather than left for someone to discover.
///
/// `--fixed-strings` keeps this cheap: git prefilters to the handful of commits that quote the sha
/// at all. Without it this forks `git log` once per commit in the range.
pub fn wave_close_disavowed(rev: &str) -> Option<String> {
    wave_close_disavowed_in(Path::new("."), rev)
}

/// The previous wave's close commit = the SHA main was at when THIS wave opened.
///
/// `None` when no marker is reachable. That is a real state (a tree before wave 1) and the caller
/// refuses on it — this function does not invent a fallback, because inventing one is how `HEAD~1`
/// got here.
///
/// HEAD IS EXCLUDED DELIBERATELY. `wave --close` gates BEFORE writing its own marker, so the newest
/// reachable marker is always the previous wave's.
///
/// WHAT THAT DOES NOT MEAN: the tempting sentence to end this paragraph with
/// promised behaviour the code now refuses. It said re-gating an already-closed tree "picks the
/// previous close again and re-gates that whole wave, rather than gating nothing". The picking
/// still happens — but ORACLE 1 then refuses the result, because the close sitting AT HEAD
/// is reachable and claims a HIGHER wave than the base just derived, which is exactly the
/// contradiction that oracle exists to report. Measured at b2afc99a (wave 78's own close, checked
/// out): derives 2b144b5d, then refuses with "CONTRADICTED by the marker ledger", rc 2. That is
/// fail-CLOSED and it is not this ticket's to change, but it is not "re-gates that whole wave"
/// either, and a reader who believes the old sentence will go hunting for a bug that is really a
/// deliberate refusal.
pub fn prev_wave_close() -> Option<String> {
    let head = git_stdout(&["rev-parse", "HEAD"]).filter(|s| !s.is_empty())?;
    let list = git_stdout_lossy(&[
        "rev-list",
        "--extended-regexp",
        &format!("--grep={WAVE_CLOSE_MARKER_RE}"),
        "HEAD",
    ]);
    for sha in list.lines().filter(|l| !l.is_empty()) {
        if sha == head {
            continue;
        }
        // git's --grep matches the WHOLE message, so a body line quoting the marker would
        // false-match; `wave --close` writes it as the SUBJECT, so confirm it there. A bash glob
        // rather than grep on purpose: `rg` does not exist under `bash -c` and the two greps on
        // this machine (ugrep interactively, GNU under a shell) disagree on ERE details.
        // A `case` glob is the same program under both. The oracles keep that reasoning and move the
        // glob into wave_close_subject_ok so derivation and verification share ONE definition of
        // the format — they must not be able to disagree about what a marker is.
        let subj = subject(sha);
        if !wave_close_subject_ok(&subj) {
            continue;
        }
        // F6: a close the operator reverted is not a boundary. Skipping it lands on the
        // PREVIOUS close, which puts the disavowed wave back inside the gate range.
        if let Some(rev) = wave_close_disavowed(sha) {
            werr!(
                "gate: skipping wave-close {} — reverted by {}.",
                short(sha),
                short(&rev)
            );
            werr!("        {subj}");
            werr!(
                "        That wave was disavowed, so its span is re-gated from the close before it."
            );
            continue;
        }
        return Some(sha.to_string());
    }
    None
}

/// Tickets the plan assigns to a wave AS OF A REVISION.
///
/// The plan at a revision is `.ai/tickets/wave.lock` — read as a blob and parsed as
/// TOML. Every boundary BEFORE the cutover has no lock blob there, and refusing to read those
/// revisions would demote every historical wave close from "corroborated" to "demand operator
/// confirmation" — so an absent or unparseable lock blob falls back to the historical TSV
/// readers in [`super::super::archived_wave_plans`], the one module allowed to name the dead files. History is
/// immutable and TSV-shaped; a reader of history may name that shape.
///
/// Takes a rev because the checkout is not evidence. This has exactly one caller — oracle 2
/// — and that caller must not be able to read a plan row the commit it is grading just wrote, so
/// there is deliberately NO checkout-reading variant of this function to reach for by mistake.
///
/// A path `git show` cannot resolve yields no rows, which this check reports as silence and the
/// caller escalates. Fail-closed.
pub fn wave_plan_tickets_at(ctx: &Ctx, rev: &str, n: i64) -> Vec<String> {
    let blob = git_stdout(&["show", &format!("{rev}:{}", ctx.plan)]).unwrap_or_default();
    if !blob.is_empty()
        && let Ok(lock) = ticket_engine::wave_lock::parse(&blob)
    {
        if let Ok(w) = u32::try_from(n) {
            let open = lock.tickets_in_wave(w);
            if !open.is_empty() {
                return open;
            }
            // A CLOSED WAVE LIVES IN `[[emptied]]`, AND THAT IS STILL THE PLAN
            // SPEAKING. The open-wave list is the only place this used to look, so a wave
            // that emptied correctly — every ticket shipped, the repack freezing its set as a
            // pending entry — had NO rows here and oracle 2 reported silence. Every gate then
            // demanded `TBD_GATE_BASE_CONFIRM`, which is exactly the "give the ledger
            // something to say" the refusal asks for, refused about a ledger that WAS saying
            // it. Reading the pending entry can only ever strengthen the check: silence
            // becomes a real ticket list, which the completion test below then contradicts or
            // corroborates.
            return lock
                .emptied
                .iter()
                .find(|e| e.n == w)
                .map(|e| e.tickets.clone())
                .unwrap_or_default();
        }
        return Vec::new();
    }
    super::super::archived_wave_plans::tickets_at(rev, n)
}

/// Of these tickets, which does the registry AS OF A REVISION not call shipped (or cancelled)?
///
/// `None` when the registry could not be read or parsed at that revision (the bash's rc 3).
///
/// One reader for the whole list rather than `is_shipped`'s one-per-ticket: the blob has to be
/// materialised anyway, and a cannot-read must be distinguishable from a clean list here.
/// `is_shipped` answers "not shipped" for a registry it cannot read, which is the right answer for
/// a checkout and the wrong one for this caller — it would turn an unreadable blob into a
/// CONTRADICTION and hard-refuse the gate over a file it never actually examined.
pub fn wave_ledger_unshipped_at(ctx: &Ctx, rev: &str, tickets: &[String]) -> Option<String> {
    let _ = ctx;
    let repo = std::path::Path::new(".");
    let by = ticket_engine::registry::ticket_status_history::status_map_at_rev(repo, rev)?;
    let open: Vec<&str> = tickets
        .iter()
        .filter(|t| {
            !matches!(
                by.get(t.as_str()).map(String::as_str),
                Some("shipped") | Some("cancelled")
            )
        })
        .map(String::as_str)
        .collect();
    Some(open.join(" "))
}

/// ORACLE 1. `0` = this marker claims the highest wave number reachable, by exactly one;
/// `2` = contradicted, from either direction.
pub fn wave_close_is_newest_wave(sha: &str) -> u8 {
    let Some(n) = wave_close_number(sha) else {
        return 2;
    };
    let mut high: Option<i64> = None;
    let list = git_stdout_lossy(&[
        "rev-list",
        "--extended-regexp",
        &format!("--grep={WAVE_CLOSE_MARKER_RE}"),
        "HEAD",
    ]);
    for other in list.lines().filter(|l| !l.is_empty()) {
        if other == sha {
            continue;
        }
        let Some(on) = wave_close_number(other) else {
            continue;
        };
        // Highest number any OTHER marker claims, disavowed ones included. A reverted close still
        // proves its wave number was reached, so it still bounds what the next one may claim;
        // excluding it here would let a fake leap ahead through the very hole the F6 revert fix
        // opened.
        if high.map(|h| on > h).unwrap_or(true) {
            high = Some(on);
        }
        if on < n {
            continue;
        }
        // A DISAVOWED close is not part of the ledger, so it cannot outrank anything. Without this,
        // this check and the F6 revert fix fight each other: derivation correctly steps back past a
        // reverted `wave 76 CLOSED` to wave 75's, and then this refuses wave 75 for being older
        // than the very marker that was just thrown away. Only consulted for markers that would
        // actually refuse, so the normal path pays nothing for it.
        if wave_close_disavowed(other).is_some() {
            continue;
        }
        wprintln!(
            "gate: the derived wave base is CONTRADICTED by the marker ledger — refusing to run."
        );
        wprintln!("        derived {} claims wave {n}", short(sha));
        wprintln!("          {}", subject(sha));
        wprintln!(
            "        but {} also reachable from HEAD claims wave {on}",
            short(other)
        );
        wprintln!("          {}", subject(other));
        wprintln!(
            "        Wave numbers only ever go up: all 34 markers in history run 78 down to 45,"
        );
        wprintln!(
            "        strictly decreasing, no repeats. A newer marker claiming an equal or older wave"
        );
        wprintln!(
            "        means the newest one is not a wave boundary — it is a commit that looks like one,"
        );
        wprintln!(
            "        and gating from it would put a whole wave outside every change-scoped step."
        );
        wprintln!(
            "        If wave {n} really was re-closed, revert the first close (git revert writes the"
        );
        wprintln!("        trailer this script reads) rather than writing a second marker for it.");
        return 2;
    }

    // THE OTHER DIRECTION. "Strictly higher" alone never refuses a number that is higher by
    // a MILE, so `wave 99 CLOSED` outranked all 34 real markers and sailed through. Wave numbers do
    // not merely increase, they increase by ONE: measured 2026-08-01 across every marker reachable
    // from HEAD, 78 down to 45, 33 steps, every one of them exactly 1. So the exact bound is
    // `highest other + 1`, and anything above it is a number no wave has ever reached.
    //
    // Skipped when there is no other marker at all — the first wave ever closed has nothing to be
    // one more than, and inventing a ceiling for it would refuse a legitimate tree.
    if let Some(high) = high
        && n > high + 1
    {
        wprintln!("gate: the derived wave base claims a wave that never opened — refusing to run.");
        wprintln!("        derived {} claims wave {n}", short(sha));
        wprintln!("          {}", subject(sha));
        wprintln!(
            "        but the highest wave any other marker reachable from HEAD claims is {high}, so the"
        );
        wprintln!(
            "        next wave to close can only be {}. Wave numbers advance by exactly one:",
            high + 1
        );
        wprintln!(
            "        measured over all 34 markers, 78 down to 45, 33 steps of 1, no gaps and no repeats."
        );
        wprintln!(
            "        A marker {} waves ahead of the ledger is not a boundary this history",
            n - high
        );
        wprintln!(
            "        ever reached — and gating from it would put every wave in between outside the"
        );
        wprintln!("        range, unread, while the verdict claimed to describe them.");
        return 2;
    }
    0
}

/// ORACLE 2. `0` = ledger corroborates; `1` = ledger cannot speak; `2` = ledger contradicts.
/// Prints its own verdict either way — a check nobody sees the result of is not a check.
///
/// Read the block above for why. In one line: MEMBERSHIP comes from the
/// boundary's PARENT, COMPLETION from the boundary, and only the former can corroborate.
pub fn wave_close_ledger_says(ctx: &Ctx, sha: &str) -> u8 {
    let Some(n) = wave_close_number(sha) else {
        return 1;
    };
    // No parent = no revision before this commit to ask, so there is nothing independent to ask it.
    let Some(par) = git_stdout(&["rev-parse", "--verify", "--quiet", &format!("{sha}^1")])
        .filter(|s| !s.is_empty())
    else {
        wprintln!(
            "        ticket ledger: {} has no parent commit, so there is no",
            short(sha)
        );
        wprintln!(
            "                       revision preceding it to read {} from — cannot corroborate.",
            ctx.plan
        );
        return 1;
    };
    let tickets = wave_plan_tickets_at(ctx, &par, n);
    // `known="$(printf '%s\n' $tickets | wc -l)"` — word-split then line count, so an empty list is
    // 0 via the guard above it.
    let known = tickets.len();

    if known == 0 {
        // THE MEMBERSHIP CASE, and it deserves its own message rather than a generic silence: the plan
        // has rows for wave $n at the boundary but NOT at its parent, which means this very commit
        // filed them. That is self-corroboration, and it is what the forged wave-78 marker did.
        if !wave_plan_tickets_at(ctx, sha, n).is_empty() {
            wprintln!(
                "        ticket ledger: {} ADDED wave {n}'s own rows to {}",
                short(sha),
                ctx.plan
            );
            wprintln!(
                "                       in the same commit that claims wave {n} CLOSED. A commit cannot"
            );
            wprintln!(
                "                       corroborate itself, so this is silence, not agreement — the rows"
            );
            wprintln!(
                "                       are not there at its parent {}.",
                short(&par)
            );
            return 1;
        }
        wprintln!(
            "        ticket ledger: {} has NO rows for wave {n} at {} —",
            ctx.plan,
            short(&par)
        );
        wprintln!(
            "                       it cannot corroborate this boundary. (The plan is only maintained"
        );
        wprintln!("                       for some waves; this is silence, not agreement.)");
        return 1;
    }

    // COMPLETION, read at the boundary, because that is the only place it is ever true:
    // `wave --close` is what flips these tickets to shipped. Used to CONTRADICT only.
    let Some(open) = wave_ledger_unshipped_at(ctx, sha, &tickets) else {
        wprintln!(
            "        ticket ledger: {} could not be read at {}",
            ctx.registry,
            short(sha)
        );
        wprintln!(
            "                       — cannot corroborate. (Cannot-read is cannot-speak: reporting a"
        );
        wprintln!(
            "                       contradiction over a file nobody parsed is the defect this whole"
        );
        wprintln!("                       page exists to stop.)");
        return 1;
    };
    if !open.is_empty() {
        wprintln!(
            "gate: the derived wave base is CONTRADICTED by the ticket ledger — refusing to run."
        );
        wprintln!(
            "        {} says wave {n} CLOSED, and {} at its parent",
            short(sha),
            ctx.plan
        );
        wprintln!(
            "        {} assigns wave {n} ticket(s) that {} does not",
            short(&par),
            ctx.registry
        );
        wprintln!("        call shipped at that same commit: {open}");
        wprintln!(
            "        A wave with open tickets did not close, so this commit is not a wave boundary."
        );
        return 2;
    }
    wprintln!(
        "        ticket ledger: wave {n} has {known} ticket(s) in {} at {}",
        ctx.plan,
        short(&par)
    );
    wprintln!(
        "                       (the boundary's parent, which it cannot have written), all shipped"
    );
    wprintln!("                       at {} — corroborated.", short(sha));
    0
}

/// ORACLE 3. Marker-free. `0` = no objection; `2` = the base bisects the wave's landings.
pub fn slice_span_check(base: &str) -> u8 {
    if subject(base).starts_with("Merge branch 'slice/") {
        wprintln!(
            "gate: base {} IS a slice merge — refusing to run.",
            short(base)
        );
        wprintln!("          {}", subject(base));
        wprintln!(
            "        A wave base is the commit the wave OPENED at, which is the previous wave's"
        );
        wprintln!(
            "        close — never a landing. Starting here excludes every slice that merged"
        );
        wprintln!("        before it, and each of those is work this gate would report PASS over");
        wprintln!(
            "        without reading. (Checked from merge structure alone; no marker consulted.)"
        );
        return 2;
    }
    let base_full = git_stdout_lossy(&["rev-parse", base]);
    let merges = git_stdout_lossy(&["rev-list", "--merges", &format!("{base}..HEAD")]);
    for m in merges.lines().filter(|l| !l.is_empty()) {
        if !subject(m).starts_with("Merge branch 'slice/") {
            continue;
        }
        let Some(f) = git_stdout(&["merge-base", &format!("{m}^1"), &format!("{m}^2")]) else {
            continue;
        };
        if f == base_full {
            continue;
        }
        if !is_ancestor(&f, base) {
            continue;
        }
        wprintln!(
            "gate: base {} cuts through a slice — refusing to run.",
            short(base)
        );
        wprintln!("        {}   ({})", subject(m), short(m));
        wprintln!(
            "        merged INSIDE the range but was branched at {},",
            short(&f)
        );
        wprintln!(
            "        which is BEFORE the base. So that slice's own commits are outside {base}..HEAD"
        );
        wprintln!(
            "        while its merge is inside: the gate would examine part of one slice's work and"
        );
        wprintln!(
            "        report on all of it. Pass a base at or before {}.",
            short(&f)
        );
        wprintln!("        (Checked from merge parents alone; no marker consulted.)");
        return 2;
    }
    0
}

/// `git merge-base --is-ancestor a b`.
pub fn is_ancestor(a: &str, b: &str) -> bool {
    std::process::Command::new("git")
        .args(["merge-base", "--is-ancestor", a, b])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
