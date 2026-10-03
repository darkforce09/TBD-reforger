//! `cargo xtask platform slice-worktree`: one git worktree per slice, under the worktree base, on
//! branch `slice/<slice>`.
//!
//! **Role:** the lifecycle of slice worktrees: `new` creates one (with the reference-lane
//! symlinks), `list` reports them, `merge` lands a slice's branch, `drop` removes one worktree and
//! `reap` removes every finished one. A sub-slice lives in its parent's worktree.
//!
//! **Position:** reached through `platform_dispatch`; the mod wave driver calls `run_at` in-process
//! for its `prep` and `land`. The subcommand bodies live in `git_plain.rs` (dispatch, `new`,
//! `list`, `merge` and the git call forms) and `drop.rs` (`drop`, `reap`).
//!
//! **Signals & state:** none held; every subcommand reads and mutates the git repository at the
//! resolved root (`TBD_SLICE_WORKTREE_ROOT` overrides it) through [`process_runner::Run`].
//!
//! **Invariants:** `drop` and `reap` delete worktrees, and a worktree's uncommitted files exist
//! nowhere else, so each guard (commits not on main, uncommitted files, an unstarted branch, git's
//! own refusal) is independent and has a test proving its refusal fires; every refusal ends with a
//! re-run command spelled through `PROG`; the printed output is a contract the mod wave driver
//! reads, pinned by tests over every subcommand and error path in throwaway repositories with fixed
//! commit dates.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};
use process_runner::{Output, Run};

use repository_layout::WORKTREES_DIR;
use repository_layout::find_repository_root;

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
        repository_layout::SLICE_WORKFLOW_RUNBOOK
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
pub(crate) use git_plain::run;
pub use git_plain::run_at;
use git_plain::status_of;

mod drop;
use drop::cmd_drop;
use drop::cmd_reap;

#[cfg(test)]
use git_plain::dispatch;

#[cfg(test)]
use git_plain::{cmd_merge, cmd_new, lane_is_linked, ln_sfn};
