use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum GenCmd {
    /// Spleen 16x32 BDF → text_font_table.rs on stdout (gen-text-font-table.mjs port)
    #[command(name = "font-table")]
    FontTable { bdf: PathBuf },
}
