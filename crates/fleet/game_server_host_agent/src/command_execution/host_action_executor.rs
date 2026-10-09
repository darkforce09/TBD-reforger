//! Performing a validated command on this host.
//!
//! **Role:** Carries out each [`HostCommand`]: process actions through the systemd user manager,
//! the player list through the game server's RCON port, the operator's console line through RCON
//! in a single transmission, and a cross-terrain mission deployment through the dedicated
//! server's config and a restart.
//!
//! **Position:** `crate::ledger_client::command_loop` calls [`FleetActionExecutor::execute`] once
//! the ledger acknowledged the `executing` report, and reports the returned [`ActionVerdict`];
//! `main.rs` builds the [`HostActionExecutor`].
//!
//! **Signals & state:** none of its own; it holds handles to process control, the RCON session
//! task and the server config file.
//!
//! **Invariants:** each call performs its command once; the console line is never retransmitted,
//! and a line that gets no reply fails with "no RCON response; the command may or may not have
//! run".

use std::future::Future;

use super::console_line::ConsoleLine;
use super::console_response_capture::ConsoleResponseCapture;
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
    /// An executor that performs process actions through `process_control`, RCON reads and
    /// console lines through `rcon`, and mission switches on `server_config`.
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

    /// The line, transmitted once, its reply captured into `{"response", "response_truncated"}`.
    /// A failure reason is the RCON failure itself: a refused or unanswered login sent nothing,
    /// and a missing reply leaves open whether the line ran.
    async fn console_command(&self, line: &ConsoleLine) -> ActionVerdict {
        match self.rcon.execute_once(line.as_str()).await {
            Ok(reply) => ActionVerdict::success(ConsoleResponseCapture::of(&reply).into_outcome()),
            Err(error) => ActionVerdict::failure(&error.to_string(), None),
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
            HostCommand::ConsoleCommand(line) => self.console_command(line).await,
        }
    }
}
