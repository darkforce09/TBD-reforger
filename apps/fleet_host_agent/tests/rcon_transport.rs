//! The RCON transport against an in-process BattlEye RCon server that loses, corrupts,
//! duplicates, reorders and fragments packets, forgets logins on restart, and drops idle
//! logins the way Arma Reforger does after 45 seconds (scaled down here). A command that is safe
//! to repeat is retransmitted and sent again after a new login; a console command is transmitted
//! once, whatever happens to its answer, and its reply is captured for the ledger.

#[path = "test_support/fake_battleye_server.rs"]
mod fake_battleye_server;

use std::time::Duration;

use fake_battleye_server::{FakeBattlEyeServer, Faults, PLAYERS_RESPONSE};
use fleet_host_agent::command_execution::{
    CONSOLE_RESPONSE_MAX_BYTES, FleetActionExecutor, HostActionExecutor, HostCommand,
};
use fleet_host_agent::dedicated_server_config::DedicatedServerConfig;
use fleet_host_agent::process_control::{ProcessControl, ProcessControlSettings, SystemdUnitName};
use fleet_host_agent::rcon::{RconClient, RconError, RconSettings, RconTimings};
use fleet_host_agent::secret_text::SecretText;
use serde_json::{Value, json};
use tokio::net::UdpSocket;
use tokio::time::{Instant, sleep, timeout};

const PASSWORD: &str = "range-master";
/// The failure reason of a console command whose one transmission got no reply.
const NO_RESPONSE: &str = "no RCON response; the command may or may not have run";
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
        unit: SystemdUnitName::parse("tbd-reforger.service").unwrap(),
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

/// Waits until `condition` holds, failing after two seconds.
async fn eventually(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        sleep(Duration::from_millis(10)).await;
    }
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
async fn rcon_transport_refused_login_is_reported_and_not_retried() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, "wrong-password", timings()).await;
    assert_eq!(players(&client).await, Err(RconError::LoginRejected));
    assert_eq!(server.login_attempts(), 1);
    assert!(server.executed_commands().is_empty());
}

#[tokio::test]
async fn rcon_transport_unanswered_login_is_reported_after_every_attempt() {
    let silent = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = silent.local_addr().unwrap();
    let client = RconClient::start(RconSettings {
        server: address,
        password: SecretText::new(PASSWORD),
        timings: timings(),
    })
    .await
    .unwrap();
    assert_eq!(
        players(&client).await,
        Err(RconError::LoginUnanswered(address))
    );
    let mut buffer = [0u8; 512];
    let mut logins = 0;
    while let Ok(Ok(length)) = timeout(Duration::from_millis(100), silent.recv(&mut buffer)).await {
        assert_eq!(&buffer[7..length], b"\x00range-master", "a login packet");
        logins += 1;
    }
    assert_eq!(logins, timings().transmission_attempts);
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
async fn rcon_transport_lost_response_is_recovered_by_retransmission() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        discard_responses: 1,
        ..Faults::default()
    });
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    let transmissions = server.transmissions_of("#players");
    assert_eq!(transmissions.len(), 2, "{transmissions:?}");
    assert_eq!(transmissions[0], transmissions[1]);
    // The first transmission ran without an answer reaching the client: delivery within a
    // session is at least once.
    assert_eq!(server.executed_commands(), vec!["#players", "#players"]);
}

#[tokio::test]
async fn rcon_transport_corrupted_response_is_dropped_and_the_command_retransmitted() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        corrupt_responses: 1,
        ..Faults::default()
    });
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(server.transmissions_of("#players").len(), 2);
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
async fn rcon_transport_sequence_numbers_wrap_after_255_and_are_reused() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    for index in 0..300u32 {
        let command = format!("#echo {index}");
        assert_eq!(
            client.execute(&command).await.unwrap(),
            format!("executed {command}"),
            "each response answers its own command"
        );
    }
    let sequences: Vec<u8> = server
        .received_commands()
        .iter()
        .filter(|command| command.text.starts_with("#echo"))
        .map(|command| command.sequence)
        .collect();
    let expected: Vec<u8> = (0..=255u8).chain(0..44).collect();
    assert_eq!(sequences, expected);
}

#[tokio::test]
async fn rcon_transport_keep_alive_holds_the_login_past_the_server_timeout() {
    let server = FakeBattlEyeServer::start(PASSWORD, Duration::from_millis(400)).await;
    let keeping_alive = RconTimings {
        keep_alive_interval: Duration::from_millis(100),
        ..timings()
    };
    let client = client_of(&server, PASSWORD, keeping_alive).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    sleep(Duration::from_millis(1_300)).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(server.login_attempts(), 1, "the login never lapsed");
    let keep_alives = server.transmissions_of("").len();
    assert!(keep_alives >= 5, "{keep_alives} keep-alive packets");
}

#[tokio::test]
async fn rcon_transport_lapsed_login_is_renewed_by_the_next_command() {
    let server = FakeBattlEyeServer::start(PASSWORD, Duration::from_millis(300)).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    sleep(Duration::from_millis(700)).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(server.login_attempts(), 2);
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
async fn rcon_transport_server_messages_are_acknowledged_with_their_sequence_number() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    server
        .send_server_message(0, "Player #3 Rhodes connected")
        .await;
    // Sent again as when the first acknowledgement is lost: acknowledged again.
    server
        .send_server_message(0, "Player #3 Rhodes connected")
        .await;
    server
        .send_server_message(1, "Player #4 Kovac connected")
        .await;
    eventually("three acknowledgements", || {
        server.acknowledgements().len() == 3
    })
    .await;
    assert_eq!(server.acknowledgements(), vec![0, 0, 1]);
}

