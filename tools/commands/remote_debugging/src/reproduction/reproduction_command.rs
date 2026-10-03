//! The `cargo xtask repro` command line.
//!
//! **Role:** the [`ReproCmd`] clap enum: the upload orchestrator and its two helpers.
//! **Position:** mounted by `xtask`'s `cli` as the `repro` group; routed by
//! [`crate::reproduction::run`].
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant names one command.

use clap::Subcommand;
use std::path::PathBuf;

/// The `cargo xtask repro` subcommands.
#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)] // mission-id / mission-version-body / mission-upload
pub enum ReproCmd {
    /// stdin JSON → print .id
    #[command(name = "mission-id")]
    MissionId,
    /// Write padded mission-version POST body
    #[command(name = "mission-version-body")]
    MissionVersionBody {
        /// The file the body is written to.
        #[arg(long)]
        out: PathBuf,
        /// The `editor_notes` padding in MiB (at least 1).
        #[arg(long)]
        mb: u64,
        /// The version's `semver`.
        #[arg(long)]
        semver: String,
    },
    /// Orchestrate the mission-version upload reproduction
    #[command(name = "mission-upload")]
    MissionUpload,
}
