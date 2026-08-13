//! ── WAVE GATE BASE ──────────────────────────────────────────────────────────────────────────
//!
//! T-602. THE FIX EXISTED; THE DEFAULT WAS THE HAZARD.
//!
//! `cmd_gate` took `${1:-HEAD~1}` and every change-scoped step keys off it, so omitting the
//! argument silently shrank the gate's scope to the last commit and the verdict still read PASS.
//!
//! OBSERVED, closing wave 75: the command center ran `wave.sh gate` with no base. After five merges
//! `HEAD~1` was the LAST MERGE ONLY. GATE reported PASS 26/26 over a wave in which 4 of 5 slices
//! changed the frontend and `trunk build` never ran; re-run against the real base it was 27/27 with
//! the trunk build actually building. REPRODUCED on wave 76's committed history before fixing:
//!
//! ```text
//! base HEAD~1     ->  wasm32 (frontend)  PASS   "frontend untouched"
//!                     trunk build        SKIP (frontend untouched this wave)
//!                     touch_changed      0 changed .rs file(s)
//! base 1614c557   ->  apps/website/frontend/src/arsenal_rules.rs changed this wave
//!                     trunk build        WOULD RUN
//!                     touch_changed      1 changed .rs file(s)
//! ```
//!
//! Four steps narrow, not the two first blamed: touch_changed, wasm32 (frontend), fmt (changed) and
//! the trunk conditional. `test xtask+tbd-tools` and the other unconditional steps are unaffected.
//!
//! WHY DERIVE-AND-VERIFY RATHER THAN "MAKE THE BASE MANDATORY".
//! Mandatory moves the computation to the operator — the same operator who got it wrong, and who
//! has no cheaper way to compute it than this function does. It would also have to be threaded
//! through `wave --close`, which already passes a base and already passes a WRONG one. So:
//!   * with no argument, DERIVE the base from the wave-close marker. Exact, not a guess.
//!   * with or without an argument, VERIFY the base covers the whole wave and REFUSE if it does
//!     not. Verification is what catches an explicit base, which is the half a mandatory argument
//!     cannot.
//!   * never fall back to HEAD~1. There is no wave for which "the last commit" is a safe default.
//!
//! `origin/main` was the ticket's suggested derivation and it is measurably wrong here: main is
//! pushed at every wave close, so at gate time `origin/main` == HEAD and `git merge-base
//! origin/main HEAD` returns HEAD — the vacuous range this function exists to refuse. Verified
//! 2026-07-31: `git rev-parse origin/main` == `git rev-parse HEAD` == efc3851c.
//!
//! The commit `wave --close` writes at the end of every wave. 33 in history (waves 45-77,
//! recounted 2026-08-01), one format, varying only after the word CLOSED: `wave 76 CLOSED — …`,
//! `wave 75 CLOSED: …`. Nothing else has ever followed `CLOSED` in any of them.
//!
//! T-613 — THIS IS ANCHORED, AND THE ANCHOR IS HALF THE FIX. It used to accept ANYTHING after
//! `CLOSED`, so a subject that merely CONTINUES past the word became a wave base. Wave 77's
//! verifier proved it in a clone with `wave 76 CLOSED? reopened — reverting T-608 pending re-gate`:
//! derivation returned the fabricated commit, the gate range collapsed to ONE commit, and the
//! entire wave sat outside every change-scoped step — the wave-75 incident T-602 exists to prevent,
//! reachable through the front door.
//!
//! THE DELIMITER SET IS end-of-subject, `:`, ` —`, ` -` AND NOTHING ELSE. Reasoning, because the
//! next reader will want to widen it: `CLOSED` alone plus the two forms above are everything
//! `wave --close` and 33 real commits have produced, and the ASCII ` -` is admitted only because
//! the em dash is a keyboard hazard, not because anything writes it. Every widening admits a class
//! of English continuation — `CLOSED?`, `CLOSED,`, `CLOSED (partially)`, `CLOSEDish` — and each of
//! those is a plausible thing a hurried operator writes about a wave that DID NOT close. The cost
//! of being too strict is a wave-close commit that has to be reworded once; the cost of being too
//! loose is a gate reporting PASS over a wave it never read.

