use super::cli::AiCmd;
use anyhow::Result;

pub(crate) fn run(cmd: AiCmd) -> Result<u8> {
    match cmd {
        AiCmd::Guard => Ok(crate::commands::agent_context::guards::cmd_guard()),
        AiCmd::Run { args } => crate::commands::agent_context::guards::cmd_run(&args),
    }
}
