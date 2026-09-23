//! The RCON transport against an in-process BattlEye RCon server that loses, corrupts,
//! duplicates, reorders and fragments packets, forgets logins on restart, and drops idle
//! logins the way Arma Reforger does after 45 seconds (scaled down here).

#[path = "test_support/fake_battleye_server.rs"]
mod fake_battleye_server;

use std::time::Duration;

use fake_battleye_server::{FakeBattlEyeServer, Faults, PLAYERS_RESPONSE};
use fleet_host_agent::rcon::{RconClient, RconError, RconSettings, RconTimings};
use fleet_host_agent::secret_text::SecretText;
use tokio::net::UdpSocket;
use tokio::time::{Instant, sleep, timeout};

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
    }
    assert_eq!(server.login_attempts(), 0, "nothing was sent");
}