use super::{Ctx, git_stdout, git_stdout_lossy, ledger, short, subject};
use crate::{werr, wprintln};

/// The PREFILTER ONLY.
///
/// `git rev-list --grep` runs it through GIT's regex engine, not the system grep, so the
/// ugrep/GNU divergence noted inside [`prev_wave_close`] does not reach it. The AUTHORITY is
/// [`wave_close_subject_ok`] below, which is pure `case` and therefore the same program under every
/// shell here. Measured 2026-08-01: this pattern and the old loose one select exactly the same 33
/// commits, so anchoring the prefilter cannot lose a real marker.
///
/// It is handed to git verbatim, so the engine that evaluates it is unchanged by this port.
pub const WAVE_CLOSE_MARKER_RE: &str = r"^wave [0-9]+ CLOSED([:]|$| —| -)";

/// Is this SUBJECT a wave-close marker? Pure prefix matching, no regex at all — see the note inside
/// [`prev_wave_close`] for why a glob and not grep, which T-613 preserves rather than replaces. The
/// number is validated as digits, so `wave 7x CLOSED` cannot become a boundary either.
pub fn wave_close_subject_ok(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("wave ") else {
        return false;
    };
    // `n="${rest%% *}"` — up to the first space, or all of it when there is none.
    let n = match rest.find(' ') {
        Some(i) => &rest[..i],
        None => rest,
    };
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let rest = &rest[n.len()..];
    rest == " CLOSED"
        || rest.starts_with(" CLOSED:")
        || rest.starts_with(" CLOSED —")
        || rest.starts_with(" CLOSED -")
}

/// The wave NUMBER a marker claims. `None` for anything that is not an anchored marker.
pub fn wave_close_number(rev: &str) -> Option<i64> {
    let s = git_stdout(&["log", "-1", "--format=%s", rev])?;
    if !wave_close_subject_ok(&s) {
        return None;
    }
    let rest = s.strip_prefix("wave ")?;
    let n = match rest.find(' ') {
        Some(i) => &rest[..i],
        None => rest,
    };
    n.parse().ok()
}

/// Has this wave-close been DISAVOWED by a later revert? Returns the reverting commit.
///
/// T-613 (verifier F6). A reverted close still derived as the base, so a wave the operator had
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
    let full = git_stdout(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{rev}^{{commit}}"),
    ])
    .filter(|s| !s.is_empty())?;
    let needle = format!("This reverts commit {full}.");
    let list = git_stdout_lossy(&[
        "rev-list",
        "--fixed-strings",
        &format!("--grep={needle}"),
        &format!("{full}..HEAD"),
    ]);
    for c in list.lines().filter(|l| !l.is_empty()) {
        let body = git_stdout(&["log", "-1", "--format=%B", c]).unwrap_or_default();
        if body.contains(&needle) {
            return Some(c.to_string());
        }
    }
    None
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
/// WHAT THAT NO LONGER MEANS, corrected T-618 because the sentence that used to end this paragraph
/// promised behaviour the code now refuses. It said re-gating an already-closed tree "picks the
/// previous close again and re-gates that whole wave, rather than gating nothing". The picking
/// still happens — but T-613's ORACLE 1 then refuses the result, because the close sitting AT HEAD
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
        // this machine (ugrep interactively, GNU under `bash script.sh`) disagree on ERE details.
        // A `case` glob is the same program under both. T-613 keeps that reasoning and moves the
        // glob into wave_close_subject_ok so derivation and verification share ONE definition of
        // the format — they must not be able to disagree about what a marker is.
        let subj = subject(sha);
        if !wave_close_subject_ok(&subj) {
            continue;
        }
        // T-613 / F6: a close the operator reverted is not a boundary. Skipping it lands on the
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

