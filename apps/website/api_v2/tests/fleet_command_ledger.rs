//! The fleet command ledger through the real routes: operators get durable receipts, each
//! executor kind claims only its own server's commands, process changes serialize per server,
//! stale executors are fenced, lapsed leases never repeat an uncertain effect, requester
//! authority is revalidated at claim, and kicks are bound to the runtime session they target.

use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;

mod common;
mod event_eligibility_support;
mod fleet_support;

use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{credential, machine, register_server, session};

const SUITE: &str = "fleet_command_ledger";

async fn request(f: &Fixture, server: Uuid, body: Value) -> (StatusCode, Value) {
    f.call(
        &f.admin,
        "POST",
        &format!("/api/v1/servers/{server}/commands"),
        Some(body),
    )
    .await
}

async fn requested(f: &Fixture, server: Uuid, body: Value) -> String {
    let (status, receipt) = request(f, server, body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{receipt}");
    assert_eq!(receipt["state"], "queued");
    receipt["id"].as_str().unwrap().to_owned()
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

async fn receipt(f: &Fixture, server: Uuid, command: &str) -> Value {
    let (status, body) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/servers/{server}/commands/{command}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn lapse(f: &Fixture, command: &str) {
    sqlx::query("UPDATE fleet_commands SET lease_expires_at = clock_timestamp() - interval '1 second' WHERE id = $1::uuid")
        .bind(command)
        .execute(f.pool())
        .await
        .unwrap();
}

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn fleet_commands_are_claimed_only_by_their_executor_kind_and_server() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let (server, other) = (
        register_server(&f, "Ledger host").await,
        register_server(&f, "Other host").await,
    );
    let agent = credential(&f, server, "host_agent").await;
    let runtime = credential(&f, server, "mod_runtime").await;
    let foreign_agent = credential(&f, other, "host_agent").await;
    let (live, _) = session(&f, &runtime).await;
    let restart = requested(&f, server, json!({"action": "restart"})).await;
    let kick = requested(
        &f,
        server,
        json!({"action": "kick", "arguments": {"arma_id": "arma-target", "runtime_session_id": live}}),
    )
    .await;
    assert_eq!(
        claim(&f, &foreign_agent, None).await.0,
        StatusCode::NO_CONTENT
    );
    let (status, host) = claim(&f, &agent, None).await;
    assert_eq!(
        (status, host["command_id"].as_str()),
        (StatusCode::OK, Some(restart.as_str()))
    );
    assert_eq!(host["action"], "restart");
    assert_eq!(claim(&f, &agent, None).await.0, StatusCode::NO_CONTENT);
    let (status, game) = claim(&f, &runtime, Some(live)).await;
    assert_eq!(
        (status, game["command_id"].as_str()),
        (StatusCode::OK, Some(kick.as_str()))
    );
    assert_eq!(
        game["arguments"],
        json!({"arma_id": "arma-target", "runtime_session_id": live})
    );
    // A broadcast reaches players through the game runtime: Reforger's RCON has no broadcast.
    let broadcast = requested(
        &f,
        server,
        json!({"action": "broadcast", "arguments": {"message": "Restart soon"}}),
    )
    .await;
    assert_eq!(claim(&f, &agent, None).await.0, StatusCode::NO_CONTENT);
    let (status, message) = claim(&f, &runtime, Some(live)).await;
    assert_eq!(
        (status, message["command_id"].as_str()),
        (StatusCode::OK, Some(broadcast.as_str()))
    );
    // A game runtime claims within its own open session only.
    assert_eq!(claim(&f, &runtime, None).await.0, StatusCode::BAD_REQUEST);
    let (foreign_session, _) = {
        let foreign_runtime = credential(&f, other, "mod_runtime").await;
        session(&f, &foreign_runtime).await
    };
    assert_eq!(
        claim(&f, &runtime, Some(foreign_session)).await.0,
        StatusCode::FORBIDDEN
    );
    // Reports come only from the claiming credential of the command's server.
    let token = host["fencing_token"].clone();
    let (status, _) = report(
        &f,
        &foreign_agent,
        &restart,
        "executing",
        json!({"fencing_token": token}),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let members = f.member("not-a-machine").await;
    let (status, _) = f
        .call(
            &members,
            "POST",
            "/api/v1/fleet-executor/commands/claim",
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    f.pool().close().await;
}

#[tokio::test]
async fn command_receipts_follow_every_state_to_its_observed_outcome() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Receipt host").await;
    let agent = credential(&f, server, "host_agent").await;
    let players = requested(&f, server, json!({"action": "list_players"})).await;
    let (_, claimed) = claim(&f, &agent, None).await;
    let token = claimed["fencing_token"].clone();
    let seen = receipt(&f, server, &players).await;
    assert_eq!(
        (seen["state"].as_str(), seen["attempts"].as_i64()),
        (Some("claimed"), Some(1))
    );
    // Success is accepted only after the effect was reported as starting.
    let (status, early) = report(
        &f,
        &agent,
        &players,
        "result",
        json!({"fencing_token": token, "succeeded": true}),
    )
    .await;
    assert_eq!(
        (status, code(&early)),
        (StatusCode::CONFLICT, "COMMAND_NOT_EXECUTING")
    );
    let (status, executing) = report(
        &f,
        &agent,
        &players,
        "executing",
        json!({"fencing_token": token}),
    )
    .await;
    assert_eq!(
        (status, executing["state"].as_str()),
        (StatusCode::OK, Some("executing"))
    );
    let (status, done) = report(
        &f,
        &agent,
        &players,
        "result",
        json!({"fencing_token": token, "succeeded": true, "outcome": {"players": [{"arma_id": "arma-1", "name": "Rhodes"}]}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{done}");
    let done = receipt(&f, server, &players).await;
    assert_eq!(done["state"], "succeeded");
    assert_eq!(done["outcome"]["players"][0]["arma_id"], "arma-1");
    assert!(done.get("finished_at").is_some() && done.get("executing_at").is_some());

    // A failure needs its reason and may end a claim before any effect.
    let failing = requested(&f, server, json!({"action": "list_players"})).await;
    let (_, claimed) = claim(&f, &agent, None).await;
    let token = claimed["fencing_token"].clone();
    let (status, _) = report(
        &f,
        &agent,
        &failing,
        "result",
        json!({"fencing_token": token, "succeeded": false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, failed) = report(
        &f,
        &agent,
        &failing,
        "result",
        json!({"fencing_token": token, "succeeded": false, "failure_reason": "RCON login refused"}),
    )
    .await;
    assert_eq!(
        (status, failed["state"].as_str()),
        (StatusCode::OK, Some("failed"))
    );
    assert_eq!(failed["failure_reason"], "RCON login refused");

    // Only an unclaimed command can be cancelled; cancelling twice changes nothing.
    let stop = requested(&f, server, json!({"action": "stop"})).await;
    let cancel = format!("/api/v1/servers/{server}/commands/{stop}/cancel");
    let (status, cancelled) = f.call(&f.admin, "POST", &cancel, None).await;
    assert_eq!(
        (status, cancelled["state"].as_str()),
        (StatusCode::OK, Some("cancelled"))
    );
    assert_eq!(
        f.call(&f.admin, "POST", &cancel, None).await.1["state"],
        "cancelled"
    );
    let finished = format!("/api/v1/servers/{server}/commands/{players}/cancel");
    let (status, refused) = f.call(&f.admin, "POST", &finished, None).await;
    assert_eq!(
        (status, code(&refused)),
        (StatusCode::CONFLICT, "COMMAND_NOT_CANCELLABLE")
    );
    let (_, listed) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/servers/{server}/commands"),
            None,
        )
        .await;
    let states: Vec<&str> = listed["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["state"].as_str().unwrap())
        .collect();
    assert_eq!(
        states,
        vec!["cancelled", "failed", "succeeded"],
        "newest first"
    );
    for (action, command) in [
        ("server.command_requested", &players),
        ("server.command_claimed", &players),
        ("server.command_succeeded", &players),
    ] {
        assert_eq!(f.audit_count(action, command).await, 1, "{action}");
    }
    f.pool().close().await;
}

#[tokio::test]
async fn command_recovery_fences_stale_executors_and_never_repeats_an_uncertain_effect() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Recovery host").await;
    let (first, second) = (
        credential(&f, server, "host_agent").await,
        credential(&f, server, "host_agent").await,
    );
    let restart = requested(&f, server, json!({"action": "restart"})).await;
    let (_, stale) = claim(&f, &first, None).await;
    let stale_token = stale["fencing_token"].clone();
    // The first executor stalls before starting: the claim returns to the queue.
    lapse(&f, &restart).await;
    assert_eq!(
        reconcile_fleet_commands(f.pool()).await.unwrap().requeued,
        1
    );
    assert_eq!(receipt(&f, server, &restart).await["state"], "queued");
    let (status, fenced) = report(
        &f,
        &first,
        &restart,
        "executing",
        json!({"fencing_token": stale_token}),
    )
    .await;
    assert_eq!(
        (status, code(&fenced)),
        (StatusCode::CONFLICT, "STALE_FENCING_TOKEN")
    );
    let (_, current) = claim(&f, &second, None).await;
    assert!(current["fencing_token"].as_i64() > stale_token.as_i64());
    let (status, _) = report(
        &f,
        &second,
        &restart,
        "executing",
        json!({"fencing_token": current["fencing_token"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // The second executor stops reporting mid-restart: the outcome is unknown and nothing repeats it.
    lapse(&f, &restart).await;
    assert_eq!(
        reconcile_fleet_commands(f.pool())
            .await
            .unwrap()
            .indeterminate,
        1
    );
    let uncertain = receipt(&f, server, &restart).await;
    assert_eq!(uncertain["state"], "indeterminate");
    assert_eq!(uncertain["attempts"], 2);
    assert_eq!(claim(&f, &second, None).await.0, StatusCode::NO_CONTENT);
    let (status, late) = report(
        &f,
        &second,
        &restart,
        "result",
        json!({"fencing_token": current["fencing_token"], "succeeded": true}),
    )
    .await;
    assert_eq!(
        (status, code(&late)),
        (StatusCode::CONFLICT, "STALE_FENCING_TOKEN"),
        "{late}"
    );

    // An idempotent command whose executor stalled mid-effect is safely retried.
    let start = requested(&f, server, json!({"action": "start"})).await;
    let (_, claimed) = claim(&f, &first, None).await;
    report(
        &f,
        &first,
        &start,
        "executing",
        json!({"fencing_token": claimed["fencing_token"]}),
    )
    .await;
    lapse(&f, &start).await;
    assert_eq!(
        reconcile_fleet_commands(f.pool()).await.unwrap().requeued,
        1
    );
    let (_, retried) = claim(&f, &second, None).await;
    assert_eq!(retried["command_id"].as_str(), Some(start.as_str()));
    assert_eq!(receipt(&f, server, &start).await["attempts"], 2);

    // A command nobody claims in time expires.
    let stop = requested(&f, server, json!({"action": "stop"})).await;
    sqlx::query("UPDATE fleet_commands SET requested_at = clock_timestamp() - interval '10 minutes', expires_at = clock_timestamp() - interval '1 second' WHERE id = $1::uuid")
        .bind(&stop)
        .execute(f.pool())
        .await
        .unwrap();
    assert_eq!(reconcile_fleet_commands(f.pool()).await.unwrap().expired, 1);
    assert_eq!(receipt(&f, server, &stop).await["state"], "expired");

    // The requester's authority is revalidated when an executor claims.
    let revalidated = requested(&f, server, json!({"action": "list_players"})).await;
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&f.admin.id)
        .execute(f.pool())
        .await
        .unwrap();
    assert_eq!(claim(&f, &first, None).await.0, StatusCode::NO_CONTENT);
    let revoked = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT state, failure_reason FROM fleet_commands WHERE id = $1::uuid",
    )
    .bind(&revalidated)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(revoked.0, "cancelled");
    assert!(revoked.1.unwrap().contains("administrator authority"));
    f.pool().close().await;
}

#[tokio::test]
async fn process_control_changes_serialize_per_server_while_reads_proceed() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Serial host").await;
    let (first, second) = (
        credential(&f, server, "host_agent").await,
        credential(&f, server, "host_agent").await,
    );
    let restart = requested(&f, server, json!({"action": "restart"})).await;
    let stop = requested(&f, server, json!({"action": "stop"})).await;
    let players = requested(&f, server, json!({"action": "list_players"})).await;
    // Two executors race for the first process change: exactly one claims it.
    let (a, b) = tokio::join!(claim(&f, &first, None), claim(&f, &second, None));
    let claimed: Vec<&str> = [&a.1, &b.1]
        .iter()
        .filter_map(|body| body["command_id"].as_str())
        .collect();
    assert!(claimed.contains(&restart.as_str()), "{a:?} {b:?}");
    assert!(
        !claimed.contains(&stop.as_str()),
        "a second process change waits: {a:?} {b:?}"
    );
    let holder = if a.1["command_id"] == restart.as_str() {
        &first
    } else {
        &second
    };
    let restart_token = if a.1["command_id"] == restart.as_str() {
        a.1["fencing_token"].clone()
    } else {
        b.1["fencing_token"].clone()
    };
    // The player list does not wait for the process change.
    let claimed_players = claimed.contains(&players.as_str()) || {
        let (_, next) = claim(&f, &first, None).await;
        next["command_id"] == players.as_str()
    };
    assert!(claimed_players);
    assert_eq!(
        claim(&f, &second, None).await.0,
        StatusCode::NO_CONTENT,
        "stop waits for the restart"
    );
    report(
        &f,
        holder,
        &restart,
        "executing",
        json!({"fencing_token": restart_token}),
    )
    .await;
    report(&f, holder, &restart, "result", json!({"fencing_token": restart_token, "succeeded": true, "outcome": {"unit_state": "active"}})).await;
    let (_, next) = claim(&f, &second, None).await;
    assert_eq!(next["command_id"].as_str(), Some(stop.as_str()));
    f.pool().close().await;
}

#[tokio::test]
async fn kick_identity_is_bound_to_the_runtime_session_it_targets() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Kick host").await;
    let runtime = credential(&f, server, "mod_runtime").await;
    let (first, _) = session(&f, &runtime).await;
    let kick = json!({"action": "kick", "arguments": {"arma_id": "arma-griefer", "runtime_session_id": first, "reason": "Team killing"}});
    let bound = requested(&f, server, kick.clone()).await;
    // The game restarts before the kick is claimed: the kick never reaches the new session.
    let (second, _) = session(&f, &runtime).await;
    assert_eq!(
        claim(&f, &runtime, Some(second)).await.0,
        StatusCode::NO_CONTENT
    );
    let failed = receipt(&f, server, &bound).await;
    assert_eq!(failed["state"], "failed");
    assert!(
        failed["failure_reason"]
            .as_str()
            .unwrap()
            .contains("runtime session")
    );
    // A kick names an open session of this server.
    let (status, refused) = request(&f, server, kick).await;
    assert_eq!(
        (status, code(&refused)),
        (StatusCode::CONFLICT, "RUNTIME_SESSION_ENDED")
    );
    let (status, _) = request(&f, server, json!({"action": "kick", "arguments": {"arma_id": "arma-griefer", "runtime_session_id": Uuid::new_v4()}})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = request(
        &f,
        server,
        json!({"action": "kick", "arguments": {"player": 12}}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a transient player number never names a kick target"
    );
    // A kick for the current session is delivered with its identity and session.
    let current = requested(&f, server, json!({"action": "kick", "arguments": {"arma_id": "arma-griefer", "runtime_session_id": second}})).await;
    let (status, delivered) = claim(&f, &runtime, Some(second)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(delivered["command_id"].as_str(), Some(current.as_str()));
    assert_eq!(
        delivered["arguments"],
        json!({"arma_id": "arma-griefer", "runtime_session_id": second})
    );
    f.pool().close().await;
}
