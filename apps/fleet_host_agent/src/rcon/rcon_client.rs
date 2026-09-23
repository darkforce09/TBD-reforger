//! The handle through which the agent sends RCON commands. One background task owns the UDP
//! socket and the session ([`super::rcon_session`]); commands queue on a channel and run one
//! at a time.
//!
//! Delivery is at least once. Within a session an unanswered command is retransmitted under
//! its sequence number, and the protocol does not say whether the server runs a retransmitted
//! command once or again; after a lost session the command is sent once more following a new
//! login. The client therefore carries only commands that are safe to repeat: the agent sends
//! the `#players` read alone.

use std::net::SocketAddr;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use super::rcon_session::{RconSession, SessionRequest};
use crate::secret_text::SecretText;

/// Longest command text sent in one packet.
pub const MAX_COMMAND_BYTES: usize = 1024;

/// Commands waiting while the session is busy with another one.
const QUEUED_COMMANDS: usize = 16;

#[derive(Debug, Clone)]
pub struct RconSettings {
    /// The game server's RCON address and port.
    pub server: SocketAddr,
    pub password: SecretText,
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
    /// One command, login included, takes at most 16 s even when the session has to be
    /// re-established, inside the ledger's 30 s execution window for RCON actions.
    fn default() -> Self {
        Self {
            response_timeout: Duration::from_secs(1),
            transmission_attempts: 4,
            keep_alive_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RconError {
    #[error("the RCON server refused the password")]
    LoginRejected,
    #[error(
        "the RCON server at {0} did not answer the login (RCON disabled, wrong address or port, \
         or the game server is not running)"
    )]
    LoginUnanswered(SocketAddr),
    #[error("the RCON server stopped answering; the command may or may not have run")]
    NoResponse,
    #[error(
        "the RCON command is empty, longer than {max} bytes, or contains control characters",
        max = MAX_COMMAND_BYTES
    )]
    InvalidCommand,
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
    /// command, which logs in.
    pub async fn start(settings: RconSettings) -> std::io::Result<Self> {
        let session = RconSession::open(settings).await?;
        let (requests, queue) = mpsc::channel(QUEUED_COMMANDS);
        tokio::spawn(session.run(queue));
        Ok(Self { requests })
    }

    /// Sends `command` and returns the server's response text.
    pub async fn execute(&self, command: &str) -> Result<String, RconError> {
        if command.is_empty()
            || command.len() > MAX_COMMAND_BYTES
            || command.chars().any(char::is_control)
        {
            return Err(RconError::InvalidCommand);
        }
        let (reply, response) = oneshot::channel();
        self.requests
            .send(SessionRequest::Execute {
                command: command.to_owned(),
                reply,
            })
            .await
            .map_err(|_| RconError::ClientStopped)?;
        response.await.map_err(|_| RconError::ClientStopped)?
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
}
