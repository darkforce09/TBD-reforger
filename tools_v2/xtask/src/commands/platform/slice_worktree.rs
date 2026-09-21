//! T-853 — port of `scripts/mod/slice-worktree.sh` → `cargo xtask platform slice-worktree`.
//!
//! One worktree per SLICE, under `.ai/artifacts/worktrees/<slice>`, on branch `slice/<slice>`.
//! Sub-slices (`T-181.7.1`) live in their parent's tree (`T-181.7`) because they are the same
//! slice's work. Subcommands: `new` `list` `merge` `drop` `reap`.
//!
//! ── THIS FILE DESTROYS WORK IF IT IS WRONG ───────────────────────────────────────────────────
//! `drop` and `reap` delete git worktrees, and a worktree's UNCOMMITTED files exist nowhere else —
//! not in the object database, not in a reflog, nowhere. Both of the bash's incident reports are
//! about this file deleting live agents' work: `reap` wiped FIVE mid-slice worktrees ([`cmd_reap`])
//! and `drop` did the same to T-352 ([`cmd_drop`]). Every guard is load-bearing scar tissue with a
//! test proving its refusal still fires; do not "simplify" one for looking redundant with another —
//! the bash records that `drop`'s first guard was ported from `reap` and was THE WRONG ONE OF THE
//! THREE, measured silent on the incident it cited.
//!
//! OUTPUT IS A CONTRACT: `wave.sh` and `xtask mod wave` scrape this. Accepted by diffing
//! stdout+stderr+rc against the bash over 29 scenarios covering every subcommand and error path, in
//! throwaway repos with pinned commit dates so even the short SHAs match. Intended deviations: the
//! three marked FAIL-OPEN CLOSED (1..3) and the note on `passthru`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::proc::{Output, Run};

use crate::core::repository_root::find_repo_root;

/// Where slice worktrees live, relative to the repo root. Kept RELATIVE because the bash `cd`s to
/// `$ROOT` and interpolates `$dir` straight into its messages (`already exists:
/// .ai/artifacts/worktrees/T-212`). Only the final `worktree:` line is absolute, built as
/// `$ROOT/$dir`.
const BASE: &str = ".ai/artifacts/worktrees";

/// What the bash prints for `$0` — whatever the caller typed, so "byte-identical" is only defined
/// against one invocation, and the baselines were captured as `bash scripts/mod/slice-worktree.sh`.
/// Not a stale lie *yet*: the `.sh` cannot be deleted while `tools_v2/xtask/src/commands/mod_ops/wave_execution.rs:317,471` and
/// `scripts/platform/wave.sh:3236` still shell out to it, so the `--force` advice below runs today.
/// WHEN THOSE THREE CALL SITES MOVE TO `xtask`, delete the script and repoint this one constant at
/// the `cargo xtask` spelling — that is the whole edit.
/// How the operator re-runs this tool, as printed in every refusal message.
///
/// T-853 REPOINTED THIS when the bash was deleted. It was
/// `scripts/mod/slice-worktree.sh`, which the port had to keep verbatim while the byte-for-byte
/// diff against that script was the acceptance criterion. The moment the script went away, that
/// contract became moot and the string became actively harmful: every guard refusal
/// ("Merge them, or re-run with: …") was telling the operator to run a file that does not exist,
/// at exactly the moment they are trying to get unstuck.
///
/// MEASURED 2026-08-12 — this was found by using the tool for real, dropping six stale slices;
/// the refusal fired correctly and then named a deleted script.
const PROG: &str = "cargo xtask platform slice-worktree --";

/// What an unknown or empty subcommand prints: the lifecycle rule first, then every subcommand
/// spelled the way the operator must retype it, through [`PROG`] — the same authority the guard
/// refusals use, so usage and refusal can never name two different commands.
const USAGE: &str = "\
# Slice worktree lifecycle — see docs/mod/SLICE_WORKFLOW.md (operator-defined, binding).
#
# One worktree per SLICE. Sub-slices (T-181.7.1) live in their parent's worktree (T-181.7),
# because they are the same slice's work. Three worktrees at a time; merge when all three are
# complete; DELETE immediately after merging — leftover trees fill the disk.
#
#   cargo xtask platform slice-worktree -- new   T-181.7
#   cargo xtask platform slice-worktree -- list
#   cargo xtask platform slice-worktree -- merge T-181.7
#   cargo xtask platform slice-worktree -- drop  T-181.7
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