// ── T-613: DOES ANYTHING OTHER THAN THE MARKER AGREE? ───────────────────────────────────────────
//
// THE HONEST STATEMENT FIRST, because the ticket asked for an INDEPENDENT oracle and the truthful
// answer is that a FULLY independent one DOES NOT EXIST in this repository today.
//
// A wave boundary is recorded in exactly ONE place: the subject of the commit left behind when a
// wave closes. Everything else was checked, 2026-08-01, and none of it is anchored to a sha:
//   * `.ai/artifacts/last-verified` is GITIGNORED (.gitignore:55) — one line, no history.
//   * there are no wave tags: `git tag -l` is 100% `T-*` ticket tags, zero wave-shaped refs.
//   * `slice/*` branches are never deleted — 17 of them survive, spanning waves 75 to 78, so a
//     branch ref cannot say which wave is current.
//   * `docs/platform/wave_plan.tsv` names TICKETS, not commits.
// And structurally the two cases are twins: "the previous wave closed HERE" and "the previous wave
// closed at HEAD~1" both look like `landings, boundary, landings` to the graph. So a checker that
// can CONFIRM the boundary from other evidence cannot be written today, and asserting one would be
// this program's signature defect wearing a new hat.
//
// WHAT CAN BE WRITTEN is a set of checks that can REFUSE. Three of them, and exactly what each
// proves — stated together with what it CANNOT prove, because a check that overstates its own reach
// is worse than no check at all: the next reader stops looking. That is not hypothetical. The
// sentence that used to stand here said each check drew on "evidence the commit under test did not
// itself produce", and for check 2 it was FALSE. T-618 exists because it was believed.
//
//   1. MARKER LEDGER — wave_close_is_newest_wave. Evidence: the OTHER 33 markers. A derived
//      boundary must claim a wave number strictly HIGHER than every other marker reachable from
//      HEAD, and NOT MORE THAN ONE higher. Measured 2026-08-01 over all 34: 78 down to 45, strictly
//      decreasing, no repeats, NO GAPS — 33 steps of exactly 1 — so `highest other + 1` is an exact
//      upper bound rather than a guess. This is not independent of the marker FAMILY, but it is
//      independent of the commit being checked: the constraint comes from commits the forger did
//      not write, so a fake cannot self-approve. The lower bound kills the replay/re-close shape
//      ("wave 76 CLOSED — reopened and re-closed…"); the upper bound kills the leap-ahead shape
//      ("wave 99 CLOSED"), which a bare "strictly higher" test waves straight through.
//
//   2. TICKET LEDGER — wave_close_ledger_says. THIS CHECK WAS THE HOLE T-618 CLOSES, and the hole
//      was in this comment as much as in the code: it claimed independent evidence while reading
//      docs/platform/wave_plan.tsv and .ai/tickets/registry.json OUT OF HEAD'S CHECKOUT — where a
//      marker commit that edits both files in the same commit has already written the answer it is
//      about to be graded on. Wave 78's verifier did exactly that, and the gate returned rc 0 over
//      a range of 1 commit whose honest span was 9.
//
//      WHAT THESE TWO FILES ACTUALLY ARE, measured across the closes of waves 73-78:
//        * registry.json is a RESULT, and `wave --close` IS WHAT WRITES IT. At the PARENT of every
//          real close the wave's own tickets are still `deferred`/`ready` — wave 77's five were all
//          `deferred` one commit before 2b144b5d flipped them to `shipped`. So the registry cannot
//          be read from before the boundary: that would contradict, and hard-refuse, every
//          legitimate wave close in this repository's history.
//        * wave_plan.tsv is a SCHEDULE, and usually predates the close — w77's rows were filed by
//          2a8b41e2, a separate ticket-filing commit. But not always: the real wave-78 close
//          b2afc99a appended its own four w78 rows.
//
//      Hence the split this check now makes. MEMBERSHIP — which tickets wave N contains — is read
//      with `git show` at the boundary's PARENT, a revision the commit under test cannot have
//      written. COMPLETION — are they shipped — is read at the boundary itself, and is used ONLY to
//      CONTRADICT, never as the corroborating leg. Corroboration therefore rests entirely on the
//      leg that is independent; when the boundary filed its own plan rows, that leg is absent and
//      this reports SILENCE, which the caller escalates to demand_base_confirmation.
//
//      WHAT IT STILL CANNOT DO, stated plainly so nobody re-derives it as a surprise: a forger who
//      files the plan rows in a SEPARATE, EARLIER commit and flips the registry in the marker
//      satisfies both legs. Measured in a scratch clone — that shape passes. It is not detectable
//      here, because it is byte-for-byte the shape of a legitimate close (w77 was filed in exactly
//      that order). Nothing but demand_base_confirmation stands behind it, and an operator naming
//      the sha is the honest floor of this whole page.
//
//   3. SLICE SPAN — slice_span_check. Evidence: MERGE PARENTS. Reads no marker at all, which makes
//      it the one genuinely marker-independent check here. Two clauses: a base may not BE a slice
//      merge (a wave base is the previous close, never a landing), and no slice merge inside
//      base..HEAD may fork from before the base (that would mean the range bisects a slice, so the
//      gate reads half of somebody's work). Measured against all 33 waves of real history: 12 slice
//      merges examined, 0 violations — it has never fired on a legitimate base. It catches the
//      narrowing shapes (HEAD~1, mid-wave) on graph structure alone; it cannot catch a base placed
//      AFTER the whole wave, which is why 1 and 2 exist.
//
// WHEN NOTHING CAN SPEAK, THE GATE REFUSES AND ASKS. There is no silent pass left on this path:
// TBD_GATE_BASE_CONFIRM must name the exact sha, so confirming requires reading the sha.

