//! T-887 — port of `scripts/deploy/backup-drill.sh` → `cargo xtask deploy db drill`.
//!
//! Restore-into-scratch recoverability proof. CREATE/DROP only allow-listed scratch DBs
//! (`tbd_drill_probe`, etc.). Live `tbd_reforger` is dump SOURCE only — never a restore target
//! (T-381 via [`crate::commands::deploy::database_operations::refuse_unsafe_restore_target`]).
//!
//! Calls `deploy_db_backup` / `deploy_db_restore` in-process (no `emit-bash-fns`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::commands::deploy::database_operations::{
    count_db_rows, ct_capture, database_exists, db_user, die, info, refuse_unsafe_restore_target,
    require_container, require_pg_tool, warn,
};
use crate::commands::deploy::database_restore::RestoreArgs;
use crate::core::repository_root::find_repo_root;

struct ScratchGuard {
    scratch: String,
    keep: bool,
}

impl Drop for ScratchGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = drop_scratch_db(&self.scratch);
        }
    }
}

#[cfg(test)]
#[path = "tests/database_restore_drill/tests.rs"]
mod tests;

mod execution;
pub use execution::run;

mod sha384_hex;
use sha384_hex::drop_scratch_db;
use sha384_hex::psql_query;
use sha384_hex::psql_scalar;
use sha384_hex::sha384_hex;

#[cfg(test)]
use execution::mig_ver;
