//! The `gen` command group's arguments.
use clap::Subcommand;
use std::path::PathBuf;

/// `cargo xtask gen <command>`.
#[derive(Subcommand, Debug)]
pub enum GenCmd {
    /// Spleen 16x32 BDF → a Rust font table on stdout
    #[command(name = "font-table")]
    FontTable {
        /// The Spleen 16x32 BDF file to read
        bdf: PathBuf,
    },
}