/// Tickets the plan assigns to a wave AS OF A REVISION, accepting both label spellings in the file
/// (`77` and `w77`).
///
/// THE `sub(/^w/,"",w)` BELOW IS NOT STYLE TOLERANCE AND MUST NOT BE "TIDIED UP" NOW THAT T-616 HAS
/// NORMALISED THE COLUMN TO BARE INTEGERS. T-616 normalised the WORKING TREE; it cannot normalise
/// HISTORY, and this function reads history exclusively — `git show "$1:$PLAN"` at the boundary's
/// PARENT. Every revision at or before wave 79's close still spells those rows `w76`…`w79`, because
/// that is what was committed. MEASURED 2026-08-01 against wave 79's close 6b2f4750: the parent
/// 3c44b6ea holds 5 rows literally beginning `w79`, and stripping the prefix is the only reason
/// oracle 2 can still say "corroborated" instead of falling silent. Delete it and every wave close
/// in this repository's history becomes unverifiable in one commit.
///
/// T-618: takes a rev because the checkout is not evidence. This has exactly one caller — oracle 2
/// — and that caller must not be able to read a plan row the commit it is grading just wrote, so
/// there is deliberately NO checkout-reading variant of this function to reach for by mistake.
///
/// The two filters are `plan_rows`' filters, repeated rather than reused, because `plan_rows` reads
/// a FILE and this reads a BLOB.
///
/// A `$PLAN` that TBD_WAVE_PLAN has pointed outside the repo is not a path `git show` can resolve;
/// that yields no rows, which this check reports as silence and the caller escalates. Fail-closed.
pub fn wave_plan_tickets_at(ctx: &Ctx, rev: &str, n: i64) -> Vec<String> {
    let blob = git_stdout(&["show", &format!("{rev}:{}", ctx.plan)]).unwrap_or_default();
    let want = n.to_string();
    blob.lines()
        .filter(|l| !l.starts_with('#'))
        .filter(|l| {
            !(l.starts_with("wave")
                && l[4..]
                    .chars()
                    .next()
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false))
        })
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            let mut it = l.split('\t');
            let w = it.next().unwrap_or("");
            let t = it.next().unwrap_or("");
            let w = w.strip_prefix('w').unwrap_or(w);
            // awk's `w == n` after `sub()`. Both sides look numeric on every row that matters, so
            // the numeric and string readings agree; see ledger::awk_eq for the general rule.
            let eq = match (w.parse::<f64>(), want.parse::<f64>()) {
                (Ok(a), Ok(b)) => a == b,
                _ => w == want,
            };
            if eq { Some(t.to_string()) } else { None }
        })
        .collect()
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
    let by = crate::tickets_store::status_map_at_rev(repo, rev)?;
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

    // T-618, THE OTHER DIRECTION. "Strictly higher" alone never refuses a number that is higher by
    // a MILE, so `wave 99 CLOSED` outranked all 34 real markers and sailed through. Wave numbers do
    // not merely increase, they increase by ONE: measured 2026-08-01 across every marker reachable
    // from HEAD, 78 down to 45, 33 steps, every one of them exactly 1. So the exact bound is
    // `highest other + 1`, and anything above it is a number no wave has ever reached.
    //
    // Skipped when there is no other marker at all — the first wave ever closed has nothing to be
    // one more than, and inventing a ceiling for it would refuse a legitimate tree.
    if let Some(high) = high {
        if n > high + 1 {
            wprintln!(
                "gate: the derived wave base claims a wave that never opened — refusing to run."
            );
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
    }
    0
}