#[tokio::test]
async fn rcon_transport_log_out_frees_the_session_slot() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    client.log_out().await;
    eventually("the logout command", || {
        server.transmissions_of("@logout").len() == 1
    })
    .await;
    assert_eq!(players(&client).await, Err(RconError::ClientStopped));
}

#[tokio::test]
async fn rcon_transport_refuses_commands_that_cannot_be_one_packet_of_text() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    let too_long = "x".repeat(2_000);
    for command in ["", "#players\n#shutdown", too_long.as_str()] {
        assert_eq!(
            client.execute(command).await,
            Err(RconError::InvalidCommand),
            "{command:?}"
        );
        assert_eq!(
            client.execute_once(command).await,
            Err(RconError::InvalidCommand),
            "{command:?}"
        );
    }
    assert_eq!(server.login_attempts(), 0, "nothing was sent");
}

#[tokio::test]
async fn rcon_transport_execute_once_transmits_the_command_in_a_single_packet() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(
        client.execute_once("#restart").await.unwrap(),
        "executed #restart"
    );
    assert_eq!(server.transmissions_of("#restart").len(), 1);
    assert_eq!(server.executed_commands(), vec!["#restart"]);
    assert_eq!(server.login_attempts(), 1);
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
async fn rcon_transport_execute_once_retries_the_login_but_never_sends_the_command_without_one() {
    let silent = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = silent.local_addr().unwrap();
    let client = RconClient::start(RconSettings {
        server: address,
        password: SecretText::new(PASSWORD),
        timings: timings(),
    })
    .await
    .unwrap();
    assert_eq!(
        client.execute_once("#restart").await,
        Err(RconError::LoginUnanswered(address))
    );
    let mut buffer = [0u8; 512];
    let mut logins = 0;
    while let Ok(Ok(length)) = timeout(Duration::from_millis(100), silent.recv(&mut buffer)).await {
        assert_eq!(&buffer[7..length], b"\x00range-master", "a login packet");
        logins += 1;
    }
    assert_eq!(logins, timings().transmission_attempts);
}

#[tokio::test]
async fn rcon_transport_execute_once_reassembles_a_multi_part_reply() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    server.set_faults(Faults {
        fragment_bytes: Some(7),
        reverse_fragments: true,
        duplicate_fragments: true,
        ..Faults::default()
    });
    assert_eq!(
        client.execute_once("#players").await.unwrap(),
        PLAYERS_RESPONSE
    );
    assert_eq!(server.transmissions_of("#players").len(), 1);
}

#[tokio::test]
async fn rcon_transport_execute_once_lost_reply_is_not_resent_after_the_new_login() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        discard_responses: 1,
        ..Faults::default()
    });
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(
        client.execute_once("#restart").await,
        Err(RconError::NoResponse)
    );
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    assert_eq!(
        server.login_attempts(),
        2,
        "the next command logged in again"
    );
    assert_eq!(server.transmissions_of("#restart").len(), 1);
    assert_eq!(server.executed_commands(), vec!["#restart", "#players"]);
}

#[tokio::test]
async fn rcon_transport_execute_once_renews_a_forgotten_login_before_the_command_leaves() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    let client = client_of(&server, PASSWORD, timings()).await;
    assert_eq!(players(&client).await.unwrap(), PLAYERS_RESPONSE);
    server.restart();
    assert_eq!(
        client.execute_once("#restart").await.unwrap(),
        "executed #restart"
    );
    assert_eq!(server.login_attempts(), 2);
    assert_eq!(server.transmissions_of("#restart").len(), 1);
    assert_eq!(server.executed_commands(), vec!["#players", "#restart"]);
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

#[tokio::test]
async fn rcon_transport_console_reply_beyond_the_capture_is_cut_on_a_character_boundary() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    // A three-byte character across the 4,096-byte bound, split between the reply's two parts.
    let reply = format!(
        "{}€{}",
        "x".repeat(CONSOLE_RESPONSE_MAX_BYTES - 2),
        "y".repeat(900)
    );
    server.set_reply("#dump", &reply);
    server.set_faults(Faults {
        fragment_bytes: Some(CONSOLE_RESPONSE_MAX_BYTES - 1),
        reverse_fragments: true,
        ..Faults::default()
    });
    let host = console_host(&server).await;
    let verdict = host.execute(&console_command("#dump")).await;
    assert!(verdict.succeeded(), "{verdict:?}");
    assert_eq!(
        verdict.outcome().cloned().map(Value::Object),
        Some(json!({
            "response": &reply[..CONSOLE_RESPONSE_MAX_BYTES - 2],
            "response_truncated": true,
        }))
    );
    assert_eq!(server.transmissions_of("#dump").len(), 1);
}

#[tokio::test]
async fn rcon_transport_console_command_without_a_reply_may_or_may_not_have_run() {
    let server = FakeBattlEyeServer::start(PASSWORD, PATIENT_SERVER).await;
    server.set_faults(Faults {
        discard_responses: 1,
        ..Faults::default()
    });
    let host = console_host(&server).await;
    let verdict = host.execute(&console_command("#restart")).await;
    assert!(!verdict.succeeded());
    assert_eq!(verdict.failure_reason(), Some(NO_RESPONSE));
    assert_eq!(verdict.outcome(), None);
    assert_eq!(server.executed_commands(), vec!["#restart"], "it did run");
    assert_eq!(server.transmissions_of("#restart").len(), 1);
}
