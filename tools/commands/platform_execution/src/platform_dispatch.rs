//! The `cargo xtask platform` dispatcher.
//!
//! **Role:** [`run`] hands each [`PlatformCmd`] to its driver: the preflight, the slice worktree
//! lifecycle, the wave driver or the slice runner (which loads the ticket registry first).
//! **Position:** the crate's entry; the xtask binary's `platform` arm calls it with the parsed
//! command.
//! **Signals & state:** none of its own; the drivers it calls own theirs.
//! **Invariants:** a driver's exit code is returned unchanged; `slice-run` exits 0 only when the
//! run wrote its receipt (or was a dry run).

use repository_root::find_repository_root;
use ticket_registry::load_registry;

use crate::Result;
use crate::platform_command::PlatformCmd;

/// Run one `cargo xtask platform` command and return its exit code.
pub fn run(cmd: PlatformCmd) -> Result<u8> {
    match cmd {
        PlatformCmd::Preflight { warn } => crate::preflight::run(warn),
        PlatformCmd::SliceWorktree { args } => crate::slice_worktree::run(&args),
        PlatformCmd::Wave { args } => crate::wave_execution::run(&args),
        PlatformCmd::SliceRun {
            id,
            fixture,
            started,
            dry_run,
        } => {
            let root = find_repository_root()?;
            let reg = load_registry(&root)?;
            let opts = crate::slice_execution::SliceRunOpts {
                fixture,
                started,
                agent_cmd_override: None,
                dry_run,
            };
            crate::slice_execution::run_slice(&root, &reg, &id, &opts)?;
            Ok(0)
        }
    }
}
