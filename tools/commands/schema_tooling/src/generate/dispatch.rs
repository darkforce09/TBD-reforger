//! Runs a `gen` command.
use super::cli::GenCmd;
use crate::error::Result;

/// Runs `cmd` and returns the process exit code.
pub fn run_gen_command(cmd: GenCmd) -> Result<u8> {
    match cmd {
        GenCmd::FontTable { bdf } => super::font_table::gen_font_table(&bdf),
    }
}
