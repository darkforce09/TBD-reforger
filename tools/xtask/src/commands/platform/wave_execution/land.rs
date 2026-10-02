//! `land`, `revert`, `verified`, `wave --close` — the irreversible half.
//!
//! `land` merges to main and pushes. Everything here is written so that the failure mode is a
//! refusal, never a silent widening: the argument parser is an allowlist, the merge loop stops at
//! the first conflict without dropping anything, and a red gate after merge KEEPS every worktree.

use std::path::Path;

use super::status::lock_or_refuse;
use super::{Ctx, gate, git_stdout, git_stdout_lossy, ledger, push, short, verdict};
use crate::{werr, wprintln};

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

#[cfg(test)]
#[path = "tests/land/tests.rs"]
mod tests;

mod merge_execution;
pub use merge_execution::cmd_land;
pub use merge_execution::cmd_revert;
pub use merge_execution::cmd_verified;

mod wave_close;
use wave_close::close_subject;
pub use wave_close::cmd_wave_close;
use wave_close::git_at;

mod close_ceremony;
use close_ceremony::close_ceremony;

#[cfg(test)]
use wave_close::{close_target, parse_close_args, sanitize_summary};

#[cfg(test)]
use merge_execution::is_ticket_glob;
