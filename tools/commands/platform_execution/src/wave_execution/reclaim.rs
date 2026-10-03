//! `wave reclaim`: reclaim orphaned build caches before a full disk fails the gates.
//!
//! **Role:** re-exports `cmd_reclaim`, which deletes the build folders of slices that already
//! shipped, the retired root-level build folders, and, on request, the gate-private folders.
//!
//! **Position:** reached through the wave command table; the bodies live in
//! `reclaim/reclaim_command.rs`, `reclaim/build_output_folders.rs` and `reclaim/adhoc_token.rs`.
//!
//! **Signals & state:** none; module declarations and re-exports.
//!
//! **Invariants:** a full disk fails gate steps with errors that read like build errors, so reclaim
//! is part of running the factory; a folder belonging to a slice whose worktree still exists is
//! never deleted; per-slice folders are swept by default, the gate-private `target/gate-*` folders
//! only with `--gate-dirs` (optionally `--gate-dirs-older-than-days N`); the rest of `target/` is
//! measured and spared.

use std::path::{Path, PathBuf};

use super::Ctx;
use crate::wave_execution::{werr, wprintln};

#[cfg(test)]
#[path = "tests/reclaim/tests.rs"]
mod tests;

mod reclaim_command;
pub(crate) use reclaim_command::cmd_reclaim;

mod build_output_folders;

mod adhoc_token;
use adhoc_token::adhoc_token;
use adhoc_token::df_avail;
use adhoc_token::dir_age_days;

#[cfg(test)]
use build_output_folders::{gate_folders, sweep_gate_folders, sweep_retired_root_level_folders};
#[cfg(test)]
use reclaim_command::{key_of, slice_token};
