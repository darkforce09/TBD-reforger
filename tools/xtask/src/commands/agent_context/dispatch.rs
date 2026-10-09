//! Routes a `cargo xtask ai` command to its entry point.
//!
//! **Role:** runs `guard` or `run` and returns its process exit code.
//!
//! **Position:** called by [`crate::cli::dispatch`]; hands the work to the
//! `agent_context_guards` crate ([`agent_context_guards::run_tool_call_guard`],
//! [`agent_context_guards::run_filtered_command`]).
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the exit code is the entry point's own: the guard's 0 or 2, the filtered
//! command's exit code.

use super::cli::AiCmd;
use agent_context_guards::prelude::*;
use anyhow::Result;

/// Run `cmd` and return its process exit code.
pub(crate) fn run(cmd: AiCmd) -> Result<u8> {
    match cmd {
        AiCmd::Guard => Ok(run_tool_call_guard()),
        AiCmd::Run { args } => Ok(run_filtered_command(&args)?),
    }
}
