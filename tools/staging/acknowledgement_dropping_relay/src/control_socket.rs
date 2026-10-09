//! The relay's control socket: arm, disarm and status over a mode-600 Unix socket, and the client
//! the `control` command uses.
//!
//! - **Role:** [`ControlSocket::bind`] creates the socket and [`ControlSocket::serve`] answers one
//!   command per connection with the relay's status; [`send_control_command`] is the client.
//! - **Position:** under [`super`]; `serve` runs the socket beside the relay's listener, and
//!   `control` connects to it, on the staging host, as the relay's own user.
//! - **Signals & state:** the bound socket, whose file is removed when it is dropped; the commands
//!   act on the shared [`DropPolicy`].
//! - **Invariants:**
//!   - The socket file is mode 600, so only the relay's user can arm it.
//!   - A command is one line of at most [`MAXIMUM_COMMAND_BYTES`] bytes; the answer is one JSON
//!     line, `{"status": …}` or `{"error": "…"}`.
//!   - A socket a live relay answers on, and a path that is not a socket, are never replaced.

use std::fs;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::drop_policy::{DropPolicy, DropTarget, RelayStatus};
use crate::error::{Error, Result};
use crate::relay::RelayLog;

/// Longest command line the socket reads.
pub const MAXIMUM_COMMAND_BYTES: u64 = 256;
/// Longest answer the client reads.
const MAXIMUM_ANSWER_BYTES: u64 = 64 * 1024;
/// How long either side waits for the other.
const CONTROL_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(5);
/// The pause after a failed accept.
const ACCEPT_RETRY_PAUSE: Duration = Duration::from_millis(200);
/// The socket file's mode: read and write for the owner only.
const SOCKET_MODE: u32 = 0o600;

/// One control command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCommand {
    /// Withhold the next `200` answer the target names.
    Arm(DropTarget),
    /// Pass every answer through.
    Disarm,
    /// Report the arming, the counts and the last drop.
    Status,
}

impl ControlCommand {
    /// The command as the socket reads it, without its newline.
    pub fn line(self) -> String {
        match self {
            Self::Arm(target) => format!("arm {}", target.word()),
            Self::Disarm => "disarm".to_string(),
            Self::Status => "status".to_string(),
        }
    }

    /// The command `line` spells.
    ///
    /// # Errors
    ///
    /// `line` is not `arm <target>`, `disarm` or `status`.
    pub fn parse(line: &str) -> std::result::Result<Self, String> {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["arm", target] => DropTarget::from_word(target).map(Self::Arm).ok_or_else(|| {
                format!(
                    "`arm` takes {} or {}",
                    DropTarget::DropNextClaimResponse.word(),
                    DropTarget::DropNextResultResponse.word()
                )
            }),
            ["disarm"] => Ok(Self::Disarm),
            ["status"] => Ok(Self::Status),
            _ => Err("the commands are `arm <target>`, `disarm` and `status`".to_string()),
        }
    }
}

/// The one line the socket answers with.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ControlAnswer {
    Status(RelayStatus),
    Error(String),
}

/// The bound control socket; dropping it removes the socket file.
#[derive(Debug)]
pub(super) struct ControlSocket {
    listener: UnixListener,
    path: PathBuf,
}

impl ControlSocket {
    /// Create the socket at `path`, mode 600, replacing only a socket nobody answers on.
    ///
    /// # Errors
    ///
    /// A relay answers on `path`, something other than a socket is there, or the socket cannot be
    /// created or restricted.
    pub(super) fn bind(path: &Path) -> Result<Self> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                if std::os::unix::net::UnixStream::connect(path).is_ok() {
                    return Err(Error::refused(format!(
                        "a relay already answers on the control socket {}",
                        path.display()
                    )));
                }
                fs::remove_file(path).map_err(|error| {
                    Error::io(
                        format!("cannot remove the unanswered socket {}", path.display()),
                        error,
                    )
                })?;
            }
            Ok(_) => {
                return Err(Error::refused(format!(
                    "{} is not a socket; the relay leaves it in place and does not start",
                    path.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::io(
                    format!("cannot inspect {}", path.display()),
                    error,
                ));
            }
        }
        let listener = UnixListener::bind(path).map_err(|error| {
            Error::io(
                format!("cannot create the control socket {}", path.display()),
                error,
            )
        })?;
        let socket = Self {
            listener,
            path: path.to_path_buf(),
        };
        fs::set_permissions(path, fs::Permissions::from_mode(SOCKET_MODE)).map_err(|error| {
            Error::io(
                format!("cannot restrict {} to mode 600", path.display()),
                error,
            )
        })?;
        Ok(socket)
    }

    /// Answer commands until the returned future is dropped.
    pub(super) async fn serve(self, policy: Arc<DropPolicy>, log: RelayLog) {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(answer_one_command(stream, policy.clone(), log.clone()));
                }
                Err(error) => {
                    log.line(format!("the control socket could not accept: {error}"));
                    tokio::time::sleep(ACCEPT_RETRY_PAUSE).await;
                }
            }
        }
    }
}

