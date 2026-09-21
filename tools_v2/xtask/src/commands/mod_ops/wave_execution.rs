//! `cargo xtask mod wave` — the wave driver for the game-mod programme. **Not**
//! `cargo xtask platform wave`, which drives the platform programme.
//!
//! Subcommands: `status` | `gate` | `land` | `prep [N]` | `push`; an unknown one prints the
//! header on stdout and exits 2.
//!
//! The driver reads the shared wave lock, filtered to its own programme's ids: the driver
//! introduces itself as "T-181 wave status" and `shipped_slices` reads that programme's slice
//! plan exclusively, so a lock wave holding none of its ids is another programme's business. A
//! MISSING lock is a refusal (rc 2), never `ALL PLANNED WAVES SHIPPED` — a driver that shrugs at
//! a missing plan reports green for work it never looked at.
//!
//! Deliberate asymmetries (do not "fix"):
//! - `land`'s dirty refusal uses the raw slice id under the worktree base, while `tree_state`
//!   uses `parent_slice`; the sub-slice path mismatch is intended.
//! - A non-git directory under the worktree base that exists reads as `committed`.
//! - Status ACTION lines name `cargo run -q -p xtask -- mod wave …`.
//! - Push bypasses the pre-push hook with `--no-verify` only when no `assets_v2/terrains/` paths
//!   are in the push.

use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use verification_core::proc::Run;

use crate::core::repository_layout::WORKTREES_DIR;
use crate::core::repository_root::find_repo_root;

/// What an unknown or empty subcommand prints: why the driver exists and every subcommand it
/// accepts, so a session resuming with no memory of where it was can find out from the tool.
fn unknown_help() -> String {
    format!(
        "# Wave lifecycle automation — the programmatic form of {}.\n{UNKNOWN_HELP_BODY}",
        crate::core::repository_layout::documentation::SLICE_WORKFLOW_RUNBOOK
    )
}

/// Everything the help prints after its first line.
const UNKNOWN_HELP_BODY: &str = r###"#
# WHY THIS EXISTS
# ---------------
# The wave cycle (dispatch 3 → merge → reap → verify → next 3) must not depend on any session
# remembering where it was. This driver reads the wave lock and the live git and worktree state
# and derives the answer, so a fresh session — or one resuming after a context compaction — runs
# `cargo run -q -p xtask -- mod wave status` and knows exactly what to do next.
#
#   cargo run -q -p xtask -- mod wave status     # where are we? what is blocking?
#   cargo run -q -p xtask -- mod wave gate       # run every verification gate (the wave gate)
#   cargo run -q -p xtask -- mod wave land       # merge all complete slices, reap trees, run the gate
#   cargo run -q -p xtask -- mod wave prep N     # create worktrees for wave N
#   cargo run -q -p xtask -- mod wave push       # push main to GitHub (refuses to skip a real LFS push)
#
# `land` is deliberately conservative: it REFUSES to merge a worktree with uncommitted changes,
# and it runs the full gate AFTER merging so a bad slice is caught on main immediately.

"###;

// ── plan / registry ───────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq)]
enum TreeState {
    Absent,
    Dirty,
    Committed,
}

impl TreeState {
    fn as_str(&self) -> &'static str {
        match self {
            TreeState::Absent => "absent",
            TreeState::Dirty => "dirty",
            TreeState::Committed => "committed",
        }
    }
}

// ── commands ──────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "tests/wave_execution.rs"]
mod tests;

mod execution;
use execution::cmd_gate;
use execution::current_wave;
use execution::has_work;
pub use execution::run;
use execution::tree_state;
use execution::wave_slices;

mod land;
use land::cmd_land;
use land::cmd_push;

#[cfg(test)]
use execution::{parent_slice, run_with_root};
