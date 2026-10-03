//! Failure injection at the executor boundaries of the fleet command ledger.
//!
//! **Role:** proves what an executor observes when an answer or a commit of the ledger is lost:
//! a claim whose answer is lost holds the command until its lease lapses and reconciliation
//! returns it to the queue under a new fencing token; a result failed before its commit leaves
//! the command executing, a clean retry records it once, and without a retry reconciliation marks
//! the non-idempotent command indeterminate and never hands it out again; a result whose
//! acknowledgement is lost after the commit is recorded once, and its retry is answered with the
//! 409 that ends an executor's report loop.
//! **Position:** its own test binary over the real executor routes
//! (`/api/v1/fleet-executor/commands/…`) of an `event_eligibility_support` fixture, with host-agent
//! credentials from `fleet_support`; the failpoints `FleetCommandClaimAfterCommit`,
//! `FleetCommandResultBeforeCommit` and `FleetCommandResultAfterCommit` sit in
//! `api_server_infrastructure::handlers::fleet_executor` and
//! `api_server_infrastructure::services::fleet_commands::executor_claims::record_result`.
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case holds for its whole body; this binary's private database from `tests/common`.
//! **Invariants:** every command records at most one outcome audit, equal to its state; a lease is
//! lapsed by moving `lease_expires_at` into the past, the instant reconciliation compares with
//! `clock_timestamp()`, never by sleeping.

mod common;
mod event_eligibility_support;
mod failpoint_and_race_support;
mod fleet_support;

use api_server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;
use axum::http::StatusCode;
use event_eligibility_support::{EventShape, Fixture};
use failpoint_and_race_support::{
    Failpoint, FailpointArming, RowCounts, assert_injected_failure, audit_evidence,
    check_fleet_outcome_recorded_once, lock_suite,
};
use fleet_support::{credential, machine, register_server};
use serde_json::{Value, json};
use uuid::Uuid;

const SUITE: &str = "failure_injection_fleet";

/// One registered server with a host-agent credential, over a fresh fixture.
struct FleetHost {
    fixture: Fixture,
    server: Uuid,
    secret: String,
}

impl FleetHost {
    async fn open(name: &str) -> Self {
        let fixture = Fixture::new(
            SUITE,
            EventShape {
                max_slots: 0,
                missions: &[&["Alpha"]],
            },
        )
        .await;
        let server = register_server(&fixture, name).await;
        let secret = credential(&fixture, server, "host_agent").await;
        Self {
            fixture,
            server,
            secret,
        }
    }

    /// An administrator's `restart`, a non-idempotent process change; answers the command id.
    async fn request_restart(&self) -> Uuid {
        let (status, receipt) = self
            .fixture
            .call(
                &self.fixture.admin,
                "POST",
                &format!("/api/v1/servers/{}/commands", self.server),
                Some(json!({"action": "restart"})),
            )
            .await;
        assert_eq!(status, StatusCode::ACCEPTED, "{receipt}");
        assert_eq!(receipt["state"], "queued");
        receipt["id"].as_str().unwrap().parse().unwrap()
    }

    async fn claim(&self) -> (StatusCode, Value) {
        self.fixture
            .call(
                &machine(&self.secret),
                "POST",
                "/api/v1/fleet-executor/commands/claim",
                Some(json!({})),
            )
            .await
    }

    /// Claims the only queued command and reports its effect as starting; answers its token.
    async fn claim_and_start(&self, command: Uuid) -> i64 {
        let (status, claimed) = self.claim().await;
        assert_eq!(status, StatusCode::OK, "{claimed}");
        assert_eq!(claimed["command_id"], command.to_string());
        let token = claimed["fencing_token"].as_i64().unwrap();
        let (status, started) = self
            .report(command, "executing", json!({"fencing_token": token}))
            .await;
        assert_eq!(status, StatusCode::OK, "{started}");
        assert_eq!(started["state"], "executing");
        token
    }

    async fn report(&self, command: Uuid, step: &str, body: Value) -> (StatusCode, Value) {
        self.fixture
            .call(
                &machine(&self.secret),
                "POST",
                &format!("/api/v1/fleet-executor/commands/{command}/{step}"),
                Some(body),
            )
            .await
    }

    async fn report_success(&self, command: Uuid, token: i64) -> (StatusCode, Value) {
        self.report(
            command,
            "result",
            json!({"fencing_token": token, "succeeded": true}),
        )
        .await
    }

