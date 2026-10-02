use super::cli::SetupCmd;
use anyhow::Result;

pub(crate) fn run(cmd: SetupCmd) -> Result<u8> {
    match cmd {
        SetupCmd::ServerProfile { profile } => {
            crate::commands::setup::server_profile::run(profile.as_deref())
        }
        SetupCmd::Workbench => crate::commands::setup::workbench_linux::run(),
        SetupCmd::McpGameRoot { game, fake } => {
            crate::commands::setup::mcp_game_root::run(game.as_deref(), fake.as_deref())
        }
        SetupCmd::ClientAddons => crate::commands::setup::client_addons::run(),
    }
}
