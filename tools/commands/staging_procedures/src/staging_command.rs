//! The `cargo xtask staging` command line.
//!
//! **Role:** the clap declarations of every staging subcommand: the read-only commands, the
//! confirmed host actions and the recorded runs.
//!
//! **Position:** the xtask binary embeds [`StagingCmd`] as `TopCmd::Staging` in
//! `tools/xtask/src/cli/mod.rs`; parsed values go to [`crate::run`] in `staging_dispatch.rs`.
//!
//! **Signals & state:** none; parsed values.
//!
//! **Invariants:** a recorded run needs its explicit `--record` switch (and the load run its
//! token file); `--rehearse-local` excludes `--record`; `--cases` excludes `--recovery`; a
//! rotation is exactly one of `--stage` and `--promote`.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::remote_actions::host_fixture_commands::CredentialExecutor;

/// The three procedures: `Fleet` records `staging_fleet`, `Discord` `staging_discord` and `Load`
/// `staging_load`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
#[allow(
    missing_docs,
    reason = "clap turns a doc comment on a value into per-value help, which changes `--help`"
)]
pub enum ProcedureName {
    Fleet,
    Discord,
    Load,
}

/// The flag every confirmed host action takes.
#[derive(Args, Debug)]
pub struct PlanOnly {
    /// Print the command the host would run instead of running it
    #[arg(long)]
    pub dry_run: bool,
}

/// The switch of a recorded run.
#[derive(Args, Debug)]
pub struct RecordSwitch {
    /// Run the procedure and write its receipt into target/staging/receipts/
    #[arg(long, required = true)]
    pub record: bool,
}

/// The subcommands of `cargo xtask staging`.
#[derive(Subcommand, Debug)]
pub enum StagingCmd {
    /// Check the preconditions of the fleet and load procedures without changing anything
    Preflight {
        /// Also check the Discord procedure's preconditions
        #[arg(long)]
        discord: bool,
    },
    /// Print the host's resting state and setup content, each beside its expected value
    Status {
        /// Print the host's load, memory and each fleet unit's memory and CPU instead
        #[arg(long)]
        capacity: bool,
    },
    /// Print a procedure's numbered real actions for the operator's approval
    #[command(name = "action-list")]
    ActionList {
        /// The procedure
        #[arg(value_enum)]
        procedure: ProcedureName,
        /// Print the declared cases instead
        #[arg(long, conflicts_with = "recovery")]
        cases: bool,
        /// Print the actions that put the host back after a stopped run instead
        #[arg(long)]
        recovery: bool,
    },
    /// Take a verified pg_dump -Fc of tbd_reforger under ~/tbd/backups/<date>/ on the host
    #[allow(
        rustdoc::invalid_html_tags,
        reason = "this doc comment is the subcommand's `--help` text, where `<date>` is a placeholder"
    )]
    Backup {
        /// The backup's label, [a-z0-9-]+
        #[arg(long)]
        label: String,
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Update and validate the Experimental dedicated server (app 1890870) at TBD_SERVER_DIR
    #[command(name = "update-game-server")]
    UpdateGameServer {
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Register the fleet's servers and write their credentials on the host
    #[command(name = "provision-fleet")]
    ProvisionFleet {
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Stage or promote a new machine credential of one instance
    #[command(name = "rotate-credential")]
    RotateCredential {
        /// The instance number
        #[arg(long)]
        instance: u16,
        /// Whose credential
        #[arg(long, value_enum)]
        executor: CredentialExecutor,
        /// Issue the new credential into the staged file
        #[arg(long, conflicts_with = "promote", required_unless_present = "promote")]
        stage: bool,
        /// Move the staged credential over the live one
        #[arg(long)]
        promote: bool,
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Seed the synthetic load population and the [Load fixture] events on the host
    #[command(name = "seed-load")]
    SeedLoad {
        /// Where the accounts' refresh tokens go on this workstation (created, mode 600)
        #[arg(long, value_name = "PATH")]
        token_file: PathBuf,
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Delete the synthetic load population and the [Load fixture] events on the host
    #[command(name = "clean-load")]
    CleanLoad {
        /// The `--dry-run` switch.
        #[command(flatten)]
        plan: PlanOnly,
    },
    /// Record the staging_fleet receipt
    Fleet {
        /// The `--record` switch.
        #[command(flatten)]
        run: RecordSwitch,
    },
    /// Record the staging_discord receipt
    Discord {
        /// The `--record` switch.
        #[command(flatten)]
        run: RecordSwitch,
    },
    /// Record the staging_load receipt, or rehearse the load path locally
    Load {
        /// Run the load procedure and write its receipt into target/staging/receipts/
        #[arg(
            long,
            requires = "token_file",
            conflicts_with = "rehearse_local",
            required_unless_present = "rehearse_local"
        )]
        record: bool,
        /// The refresh token file the recorded run reads once
        #[arg(long, value_name = "PATH")]
        token_file: Option<PathBuf>,
        /// Exercise the load path against the local stack; records nothing
        #[arg(long)]
        rehearse_local: bool,
    },
}
