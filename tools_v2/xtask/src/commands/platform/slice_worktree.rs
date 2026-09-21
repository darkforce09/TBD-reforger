//! `cargo xtask platform slice-worktree` — one git worktree per SLICE, under the worktree base,
//! on branch `slice/<slice>`.
//!
//! A sub-slice lives in its parent's tree because they are the same
//! slice's work. Subcommands: `new` `list` `merge` `drop` `reap`.
//!
//! ── THIS FILE DESTROYS WORK IF IT IS WRONG ───────────────────────────────────────────────────
//! `drop` and `reap` delete git worktrees, and a worktree's UNCOMMITTED files exist nowhere else —
//! not in the object database, not in a reflog, nowhere. Two recorded incidents are about this
//! file deleting live agents' work: `reap` wiped FIVE mid-slice worktrees ([`cmd_reap`]) and
//! `drop` did the same to one more ([`cmd_drop`]). Every guard is load-bearing scar tissue with a
//! test proving its refusal still fires; do not "simplify" one for looking redundant with another —
//! one of `drop`'s guards was measured silent on the very incident it cites, and the other two
//! caught it.
//!
//! OUTPUT IS A CONTRACT: `cargo xtask mod wave` scrapes this. Its shape is pinned by tests over 29
//! scenarios covering every subcommand and error path, in throwaway repositories with pinned
//! commit dates so even the short SHAs match.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::proc::{Output, Run};

use crate::core::repository_layout::WORKTREES_DIR;
use crate::core::repository_root::find_repo_root;

/// How the operator re-runs this tool, as printed in every refusal message.
///
/// Every guard refusal ends "Merge them, or re-run with: …", which is read at exactly the moment
/// the operator is trying to get unstuck, so this must be a command that runs.
const PROG: &str = "cargo xtask platform slice-worktree --";

/// What an unknown or empty subcommand prints: the lifecycle rule first, then every subcommand
/// spelled the way the operator must retype it, through [`PROG`] — the same authority the guard
/// refusals use, so usage and refusal can never name two different commands.
fn usage() -> String {
    format!(
        "# Slice worktree lifecycle — see {} (operator-defined, binding).\n{USAGE_BODY}",
        crate::core::repository_layout::documentation::SLICE_WORKFLOW_RUNBOOK
    )
}

/// The subcommands, spelled the way the operator must retype them.
const USAGE_BODY: &str = "\
#
# One worktree per SLICE. A sub-slice lives in its parent's worktree,
# because they are the same slice's work. Three worktrees at a time; merge when all three are
# complete; DELETE immediately after merging — leftover trees fill the disk.
#
#   cargo xtask platform slice-worktree -- new   <slice id>
#   cargo xtask platform slice-worktree -- list
#   cargo xtask platform slice-worktree -- merge <slice id>
#   cargo xtask platform slice-worktree -- drop  <slice id>
#   cargo xtask platform slice-worktree -- reap
";

/// Whether a missing oracle lane is fatal. See the licence/policy essay in `cmd_new`.
#[derive(PartialEq, Clone, Copy)]
enum Policy {
    Required,
    Optional,
}

// ── tests ────────────────────────────────────────────────────────────────────────────────────
// Every test builds its own throwaway repo under
// `temp_dir()` and calls `cmd_*` with an explicit `root` — nothing here can reach the real
// `.ai/artifacts/worktrees/`, because `resolve_root()` is never invoked, so there is no env var to
// race on and no path to a live slice.

#[cfg(test)]
mod tests;

mod git_plain;
use git_plain::count;
use git_plain::gn;
use git_plain::gp;
use git_plain::parent_slice;
use git_plain::pt;
use git_plain::pt_stdout;
pub use git_plain::run;
pub use git_plain::run_at;
use git_plain::status_of;

mod drop;
use drop::cmd_drop;
use drop::cmd_reap;

#[cfg(test)]
use git_plain::dispatch;

#[cfg(test)]
use git_plain::{cmd_merge, cmd_new, lane_is_linked, ln_sfn};
