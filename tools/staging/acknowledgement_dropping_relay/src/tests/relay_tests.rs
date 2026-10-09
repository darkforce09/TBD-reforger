//! The relay against a stub API: pass-through fidelity, one drop per arming, the `204` claim
//! answer, the hold past the agent's timeout, secret hygiene and the loopback rule.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::super::control_socket::{ControlCommand, send_control_command};
use super::super::drop_policy::{
    Arming, DropTarget, ExecutorResponse, FleetCommandId, RelayStatus,
};
use super::super::relay_settings::{RelaySettings, UpstreamOrigin};
use super::super::stub_upstream::{
    PlannedAnswer, StubUpstream, TemporaryFolder, claim_answer, nothing_claimable, result_conflict,
    result_receipt,
};
use super::{RelayLog, RunningRelay, start};

/// The tests' hold: long enough that no loopback exchange outlasts it.
const TEST_HOLD: Duration = Duration::from_millis(1200);
/// A client timeout below the hold, standing for the host agent's.
const AGENT_LIKE_TIMEOUT: Duration = Duration::from_millis(500);
/// A client timeout above the hold, for a client that waits the hold out.
const PATIENT_TIMEOUT: Duration = Duration::from_secs(10);
const CLAIM_PATH: &str = "/api/v1/fleet-executor/commands/claim";
const COMMAND_A: &str = "0b6c3a52-9d1e-4b8a-9a55-4f0e8c1d2a01";
const COMMAND_B: &str = "0b6c3a52-9d1e-4b8a-9a55-4f0e8c1d2a02";
const CREDENTIAL_CANARY: &str = "Bearer tbdmc_relay-canary-7f3e19c4";

struct Harness {
    relay: RunningRelay,
    stub: StubUpstream,
    folder: TemporaryFolder,
    log: Arc<Mutex<Vec<String>>>,
}

impl Harness {
    async fn start() -> Self {
        let stub = StubUpstream::start().await;
        let upstream = UpstreamOrigin::parse(&stub.origin()).expect("stub origin");
        Self::start_with_upstream(stub, upstream).await
    }

    async fn start_with_upstream(stub: StubUpstream, upstream: UpstreamOrigin) -> Self {
        let folder = TemporaryFolder::new("exchange");
        let log = Arc::new(Mutex::new(Vec::new()));
        let settings = RelaySettings {
            listen: "127.0.0.1:0".parse().expect("listen address"),
            upstream,
            control_socket: folder.socket_path(),
            withhold: TEST_HOLD,
        };
        let relay = start(settings, RelayLog::Captured(log.clone()))
            .await
            .expect("relay starts");
        Self {
            relay,
            stub,
            folder,
            log,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.relay.listen_address())
    }

    async fn control(&self, command: ControlCommand) -> RelayStatus {
        let socket = self.folder.socket_path();
        tokio::task::spawn_blocking(move || send_control_command(&socket, command))
            .await
            .expect("control task")
            .expect("the relay answers its control socket")
    }

