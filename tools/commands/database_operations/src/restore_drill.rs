//! `cargo xtask deploy db drill`: restore the newest dump into a scratch database and verify it.
//!
//! **Role:** the restore drill: the newest (or a fresh) dump restored into a scratch database, its
//! migrations and rows compared with the source, then the scratch dropped.
//! **Position:** inside `database_operations`; `cargo xtask db backup-drill` and `deploy db drill`
//! call [`run`]; it calls [`crate::backup`] and [`crate::restore`] in process and the container
//! layer for every query.
//! **Signals & state:** the scratch database, owned by a drop guard for the length of one run.
//! **Invariants:** only allow-listed scratch names are created or dropped; the source database is
//! read, never restored into; the scratch is dropped on every exit path unless kept on purpose.
//!
//! Restore-into-scratch recoverability proof. CREATE/DROP only allow-listed scratch DBs
//! (`tbd_drill_probe`, etc.). Live `tbd_reforger` is dump SOURCE only — never a restore target
//! (via `crate::container_database::refuse_unsafe_restore_target`).
//!
//! Calls `deploy_db_backup` / `deploy_db_restore` in-process.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::container_database::{
    count_db_rows, ct_capture, database_exists, db_user, info, refuse_unsafe_restore_target,
    require_container, require_pg_tool, warn,
};
use crate::error::{Result, stop};
use crate::restore::RestoreArgs;
use repository_root::find_repository_root;

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
#[path = "tests/restore_drill/tests.rs"]
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
