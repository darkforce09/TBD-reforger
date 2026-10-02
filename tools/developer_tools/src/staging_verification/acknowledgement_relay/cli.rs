//! The command line of the `acknowledgement-dropping-relay` executable.
//!
//! - **Role:** parses `serve` and `control`, runs them, and maps the outcome to the exit code: 0
//!   done, 1 refused or failed, 2 a usage error (clap's).
//! - **Position:** `src/bin/acknowledgement_dropping_relay.rs` calls [`entrypoint`]; the unit
//!   `acknowledgement-dropping-relay@N` runs `serve`, and the staging fleet procedure runs `control`
//!   on the staging host.
//! - **Signals & state:** none beyond the command it runs.
//! - **Invariants:** `control` prints the relay's status as one JSON line on standard output and
//!   nothing else; every error goes to standard error.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

use super::control_socket::{ControlCommand, send_control_command};
use super::drop_policy::DropTarget;
use super::relay::{RelayLog, serve};
use super::relay_settings::RelaySettings;

/// A loopback relay between one fleet host agent and the API that, when armed, withholds one
/// executor answer past the agent's request timeout and then closes its connection.
#[derive(Debug, Parser)]
#[command(name = "acknowledgement-dropping-relay")]
struct RelayCommandLine {
    #[command(subcommand)]
    command: RelayCommand,
}

#[derive(Debug, Subcommand)]
enum RelayCommand {
    /// Relay every exchange between a host agent and the API until SIGTERM or SIGINT.
    Serve {
        /// Loopback address and port to listen on, such as 127.0.0.1:18085.
        #[arg(long)]
        listen: String,
        /// The API's http origin on a loopback host, such as http://127.0.0.1:8080.
        #[arg(long)]
        upstream: String,
        /// Path of the control socket to create, mode 600.
        #[arg(long)]
        control_socket: PathBuf,
    },
    /// Arm, disarm or read a running relay, and print its status as one JSON line.
    Control {
        /// Path of the running relay's control socket.
        #[arg(long)]
        control_socket: PathBuf,
        #[command(subcommand)]
        request: ControlRequest,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
enum ControlRequest {
    /// Withhold the next 200 answer of this kind; a 204 claim answer never counts.
    Arm {
        #[arg(value_enum)]
        target: DropTarget,
    },
    /// Pass every answer through.
    Disarm,
    /// Print the arming, the counts and the last withheld answer.
    Status,
}

impl From<ControlRequest> for ControlCommand {
    fn from(request: ControlRequest) -> Self {
        match request {
            ControlRequest::Arm { target } => Self::Arm(target),
            ControlRequest::Disarm => Self::Disarm,
            ControlRequest::Status => Self::Status,
        }
    }
}

/// Parse the process arguments, run the command, and return its exit code.
pub fn entrypoint() -> ExitCode {
    let command_line = RelayCommandLine::parse();
    match run(command_line.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("acknowledgement-dropping-relay: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn run(command: RelayCommand) -> Result<()> {
    match command {
        RelayCommand::Serve {
            listen,
            upstream,
            control_socket,
        } => serve(
            RelaySettings::from_flags(&listen, &upstream, control_socket)?,
            RelayLog::StandardError,
        ),
        RelayCommand::Control {
            control_socket,
            request,
        } => {
            let status = send_control_command(&control_socket, request.into())?;
            println!("{}", serde_json::to_string(&status)?);
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "tests/cli_tests.rs"]
mod tests;