    async fn receipt(&self, command: Uuid) -> Value {
        let (status, body) = self
            .fixture
            .call(
                &self.fixture.admin,
                "GET",
                &format!("/api/v1/servers/{}/commands/{command}", self.server),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    /// The command's lease lapses: its expiry moves into the past.
    async fn lapse(&self, command: Uuid) {
        sqlx::query(
            "UPDATE fleet_commands SET lease_expires_at = clock_timestamp() - interval '1 second'
             WHERE id = $1",
        )
        .bind(command)
        .execute(self.fixture.pool())
        .await
        .unwrap();
    }

    async fn audits(&self, action: &str, command: Uuid) -> i64 {
        audit_evidence(self.fixture.pool(), action, &command.to_string())
            .await
            .rows
    }

    async fn check_outcome_once(&self, command: Uuid) {
        check_fleet_outcome_recorded_once(self.fixture.pool(), command)
            .await
            .unwrap();
    }
}

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn failure_injection_fleet_claim_answer_lost_returns_the_command_to_the_queue_after_its_lease()
 {
    let suite = lock_suite().await;
    let host = FleetHost::open("Claim answer lost").await;
    let restart = host.request_restart().await;

    {
        let guard = suite.fail(Failpoint::FleetCommandClaimAfterCommit);
        let (status, body) = host.claim().await;
        assert_injected_failure(status, &body, Failpoint::FleetCommandClaimAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    // The claim committed; its answer, with the fencing token, never reached the executor.
    let held = host.receipt(restart).await;
    assert_eq!(held["state"], "claimed");
    assert_eq!(held["attempts"], 1);
    assert_eq!(host.audits("server.command_claimed", restart).await, 1);
    let lost_token: i64 =
        sqlx::query_scalar("SELECT fencing_token FROM fleet_commands WHERE id = $1")
            .bind(restart)
            .fetch_one(host.fixture.pool())
            .await
            .unwrap();
    let (status, body) = host.claim().await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "the lost claim holds the command until its lease lapses: {body}"
    );

    host.lapse(restart).await;
    let reconciled = reconcile_fleet_commands(host.fixture.pool()).await.unwrap();
    assert_eq!((reconciled.requeued, reconciled.indeterminate), (1, 0));
    assert_eq!(host.receipt(restart).await["state"], "queued");
    let (status, reclaimed) = host.claim().await;
    assert_eq!(status, StatusCode::OK, "{reclaimed}");
    assert_eq!(reclaimed["command_id"], restart.to_string());
    let token = reclaimed["fencing_token"].as_i64().unwrap();
    assert!(token > lost_token, "the new claim fences the lost one");
    assert_eq!(host.receipt(restart).await["attempts"], 2);
    let (status, fenced) = host
        .report(restart, "executing", json!({"fencing_token": lost_token}))
        .await;
    assert_eq!(
        (status, code(&fenced)),
        (StatusCode::CONFLICT, "STALE_FENCING_TOKEN")
    );

    let (status, started) = host
        .report(restart, "executing", json!({"fencing_token": token}))
        .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let (status, finished) = host.report_success(restart, token).await;
    assert_eq!(status, StatusCode::OK, "{finished}");
    assert_eq!(finished["state"], "succeeded");
    assert_eq!(host.audits("server.command_succeeded", restart).await, 1);
    host.check_outcome_once(restart).await;
}

#[tokio::test]
async fn failure_injection_fleet_claim_answer_lost_with_nothing_claimable_changes_nothing() {
    let suite = lock_suite().await;
    let host = FleetHost::open("Empty claim answer lost").await;
    let before = RowCounts::capture(host.fixture.pool(), &["fleet_commands", "audit_logs"]).await;

    {
        let guard = suite.fail(Failpoint::FleetCommandClaimAfterCommit);
        let (status, body) = host.claim().await;
        assert_injected_failure(status, &body, Failpoint::FleetCommandClaimAfterCommit);
        assert_eq!(
            guard.arrivals(),
            1,
            "the point fires on the empty claim too"
        );
    }
    before.check_unchanged(host.fixture.pool()).await.unwrap();
    let (status, body) = host.claim().await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    before.check_unchanged(host.fixture.pool()).await.unwrap();
}

#[tokio::test]
async fn failure_injection_fleet_result_before_commit_rolls_back_and_the_retried_report_records_once()
 {
    let suite = lock_suite().await;
    let host = FleetHost::open("Result rolled back").await;
    let restart = host.request_restart().await;
    let token = host.claim_and_start(restart).await;

    {
        let guard = suite.fail(Failpoint::FleetCommandResultBeforeCommit);
        let (status, body) = host.report_success(restart, token).await;
        assert_injected_failure(status, &body, Failpoint::FleetCommandResultBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    let executing = host.receipt(restart).await;
    assert_eq!(executing["state"], "executing");
    assert_eq!(host.audits("server.command_succeeded", restart).await, 0);
    host.check_outcome_once(restart).await;

    let (status, finished) = host.report_success(restart, token).await;
    assert_eq!(status, StatusCode::OK, "{finished}");
    assert_eq!(finished["state"], "succeeded");
    assert_eq!(finished["attempts"], 1, "the effect ran once");
    assert_eq!(host.audits("server.command_succeeded", restart).await, 1);
    host.check_outcome_once(restart).await;
}

#[tokio::test]
async fn failure_injection_fleet_result_before_commit_unretried_becomes_indeterminate_and_never_reexecutes()
 {
    let suite = lock_suite().await;
    let host = FleetHost::open("Result never retried").await;
    let restart = host.request_restart().await;
    let token = host.claim_and_start(restart).await;

    {
        let guard = suite.fail(Failpoint::FleetCommandResultBeforeCommit);
        let (status, body) = host.report_success(restart, token).await;
        assert_injected_failure(status, &body, Failpoint::FleetCommandResultBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    assert_eq!(host.receipt(restart).await["state"], "executing");

    // The executor stops reporting: whether the restart happened is unknown.
    host.lapse(restart).await;
    let reconciled = reconcile_fleet_commands(host.fixture.pool()).await.unwrap();
    assert_eq!((reconciled.requeued, reconciled.indeterminate), (0, 1));
    let uncertain = host.receipt(restart).await;
    assert_eq!(uncertain["state"], "indeterminate");
    assert_eq!(uncertain["attempts"], 1);
    let (status, body) = host.claim().await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "an indeterminate command is never handed out again: {body}"
    );
    let reconciled = reconcile_fleet_commands(host.fixture.pool()).await.unwrap();
    assert_eq!((reconciled.requeued, reconciled.indeterminate), (0, 0));
    let (status, late) = host.report_success(restart, token).await;
    assert_eq!(
        (status, code(&late)),
        (StatusCode::CONFLICT, "STALE_FENCING_TOKEN"),
        "{late}"
    );
    assert_eq!(host.receipt(restart).await["state"], "indeterminate");
    assert_eq!(host.audits("server.command_succeeded", restart).await, 0);
    host.check_outcome_once(restart).await;
}

#[tokio::test]
async fn failure_injection_fleet_result_after_commit_ack_lost_retry_records_once() {
    let suite = lock_suite().await;
    let host = FleetHost::open("Result acknowledgement lost").await;
    let restart = host.request_restart().await;
    let token = host.claim_and_start(restart).await;

    {
        let guard = suite.fail(Failpoint::FleetCommandResultAfterCommit);
        let (status, body) = host.report_success(restart, token).await;
        assert_injected_failure(status, &body, Failpoint::FleetCommandResultAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    let recorded = host.receipt(restart).await;
    assert_eq!(recorded["state"], "succeeded");
    assert_eq!(host.audits("server.command_succeeded", restart).await, 1);

    // The executor retries the report it saw fail; the ledger already holds the outcome.
    let (status, retry) = host.report_success(restart, token).await;
    assert_eq!(
        (status, code(&retry)),
        (StatusCode::CONFLICT, "STALE_FENCING_TOKEN"),
        "{retry}"
    );
    assert_eq!(
        retry["details"]["state"], "succeeded",
        "the refusal names the recorded outcome"
    );
    assert_eq!(host.receipt(restart).await, recorded);
    assert_eq!(host.audits("server.command_succeeded", restart).await, 1);
    let reconciled = reconcile_fleet_commands(host.fixture.pool()).await.unwrap();
    assert_eq!((reconciled.requeued, reconciled.indeterminate), (0, 0));
    assert_eq!(host.claim().await.0, StatusCode::NO_CONTENT);
    host.check_outcome_once(restart).await;
}
