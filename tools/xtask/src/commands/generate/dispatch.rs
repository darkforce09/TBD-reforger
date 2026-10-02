use super::cli::GenCmd;
use anyhow::Result;

pub(crate) fn run(cmd: GenCmd) -> Result<u8> {
    {
        let code = match cmd {
            GenCmd::FontTable { bdf } => {
                crate::verifications::language_bans::node_and_file_limits::gen_font_table(&bdf)?
            }
        };
        Ok(code)
    }
}
