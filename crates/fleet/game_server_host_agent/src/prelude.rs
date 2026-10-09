//! The names a caller of the game server host agent imports with
//! `use game_server_host_agent::prelude::*;`: the configuration, the command loop and its
//! executor, the three host channels (systemd, RCON, the server config), the typed ids and the
//! crate's error.

pub use crate::action_verdict::ActionVerdict;
pub use crate::agent_configuration::{AgentConfiguration, ConfigurationError};
pub use crate::command_execution::{
    CommandRefusal, FleetActionExecutor, HostActionExecutor, HostCommand,
};
pub use crate::dedicated_server_config::{DedicatedServerConfig, ServerConfigError};
pub use crate::error::{Error, Result};
pub use crate::identifiers::{ArmaPlayerId, ScenarioId, SessionPlayerId};
pub use crate::ledger_client::{
    ClaimOutcome, CommandLoop, LedgerApi, LedgerApiSetupError, LedgerError, LedgerTimings,
};
pub use crate::process_control::{
    ProcessAction, ProcessControl, ProcessControlSettings, SystemdUnitName, SystemdUnitNameProblem,
};
pub use crate::rcon::{RconClient, RconError, RconSettings, RconTimings};
pub use crate::secret_text::SecretText;