/// ORACLE 2. `0` = ledger corroborates; `1` = ledger cannot speak; `2` = ledger contradicts.
/// Prints its own verdict either way — a check nobody sees the result of is not a check.
///
/// T-618. Read the block above for what changed and why. In one line: MEMBERSHIP comes from the
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
        // THE T-618 CASE, and it deserves its own message rather than a generic silence: the plan
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

/// Print the derived base loudly, then demand the operator name it. Used when NOTHING could
/// corroborate. Loud-and-blocked, not quiet-and-passed: the whole point of T-613.
pub fn demand_base_confirmation(ctx: &Ctx, bsha: &str, why: &str) -> u8 {
    let confirm = std::env::var("TBD_GATE_BASE_CONFIRM").unwrap_or_default();
    if confirm == bsha || confirm == short(bsha) {
        wprintln!("        base confirmed by TBD_GATE_BASE_CONFIRM.");
        return 0;
    }
    wprintln!("gate: nothing could corroborate this wave base — refusing to run unconfirmed.");
    wprintln!("        base   {bsha}");
    wprintln!("               {}", subject(bsha));
    wprintln!(
        "               {}",
        git_stdout(&["log", "-1", "--format=%an, %ad", "--date=short", bsha]).unwrap_or_default()
    );
    wprintln!(
        "        range  {} commit(s) to HEAD {}",
        git_stdout(&["rev-list", "--count", &format!("{bsha}..HEAD")]).unwrap_or_default(),
        short("HEAD")
    );
    wprintln!("        why    {why}");
    wprintln!();
    wprintln!(
        "        This is NOT a claim that the base is wrong. It is a refusal to claim it is right."
    );
    wprintln!(
        "        Read the subject above. If that is genuinely where this wave opened, re-run with:"
    );
    wprintln!("            TBD_GATE_BASE_CONFIRM={bsha} cargo xtask platform wave gate ...");
    wprintln!(
        "        The better fix is to give the ledger something to say: add this wave's rows to"
    );
    wprintln!(
        "        {} BEFORE the wave closes — in the commit that files its tickets, the way",
        ctx.plan
    );
    wprintln!(
        "        2a8b41e2 filed wave 77's. Rows appended by the closing commit itself corroborate"
    );
    wprintln!(
        "        nothing (T-618): oracle 2 reads the plan at the boundary's PARENT precisely so a"
    );
    wprintln!(
        "        commit cannot vouch for itself, so rows that arrive with the marker are not there."
    );
    2
}

