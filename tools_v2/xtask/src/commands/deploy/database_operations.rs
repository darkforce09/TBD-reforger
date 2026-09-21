//! Shared plumbing for `cargo xtask deploy db …`.
//!
//! Shared plumbing for backup-db / restore-db / backup-drill. Ported
//! FIRST so three callers cannot invent three dump-verifiers. Same propagation argument as
//! `tools_v2/verification-core`.
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
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use verification_core::Pattern;
use verification_core::gate;

/// Subcommands under `cargo xtask deploy db`.
#[derive(Subcommand, Debug)]
pub enum DeployDbCmd {
    /// Restore-target guard (refuses the live `tbd_reforger` database by default).
    #[command(name = "refuse-unsafe")]
    RefuseUnsafe {
        #[arg(long = "db")]
        db: String,
        #[arg(long = "confirm")]
        confirm: Option<String>,
    },
    /// Parse a single ASCII database name out of a postgres URL.
    #[command(name = "database-name-from-url")]
    DatabaseNameFromUrl { url: String },
    /// Exit 0 if the name is an allow-listed scratch DB.
    #[command(name = "is-safe-scratch")]
    IsSafeScratch {
        #[arg(long = "db")]
        db: String,
    },
    /// Fail closed unless the compose container is running.
    #[command(name = "require-container")]
    RequireContainer,
    /// Print the path of a postgres tool inside the container, or die.
    #[command(name = "require-pg-tool")]
    RequirePgTool { tool: String },
    /// Exit 0 if the database exists.
    #[command(name = "database-exists")]
    DatabaseExists {
        #[arg(long = "db")]
        db: String,
    },
    /// Exact live row count across user tables (not `reltuples`).
    #[command(name = "count-rows")]
    CountRows {
        #[arg(long = "db")]
        db: String,
    },
    /// Five-check dump verifier; prints row count on success.
    #[command(name = "verify-dump")]
    VerifyDump {
        #[arg(long = "file")]
        file: PathBuf,
        #[arg(long = "min-rows", default_value_t = 1)]
        min_rows: u64,
        /// Empty string skips the identity check (and says so on stderr).
        #[arg(long = "expect-db", default_value = "")]
        expect_db: String,
    },
    /// Verified pg_dump + retention prune.
    #[command(name = "backup", disable_help_flag = true)]
    Backup {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// `runtime exec $CONTAINER …` (no TTY — binary-safe).
    #[command(name = "ct", disable_help_flag = true)]
    Ct {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// `runtime exec -i $CONTAINER …` (stdin inherited).
    #[command(name = "ct-i", disable_help_flag = true)]
    CtI {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Guarded pg_restore.
    #[command(name = "restore")]
    Restore(crate::commands::deploy::database_restore::RestoreArgs),
    /// Restore-into-scratch recoverability proof.
    #[command(name = "drill", disable_help_flag = true)]
    Drill {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

// ─────────────────────────── messaging (bash die / info / warn) ───────────────────────────

// ─────────────────────────── container runtime ───────────────────────────

// ─────────────────────────── restore target guard ───────────────────────────

// ─────────────────────────── dump verification ───────────────────────────

pub(crate) struct VerifyFail;

// ─────────────────────────── bash bridge ───────────────────────────

#[cfg(test)]
#[path = "tests/database_operations/tests.rs"]
mod tests;

mod execution;
pub(crate) use execution::ct_capture;
pub(crate) use execution::ct_i_stdin_capture;
pub(crate) use execution::ct_i_to_files;
pub use execution::database_name_from_url;
pub(crate) use execution::db_container;
pub(crate) use execution::db_user;
pub(crate) use execution::die;
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
