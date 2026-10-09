//! The handle through which the agent sends RCON commands.
//!
//! **Role:** Queues each command for the one session task ([`super::rcon_session`]) that owns
//! the UDP socket, in one of two deliveries. [`RconClient::execute`] delivers at least once:
//! within a session an unanswered command is retransmitted under its sequence number, and the
//! protocol does not say whether the server runs a retransmitted command once or again; after a
//! lost session the command is sent once more following a new login. [`RconClient::execute_once`]
//! transmits the command in exactly one packet and never again, not even after a new login; only
//! the login, and the empty command packet that confirms it, are repeated.
//!
//! **Position:** `main.rs` starts the client from the [`RconSettings`] that
//! `crate::agent_configuration` builds; `crate::command_execution` sends the `#players` read
//! through `execute` and the operator's console line through `execute_once`; `main.rs` calls
//! [`RconClient::log_out`] on shutdown.
//!
//! **Signals & state:** the sending half of the session task's request channel, 16 commands
//! deep; the session task owns every other piece of state.
//!
//! **Invariants:** a command is 1 to [`MAX_COMMAND_BYTES`] bytes of text without control
//! characters, or it is refused before anything is sent; `execute` carries only commands that are
//! safe to repeat; a command whose last transmission goes unanswered fails with
//! [`RconError::NoResponse`], because the server may or may not have run it.

use std::net::SocketAddr;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use super::rcon_session::{CommandReply, RconSession, SessionRequest};
use crate::secret_text::SecretText;

/// Longest command text sent in one packet.
pub const MAX_COMMAND_BYTES: usize = 1024;

/// Commands waiting while the session is busy with another one.
const QUEUED_COMMANDS: usize = 16;

/// Where the game server's RCON port listens, its password and the protocol timings.
#[derive(Debug, Clone)]
pub struct RconSettings {
    /// The game server's RCON address and port.
    pub server: SocketAddr,
    /// The server config's `rcon.password`, exposed only in the login packet.
    pub password: SecretText,
    /// The keep-alive, retransmission and answer timings.
    pub timings: RconTimings,
}

/// Protocol timings. The server de-authenticates a client that sends no command packet for 45
/// seconds, so the keep-alive interval stays well below that even when the keep-alive itself
/// has to be retransmitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RconTimings {
    /// How long a packet waits for its answer before it is sent again.
    pub response_timeout: Duration,
    /// Transmissions of one packet before the server counts as not answering.
    pub transmission_attempts: u32,
    /// Idle time after which an empty command packet keeps the login alive.
    pub keep_alive_interval: Duration,
}

impl Default for RconTimings {
    /// One `execute`, login included, takes at most 16 s even when the session has to be
    /// re-established; one `execute_once` takes at most 12 s: 4 s to confirm a held login, 4 s
    /// to log in again, and 4 s of waiting for the answer to its single transmission. Both fit
    /// inside the ledger's 30 s execution window for RCON actions.
    fn default() -> Self {
        Self {
            response_timeout: Duration::from_secs(1),
            transmission_attempts: 4,
            keep_alive_interval: Duration::from_secs(30),
        }
    }
}

/// Why a command produced no response text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RconError {
    /// The server refused the login; the command was not sent.
    #[error("the RCON server refused the password")]
    LoginRejected,
    /// No transmission of the login was answered; the command was not sent.
    #[error(
        "the RCON server at {0} did not answer the login (RCON disabled, wrong address or port, \
         or the game server is not running)"
    )]
    LoginUnanswered(SocketAddr),
    /// The command's last transmission went unanswered: the server may or may not have run it.
    #[error("no RCON response; the command may or may not have run")]
    NoResponse,
    /// The command cannot be one packet of text; nothing was sent.
    #[error(
        "the RCON command is empty, longer than {max} bytes, or contains control characters",
        max = MAX_COMMAND_BYTES
    )]
    InvalidCommand,
    /// The session task has stopped.
    #[error("the RCON client has stopped")]
    ClientStopped,
}

/// A cloneable handle to the RCON session of one game server.
#[derive(Debug, Clone)]
pub struct RconClient {
    requests: mpsc::Sender<SessionRequest>,
}

impl RconClient {
    /// Binds the local UDP socket and starts the session task. Nothing is sent until the first
    /// command, which logs in. A socket that cannot be bound or connected is
    /// [`crate::Error::RconSocket`].
    pub async fn start(settings: RconSettings) -> crate::Result<Self> {
        let session = RconSession::open(settings)
            .await
            .map_err(crate::Error::RconSocket)?;
        let (requests, queue) = mpsc::channel(QUEUED_COMMANDS);
        tokio::spawn(session.run(queue));
        Ok(Self { requests })
    }

    /// Sends `command`, which must be safe to repeat, and returns the server's response text.
    /// An unanswered command is retransmitted, and after a lost session sent once more
    /// following a new login.
    pub async fn execute(&self, command: &str) -> Result<String, RconError> {
        let command = packet_text(command)?;
        self.submit(|reply| SessionRequest::Execute { command, reply })
            .await
    }

    /// Sends `command` in exactly one transmission and returns the server's response text. The
    /// login is confirmed, or renewed, before the command leaves; the command itself is never
    /// sent again, and without its complete response it fails with [`RconError::NoResponse`].
    pub async fn execute_once(&self, command: &str) -> Result<String, RconError> {
        let command = packet_text(command)?;
        self.submit(|reply| SessionRequest::ExecuteOnce { command, reply })
            .await
    }

    /// Ends the session: a logged-in session sends `@logout`, which frees its slot on the server
    /// at once, and the session task stops.
    pub async fn log_out(&self) {
        let (reply, done) = oneshot::channel();
        if self
            .requests
            .send(SessionRequest::LogOut { reply })
            .await
            .is_ok()
        {
            // The session answers once the logout packet is sent, or drops the reply if it has
            // already stopped; either way nothing is left to wait for.
            let _ = done.await;
        }
    }

    /// Queues the request `build` makes around the reply channel and waits for its answer.
    async fn submit(
        &self,
        build: impl FnOnce(CommandReply) -> SessionRequest,
    ) -> Result<String, RconError> {
        let (reply, response) = oneshot::channel();
        self.requests
            .send(build(reply))
            .await
            .map_err(|_| RconError::ClientStopped)?;
        response.await.map_err(|_| RconError::ClientStopped)?
    }
}

/// `command` as the text of one command packet: 1 to [`MAX_COMMAND_BYTES`] bytes without control
/// characters.
fn packet_text(command: &str) -> Result<String, RconError> {
    if command.is_empty()
        || command.len() > MAX_COMMAND_BYTES
        || command.chars().any(char::is_control)
    {
        return Err(RconError::InvalidCommand);
    }
    Ok(command.to_owned())
}
