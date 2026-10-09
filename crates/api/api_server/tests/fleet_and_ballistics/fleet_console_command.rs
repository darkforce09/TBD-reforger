//! The console command through the real routes: an administrator's single console line passes
//! the argument gate or is refused with 400, only an administrator may send one, and it is queued
//! for the host agent as a non-idempotent process change.

use crate::{event_eligibility_support, fleet_support};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use contract_schema_types::server_infrastructure::fleet_command::ConsoleCommandArguments;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{credential, machine, register_server, session};

const SUITE: &str = "fleet_console_command";
/// The line the staging procedure sends: the session's player list.
const PLAYERS_LINE: &str = "#players";

async fn fixture() -> Fixture {
    Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await
}

fn console(line: impl Into<Value>) -> Value {
    json!({"action": "console_command", "arguments": {"line": line.into()}})
}

fn commands_uri(server: Uuid) -> String {
    format!("/api/v1/servers/{server}/commands")
}

async fn request(f: &Fixture, server: Uuid, body: Value) -> (StatusCode, Value) {
    f.call(&f.admin, "POST", &commands_uri(server), Some(body))
        .await
}

async fn requested(f: &Fixture, server: Uuid, body: Value) -> String {
    let (status, receipt) = request(f, server, body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{receipt}");
    assert_eq!(receipt["state"], "queued");
    receipt["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_owned()
}

/// The same request with no `Authorization` header at all.
async fn anonymous(f: &Fixture, uri: &str, body: &Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("the request builds");
    let response = f.app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn claim(f: &Fixture, secret: &str, session: Option<Uuid>) -> (StatusCode, Value) {
    let body = match session {
        Some(session) => json!({ "runtime_session_id": session }),
        None => json!({}),
    };
    f.call(
        &machine(secret),
        "POST",
        "/api/v1/fleet-executor/commands/claim",
        Some(body),
    )
    .await
}

async fn report(
    f: &Fixture,
    secret: &str,
    command: &str,
    step: &str,
    body: Value,
) -> (StatusCode, Value) {
    f.call(
        &machine(secret),
        "POST",
        &format!("/api/v1/fleet-executor/commands/{command}/{step}"),
        Some(body),
    )
    .await
}

async fn listed(f: &Fixture, server: Uuid) -> Vec<Value> {
    let (status, list) = f.call(&f.admin, "GET", &commands_uri(server), None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    list["items"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn fleet_console_command_refuses_lines_outside_the_gate() {
    let f = fixture().await;
    let server = register_server(&f, "Console gate host").await;
    let refused = [
        ("an empty line", console("")),
        ("a blank line", console("   ")),
        ("257 bytes", console("p".repeat(257))),
        ("258 bytes in 129 characters", console("é".repeat(129))),
        ("a newline", console("#players\n#shutdown")),
        ("a trailing newline", console("#players\n")),
        ("a carriage return", console("#players\r")),
        ("a control character", console("#players\u{7}")),
        ("an escape sequence", console("\u{1b}[2J#players")),
        ("a line separator", console("#players\u{2028}#shutdown")),
        ("a leading @", console("@logout")),
        ("a leading @ after spaces", console("  @logout")),
        ("a number", console(7)),
        (
            "no line",
            json!({"action": "console_command", "arguments": {}}),
        ),
        ("no arguments", json!({"action": "console_command"})),
        (
            "an unknown argument",
            json!({"action": "console_command",
                "arguments": {"line": PLAYERS_LINE, "repeat": 2}}),
        ),
    ];
    for (case, body) in refused {
        let (status, answer) = request(&f, server, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{case}: {answer}");
        assert!(
            answer["error"]
                .as_str()
                .is_some_and(|error| !error.is_empty()),
            "{case}: {answer}"
        );
    }
    assert!(
        listed(&f, server).await.is_empty(),
        "a refused line queues nothing"
    );
    // The contract's generated type refuses the same lines the gate does.
    for line in [
        "",
        "#players\n#shutdown",
        "#players\u{7}",
        "#players\u{2028}#shutdown",
        "@logout",
    ] {
        assert!(
            serde_json::from_value::<ConsoleCommandArguments>(json!({ "line": line })).is_err(),
            "the contract admits {line:?}"
        );
    }
    // The bounds themselves pass: 256 bytes, in single- or multi-byte characters, and an @ that
    // does not lead the line.
    for line in [
        "p".repeat(256),
        "é".repeat(128),
        "say -1 meet @ the gate".to_owned(),
    ] {
        let (status, receipt) = request(&f, server, console(line.clone())).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{line:?}: {receipt}");
        assert_eq!(receipt["arguments"], json!({ "line": line }));
        let stored: ConsoleCommandArguments =
            serde_json::from_value(receipt["arguments"].clone()).expect("the contract's shape");
        assert_eq!(stored.line.as_str(), line);
    }
}

#[tokio::test]
async fn fleet_console_command_is_refused_to_non_administrators_and_anonymous_callers() {
    let f = fixture().await;
    let server = register_server(&f, "Console authority host").await;
    let uri = commands_uri(server);
    for role in ["guest", "enlisted", "leader", "mission_maker"] {
        let caller = f.account(role, role).await;
        let (status, answer) = f
            .call(&caller, "POST", &uri, Some(console(PLAYERS_LINE)))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{role}: {answer}");
    }
    let (status, answer) = anonymous(&f, &uri, &console(PLAYERS_LINE)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{answer}");
    assert!(
        listed(&f, server).await.is_empty(),
        "a refusal queues nothing"
    );
    requested(&f, server, console(PLAYERS_LINE)).await;
    assert_eq!(listed(&f, server).await.len(), 1);
}

#[tokio::test]
async fn fleet_console_command_is_queued_for_the_host_agent_as_a_non_idempotent_process_change() {
    let f = fixture().await;
    let server = register_server(&f, "Console queue host").await;
    let agent = credential(&f, server, "host_agent").await;
    let runtime = credential(&f, server, "mod_runtime").await;
    let (live, _) = session(&f, &runtime).await;
    let (status, receipt) = request(&f, server, console(format!("  {PLAYERS_LINE}  "))).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{receipt}");
    assert_eq!(receipt["action"], "console_command");
    assert_eq!(receipt["executor_kind"], "host_agent");
    assert_eq!(receipt["state"], "queued");
    assert_eq!(receipt["arguments"], json!({ "line": PLAYERS_LINE }));
    let command = receipt["id"].as_str().unwrap().to_owned();
    let (idempotent, process_changing): (bool, bool) = sqlx::query_as(
        "SELECT idempotent, process_changing FROM fleet_commands WHERE id = $1::uuid",
    )
    .bind(&command)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!((idempotent, process_changing), (false, true));
    // The game runtime never claims it; the host agent receives exactly the stored line.
    assert_eq!(
        claim(&f, &runtime, Some(live)).await.0,
        StatusCode::NO_CONTENT
    );
    let (status, claimed) = claim(&f, &agent, None).await;
    assert_eq!(
        (status, claimed["command_id"].as_str()),
        (StatusCode::OK, Some(command.as_str()))
    );
    assert_eq!(claimed["action"], "console_command");
    assert_eq!(claimed["arguments"], json!({ "line": PLAYERS_LINE }));
    let (status, started) = report(
        &f,
        &agent,
        &command,
        "executing",
        json!({"fencing_token": claimed["fencing_token"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let window: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM lease_expires_at - executing_at)::float8 FROM fleet_commands
         WHERE id = $1::uuid",
    )
    .bind(&command)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(window.round(), 30.0, "the console effect has a 30 s window");
}
