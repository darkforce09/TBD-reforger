use super::cli::PlatformCmd;
use crate::commands::ticket::*;
use crate::core::repository_root::find_repo_root;
use anyhow::Result;

pub(crate) fn run(cmd: PlatformCmd) -> Result<u8> {
    match cmd {
        PlatformCmd::Preflight { warn } => crate::commands::platform::preflight::run(warn),
        PlatformCmd::SliceWorktree { args } => {
            crate::commands::platform::slice_worktree::run(&args)
        }
        PlatformCmd::Wave { args } => crate::commands::platform::wave_execution::run(&args),
        PlatformCmd::SliceRun {
            id,
            fixture,
            started,
            dry_run,
        } => {
            let root = find_repo_root()?;
            let reg = load_registry(&root)?;
            let opts = crate::commands::platform::slice_execution::SliceRunOpts {
                fixture,
                started,
                agent_cmd_override: None,
                dry_run,
            };
            crate::commands::platform::slice_execution::run_slice(&root, &reg, &id, &opts)?;
            Ok(0)
        }
    }
}
