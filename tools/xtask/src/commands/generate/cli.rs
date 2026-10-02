use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum GenCmd {
    /// Spleen 16x32 BDF → a Rust font table on stdout
    #[command(name = "font-table")]
    FontTable { bdf: PathBuf },
}
