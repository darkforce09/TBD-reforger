//! The `ballistics` command group's arguments.
use clap::Subcommand;
use std::path::PathBuf;

/// `cargo xtask ballistics <command>`.
#[derive(Subcommand, Debug)]
pub(crate) enum BallisticsCmd {
    /// Write the vanilla mortar ballistics catalog, its calibration bundle and the refused
    /// bundles from one gameplay export generation and the ballistics oracle's output for it
    #[command(name = "trim-export")]
    TrimExport {
        /// Gameplay export generation id (16 uppercase hexadecimal digits)
        #[arg(long)]
        generation: String,
        /// Oracle output folder; defaults to assets_v2/scratch/ballistics_oracle/<generation>
        #[arg(long)]
        oracle: Option<PathBuf>,
    },
}