/// Refuse a base that does not cover the whole wave.
///
/// One rule: THE BASE MUST BE AT OR BEFORE THE COMMIT THIS WAVE OPENED AT — i.e. it is an
/// ancestor-or-equal of the previous wave's close. A base OLDER than that passes on purpose:
/// over-broad gates more than it must, and over-broad has never been the failure mode here. Narrow
/// is, every time.
///
/// NOT "every slice MERGE is inside base..HEAD", which is how the ticket phrased it. Measured: wave
/// 76 landed T-608 as a plain commit with no merge at all, and wave 74 landed three that way
/// (`c7a3ff78`, `bed4f269`, `0a1a53ac`). Enumerating merges would have called such a wave covered
/// while its non-merge landings sat outside the range — the same lie in a new place. The ancestor
/// test is landing-shape-independent. ([`slice_span_check`] enumerates merges for a DIFFERENT
/// question — whether the range bisects one — where the shape is exactly what is being asked
/// about.)
///
/// T-613 — THE ANCESTOR TEST BELOW IS STILL ASKED OF [`prev_wave_close`], THE FUNCTION THAT
/// PRODUCED THE ANSWER, and that cannot be fixed by moving the call: there is no second record of
/// the boundary to ask instead. What changed is that the derived boundary must now survive three
/// cross-checks that do NOT come from it, and that a boundary nothing can corroborate is refused
/// rather than trusted.
pub fn gate_base_covers_wave(ctx: &Ctx, base: &str) -> u8 {
    let Some(bsha) = git_stdout(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{base}^{{commit}}"),
    ])
    .filter(|s| !s.is_empty()) else {
        return 2;
    };
    // A base off HEAD's history makes `base..HEAD` an unrelated set, not "this wave".
    if !is_ancestor(&bsha, "HEAD") {
        wprintln!("gate: base '{base}' is not an ancestor of HEAD — refusing to run.");
        wprintln!("        '{base}..HEAD' would describe a set of commits nobody asked about.");
        return 2;
    }
    // No marker to compare against. This used to `return 0` — an explicit base plus a silent pass,
    // which is the shape this file exists to hunt. ORACLE 3 needs no marker, so it still speaks
    // here; after that, say what could not be checked and make the operator name the sha.
    let Some(psha) = prev_wave_close() else {
        if slice_span_check(&bsha) != 0 {
            return 2;
        }
        if demand_base_confirmation(
            ctx,
            &bsha,
            "no 'wave N CLOSED' commit is reachable from HEAD, so the previous wave's close is unknown",
        ) != 0
        {
            return 2;
        }
        return 0;
    };
    // ORACLES 1 and 2, against the DERIVED boundary, before it is allowed to judge anything.
    if wave_close_is_newest_wave(&psha) != 0 {
        return 2;
    }
    wprintln!("gate: cross-checking derived wave base {}", short(&psha));
    let lrc = wave_close_ledger_says(ctx, &psha);
    if lrc == 2 {
        return 2;
    }
    if lrc == 1 {
        let why = format!(
            "the marker ledger accepts it (wave {} is the newest closed wave) but the ticket ledger has no rows for that wave that the boundary did not write itself, so only one family of evidence agrees",
            wave_close_number(&psha)
                .map(|n| n.to_string())
                .unwrap_or_default()
        );
        if demand_base_confirmation(ctx, &psha, &why) != 0 {
            return 2;
        }
    }
    // The primary rule, with the message that names the exact cost. ORACLE 3 runs after it, not
    // before, so a narrowing base is diagnosed by the check that can say how much it narrows by.
    if is_ancestor(&bsha, &psha) {
        if slice_span_check(&bsha) != 0 {
            return 2;
        }
        return 0;
    }
    // psha..bsha, NOT bsha..psha. bsha is the NEWER of the two here (that is what makes it wrong),
    // so this counts the wave's commits that the base skips past — the ones every change-scoped
    // step would never see. Reversed, it is always 0, which is exactly the reassuring lie to avoid.
    let missed = git_stdout(&["rev-list", "--count", &format!("{psha}..{bsha}")])
        .unwrap_or_else(|| "?".into());
    wprintln!(
        "gate: base {} starts AFTER this wave opened — refusing to run.",
        short(&bsha)
    );
    wprintln!("        this wave opened at {}", short(&psha));
    wprintln!("          {}", subject(&psha));
    wprintln!(
        "        {missed} commit(s) of this wave sit OUTSIDE {base}..HEAD. touch_changed, wasm32,"
    );
    wprintln!(
        "        fmt and the trunk build would each report PASS/SKIP without reading one of them,"
    );
    wprintln!(
        "        and the verdict would describe a fraction of the wave. That is T-602 verbatim."
    );
    wprintln!(
        "        Fix: run 'cargo xtask platform wave gate' with NO base (it derives {}),",
        short(&psha)
    );
    wprintln!("             or pass a base at or before {}.", short(&psha));
    2
}

