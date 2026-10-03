use super::cli::WaveLockCmd;
use crate::*;
use anyhow::Result;
use repository_layout::find_repository_root;

pub(crate) fn run(cmd: WaveLockCmd) -> Result<u8> {
    {
        let root = find_repository_root()?;
        match cmd {
            WaveLockCmd::Repack { reserve } => {
                let ids: Vec<String> = reserve
                    .as_deref()
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                commands::wave::cmd_repack(&root, &ids)
            }
            WaveLockCmd::Check => commands::wave::cmd_check(&root),
        }
    }
}
