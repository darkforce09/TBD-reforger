//! The database container layer and `cargo xtask deploy db …`.
//!
//! **Role:** the one container runtime resolution, container exec, scratch-name allow-list and
//! dump verifier that the backup, the restore, the drill and the local database lane share, so
//! three callers cannot invent three dump verifiers; [`DeployDbCmd`] and [`run`] are the
//! `deploy db` verbs.
//! **Position:** inside `database_operations`; the deployment crate's `deploy` dispatch calls
//! [`run`], and [`crate::backup`], [`crate::restore`], [`crate::restore_drill`] and
//! [`crate::local_database`] call the helpers.
//! **Signals & state:** none; every call resolves the runtime and runs its child afresh.
//! **Invariants:** a missing runtime, container or tool is an operator stop (`FATAL:`), never a
//! reported success; a dump is vouched for only when all five checks hold.
//!
//! ── Closed fail-opens (measured in the bash header, preserved here) ─────────────────────────
//!
//! - `pg_restore --list` alone is NOT verification — TOC lives at the head; truncated /
//!   mid-file-corrupt dumps still pass `--list`. Check 5 runs `--data-only` and counts COPY rows.
//! - Identity: `dbname:` header + `_sqlx_migrations` TOC entry before the body read.
//! - The scratch allow-list refuses `tbd_reforger` unless `--confirm` spells the name twice.
//!
//! `_sqlx_migrations` probes use `verification_core::gate::probe_str`.
//!
//! The three verbs call this module from Rust directly; there is no shell bridge.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use process_runner::Run;
use verification_core::Pattern;
use verification_core::gate;

use crate::error::{Error, Result, ResultExt, refuse, stop};

/// Subcommands under `cargo xtask deploy db`.
#[derive(Subcommand, Debug)]
pub enum DeployDbCmd {
    /// Restore-target guard (refuses the live `tbd_reforger` database by default).
    #[command(name = "refuse-unsafe")]
    RefuseUnsafe {
        /// The restore target to judge.
        #[arg(long = "db")]
        db: String,
        /// The target's name again, to pass a name off the allow-list.
        #[arg(long = "confirm")]
        confirm: Option<String>,
    },
    /// Parse a single ASCII database name out of a postgres URL.
    #[command(name = "database-name-from-url")]
    DatabaseNameFromUrl {
        /// The `postgres://` URL.
        url: String,
    },
    /// Exit 0 if the name is an allow-listed scratch DB.
    #[command(name = "is-safe-scratch")]
    IsSafeScratch {
        /// The database name to judge.
        #[arg(long = "db")]
        db: String,
    },
    /// Fail closed unless the compose container is running.
    #[command(name = "require-container")]
    RequireContainer,
    /// Print the path of a postgres tool inside the container, or die.
    #[command(name = "require-pg-tool")]
    RequirePgTool {
        /// The tool's name, such as `pg_dump`.
        tool: String,
    },
    /// Exit 0 if the database exists.
    #[command(name = "database-exists")]
    DatabaseExists {
        /// The database name to look up.
        #[arg(long = "db")]
        db: String,
    },
    /// Exact live row count across user tables (not `reltuples`).
    #[command(name = "count-rows")]
    CountRows {
        /// The database whose rows are counted.
        #[arg(long = "db")]
        db: String,
    },
    /// Five-check dump verifier; prints row count on success.
    #[command(name = "verify-dump")]
    VerifyDump {
        /// The custom-format dump file.
        #[arg(long = "file")]
        file: PathBuf,
        /// The fewest data rows the dump may hold.
        #[arg(long = "min-rows", default_value_t = 1)]
        min_rows: u64,
        /// Empty string skips the identity check (and says so on stderr).
        #[arg(long = "expect-db", default_value = "")]
        expect_db: String,
    },
    /// Verified pg_dump + retention prune.
    #[command(name = "backup", disable_help_flag = true)]
    Backup {
        /// The backup's own arguments, passed through.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// `runtime exec $CONTAINER …` (no TTY — binary-safe).
    #[command(name = "ct", disable_help_flag = true)]
    Ct {
        /// The command run inside the container.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// `runtime exec -i $CONTAINER …` (stdin inherited).
    #[command(name = "ct-i", disable_help_flag = true)]
    CtI {
        /// The command run inside the container.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Guarded pg_restore.
    #[command(name = "restore")]
    Restore(crate::restore::RestoreArgs),
    /// Restore-into-scratch recoverability proof.
    #[command(name = "drill", disable_help_flag = true)]
    Drill {
        /// The drill's own arguments, passed through.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

// ─────────────────────────── messaging (bash die / info / warn) ───────────────────────────

// ─────────────────────────── container runtime ───────────────────────────

// ─────────────────────────── restore target guard ───────────────────────────

// ─────────────────────────── dump verification ───────────────────────────

/// A dump that failed one of the five checks; the reason is already on stderr.
pub(crate) struct VerifyFail;

/// Why the five checks stopped: a failed check, or an operator stop from the container layer.
pub(crate) enum DumpCheckFailure {
    /// A check failed; its `VERIFY FAIL:` lines are already on stderr.
    Rejected,
    /// The container layer stopped the verb (no runtime): nothing was checked.
    Stopped(Error),
}

impl From<VerifyFail> for DumpCheckFailure {
    fn from(_: VerifyFail) -> Self {
        DumpCheckFailure::Rejected
    }
}

// ─────────────────────────── bash bridge ───────────────────────────

#[cfg(test)]
#[path = "tests/container_database/tests.rs"]
mod tests;

mod execution;
pub(crate) use execution::ct_capture;
pub(crate) use execution::ct_i_stdin_capture;
pub(crate) use execution::ct_i_to_files;
pub use execution::database_name_from_url;
pub(crate) use execution::db_container;
pub(crate) use execution::db_user;
pub(crate) use execution::info;
pub use execution::is_safe_scratch_database_name;
pub(crate) use execution::refuse_unsafe_restore_target;
pub(crate) use execution::require_container;
pub(crate) use execution::require_pg_tool;
pub(crate) use execution::resolve_runtime;
pub use execution::run;
pub(crate) use execution::warn;

mod verify_dump;
pub(crate) use verify_dump::count_db_rows;
pub(crate) use verify_dump::database_exists;
pub(crate) use verify_dump::verify_dump;

#[cfg(test)]
use verify_dump::count_copy_rows;
