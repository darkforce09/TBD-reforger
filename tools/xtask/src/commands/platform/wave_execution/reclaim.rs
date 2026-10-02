//! Reclaim orphan build caches. THIS IS NOT OPTIONAL HOUSEKEEPING: a full disk fails gate steps
//! with "No space left on device", which reads exactly like a build error, and agent target dirs
//! from slices that already shipped are what fills it — every agent is told to remove its own, and
//! an agent killed by a session limit cannot.
//!
//! Skips any dir belonging to a slice whose worktree still exists, so a live agent's cache
//! survives.
//!
//! Gate-private dirs live in the main checkout's build output folder (`target/gate-*`): an
//! expensive warm cache (a cold slice gate measures 23.4 s against 9.3 s warm). Default reclaim
//! does NOT touch them; opt in with `--gate-dirs`. Optional `--gate-dirs-older-than-days N` only
//! removes gate dirs whose directory mtime is older than N days. The rest of `target/` (the shared
//! cache, `target/dev-api`, `target/ci`, `target/dev-mcpd`, `target/db-selftest`) is measured and
//! spared.
//!
//! The retired root-level build folders (`target-dev-api`, `target-ci`, `target-dev-mcpd`,
//! `target-mk-db-selftest`, `target-gate-*`, `dist-gate-*`) are deleted BY DEFAULT: no tool
//! writes them, and a machine that ran earlier tooling still holds tens of gigabytes of them.
//!
//! PER-SLICE private dirs (`target-<SLICE>`, `target-<SLICE>-api`) live at MAIN_ROOT, and nothing
//! else reaps them. See the block inside `reclaim_command.rs` for why they are swept BY DEFAULT
//! while the gate set stays opt-in — the two look alike and are opposites.

use std::path::{Path, PathBuf};

use super::Ctx;
use crate::{werr, wprintln};

#[cfg(test)]
#[path = "tests/reclaim/tests.rs"]
mod tests;

mod reclaim_command;
pub use reclaim_command::cmd_reclaim;

mod build_output_folders;

mod adhoc_token;
use adhoc_token::adhoc_token;
use adhoc_token::df_avail;
use adhoc_token::dir_age_days;

#[cfg(test)]
use build_output_folders::{gate_folders, sweep_gate_folders, sweep_retired_root_level_folders};
#[cfg(test)]
use reclaim_command::{key_of, slice_token};
