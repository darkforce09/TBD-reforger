//! `land`, `revert`, `verified` and `wave --close`: the irreversible half of the driver.
//!
//! **Role:** re-exports the landing commands: `cmd_land` merges every ready slice to main and
//! pushes, `cmd_revert` rolls a wave back to its base, `cmd_verified` records the verifier's sha,
//! and `cmd_wave_close` closes a finished wave.
//!
//! **Position:** reached through the wave command table; the bodies live in
//! `land/merge_execution.rs`, `land/wave_close.rs` and `land/close_ceremony.rs`.
//!
//! **Signals & state:** none; module declarations and re-exports.
//!
//! **Invariants:** every failure mode is a refusal, never a silent widening: the argument parser is
//! an allowlist, the merge loop stops at the first conflict without dropping anything, and a red
//! gate after a merge keeps every worktree.

use std::path::Path;

use super::status::lock_or_refuse;
use super::{Ctx, gate, git_stdout, git_stdout_lossy, ledger, push, short, verdict};
use crate::wave_execution::{werr, wprintln};

/// The `wave --close` argument allowlist: `--summary <text>` and `--dry-run`, nothing else.
/// A filter-shaped argument MUST filter or MUST refuse — same rule as `cmd_land`'s parser.
/// `(summary, dry_run, operator-vouched ticket set)` — the parsed shape of `wave --close`.
type CloseArgs = (Option<String>, bool, Option<Vec<String>>);

// ── THE CLOSE CEREMONY ──────────────────────────────────────────────────────────────────────────
//
// `wave --close` used to end at a PRINT, and a human typed the marker commit. The ledger records
// what that produced: every hand-typed marker since wave 132 was malformed — waves 231–235 carry
// prefixed subjects the anchored authority rejects as non-markers, and 218/233 needed
// disavow reverts. So the print is replaced by the ceremony itself: the ONLY writer of marker
// commits is now the code that defines what a marker is.
//
// THE SELF-CHECK RUNS THE REAL AUTHORITY ON THE REAL OBJECT. A string-level re-implementation of
// the oracle would drift from it — the oracles' lesson in miniature — so the candidate marker is
// created first as an UNREACHABLE commit object (`git commit-tree`: object store only, no ref
// moves, `git log` unchanged), [`super::base::wave_close_number`] and
// [`super::base::wave_close_is_newest_wave`] are run against that object, and only an accepted
// candidate is fast-forwarded into the branch (`git update-ref` with the old-value guard). What
// was validated IS what lands, by sha identity; a refused candidate never becomes reachable and
// is garbage for `git gc`.

mod merge_execution;
pub(crate) use merge_execution::cmd_land;
pub(crate) use merge_execution::cmd_revert;
pub(crate) use merge_execution::cmd_verified;

mod wave_close;
use wave_close::close_subject;
pub(crate) use wave_close::cmd_wave_close;
use wave_close::git_at;

mod close_ceremony;
use close_ceremony::close_ceremony;
