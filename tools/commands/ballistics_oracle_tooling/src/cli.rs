//! The `ballistics` command group's arguments.
use clap::Subcommand;
use std::path::PathBuf;

/// `cargo xtask ballistics <command>`.
#[derive(Subcommand, Debug)]
pub enum BallisticsCmd {
    /// Write the vanilla mortar ballistics catalog, its calibration bundle and the refused
    /// bundles from one gameplay export generation and the ballistics oracle's output for it
    #[command(name = "trim-export")]
    TrimExport {
        /// Gameplay export generation id (16 uppercase hexadecimal digits)
        #[arg(long)]
        generation: String,
        /// Oracle output folder; defaults to `assets/scratch/ballistics_oracle/<generation>`.
        /// The explicit `help` keeps the printed text free of the code-span backticks.
        #[arg(
            long,
            help = "Oracle output folder; defaults to assets/scratch/ballistics_oracle/<generation>"
        )]
        oracle: Option<PathBuf>,
    },
}
