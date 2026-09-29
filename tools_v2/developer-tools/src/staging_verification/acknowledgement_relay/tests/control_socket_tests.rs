//! The control socket: its mode, what it refuses to replace, its line protocol and its client.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::super::drop_policy::{Arming, DropPolicy, DropTarget, RelayIdentity};
use super::super::relay::RelayLog;
use super::super::stub_upstream::TemporaryFolder;
use super::{ControlCommand, ControlSocket, send_control_command};

fn policy() -> Arc<DropPolicy> {
    Arc::new(DropPolicy::new(RelayIdentity {
        listen: "127.0.0.1:18085".to_string(),
        upstream: "http://127.0.0.1:8080".to_string(),
        withhold: Duration::from_secs(30),
    }))
}

/// Serve a fresh control socket in `folder`; the returned task owns it.
fn serve_in(folder: &TemporaryFolder) -> (tokio::task::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
    let socket = ControlSocket::bind(&folder.socket_path()).expect("bind");
    let lines = Arc::new(Mutex::new(Vec::new()));
    let task = tokio::spawn(socket.serve(policy(), RelayLog::Captured(lines.clone())));
    (task, lines)
}

async fn send(path: PathBuf, command: ControlCommand) -> anyhow::Result<super::RelayStatus> {
    tokio::task::spawn_blocking(move || send_control_command(&path, command))
        .await
        .expect("control task")
}

async fn raw_exchange(path: PathBuf, request: &'static [u8]) -> String {
    tokio::task::spawn_blocking(move || {
        let mut stream = UnixStream::connect(&path).expect("connect");
        stream.write_all(request).expect("write");
        stream
            .shutdown(std::net::Shutdown::Write)
            .expect("shutdown");
        // A socket closed with unread input resets its peer after the queued answer, so the
        // answer is taken as far as it arrived.
        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer);
        String::from_utf8(answer).expect("a text answer")
    })
    .await
    .expect("raw task")
}

#[tokio::test]
async fn the_socket_is_mode_600_and_removed_when_dropped() {
    let folder = TemporaryFolder::new("mode");
    let socket = ControlSocket::bind(&folder.socket_path()).expect("bind");
    let metadata = fs::symlink_metadata(folder.socket_path()).expect("socket file");
    assert!(metadata.file_type().is_socket());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    drop(socket);
    assert!(!folder.socket_path().exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_socket_a_live_relay_answers_on_is_never_replaced() {
    let folder = TemporaryFolder::new("live");
    let (task, _) = serve_in(&folder);
    let refused = ControlSocket::bind(&folder.socket_path()).expect_err("a live socket is kept");
    assert!(refused.to_string().contains("already answers"), "{refused}");
    let status = send(folder.socket_path(), ControlCommand::Status).await;
    assert!(status.is_ok(), "the first relay still answers: {status:?}");
    task.abort();
}

#[tokio::test]
async fn a_path_that_is_not_a_socket_is_refused_and_left_in_place() {
    let folder = TemporaryFolder::new("file");
    fs::write(folder.socket_path(), "operator notes").unwrap();
    let refused = ControlSocket::bind(&folder.socket_path()).expect_err("a file is kept");
    assert!(refused.to_string().contains("is not a socket"), "{refused}");
    assert_eq!(
        fs::read_to_string(folder.socket_path()).unwrap(),
        "operator notes"
    );
}

#[tokio::test]
async fn an_unanswered_socket_left_by_a_stopped_relay_is_replaced() {
    let folder = TemporaryFolder::new("stale");
    drop(std::os::unix::net::UnixListener::bind(folder.socket_path()).unwrap());
    assert!(
        folder.socket_path().exists(),
        "the stale socket file stays behind"
    );
    let socket = ControlSocket::bind(&folder.socket_path()).expect("the stale socket is replaced");
    drop(socket);
}

#[test]
fn every_command_round_trips_through_its_line() {
    for command in [
        ControlCommand::Arm(DropTarget::DropNextClaimResponse),
        ControlCommand::Arm(DropTarget::DropNextResultResponse),
        ControlCommand::Disarm,
        ControlCommand::Status,
    ] {
        assert_eq!(ControlCommand::parse(&command.line()), Ok(command));
        assert_eq!(
            ControlCommand::parse(&format!("{}\n", command.line())),
            Ok(command)
        );
    }
    for line in [
        "",
        "arm",
        "arm drop-everything",
        "disarm now",
        "status please",
        "rearm",
    ] {
        assert!(ControlCommand::parse(line).is_err(), "{line:?} is refused");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn arm_disarm_and_status_act_on_the_policy_and_are_logged() {
    let folder = TemporaryFolder::new("commands");
    let (task, lines) = serve_in(&folder);
    let path = folder.socket_path();
    let armed = send(
        path.clone(),
        ControlCommand::Arm(DropTarget::DropNextResultResponse),
    )
    .await
    .unwrap();
    assert_eq!(armed.arming, Arming::DropNextResultResponse);
    let read = send(path.clone(), ControlCommand::Status).await.unwrap();
    assert_eq!(read, armed);
    let disarmed = send(path.clone(), ControlCommand::Disarm).await.unwrap();
    assert_eq!(disarmed.arming, Arming::Disarmed);
    assert_eq!(disarmed.listen, "127.0.0.1:18085");
    let logged = lines.lock().unwrap().clone();
    assert_eq!(logged, ["armed: drop-next-result-response", "disarmed"]);
    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_outside_the_protocol_is_answered_with_an_error() {
    let folder = TemporaryFolder::new("protocol");
    let (task, _) = serve_in(&folder);
    let answer = raw_exchange(folder.socket_path(), b"explode\n").await;
    let answer: serde_json::Value = serde_json::from_str(answer.trim_end()).unwrap();
    assert!(
        answer["error"]
            .as_str()
            .unwrap()
            .contains("the commands are")
    );
    let oversized = raw_exchange(folder.socket_path(), &[b'a'; 1024]).await;
    assert!(oversized.starts_with(r#"{"error":"#), "{oversized}");
    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_client_reports_a_socket_nobody_answers_on() {
    let folder = TemporaryFolder::new("absent");
    let missing = send(folder.socket_path(), ControlCommand::Status)
        .await
        .expect_err("nobody answers");
    assert!(
        missing.to_string().contains("no relay answers"),
        "{missing}"
    );
}
