//! The wave gate's base: which commit the wave began at, derived and verified.
//!
//! **Role:** derives the wave gate's base from the last wave-close marker commit when no base is
//! given, and verifies any base (derived or explicit) covers the whole wave before a change-scoped
//! step runs; re-exports the marker reading of `base/marker_ledger.rs`.
//!
//! **Position:** called by `gate::cmd_gate` before it takes the gate lock; the oracles live in
//! `wave_close_number.rs` and the refusals in `demand_base_confirmation.rs`. `wave --close` writes
//! the marker commits this module reads.
//!
//! **Signals & state:** none held; reads git history, and the ticket manager's record of a closed
//! wave's membership and its members' statuses at the marker's commit time.
//!
//! **Invariants:** there is no `HEAD~1` fallback — a base that is only the last commit shrinks
//! every change-scoped step (`touch_changed`, `wasm32 (frontend)`, `fmt (changed)`, the trunk
//! conditional) to the last merge while the verdict still reads PASS. The base is derived and then
//! verified rather than demanded, because verification is what catches a wrong explicit base.
//! `origin/main` is never the base: main is pushed at every wave close, so it equals HEAD at gate
//! time. A marker subject is `wave <N> CLOSED` followed by end of subject, `:`, ` —` or ` -` and
//! nothing else; every wider delimiter admits English continuations (`CLOSED?`, `CLOSED,`) that
//! describe a wave that did not close. No single check can confirm a boundary, so three checks
//! refuse: the marker ledger (the boundary claims exactly one more than the newest other marker),
//! the ticket ledger (the ticket manager's recorded membership of the wave, and each member's
//! status at the boundary's commit time from its event log, used only to contradict) and the
//! slice span (the base does not bisect the wave's landings); a
//! boundary none of them can corroborate asks the operator to confirm it.

use std::path::Path;
mod marker_ledger;
pub(crate) use marker_ledger::{
    WAVE_CLOSE_MARKER_RE, newest_close_base, wave_close_disavowed_in, wave_close_number_in,
    wave_close_subject_ok,
};

use super::{Ctx, git_stdout, git_stdout_lossy, ledger, short, subject};
use crate::wave_execution::{werr, wprintln};

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
//   * the wave plan (now held by the central ticket manager) names TICKETS, not commits; the
//     close the ticket manager records carries the marker's sha only because the close ceremony
//     hands it over, so it echoes the marker rather than confirming it.
// And structurally the two cases are twins: "the previous wave closed HERE" and "the previous wave
// closed at HEAD~1" both look like `landings, boundary, landings` to the graph. So a checker that
// can CONFIRM the boundary from other evidence cannot be written today, and asserting one would be
// this program's signature defect wearing a new hat.
//
// WHAT CAN BE WRITTEN is a set of checks that can REFUSE. Three of them, and exactly what each
// proves — stated together with what it CANNOT prove, because a check that overstates its own reach
// is worse than no check at all: the next reader stops looking. That is not hypothetical. The
// sentence that used to stand here said each check drew on "evidence the commit under test did not
// itself produce", and for check 2 over the file-based ledger that was FALSE: the marker commit
// could edit the files it was graded on. Check 2 now reads a separate store; what that does and
// does not buy is spelled out under it.
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
//   2. TICKET LEDGER — wave_close_ledger_says. Evidence: the CENTRAL TICKET MANAGER's record
//      of the wave, read with `ttm wave history <n> --as-of <the marker's committer date>`.
//      MEMBERSHIP is the set the ticket manager recorded for wave n (the frozen pending-close set,
//      or the members `ttm wave close` stored); COMPLETION is each member's status at the
//      marker's commit time, replayed from the ticket's event log.
//
//      WHAT IT PROVES: the ticket manager is a separate store with its own append-only event
//      log, so a marker commit cannot rewrite either answer by editing files in the same commit —
//      the hole the file-based ledger had (a marker that filed its own plan rows and flipped its
//      own registry statuses graded itself; wave 78's forged marker did exactly that). A member
//      whose recorded status at that instant is anything but shipped or cancelled CONTRADICTS the
//      boundary: a wave with open tickets did not close.
//
//      WHAT IT CANNOT PROVE, stated plainly so nobody re-derives it as a surprise: the
//      independence rests on the ticket manager's event log, not on git. An operator who ships
//      the tickets and then writes a marker by hand satisfies both legs — that is byte-for-byte
//      the shape of a legitimate close. A wave the ticket manager has no record of (state
//      `unknown`, no members), a member whose history does not reach the marker's time, or a
//      ticket manager that cannot answer is SILENCE, never agreement, and the caller escalates
//      it to demand_base_confirmation. Completion is used to CONTRADICT; corroboration needs
//      every member recorded complete at that instant.
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

mod wave_close_number;
pub(crate) use wave_close_number::is_ancestor;
pub(crate) use wave_close_number::prev_wave_close;
pub(crate) use wave_close_number::slice_span_check;
pub(crate) use wave_close_number::wave_close_is_newest_wave;
pub(crate) use wave_close_number::wave_close_ledger_says;
pub(crate) use wave_close_number::wave_close_number;

mod demand_base_confirmation;
pub(crate) use demand_base_confirmation::gate_base_covers_wave;
pub(crate) use demand_base_confirmation::refuse_empty_range;
