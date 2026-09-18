//! T-890 — port of `scripts/mod/wave.sh` → `cargo xtask mod wave`.
//!
//! Mod-program wave driver (T-181). **Not** `scripts/platform/wave.sh`.
//!
//! Subcommands (bash `${1:-status}`): `status` | `gate` | `land` | `prep [N]` | `push`.
//! Unknown → print the header on stdout, exit 2.
//!
//! T-912.2: the mod wave-plan TSV died with the platform one; this driver reads the shared
//! `.ai/tickets/wave.lock`, filtered to its own program's ids (`T-181.*` — the driver has always
//! introduced itself as "T-181 wave status", and `shipped_slices` reads the T-181 slice plan
//! exclusively). A lock wave with no T-181 id is another program's business. A MISSING lock is a
//! refusal (rc 2), not `ALL PLANNED WAVES SHIPPED` — that TSV-era shrug is the false-green class
//! T-912.2 exists to kill.
//!
//! Preserved oddities (do not "fix"):
//! - Registry shipped set was python3; now serde_json — same join-on-space shape.
//! - `land` dirty refuse uses `git -C "$BASE/$s"` (raw slice id), while `tree_state`
//!   uses `parent_slice` — sub-slice path mismatch preserved.
//! - Non-git dir under BASE that exists → `committed` (empty porcelain + `2>/dev/null`).
//! - Status ACTION lines name `cargo run -q -p xtask -- mod wave …` (post-shell port).
//! - Push bypasses pre-push with `--no-verify` only when no `assets_v2/terrains/` paths.

use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use verification_core::proc::Run;

use crate::core::repository_root::find_repo_root;

const BASE: &str = ".ai/artifacts/worktrees";

/// The historical bash header, retargeted at the lock when the TSV died (T-912.2).
const UNKNOWN_HELP: &str = r###"# Wave lifecycle automation — the programmatic form of docs/mod/SLICE_WORKFLOW.md.
#
# WHY THIS EXISTS
# ---------------
# The wave cycle (dispatch 3 → merge → reap → verify → next 3) must not depend on any session
# remembering where it was. This driver reads .ai/tickets/wave.lock (T-181 rows) and the live
# git/worktree state and derives the answer, so a fresh session — or one resuming after a context
# compaction — runs `cargo run -q -p xtask -- mod wave status` and knows exactly what to do next.
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
