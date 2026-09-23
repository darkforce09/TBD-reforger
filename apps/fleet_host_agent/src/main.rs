//! `fleet-host-agent <configuration-file>`: runs the fleet host agent until SIGTERM or SIGINT.
//!
//! Exit status: 0 after a requested shutdown or `--help`, 2 for a usage error, 78 (EX_CONFIG)
//! for an invalid configuration, 1 when the agent cannot start.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use fleet_host_agent::agent_configuration::AgentConfiguration;
use fleet_host_agent::command_execution::HostActionExecutor;
use fleet_host_agent::ledger_client::{CommandLoop, LedgerApi, LedgerTimings};
use fleet_host_agent::process_control::ProcessControl;
use fleet_host_agent::rcon::RconClient;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const USAGE: &str = "usage: fleet-host-agent <configuration-file>";
const EXIT_USAGE: u8 = 2;
const EXIT_CONFIGURATION: u8 = 78;

/// What the command line asks for.
enum Invocation {
    Run(PathBuf),
    Help,
    Invalid,
}

#[tokio::main]
async fn main() -> ExitCode {
    initialise_logging();
    let configuration_path = match invocation() {
        Invocation::Run(path) => path,
        Invocation::Help => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Invocation::Invalid => {
            eprintln!("{USAGE}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let configuration = match AgentConfiguration::load(&configuration_path) {
        Ok(configuration) => configuration,
        Err(problem) => {
            error!(%problem, "invalid configuration");
            return ExitCode::from(EXIT_CONFIGURATION);
        }
    };
    run(configuration).await
}

/// Log lines go to standard error, which the systemd journal collects; colour only on a
/// terminal. `RUST_LOG` overrides the default `info` level.
fn initialise_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .init();
}

/// The single argument is the configuration file, or `--help`.
fn invocation() -> Invocation {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next()) {
        (Some(argument), None) if argument == "--help" || argument == "-h" => Invocation::Help,
        (Some(path), None) => Invocation::Run(PathBuf::from(path)),
        _ => Invocation::Invalid,
    }
}

async fn run(configuration: AgentConfiguration) -> ExitCode {
    let rcon_server = configuration.rcon.server;
    let rcon = match RconClient::start(configuration.rcon).await {
        Ok(rcon) => rcon,
        Err(problem) => {
            error!(%problem, %rcon_server, "the RCON socket could not be opened");
            return ExitCode::FAILURE;
        }
    };
    let api = match LedgerApi::new(
        &configuration.api_base_url,
        &configuration.machine_credential,
    ) {
        Ok(api) => api,
        Err(problem) => {
            error!(%problem, "the API client could not be set up");
            return ExitCode::FAILURE;
        }
    };
    let process_control = ProcessControl::new(configuration.process_control);
    info!(
        api = %configuration.api_base_url,
        unit = process_control.unit().as_str(),
        server_config = %configuration.server_config.path().display(),
        %rcon_server,
        "fleet host agent started"
    );
    let executor =
        HostActionExecutor::new(process_control, rcon.clone(), configuration.server_config);
    CommandLoop::new(
        api,
        executor,
        LedgerTimings::with_poll_interval(configuration.poll_interval),
    )
    .run(shutdown_requested())
    .await;
    rcon.log_out().await;
    info!("fleet host agent stopped");
    ExitCode::SUCCESS
}

/// Completes on SIGTERM (systemd's stop) or SIGINT.
async fn shutdown_requested() {
    let mut terminate = match signal(SignalKind::terminate()) {
        Ok(terminate) => terminate,
        Err(problem) => {
            error!(%problem, "SIGTERM cannot be observed; only SIGINT stops the agent");
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };
    tokio::select! {
        _ = terminate.recv() => info!("SIGTERM received"),
        _ = tokio::signal::ctrl_c() => info!("SIGINT received"),
    }
}
