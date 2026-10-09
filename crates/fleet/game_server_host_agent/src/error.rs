//! Why a step of the game server host agent fails.
//!
//! **Role:** the crate's one error type and its `Result` alias, gathering the error of every
//! fallible family of the public API: the configuration load, the re-validation of a claimed
//! command, the server config switch, the systemd unit name, the RCON socket and its commands,
//! and the ledger client's setup and requests.
//! **Position:** converted into with `?` from each family's own error; each module returns its
//! family's error, because its callers branch on the variants (a transient ledger failure is
//! retried, a refusal becomes the command's failure reason, an unanswered console line is
//! reported as possibly run), and a caller that only reports a failure holds this type.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant wraps its family's error; the message and the source are the
//! wrapped error's own, except a systemd unit name problem, a phrase that follows the value, which
//! the message prefixes with what it is about. No variant quotes a secret.

use crate::agent_configuration::ConfigurationError;
use crate::command_execution::CommandRefusal;
use crate::dedicated_server_config::ServerConfigError;
use crate::ledger_client::{LedgerApiSetupError, LedgerError};
use crate::process_control::SystemdUnitNameProblem;
use crate::rcon::RconError;

/// Why the game server host agent cannot start or a step of a command fails.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The configuration file or one of its secret files is refused.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),

    /// A claimed command is refused without acting.
    #[error(transparent)]
    CommandRefused(#[from] CommandRefusal),

    /// The server config cannot be switched to another scenario; the file is unchanged.
    #[error(transparent)]
    ServerConfig(#[from] ServerConfigError),

    /// A systemd unit name is not a valid service unit name.
    #[error("the systemd unit name {0}")]
    SystemdUnitName(#[from] SystemdUnitNameProblem),

    /// The local UDP socket for the game server's RCON port cannot be opened.
    #[error(transparent)]
    RconSocket(std::io::Error),

    /// An RCON command produced no response text.
    #[error(transparent)]
    Rcon(#[from] RconError),

    /// The HTTP client of the command ledger cannot be set up.
    #[error(transparent)]
    LedgerApiSetup(#[from] LedgerApiSetupError),

    /// A command ledger request failed.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