    fn log_lines(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    async fn stop(self) {
        self.relay.shut_down().await.expect("relay shuts down");
        assert!(
            !self.folder.socket_path().exists(),
            "the control socket is removed"
        );
    }
}

/// A client on fresh connections, so no answer is ever read from a reused one.
fn client(timeout: Duration) -> reqwest::Client {
    crate::http_client::http_client_builder()
        .no_proxy()
        .pool_max_idle_per_host(0)
        .timeout(timeout)
        .build()
        .expect("client")
}

async fn claim(harness: &Harness, timeout: Duration) -> reqwest::Result<reqwest::Response> {
    client(timeout)
        .post(harness.url(CLAIM_PATH))
        .header("authorization", CREDENTIAL_CANARY)
        .json(&serde_json::json!({ "runtime_session_id": null }))
        .send()
        .await
}

fn echo_answer() -> PlannedAnswer {
    PlannedAnswer {
        status: StatusCode::IM_A_TEAPOT,
        headers: vec![
            ("content-type", "text/plain; charset=utf-8"),
            ("x-upstream-marker", "kept"),
            ("x-upstream-marker", "twice"),
        ],
        body: "the upstream's own words ✓".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_exchange_passes_through_unchanged_while_disarmed() {
    let harness = Harness::start().await;
    harness.stub.plan(echo_answer());
    let answer = client(PATIENT_TIMEOUT)
        .put(harness.url("/api/v1/any/route?first=1&second=two%20words"))
        .header("authorization", CREDENTIAL_CANARY)
        .header("x-agent-marker", "sent")
        .header("content-type", "application/json")
        .body(r#"{"exact":"bytes","n":1}"#)
        .send()
        .await
        .expect("the relay answers");
    assert_eq!(answer.status(), StatusCode::IM_A_TEAPOT);
    let markers: Vec<_> = answer
        .headers()
        .get_all("x-upstream-marker")
        .iter()
        .collect();
    assert_eq!(markers, ["kept", "twice"]);
    assert_eq!(
        answer.headers()["content-type"],
        "text/plain; charset=utf-8"
    );
    assert!(answer.headers().get("via").is_none());
    assert_eq!(answer.text().await.unwrap(), "the upstream's own words ✓");

    let received = harness.stub.received();
    assert_eq!(received.len(), 1);
    let request = &received[0];
    assert_eq!(request.method, "PUT");
    assert_eq!(
        request.uri.to_string(),
        "/api/v1/any/route?first=1&second=two%20words"
    );
    assert_eq!(request.headers["authorization"], CREDENTIAL_CANARY);
    assert_eq!(request.headers["x-agent-marker"], "sent");
    assert_eq!(request.headers["content-type"], "application/json");
    assert!(request.headers.get("x-forwarded-for").is_none());
    assert!(request.headers.get("via").is_none());
    assert_eq!(&request.body[..], br#"{"exact":"bytes","n":1}"#);

    harness.stub.plan(claim_answer(COMMAND_A, 1));
    let claimed = claim(&harness, PATIENT_TIMEOUT)
        .await
        .expect("claim passes");
    assert_eq!(claimed.status(), StatusCode::OK);
    assert!(claimed.text().await.unwrap().contains(COMMAND_A));
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!(status.arming, Arming::Disarmed);
    assert_eq!((status.forwarded_count, status.drop_count), (2, 0));
    assert_eq!(status.last_drop, None);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_armed_claim_drop_withholds_exactly_one_answer() {
    let harness = Harness::start().await;
    let armed = harness
        .control(ControlCommand::Arm(DropTarget::DropNextClaimResponse))
        .await;
    assert_eq!(armed.arming, Arming::DropNextClaimResponse);
    harness.stub.plan(claim_answer(COMMAND_A, 1));
    harness.stub.plan(claim_answer(COMMAND_B, 2));

    let lost = claim(&harness, AGENT_LIKE_TIMEOUT).await;
    assert!(
        lost.expect_err("the first claim answer is withheld")
            .is_timeout()
    );
    let passed = claim(&harness, PATIENT_TIMEOUT)
        .await
        .expect("second claim");
    assert_eq!(passed.status(), StatusCode::OK);
    assert!(passed.text().await.unwrap().contains(COMMAND_B));

    assert_eq!(
        harness.stub.received().len(),
        2,
        "both claims reached the upstream"
    );
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!(status.arming, Arming::Disarmed);
    assert_eq!((status.drop_count, status.forwarded_count), (1, 1));
    let dropped = status.last_drop.expect("the drop is recorded");
    assert_eq!(dropped.response, ExecutorResponse::Claim);
    assert_eq!(
        dropped.command_id.as_ref().map(FleetCommandId::as_str),
        Some(COMMAND_A)
    );
    assert_eq!(dropped.fencing_token, Some(1));
    assert_eq!(dropped.upstream_status, 200);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_claims_spend_one_arming_on_exactly_one_answer() {
    let harness = Harness::start().await;
    harness
        .control(ControlCommand::Arm(DropTarget::DropNextClaimResponse))
        .await;
    harness.stub.plan(claim_answer(COMMAND_A, 1));
    harness.stub.plan(claim_answer(COMMAND_B, 1));
    let (first, second) = tokio::join!(
        claim(&harness, PATIENT_TIMEOUT),
        claim(&harness, PATIENT_TIMEOUT)
    );
    let answered: Vec<_> = [first, second].into_iter().filter_map(Result::ok).collect();
    assert_eq!(
        answered.len(),
        1,
        "exactly one of the two answers is withheld"
    );
    assert_eq!(answered[0].status(), StatusCode::OK);
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!((status.drop_count, status.forwarded_count), (1, 1));
    assert_eq!(status.arming, Arming::Disarmed);
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_204_claim_answer_passes_through_and_keeps_the_arming() {
    let harness = Harness::start().await;
    harness
        .control(ControlCommand::Arm(DropTarget::DropNextClaimResponse))
        .await;
    harness.stub.plan(nothing_claimable());
    let idle = claim(&harness, PATIENT_TIMEOUT)
        .await
        .expect("a 204 passes");
    assert_eq!(idle.status(), StatusCode::NO_CONTENT);
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!(status.arming, Arming::DropNextClaimResponse);
    assert_eq!((status.drop_count, status.forwarded_count), (0, 1));

    harness.stub.plan(claim_answer(COMMAND_A, 1));
    let lost = claim(&harness, AGENT_LIKE_TIMEOUT).await;
    assert!(lost.expect_err("the 200 after it is withheld").is_timeout());
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!((status.drop_count, status.arming), (1, Arming::Disarmed));
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_withheld_claim_answer_outlasts_the_hold_and_its_connection_closes_without_a_byte() {
    let harness = Harness::start().await;
    harness
        .control(ControlCommand::Arm(DropTarget::DropNextClaimResponse))
        .await;
    harness.stub.plan(claim_answer(COMMAND_A, 1));
    let mut connection = TcpStream::connect(harness.relay.listen_address())
        .await
        .expect("connect");
    let request = format!(
        "POST {CLAIM_PATH} HTTP/1.1\r\nhost: relay\r\nauthorization: {CREDENTIAL_CANARY}\r\n\
         content-type: application/json\r\ncontent-length: 2\r\n\r\n{{}}"
    );
    let sent_at = Instant::now();
    connection
        .write_all(request.as_bytes())
        .await
        .expect("send");
    let mut received = Vec::new();
    // A close reads as the end of the stream and a reset as an error; either way nothing arrives.
    let ending = tokio::time::timeout(PATIENT_TIMEOUT, connection.read_to_end(&mut received))
        .await
        .expect("the relay closes the connection");
    let held = sent_at.elapsed();
    assert!(
        received.is_empty(),
        "no byte of the answer is written: {received:?} ({ending:?})"
    );
    assert!(
        held >= TEST_HOLD,
        "held {held:?}, less than the hold {TEST_HOLD:?}"
    );
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_armed_result_drop_passes_a_409_and_withholds_the_next_200() {
    let harness = Harness::start().await;
    harness
        .control(ControlCommand::Arm(DropTarget::DropNextResultResponse))
        .await;
    let result_path = format!("/api/v1/fleet-executor/commands/{COMMAND_A}/result");
    let report = serde_json::json!({ "fencing_token": 2, "succeeded": true });
    let send_report = |timeout| {
        client(timeout)
            .post(harness.url(&result_path))
            .header("authorization", CREDENTIAL_CANARY)
            .json(&report)
            .send()
    };
    harness.stub.plan(result_conflict());
    let refused = send_report(PATIENT_TIMEOUT).await.expect("a 409 passes");
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    harness.stub.plan(claim_answer(COMMAND_B, 1));
    let claimed = claim(&harness, PATIENT_TIMEOUT)
        .await
        .expect("a claim passes");
    assert_eq!(claimed.status(), StatusCode::OK);
    assert_eq!(
        harness.control(ControlCommand::Status).await.arming,
        Arming::DropNextResultResponse
    );

    harness.stub.plan(result_receipt(COMMAND_A));
    let lost = send_report(AGENT_LIKE_TIMEOUT).await;
    assert!(lost.expect_err("the 200 receipt is withheld").is_timeout());
    let status = harness.control(ControlCommand::Status).await;
    assert_eq!((status.drop_count, status.forwarded_count), (1, 2));
    let dropped = status.last_drop.expect("the drop is recorded");
    assert_eq!(dropped.response, ExecutorResponse::Result);
    assert_eq!(
        dropped.command_id.as_ref().map(FleetCommandId::as_str),
        Some(COMMAND_A)
    );
    assert_eq!(dropped.fencing_token, Some(2));
    harness.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_authorization_header_reaches_the_upstream_and_nowhere_else() {
    let harness = Harness::start().await;
    harness
        .control(ControlCommand::Arm(DropTarget::DropNextClaimResponse))
        .await;
    harness.stub.plan(claim_answer(COMMAND_A, 1));
    assert!(claim(&harness, AGENT_LIKE_TIMEOUT).await.is_err());
    harness.stub.plan(echo_answer());
    claim(&harness, PATIENT_TIMEOUT).await.expect("passes");
    let status = harness.control(ControlCommand::Status).await;
    let received = harness.stub.received();
    assert!(
        received
            .iter()
            .all(|request| request.headers["authorization"] == CREDENTIAL_CANARY)
    );

    let closed_port = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_origin = format!("http://{}", closed_port.local_addr().unwrap());
    drop(closed_port);
    let unreachable = Harness::start_with_upstream(
        StubUpstream::start().await,
        UpstreamOrigin::parse(&dead_origin).unwrap(),
    )
    .await;
    let failed = claim(&unreachable, PATIENT_TIMEOUT)
        .await
        .expect("an answer");
    assert_eq!(failed.status(), StatusCode::BAD_GATEWAY);

    let secret = CREDENTIAL_CANARY.trim_start_matches("Bearer ");
    let mut exposed = harness.log_lines();
    exposed.extend(unreachable.log_lines());
    exposed.push(serde_json::to_string(&status).unwrap());
    exposed.push(format!("{status:?}"));
    assert!(
        exposed
            .iter()
            .any(|line| line.contains("withholding the 200"))
    );
    assert!(exposed.iter().any(|line| line.contains("did not answer")));
    for line in &exposed {
        assert!(!line.contains(secret), "the credential leaked into: {line}");
    }
    harness.stop().await;
    unreachable.stop().await;
}

#[tokio::test]
async fn a_non_loopback_listen_address_is_refused_before_anything_binds() {
    let folder = TemporaryFolder::new("refused");
    for listen in ["0.0.0.0:0", "203.0.113.7:0", "[::]:0"] {
        let settings = RelaySettings {
            listen: listen.parse().unwrap(),
            upstream: UpstreamOrigin::parse("http://127.0.0.1:8080").unwrap(),
            control_socket: folder.socket_path(),
            withhold: TEST_HOLD,
        };
        let refused = start(settings, RelayLog::Captured(Arc::default()))
            .await
            .expect_err("a non-loopback listen address is refused");
        assert!(
            refused.to_string().contains("not a loopback address"),
            "{refused}"
        );
        assert!(
            !folder.socket_path().exists(),
            "{listen}: no control socket"
        );
    }
}
