//! Real event writes conserve participant capacity, schedules, authority and retained history.
use crate::{common, telemetry_support};

use api_configuration::configuration::Config;
use api_server::router::router;
use api_state::AppState;
use axum::{Router, http::StatusCode};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

struct Fixture {
    state: AppState,
    app: Router,
    token: String,
    actor: String,
    event: Uuid,
    attachments: Vec<Uuid>,
    initial: DateTime<Utc>,
}
async fn fixture() -> Fixture {
    let url = common::require_test_database_url().expect("isolated PostgreSQL required");
    let pool = api_database::connect(&url)
        .await
        .expect("the test database accepts a connection");
    api_database::migrate(&pool)
        .await
        .expect("the migrations apply to the test database");
    let state = api_server::composition::application_state(
        pool,
        Config::for_tests(url, "event-administration"),
    );
    let actor = format!("event-admin-{}", Uuid::new_v4());
    let token = common::access_token(
        &state,
        "event_administration_transactions",
        &actor,
        "admin",
        true,
    )
    .await;
    let (event, initial): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO events(name_override, start_time, status, max_slots, created_by)
        VALUES ('Transactional event', statement_timestamp() + interval '2 hours', 'open', 4, $1) RETURNING id, start_time")
        .bind(&actor).fetch_one(&state.pool).await.expect("the insert into events returns its row");
    let mut attachments = Vec::new();
    for index in 0..3 {
        let mission: Uuid = sqlx::query_scalar(
            "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
            VALUES ('Event transaction', $1, 'everon', 'pve_coop', 8, 'live') RETURNING id",
        )
        .bind(&actor)
        .fetch_one(&state.pool)
        .await
        .expect("the insert into missions returns its row");
        let attachment: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions(event_id, mission_id, start_time, deleted_at)
            VALUES ($1, $2, $3, CASE WHEN $4 THEN clock_timestamp() END) RETURNING id",
        )
        .bind(event)
        .bind(mission)
        .bind(initial + Duration::minutes(index * 90))
        .bind(index == 2)
        .fetch_one(&state.pool)
        .await
        .expect("the insert into event_missions returns its row");
        let slot: Uuid = sqlx::query_scalar("INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to)
            VALUES ($1, 'USA', 'Alpha', 'Rifleman', 0, $2) RETURNING id")
            .bind(attachment).bind(&actor).fetch_one(&state.pool).await.expect("the insert into orbat_slots returns its row");
        let mut fixture = state.pool.begin().await.expect("a transaction begins");
        let allocation = common::participant_allocation(&mut fixture, attachment, &actor).await;
        sqlx::query("INSERT INTO event_registrations(event_mission_id, discord_id, slot_id, attendance_state, legacy_attendance_state, allocation_id)
            VALUES ($1, $2, $3, 'attended', 'attended', $4)")
            .bind(attachment).bind(&actor).bind(slot).bind(allocation).execute(&mut *fixture).await.expect("the insert into event_registrations succeeds");
        fixture.commit().await.expect("the transaction commits");
        attachments.push(attachment);
    }
    let app = router(state.clone());
    Fixture {
        state,
        app,
        token,
        actor,
        event,
        attachments,
        initial,
    }
}
async fn request(f: &Fixture, method: &str, body: Option<Value>) -> (StatusCode, Value) {
    telemetry_support::call(
        &f.app,
        method,
        &format!("/api/v1/events/{}", f.event),
        Some(&f.token),
        None,
        body.as_ref().map(Value::to_string).as_deref(),
    )
    .await
}
async fn schedule(f: &Fixture) -> Vec<(Uuid, DateTime<Utc>)> {
    sqlx::query_as("SELECT id, start_time FROM event_missions WHERE event_id = $1 ORDER BY id")
        .bind(f.event)
        .fetch_all(&f.state.pool)
        .await
        .expect("the read of event_missions runs")
}

#[tokio::test]
async fn event_reschedule_preserves_active_offsets_and_removed_history_and_recomputes_rates() {
    let f = fixture().await;
    let original = schedule(&f).await;
    for delta in [Duration::hours(-3), Duration::hours(4)] {
        let target = f.initial + delta;
        let response = request(&f, "PATCH", Some(json!({"start_time":target}))).await;
        assert_eq!(response.0, StatusCode::OK, "{response:?}");
        for (id, actual) in schedule(&f).await {
            let previous = original.iter().find(|row| row.0 == id).unwrap().1;
            let expected = if id == f.attachments[2] {
                previous
            } else {
                previous + delta
            };
            assert_eq!(actual, expected);
        }
        let rate: f64 =
            sqlx::query_scalar("SELECT attendance_rate::float8 FROM users WHERE discord_id = $1")
                .bind(&f.actor)
                .fetch_one(&f.state.pool)
                .await
                .unwrap();
        assert_eq!(rate, if delta < Duration::zero() { 100.0 } else { 0.0 });
        let repeated = request(&f, "PATCH", Some(json!({"start_time":target}))).await;
        assert_eq!(repeated.0, StatusCode::OK);
        assert_eq!(repeated.1["start_time"], response.1["start_time"]);
    }
}

#[tokio::test]
async fn event_delete_and_cancel_release_reservations_without_erasing_attendance_or_signup_authorship()
 {
    for delete in [true, false] {
        let f = fixture().await;
        let authors: Vec<(Uuid, String, DateTime<Utc>)> = sqlx::query_as("SELECT r.id, r.discord_id, r.registered_at FROM event_registrations r
            JOIN event_missions em ON em.id = r.event_mission_id WHERE em.event_id = $1 ORDER BY r.id")
            .bind(f.event).fetch_all(&f.state.pool).await.unwrap();
        let response = if delete {
            request(&f, "DELETE", None).await
        } else {
            request(&f, "PATCH", Some(json!({"status":"cancelled"}))).await
        };
        assert_eq!(
            response.0,
            if delete {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::OK
            },
            "{response:?}"
        );
        let preserved: Vec<(Uuid, String, DateTime<Utc>)> = sqlx::query_as("SELECT r.id, r.discord_id, r.registered_at FROM event_registrations r
            JOIN event_missions em ON em.id = r.event_mission_id WHERE em.event_id = $1 ORDER BY r.id")
            .bind(f.event).fetch_all(&f.state.pool).await.unwrap();
        assert_eq!(preserved, authors);
        type ReleasedReservation = (String, Option<String>, Option<Uuid>, Option<String>);
        let rows: Vec<ReleasedReservation> = sqlx::query_as(
            "SELECT reservation_state::text, attendance_state::text, slot_id, release_reason FROM event_registrations WHERE event_mission_id = ANY($1)")
            .bind(&f.attachments[..2]).fetch_all(&f.state.pool).await.unwrap();
        for (reservation, attendance, slot, reason) in rows {
            assert_eq!(reservation, "withdrawn");
            assert_eq!(attendance.as_deref(), Some("attended"));
            assert_eq!(slot, None);
            assert_eq!(
                reason.as_deref(),
                Some(if delete {
                    "event_deleted"
                } else {
                    "event_cancelled"
                })
            );
        }
        let occupants: i64 = sqlx::query_scalar("SELECT count(*) FROM orbat_slots WHERE event_mission_id = ANY($1) AND assigned_to IS NOT NULL")
            .bind(&f.attachments[..2]).fetch_one(&f.state.pool).await.unwrap();
        assert_eq!(occupants, 0);
    }
}
