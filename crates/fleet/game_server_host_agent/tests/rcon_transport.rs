//! The RCON transport against an in-process BattlEye RCon server that loses, corrupts,
//! duplicates, reorders and fragments packets, forgets logins on restart, and drops idle
//! logins the way Arma Reforger does after 45 seconds (scaled down here). A command that is safe
//! to repeat is retransmitted and sent again after a new login; a console command is transmitted
//! once, whatever happens to its answer, and its reply is captured for the ledger.

#[path = "test_support/fake_battleye_server.rs"]
mod fake_battleye_server;

use std::time::Duration;

use fake_battleye_server::{FakeBattlEyeServer, Faults, PLAYERS_RESPONSE};
use game_server_host_agent::command_execution::{
    FleetActionExecutor, HostActionExecutor, HostCommand,
};
use game_server_host_agent::dedicated_server_config::DedicatedServerConfig;
use game_server_host_agent::process_control::{
    ProcessControl, ProcessControlSettings, SystemdUnitName,
};
use game_server_host_agent::rcon::{RconClient, RconError, RconSettings, RconTimings};
use game_server_host_agent::secret_text::SecretText;
use serde_json::{Value, json};

const PASSWORD: &str = "range-master";
/// Long enough that the server never drops a login in tests that are not about idle time.
const PATIENT_SERVER: Duration = Duration::from_secs(600);

fn timings() -> RconTimings {
    RconTimings {
        response_timeout: Duration::from_millis(200),
        transmission_attempts: 4,
        keep_alive_interval: Duration::from_secs(600),
    }
}

async fn client_of(
    server: &FakeBattlEyeServer,
    password: &str,
    timings: RconTimings,
) -> RconClient {
    RconClient::start(RconSettings {
        server: server.address(),
        password: SecretText::new(password),
        timings,
    })
    .await
    .expect("the client socket binds")
}

async fn players(client: &RconClient) -> Result<String, RconError> {
    client.execute("#players").await
}

/// The game host's executor with RCON pointed at `server`. A console command never reaches its
/// process control or its server config, which name nothing that exists.
async fn console_host(server: &FakeBattlEyeServer) -> HostActionExecutor {
    let process_control = ProcessControl::new(ProcessControlSettings {
        systemctl_program: "/nonexistent/systemctl".into(),
        unit: SystemdUnitName::parse("tbd-reforger.service").expect("the unit name is valid"),
        start_dwell: Duration::ZERO,
        verb_timeout: Duration::from_secs(1),
        state_read_timeout: Duration::from_secs(1),
    });
    HostActionExecutor::new(
        process_control,
        client_of(server, PASSWORD, timings()).await,
        DedicatedServerConfig::new("/nonexistent/server.json".into()),
    )
}

fn console_command(line: &str) -> HostCommand {
    HostCommand::from_claim("console_command", &json!({ "line": line })).expect("a console line")
}

#[tokio::test]
async fn rcon_transport_logs_in_once_and_round_trips_commands() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(client.execute("#id").await.unwrap(), "executed #id");
    assert_eq!(server.login_attempts(), 1, "the session is reused");
    assert_eq!(server.executed_commands(), vec!["#players", "#id"]);
}

#[tokio::test]
async fn rcon_transport_lost_request_is_retransmitted_with_the_same_sequence_number() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        discard_commands: 2,
        ..Faults::default()
    });
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    let transmissions = server.transmissions_of("#players");
    assert_eq!(transmissions.len(), 3, "{transmissions:?}");
    assert!(
        transmissions
            .iter()
            .all(|sequence| *sequence == transmissions[0])
    );
    assert_eq!(server.executed_commands(), vec!["#players"]);
}

#[tokio::test]
async fn rcon_transport_duplicated_and_reordered_fragments_are_reassembled_in_order() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    // Seven-byte parts, sent last to first and twice each.
    server.set_faults(Faults {
        fragment_bytes: Some(7),
        reverse_fragments: true,
        duplicate_fragments: true,
        ..Faults::default()
    });
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    // One-byte parts, last to first: every multi-byte character of the listing is split
    // across two parts.
    server.set_faults(Faults {
        fragment_bytes: Some(1),
        reverse_fragments: true,
        ..Faults::default()
    });
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
}

#[tokio::test]
async fn rcon_transport_server_restart_forces_a_new_login() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    server.restart();
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(server.login_attempts(), 2);
    assert_eq!(
        server.executed_commands(),
        vec!["#players", "#players"],
        "commands sent before the new login were ignored, not run"
    );
}

#[tokio::test]
async fn rcon_transport_execute_once_never_retransmits_an_unanswered_command() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        discard_commands: 1,
        ..Faults::default()
    });
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(
        client.execute_once("#restart").await,
        Err(RconError::NoResponse)
    );
    assert_eq!(server.transmissions_of("#restart").len(), 1, "sent once");
    assert!(server.executed_commands().is_empty());
}

#[tokio::test]
async fn rcon_transport_console_command_reports_the_reply_and_that_it_is_whole() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let host = console_host(&server).await;
    let verdict = host.execute(&console_command("#players")).await;
    assert!(verdict.succeeded(), "{verdict:?}");
    assert_eq!(
        verdict.outcome().cloned().map(Value::Object),
        Some(json!({ "response": PLAYERS_RESPONSE, "response_truncated": false }))
    );
}
