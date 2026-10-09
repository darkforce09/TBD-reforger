//! The console command through the real routes: an administrator's single console line passes
//! the argument gate or is refused with 400, only an administrator may send one, it is queued
//! for the host agent as a non-idempotent process change, it waits for and holds back the
//! server's other process changes, its reported response is bounded at 4096 bytes, a lapse
//! during its effect makes it indeterminate instead of sending the line again, and the request
//! audit carries the line.

use api_server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use contract_schema_types::server_infrastructure::fleet_command::{
    ConsoleCommandArguments, ConsoleCommandOutcome,
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

mod common;
mod event_eligibility_support;
mod fleet_support;

use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{credential, machine, register_server, session};

const SUITE: &str = "fleet_console_command";
/// The line the staging procedure sends: the session's player list.
const PLAYERS_LINE: &str = "#players";
/// What the host agent reports when the server never answers the transmitted line.
const NO_REPLY: &str = "no RCON response; the command may or may not have run";

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

/// Claim `command` as the host agent and report its effect starting; answers the fencing token.
async fn executing(f: &Fixture, agent: &str, command: &str) -> i64 {
    let (status, claimed) = claim(f, agent, None).await;
    assert_eq!(
        (status, claimed["command_id"].as_str()),
        (StatusCode::OK, Some(command)),
        "{claimed}"
    );
    let token = claimed["fencing_token"]
        .as_i64()
        .expect("the `fencing_token` field is an integer");
    let (status, answer) = report(
        f,
        agent,
        command,
        "executing",
        json!({"fencing_token": token}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    token
}

async fn receipt(f: &Fixture, server: Uuid, command: &str) -> Value {
    let uri = format!("{}/{command}", commands_uri(server));
    let (status, body) = f.call(&f.admin, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn listed(f: &Fixture, server: Uuid) -> Vec<Value> {
    let (status, list) = f.call(&f.admin, "GET", &commands_uri(server), None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    list["items"].as_array().cloned().unwrap_or_default()
}

async fn lapse(f: &Fixture, command: &str) {
    sqlx::query(
        "UPDATE fleet_commands SET lease_expires_at = clock_timestamp() - interval '1 second'
         WHERE id = $1::uuid",
    )
    .bind(command)
    .execute(f.pool())
    .await
    .expect("the update of fleet_commands succeeds");
}

fn console_result(token: i64, response: impl Into<Value>, truncated: bool) -> Value {
    json!({"fencing_token": token, "succeeded": true,
        "outcome": {"response": response.into(), "response_truncated": truncated}})
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

#[tokio::test]
async fn fleet_console_command_is_serialised_with_a_restart() {
    let f = fixture().await;
    let server = register_server(&f, "Console serialisation host").await;
    let agent = credential(&f, server, "host_agent").await;
    let restart = requested(&f, server, json!({"action": "restart"})).await;
    let line = requested(&f, server, console(PLAYERS_LINE)).await;
    // The older restart runs first and the console line waits for it.
    let restart_token = executing(&f, &agent, &restart).await;
    assert_eq!(claim(&f, &agent, None).await.0, StatusCode::NO_CONTENT);
    let (status, answer) = report(
        &f,
        &agent,
        &restart,
        "result",
        json!({"fencing_token": restart_token, "succeeded": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // While the line runs, a restart waits for it; a player list is no process change.
    let line_token = executing(&f, &agent, &line).await;
    let second_restart = requested(&f, server, json!({"action": "restart"})).await;
    let players = requested(&f, server, json!({"action": "list_players"})).await;
    let (status, claimed) = claim(&f, &agent, None).await;
    assert_eq!(
        (status, claimed["command_id"].as_str()),
        (StatusCode::OK, Some(players.as_str()))
    );
    assert_eq!(claim(&f, &agent, None).await.0, StatusCode::NO_CONTENT);
    let (status, answer) = report(
        &f,
        &agent,
        &line,
        "result",
        console_result(line_token, "Players on server:", false),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, claimed) = claim(&f, &agent, None).await;
    assert_eq!(
        (status, claimed["command_id"].as_str()),
        (StatusCode::OK, Some(second_restart.as_str()))
    );
}

#[tokio::test]
async fn fleet_console_command_refuses_an_outcome_of_4097_bytes() {
    let f = fixture().await;
    let server = register_server(&f, "Console outcome host").await;
    let agent = credential(&f, server, "host_agent").await;
    let command = requested(&f, server, console(PLAYERS_LINE)).await;
    let token = executing(&f, &agent, &command).await;
    let refused = [
        ("4097 bytes", console_result(token, "r".repeat(4097), true)),
        (
            "4098 bytes in 2049 characters",
            console_result(token, "é".repeat(2049), true),
        ),
        (
            "no truncation flag",
            json!({"fencing_token": token, "succeeded": true,
                "outcome": {"response": "Players on server:"}}),
        ),
        (
            "no response",
            json!({"fencing_token": token, "succeeded": true,
                "outcome": {"response_truncated": false}}),
        ),
        (
            "an unknown field",
            json!({"fencing_token": token, "succeeded": true,
                "outcome": {"response": "ok", "response_truncated": false, "exit_code": 0}}),
        ),
        (
            "a textual truncation flag",
            json!({"fencing_token": token, "succeeded": true,
                "outcome": {"response": "ok", "response_truncated": "no"}}),
        ),
        (
            "a success without its outcome",
            json!({"fencing_token": token, "succeeded": true}),
        ),
    ];
    for (case, body) in refused {
        let (status, answer) = report(&f, &agent, &command, "result", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{case}: {answer}");
    }
    let unchanged = receipt(&f, server, &command).await;
    assert_eq!(unchanged["state"], "executing");
    assert!(unchanged.get("outcome").is_none(), "{unchanged}");
    let longest = "r".repeat(4096);
    let (status, answer) = report(
        &f,
        &agent,
        &command,
        "result",
        console_result(token, longest.as_str(), true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["state"], "succeeded");
    assert_eq!(
        answer["outcome"],
        json!({"response": longest, "response_truncated": true})
    );
    let recorded: ConsoleCommandOutcome =
        serde_json::from_value(answer["outcome"].clone()).expect("the contract's shape");
    assert_eq!(
        (recorded.response.as_str(), recorded.response_truncated),
        (longest.as_str(), true)
    );
    // No reply at all is a failure that names the uncertainty and carries no outcome.
    let silent = requested(&f, server, console(PLAYERS_LINE)).await;
    let token = executing(&f, &agent, &silent).await;
    let (status, answer) = report(
        &f,
        &agent,
        &silent,
        "result",
        json!({"fencing_token": token, "succeeded": false, "failure_reason": NO_REPLY}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        (&answer["state"], &answer["failure_reason"]),
        (&json!("failed"), &json!(NO_REPLY))
    );
    assert!(answer.get("outcome").is_none(), "{answer}");
}

#[tokio::test]
async fn fleet_console_command_becomes_indeterminate_when_its_lease_lapses_during_the_effect() {
    let f = fixture().await;
    let server = register_server(&f, "Console lapse host").await;
    let agent = credential(&f, server, "host_agent").await;
    let command = requested(&f, server, console(PLAYERS_LINE)).await;
    // A claim that lapses before the effect starts never sent the line: it is queued again.
    let (status, stale) = claim(&f, &agent, None).await;
    assert_eq!(status, StatusCode::OK, "{stale}");
    lapse(&f, &command).await;
    reconcile_fleet_commands(f.pool()).await.unwrap();
    assert_eq!(receipt(&f, server, &command).await["state"], "queued");
    // A lease that lapses while the line may be running makes the outcome unknown.
    let token = executing(&f, &agent, &command).await;
    assert!(token > stale["fencing_token"].as_i64().unwrap());
    lapse(&f, &command).await;
    reconcile_fleet_commands(f.pool()).await.unwrap();
    let uncertain = receipt(&f, server, &command).await;
    assert_eq!(uncertain["state"], "indeterminate", "{uncertain}");
    assert_eq!(uncertain["attempts"], 2);
    assert!(
        uncertain["failure_reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("unknown")),
        "{uncertain}"
    );
    // Nothing sends the line again, and the late report changes nothing.
    assert_eq!(claim(&f, &agent, None).await.0, StatusCode::NO_CONTENT);
    let (status, late) = report(
        &f,
        &agent,
        &command,
        "result",
        console_result(token, "Players on server:", false),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{late}");
    assert_eq!(
        receipt(&f, server, &command).await["state"],
        "indeterminate"
    );
}

#[tokio::test]
async fn fleet_console_command_request_audit_carries_the_line() {
    let f = fixture().await;
    let server = register_server(&f, "Console audit host").await;
    let line = "#players console-audit";
    let command = requested(&f, server, console(line)).await;
    let (actor, message): (Option<String>, String) = sqlx::query_as(
        "SELECT actor_id, message FROM audit_logs
         WHERE action = 'server.command_requested' AND target_type = 'fleet_command'
           AND target_id = $1",
    )
    .bind(&command)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(actor.as_deref(), Some(f.admin.id.as_str()));
    assert!(message.contains("console_command"), "{message}");
    assert!(message.contains(line), "{message}");
}