/// Refuse a gate whose change set is EMPTY.
///
/// Resolvability is not non-vacuity, and the first version of this guard only checked the former.
/// Found by wave 1's adversarial verifier, which got `GATE: PASS` out of both surviving holes:
///   `gate HEAD`          -> `HEAD^{commit}` resolves, `HEAD..HEAD` is empty, every change-scoped
///                           step PASSes without invoking hostrun even once.
///   `gate --slice T-393` -> gate_slice never passed a base at all, so the helpers defaulted to
///                           `main...HEAD` — correct inside a worktree, EMPTY when run on main,
///                           and the ticket id argument is decorative so it cannot self-correct.
/// Both printed PASS having compiled nothing. Same signature defect, two more doorways.
///
/// A slice legitimately has an empty *frontend* change set — that is what the per-step SKIPs are
/// for. What is never legitimate is the WHOLE range being empty, because then no change-scoped step
/// examined anything and the verdict describes nothing.
pub fn refuse_empty_range(range: &str, what: &str) -> u8 {
    // Same committed ∪ working-tree union as changed_rs. Diffing the range alone refused
    // `gate --slice` when a slice had working-tree changes but no commits yet — contradicting
    // changed_rs's stated purpose (T-409 NIT; pre-existing, not T-406).
    // Porcelain via git_porcelain_paths (T-401) — never treat LFS filter exit 128 as empty.
    let wt = match ledger::git_porcelain_paths() {
        Ok(v) => v,
        Err(rc) => return rc as u8,
    };
    let diff = git_stdout_lossy(&["diff", "--name-only", range]);
    let mut all: Vec<String> = diff.lines().map(str::to_string).collect();
    all.extend(wt);
    all.sort();
    all.dedup();
    let n = all.iter().filter(|s| !s.is_empty()).count();
    if n > 0 {
        return 0;
    }
    wprintln!("gate: '{range}' (plus working tree) contains no changed files — refusing to run.");
    wprintln!(
        "        Every change-scoped step (wasm32, fmt, clippy, trunk) would report PASS/SKIP"
    );
    wprintln!("        without reading a line, and the verdict would describe nothing.");
    wprintln!("        {what}");
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_accepted_suffixes_and_nothing_else() {
        // T-613. Every widening admits a class of English continuation, and each of those is a
        // plausible thing a hurried operator writes about a wave that DID NOT close.
        assert!(wave_close_subject_ok("wave 76 CLOSED"));
        assert!(wave_close_subject_ok("wave 76 CLOSED: five slices"));
        assert!(wave_close_subject_ok("wave 76 CLOSED — five slices"));
        assert!(wave_close_subject_ok("wave 76 CLOSED - five slices"));

        // The exact forgery wave 77's verifier used.
        assert!(!wave_close_subject_ok(
            "wave 76 CLOSED? reopened — reverting T-608 pending re-gate"
        ));
        assert!(!wave_close_subject_ok("wave 76 CLOSED, partially"));
        assert!(!wave_close_subject_ok("wave 76 CLOSED (partially)"));
        assert!(!wave_close_subject_ok("wave 76 CLOSEDish"));
        // The number is validated as digits, so `wave 7x CLOSED` cannot become a boundary either.
        assert!(!wave_close_subject_ok("wave 7x CLOSED"));
        assert!(!wave_close_subject_ok("wave  76 CLOSED"));
        assert!(!wave_close_subject_ok("Wave 76 CLOSED"));
        assert!(!wave_close_subject_ok("wave 76 closed"));
        assert!(!wave_close_subject_ok("re: wave 76 CLOSED"));
    }

    #[test]
    fn the_prefilter_and_the_authority_agree_on_the_delimiters() {
        // The ERE is handed to git, but it must not be able to select a subject the authority
        // rejects for a delimiter reason — that is the T-613 hole in reverse.
        let re = regex::Regex::new(WAVE_CLOSE_MARKER_RE).unwrap();
        for s in [
            "wave 76 CLOSED",
            "wave 76 CLOSED: x",
            "wave 76 CLOSED — x",
            "wave 76 CLOSED - x",
        ] {
            assert!(re.is_match(s), "{s}");
            assert!(wave_close_subject_ok(s), "{s}");
        }
        for s in ["wave 76 CLOSED? x", "wave 76 CLOSEDish"] {
            assert!(!re.is_match(s), "{s}");
            assert!(!wave_close_subject_ok(s), "{s}");
        }
    }
}