impl Drop for ControlSocket {
    fn drop(&mut self) {
        // The file may already be gone with its runtime folder; nothing else is left to do.
        let _ = fs::remove_file(&self.path);
    }
}

async fn answer_one_command(stream: UnixStream, policy: Arc<DropPolicy>, log: RelayLog) {
    let (reader, mut writer) = stream.into_split();
    let mut line = String::new();
    let mut reader = BufReader::new(reader.take(MAXIMUM_COMMAND_BYTES));
    let read = tokio::time::timeout(CONTROL_EXCHANGE_TIMEOUT, reader.read_line(&mut line)).await;
    let answer = match read {
        Ok(Ok(_)) => match ControlCommand::parse(&line) {
            Ok(command) => ControlAnswer::Status(apply(command, &policy, &log)),
            Err(reason) => ControlAnswer::Error(reason),
        },
        Ok(Err(error)) => ControlAnswer::Error(format!("the command could not be read: {error}")),
        Err(_) => ControlAnswer::Error(format!(
            "no command arrived within {} s",
            CONTROL_EXCHANGE_TIMEOUT.as_secs()
        )),
    };
    let mut text = serde_json::to_string(&answer)
        .unwrap_or_else(|_| r#"{"error":"the status could not be encoded"}"#.to_string());
    text.push('\n');
    // A client that left before the answer has nothing left to read.
    let _ = writer.write_all(text.as_bytes()).await;
    let _ = writer.shutdown().await;
}

fn apply(command: ControlCommand, policy: &DropPolicy, log: &RelayLog) -> RelayStatus {
    match command {
        ControlCommand::Arm(target) => {
            log.line(format!("armed: {}", target.word()));
            policy.arm(target)
        }
        ControlCommand::Disarm => {
            log.line("disarmed");
            policy.disarm()
        }
        ControlCommand::Status => policy.status(),
    }
}

/// Send `command` to the relay whose control socket is `socket`, and return its status.
///
/// # Errors
///
/// No relay answers on `socket`, the relay does not answer within five seconds, its answer is not
/// a control answer, or it refuses the command.
pub fn send_control_command(socket: &Path, command: ControlCommand) -> Result<RelayStatus> {
    let mut stream = std::os::unix::net::UnixStream::connect(socket)
        .map_err(|error| Error::io(format!("no relay answers on {}", socket.display()), error))?;
    stream
        .set_read_timeout(Some(CONTROL_EXCHANGE_TIMEOUT))
        .map_err(Error::Transport)?;
    stream
        .set_write_timeout(Some(CONTROL_EXCHANGE_TIMEOUT))
        .map_err(Error::Transport)?;
    stream
        .write_all(format!("{}\n", command.line()).as_bytes())
        .map_err(Error::Transport)?;
    stream.shutdown(Shutdown::Write).map_err(Error::Transport)?;
    let mut answer = String::new();
    (&mut stream)
        .take(MAXIMUM_ANSWER_BYTES)
        .read_to_string(&mut answer)
        .map_err(|error| {
            Error::io(
                format!("the relay on {} did not answer", socket.display()),
                error,
            )
        })?;
    let answer: ControlAnswer =
        serde_json::from_str(answer.trim_end()).map_err(|error| Error::StatusUndecodable {
            socket: socket.display().to_string(),
            error,
        })?;
    match answer {
        ControlAnswer::Status(status) => Ok(status),
        ControlAnswer::Error(reason) => Err(Error::refused(format!(
            "the relay refused `{}`: {reason}",
            command.line()
        ))),
    }
}
