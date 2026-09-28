//! Failure injection at the reservation claim: `POST /event-missions/:emid/register` failing
//! before and after its commit.
//!
//! **Role:** proves the claim transaction's documented outcomes through the real router. A
//! failure before the commit leaves no registration, no participant allocation, no seat
//! assignment, no audit row and no pending publication, and a clean retry claims the seat. A
//! failure after the commit loses only the answer: the seat is held once, and the retry answers
//! the same reservation without a second place, allocation or audit row.
//! **Position:** its own test binary over `tests/common` (database, accounts) and
//! `tests/failpoint_and_race_support` (suite lock, arming, persisted-state checks).
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case takes first; each case owns a fresh event with one place.
//! **Invariants:** every case leaves nothing armed; whole-table row counts are compared only while
//! the suite lock is held, so no other case of this binary writes in between.

mod common;
mod failpoint_and_race_support;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use failpoint_and_race_support::{
    AuditEvidence, Failpoint, FailpointArming, RowCounts, assert_injected_failure, audit_evidence,
    check_event_seats_and_places, lock_suite,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

const SUITE: &str = "failure_injection_operations";

/// The audit action of a changed reservation.
const REGISTRATION_CHANGED: &str = "event.registration_changed";

/// The tables a claim writes; a failed claim leaves each exactly as it found it.
const CLAIM_TABLES: [&str; 4] = [
    "event_registrations",
    "event_participant_allocations",
    "audit_logs",
    "audit_publication_pending",
];

/// The next synthetic client address; every request comes from its own peer so the per-address
/// rate limiter never answers in place of the handler under test.
static PEER: AtomicU32 = AtomicU32::new(1);

fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

/// An open event with one place and one attached mission of two seats, and a linked enlisted
/// player who has not registered.
struct ReservationWorld {
    state: AppState,
    app: Router,
    player: String,
    player_token: String,
    event: Uuid,
    attachment: Uuid,
    seat: Uuid,
}

impl ReservationWorld {
    async fn new() -> Self {
        let url = common::require_test_database_url()
            .expect("the operations failure-injection suite requires PostgreSQL");
        let pool = database::connect(&url)
            .await
            .expect("connect to the suite database");
        let state = AppState::new(pool, Config::for_tests(url, "failure-injection-operations"));
        let organiser = format!("{SUITE}-organiser-{}", Uuid::new_v4());
        common::access_token(&state, SUITE, &organiser, "admin", true).await;
        let player = format!("{SUITE}-player-{}", Uuid::new_v4());
        let player_token = common::access_token(&state, SUITE, &player, "enlisted", true).await;
        let event: Uuid = sqlx::query_scalar(
            "INSERT INTO events (name_override, start_time, status, max_slots, created_by)
             VALUES ('Failure injection reservations', clock_timestamp() + interval '3 days',
                 'open', 1, $1) RETURNING id",
        )
        .bind(&organiser)
        .fetch_one(&state.pool)
        .await
        .expect("create the event");
        let mission: Uuid = sqlx::query_scalar(
            "INSERT INTO missions (title, author_id, terrain, game_mode, max_players, status)
             VALUES ('Failure injection mission', $1, 'everon', 'pve_coop', 32, 'live') RETURNING id",
        )
        .bind(&organiser)
        .fetch_one(&state.pool)
        .await
        .expect("create the mission");
        let attachment: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions (event_id, mission_id, start_time)
             VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
        )
        .bind(event)
        .bind(mission)
        .fetch_one(&state.pool)
        .await
        .expect("attach the mission");
        let mut seats = Vec::new();
        for index in 0..2 {
            let seat: Uuid = sqlx::query_scalar(
                "INSERT INTO orbat_slots (event_mission_id, faction, squad, role, slot_index)
                 VALUES ($1, 'USA', 'Alpha', 'Rifleman', $2) RETURNING id",
            )
            .bind(attachment)
            .bind(index)
            .fetch_one(&state.pool)
            .await
            .expect("create a seat");
            seats.push(seat);
        }
        let app = http_router::router(state.clone());
        Self {
            state,
            app,
            player,
            player_token,
            event,
            attachment,
            seat: seats[0],
        }
    }

    fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    /// The player claims [`Self::seat`] from a fresh synthetic peer.
    async fn claim_seat(&self) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/v1/event-missions/{}/register",
                self.attachment
            ))
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.player_token),
            )
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "slot_id": self.seat }).to_string()))
            .expect("build the claim");
        request.extensions_mut().insert(ConnectInfo(next_peer()));
        let response = self.app.clone().oneshot(request).await.expect("route");
        let status = response.status();
        assert_ne!(status, StatusCode::TOO_MANY_REQUESTS, "rate limited");
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read the body");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// The account the seat is assigned to.
    async fn seat_holder(&self) -> Option<String> {
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1")
            .bind(self.seat)
            .fetch_one(self.pool())
            .await
            .expect("read the seat")
    }

    /// The player's registration on the attachment: its reservation state and seat.
    async fn registration(&self) -> Option<(String, Option<Uuid>)> {
        sqlx::query_as(
            "SELECT reservation_state::text, slot_id FROM event_registrations
             WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(self.attachment)
        .bind(&self.player)
        .fetch_optional(self.pool())
        .await
        .expect("read the registration")
    }

    async fn registration_audits(&self) -> AuditEvidence {
        audit_evidence(
            self.pool(),
            REGISTRATION_CHANGED,
            &self.attachment.to_string(),
        )
        .await
    }
}

#[tokio::test]
async fn failure_injection_reservation_claim_before_commit_rolls_back_and_a_retry_claims_the_seat()
{
    let suite = lock_suite().await;
    let world = ReservationWorld::new().await;
    let pool = world.pool();
    let before = RowCounts::capture(pool, &CLAIM_TABLES).await;
    {
        let guard = suite.fail(Failpoint::ReservationClaimBeforeCommit);
        let (status, body) = world.claim_seat().await;
        assert_injected_failure(status, &body, Failpoint::ReservationClaimBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    before.check_unchanged(pool).await.unwrap();
    assert_eq!(world.seat_holder().await, None, "the seat stays free");
    assert_eq!(world.registration().await, None);
    assert_eq!(
        world.registration_audits().await,
        AuditEvidence {
            rows: 0,
            pending: 0,
            published: 0,
        }
    );
    check_event_seats_and_places(pool, world.event)
        .await
        .unwrap();

    // The clean retry claims the seat and audits the change once.
    let (status, body) = world.claim_seat().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["slot_id"], world.seat.to_string());
    assert_eq!(world.seat_holder().await, Some(world.player.clone()));
    let audits = world.registration_audits().await;
    assert_eq!(
        (audits.rows, audits.pending + audits.published),
        (1, 1),
        "{audits:?}"
    );
    check_event_seats_and_places(pool, world.event)
        .await
        .unwrap();
}

#[tokio::test]
async fn failure_injection_reservation_claim_after_commit_holds_the_seat_once_and_the_retry_is_idempotent()
 {
    let suite = lock_suite().await;
    let world = ReservationWorld::new().await;
    let pool = world.pool();
    let before = RowCounts::capture(pool, &CLAIM_TABLES).await;
    {
        let guard = suite.fail(Failpoint::ReservationClaimAfterCommit);
        let (status, body) = world.claim_seat().await;
        assert_injected_failure(status, &body, Failpoint::ReservationClaimAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    // The claim committed: the seat, one registration, one place and one audit row are held.
    assert_eq!(world.seat_holder().await, Some(world.player.clone()));
    let committed = world
        .registration()
        .await
        .expect("the committed claim registered the player");
    assert_eq!(committed.1, Some(world.seat));
    let after_commit = RowCounts::capture(pool, &CLAIM_TABLES).await;
    assert_eq!(
        after_commit.count("event_registrations"),
        before.count("event_registrations") + 1
    );
    let audits = world.registration_audits().await;
    assert_eq!(
        (audits.rows, audits.pending + audits.published),
        (1, 1),
        "{audits:?}"
    );
    check_event_seats_and_places(pool, world.event)
        .await
        .unwrap();

    // The retry answers the committed reservation and changes nothing: no second place in the
    // one-place event, no second allocation, no second audit row.
    let (status, body) = world.claim_seat().await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["slot_id"], world.seat.to_string());
    assert_eq!(body["reservation_state"], committed.0.as_str());
    after_commit.check_unchanged(pool).await.unwrap();
    assert_eq!(world.registration().await, Some(committed));
    assert_eq!(world.seat_holder().await, Some(world.player.clone()));
    assert_eq!(world.registration_audits().await, audits);
    check_event_seats_and_places(pool, world.event)
        .await
        .unwrap();
}
