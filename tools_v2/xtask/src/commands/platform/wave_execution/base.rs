//! ── WAVE GATE BASE ──────────────────────────────────────────────────────────────────────────
//!
//! THE FIX EXISTS; THE DEFAULT IS THE HAZARD.
//!
//! `cmd_gate` took `${1:-HEAD~1}` and every change-scoped step keys off it, so omitting the
//! argument silently shrank the gate's scope to the last commit and the verdict still read PASS.
//!
//! OBSERVED, closing wave 75: the command center ran `wave gate` with no base. After five merges
//! `HEAD~1` was the LAST MERGE ONLY. GATE reported PASS 26/26 over a wave in which 4 of 5 slices
//! changed the frontend and `trunk build` never ran; re-run against the real base it was 27/27 with
//! the trunk build actually building. REPRODUCED on wave 76's committed history before fixing:
//!
//! ```text
//! base HEAD~1     ->  wasm32 (frontend)  PASS   "frontend untouched"
//!                     trunk build        SKIP (frontend untouched this wave)
//!                     touch_changed      0 changed .rs file(s)
//! base 1614c557   ->  one frontend source changed this wave
//!                     trunk build        WOULD RUN
//!                     touch_changed      1 changed .rs file(s)
//! ```
//!
//! Four steps narrow, not the two first blamed: touch_changed, wasm32 (frontend), fmt (changed) and
//! the trunk conditional. `test xtask+developer-tools` and the other unconditional steps are
//! unaffected.
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
//! THIS IS ANCHORED, AND THE ANCHOR IS HALF THE FIX. It used to accept ANYTHING after
//! `CLOSED`, so a subject that merely CONTINUES past the word became a wave base. Wave 77's
//! A verifier proved it in a clone with `wave 76 CLOSED? reopened — reverting pending re-gate`:
//! derivation returned the fabricated commit, the gate range collapsed to ONE commit, and the
//! entire wave sat outside every change-scoped step — the wave-75 incident this base derivation
//! exists to prevent,
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

use std::path::Path;
pub use ticket_engine::wave_lock::history::{
    WAVE_CLOSE_MARKER_RE, wave_close_disavowed_in, wave_close_number_in, wave_close_subject_ok,
};

use super::{Ctx, git_stdout, git_stdout_lossy, ledger, short, subject};
use crate::{werr, wprintln};

// ── DOES ANYTHING OTHER THAN THE MARKER AGREE? ─────────────────────────────────────────────────
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
//   * the wave plan (then a TSV, now `.ai/tickets/wave.lock`) names TICKETS, not commits.
// And structurally the two cases are twins: "the previous wave closed HERE" and "the previous wave
// closed at HEAD~1" both look like `landings, boundary, landings` to the graph. So a checker that
// can CONFIRM the boundary from other evidence cannot be written today, and asserting one would be
// this program's signature defect wearing a new hat.
//
// WHAT CAN BE WRITTEN is a set of checks that can REFUSE. Three of them, and exactly what each
// proves — stated together with what it CANNOT prove, because a check that overstates its own reach
// is worse than no check at all: the next reader stops looking. That is not hypothetical. The
// sentence that used to stand here said each check drew on "evidence the commit under test did not
// itself produce", and for check 2 that is FALSE. Believing it is what oracle 2 guards against.
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
//   2. TICKET LEDGER — wave_close_ledger_says. THIS CHECK CLOSES THE HOLE, and the hole
//      was in this comment as much as in the code: it claimed independent evidence while reading
//      the wave plan and the ticket registry OUT OF HEAD'S CHECKOUT — where a marker commit that
//      edits both files in the same commit has already written the answer it is about to be
//      graded on. Wave 78's verifier did exactly that, and the gate returned rc 0 over a range of
//      1 commit whose honest span was 9.
//
//      WHAT THESE TWO FILES ACTUALLY ARE, measured across the closes of waves 73-78 (the plan was
//      a TSV then, the registry a JSON monolith; both readers now go through revision-addressed
//      blobs so the shapes of the era are still readable):
//        * the registry is a RESULT, and `wave --close` IS WHAT WRITES IT. At the PARENT of every
//          real close the wave's own tickets are still `deferred`/`ready` — wave 77's five were all
//          `deferred` one commit before 2b144b5d flipped them to `shipped`. So the registry cannot
//          be read from before the boundary: that would contradict, and hard-refuse, every
//          legitimate wave close in this repository's history.
//        * the wave plan is a SCHEDULE, and usually predates the close — w77's rows were filed by
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

#[cfg(test)]
#[path = "tests/base/tests.rs"]
mod tests;

mod wave_close_number;
pub use wave_close_number::is_ancestor;
pub use wave_close_number::prev_wave_close;
pub use wave_close_number::slice_span_check;
pub use wave_close_number::wave_close_is_newest_wave;
pub use wave_close_number::wave_close_ledger_says;
pub use wave_close_number::wave_close_number;

mod demand_base_confirmation;
pub use demand_base_confirmation::gate_base_covers_wave;
pub use demand_base_confirmation::refuse_empty_range;
