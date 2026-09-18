use super::cli::WaveLockCmd;
use crate::core::repository_root::find_repo_root;
use crate::*;
use anyhow::Result;

pub(crate) fn run(cmd: WaveLockCmd) -> Result<u8> {
    {
        let root = find_repo_root()?;
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
