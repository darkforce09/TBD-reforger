//! Performing a validated command on this host: process actions through the systemd user
//! manager, the player list through the game server's RCON port, and a cross-terrain mission
//! deployment through the dedicated server's config and a restart.

use std::future::Future;

use super::host_command::HostCommand;
use super::mission_restart::restart_with_mission;
use crate::action_verdict::ActionVerdict;
use crate::dedicated_server_config::DedicatedServerConfig;
use crate::process_control::{ProcessAction, ProcessControl};
use crate::rcon::RconClient;
use crate::rcon::reforger_commands::{PLAYERS_COMMAND, parse_player_listing};

/// Performs validated commands. The command loop calls [`FleetActionExecutor::execute`] only
/// after the ledger acknowledged that the command's effect is starting.
pub trait FleetActionExecutor: Send + Sync {
    /// Performs `command` and returns what was observed.
    fn execute(&self, command: &HostCommand) -> impl Future<Output = ActionVerdict> + Send;
}

/// The game host's executor.
#[derive(Debug, Clone)]
pub struct HostActionExecutor {
    process_control: ProcessControl,
    rcon: RconClient,
    server_config: DedicatedServerConfig,
}

impl HostActionExecutor {
    pub fn new(
        process_control: ProcessControl,
        rcon: RconClient,
        server_config: DedicatedServerConfig,
    ) -> Self {
        Self {
            process_control,
            rcon,
            server_config,
        }
    }

    async fn perform(&self, action: ProcessAction) -> ActionVerdict {
        self.process_control.perform(action).await.verdict()
    }

    /// `#players`, read into `{"players": [...]}`.
    async fn list_players(&self) -> ActionVerdict {
        match self.rcon.execute(PLAYERS_COMMAND).await {
            Ok(response) => ActionVerdict::success(parse_player_listing(&response).into_outcome()),
            Err(error) => ActionVerdict::failure(
                &format!("{PLAYERS_COMMAND} over RCON failed: {error}"),
                None,
            ),
        }
    }
}

impl FleetActionExecutor for HostActionExecutor {
    async fn execute(&self, command: &HostCommand) -> ActionVerdict {
        match command {
            HostCommand::Start => self.perform(ProcessAction::Start).await,
            HostCommand::Stop => self.perform(ProcessAction::Stop).await,
            HostCommand::Restart => self.perform(ProcessAction::Restart).await,
            HostCommand::ListPlayers => self.list_players().await,
            HostCommand::RestartWithMission(deployment) => {
                restart_with_mission(&self.server_config, &self.process_control, deployment).await
            }
        }
    }
}
