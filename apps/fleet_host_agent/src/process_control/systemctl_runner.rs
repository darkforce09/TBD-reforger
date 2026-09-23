//! Runs the configured systemctl program with a fixed argument vector: no shell, a cleared
//! environment in which only the variables that locate the user's systemd manager pass through,
//! no standard input, and a timeout after which the process is killed.

use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use thiserror::Error;
use tokio::process::Command;

/// The variables `systemctl --user` needs to reach the calling user's systemd manager.
const PASSED_ENVIRONMENT: [&str; 2] = ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"];

pub(super) struct ProgramOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

#[derive(Debug, Error)]
pub(super) enum ProgramFailure {
    #[error("could not be started: {0}")]
    NotStarted(std::io::Error),
    #[error("did not finish within {0:?}")]
    TimedOut(Duration),
}

pub(super) async fn run_systemctl(
    program: &Path,
    arguments: &[&str],
    timeout: Duration,
) -> Result<ProgramOutput, ProgramFailure> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for name in PASSED_ENVIRONMENT {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    // Dropping the unfinished `output` future at the timeout drops the child, which
    // `kill_on_drop` kills.
    match tokio::time::timeout(timeout, command.output()).await {
        Err(_elapsed) => Err(ProgramFailure::TimedOut(timeout)),
        Ok(Err(error)) => Err(ProgramFailure::NotStarted(error)),
        Ok(Ok(output)) => Ok(ProgramOutput {
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
        }),
    }
}
