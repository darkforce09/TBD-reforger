//! HTTP and PostgreSQL barriers verify attendance cannot create or restore reservations.
use crate::{common, telemetry_support};

use api_caller_identity::session_authorization::authorize_session;
use api_configuration::configuration::Config;
use api_identity_and_access::services::{
    identity_linking::{confirm_identity, unlink_identity},
    link_code_issuance::issue_link_code,
};
use api_server::router::router;
use api_state::AppState;
use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use uuid::Uuid;

use telemetry_support::match_reports::ReportingServer;

struct Fixture {
    state: AppState,
    app: Router,
    token: String,
    actor: String,
    arma: String,
    event: Uuid,
    mission: Uuid,
    attachment: Uuid,
    slot: Uuid,
    registration: Uuid,
    source: String,
    /// The game server that registers and reports `source`.
    reporter: ReportingServer,
}
async fn fixture() -> Fixture {
    let url = common::require_test_database_url().expect("scratch PostgreSQL required");
    let pool = api_database::connect(&url)
        .await
        .expect("the test database accepts a connection");
    api_database::migrate(&pool)
        .await
        .expect("the migrations apply to the test database");
    let state = api_server::composition::application_state(
        pool,
        Config::for_tests(url, "reservation-attendance"),
    );
    let actor = format!("attendance-{}", Uuid::new_v4());
    let token = common::access_token(
        &state,
        "reservation_attendance_transactions",
        &actor,
        "admin",
        true,
    )
    .await;
    let arma: String = sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
        .bind(&actor)
        .fetch_one(&state.pool)
        .await
        .expect("the read of users returns a row");
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
        VALUES ('Attendance mission', $1, 'everon', 'pve_coop', 2, 'live') RETURNING id",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .expect("the insert into missions returns its row");
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, created_by, max_slots)
        VALUES ('Attendance event', now() + interval '1 hour', 'open', $1, 1) RETURNING id",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .expect("the insert into events returns its row");
    let attachment: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time)
        VALUES ($1, $2, now() + interval '1 hour') RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(&state.pool)
    .await
    .expect("the insert into event_missions returns its row");
    let slot: Uuid = sqlx::query_scalar(
        "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to)
        VALUES ($1, 'USA', 'Alpha', 'Rifleman', 0, $2) RETURNING id",
    )
    .bind(attachment)
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .expect("the insert into orbat_slots returns its row");
    let mut fixture = state.pool.begin().await.expect("a transaction begins");
    let allocation = common::participant_allocation(&mut fixture, attachment, &actor).await;
    let registration: Uuid = sqlx::query_scalar(
        "INSERT INTO event_registrations(event_mission_id, discord_id, slot_id, allocation_id)
        VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(attachment)
    .bind(&actor)
    .bind(slot)
    .bind(allocation)
    .fetch_one(&mut *fixture)
    .await
    .expect("the insert into event_registrations returns its row");
    fixture.commit().await.expect("the transaction commits");
    let app = router(state.clone());
    let reporter = ReportingServer::open(&app, &state.pool, "Reservation attendance server").await;
    Fixture {
        state,
        app,
        token,
        actor,
        arma,
        event,
        mission,
        attachment,
        slot,
        registration,
        source: Uuid::new_v4().to_string(),
        reporter,
    }
}
/// Report the fixture's line for `source` as the match's next results revision.
async fn results(f: &Fixture, outcome: &str, event: Uuid) -> (StatusCode, Value) {
    f.reporter
        .report_results(
            &f.app,
            &json!({"match": {"source_match_id": f.source, "outcome": outcome, "event_id": event, "mission_id": f.mission},
                "players": [{"arma_id": f.arma, "source_event_id": "one", "role_played": "rifleman"}]}),
        )
        .await
}
async fn withdraw(f: &Fixture) -> (StatusCode, Value) {
    telemetry_support::call(
        &f.app,
        "DELETE",
        &format!("/api/v1/event-missions/{}/register", f.attachment),
        Some(&f.token),
        None,
        None,
    )
    .await
}
async fn snapshot(f: &Fixture) -> (String, Option<String>, Option<Uuid>, i64) {
    sqlx::query_as(
        "SELECT reservation_state::text, attendance_state::text, slot_id,
        (SELECT count(*) FROM event_registration_participation WHERE registration_id = $1)
        FROM event_registrations WHERE id = $1",
    )
    .bind(f.registration)
    .fetch_one(&f.state.pool)
    .await
    .expect("the read of event_registration_participation returns a row")
}
async fn moved_event(f: &Fixture) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, created_by)
        VALUES ('Corrected event', now(), 'open', $1) RETURNING id",
    )
    .bind(&f.actor)
    .fetch_one(&f.state.pool)
    .await
    .expect("the insert into events returns its row")
}
#[tokio::test]
async fn withdrawal_finalization_correction_and_relink_preserve_reservation_history() {
    let f = fixture().await;
    let original: (Uuid, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT id, registered_at FROM event_registrations WHERE id = $1")
            .bind(f.registration)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    assert_eq!(results(&f, "pending", f.event).await.0, StatusCode::OK);
    assert_eq!(
        snapshot(&f).await,
        ("registered".into(), None, Some(f.slot), 0)
    );
    assert_eq!(withdraw(&f).await.0, StatusCode::OK);
    assert_eq!(results(&f, "success", f.event).await.0, StatusCode::OK);
    assert_eq!(
        snapshot(&f).await,
        ("withdrawn".into(), Some("attended".into()), None, 1)
    );
    let user = authorize_session(
        &f.state.pool,
        &f.state.cfg,
        &f.state.jwt.parse(&f.token).unwrap(),
    )
    .await
    .unwrap();
    unlink_identity(&f.state, &user).await.unwrap();
    let code = issue_link_code(&f.state, &user).await.unwrap().0;
    confirm_identity(
        &f.state,
        f.reporter.server_id.into(),
        &code,
        &api_identifiers::ArmaPlayerId::new(f.arma.as_str()),
        "Player",
    )
    .await
    .unwrap();
    assert_eq!(
        snapshot(&f).await,
        ("withdrawn".into(), Some("attended".into()), None, 1)
    );
    let regression = results(&f, "pending", f.event).await;
    assert_eq!(regression.0, StatusCode::CONFLICT, "{regression:?}");
    let other = moved_event(&f).await;
    assert_eq!(results(&f, "success", other).await.0, StatusCode::OK);
    assert_eq!(snapshot(&f).await, ("withdrawn".into(), None, None, 0));
    let preserved: (Uuid, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT id, registered_at FROM event_registrations WHERE id = $1")
            .bind(f.registration)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    assert_eq!(preserved, original);
    let history: Vec<(String, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT reservation_state::text, slot_id, release_reason FROM event_registration_history WHERE registration_id = $1 ORDER BY id")
        .bind(f.registration).fetch_all(&f.state.pool).await.unwrap();
    assert_eq!(
        history,
        vec![
            ("registered".into(), Some(f.slot), None),
            (
                "withdrawn".into(),
                None,
                Some("participant_withdrew".into())
            )
        ]
    );
}
#[tokio::test]
async fn attendance_keeps_capacity_and_reregistration_returns_both_states() {
    let f = fixture().await;
    assert_eq!(results(&f, "success", f.event).await.0, StatusCode::OK);
    let other = format!("second-{}", Uuid::new_v4());
    let token = common::access_token(
        &f.state,
        "reservation_attendance_transactions",
        &other,
        "enlisted",
        true,
    )
    .await;
    let queued = telemetry_support::call(
        &f.app,
        "POST",
        &format!("/api/v1/event-missions/{}/register", f.attachment),
        Some(&token),
        None,
        Some(r#"{"slot_id":""}"#),
    )
    .await;
    // Attendance does not release event capacity: the second participant can only wait.
    assert_eq!(queued.0, StatusCode::OK, "{queued:?}");
    assert_eq!(
        queued.1["reservation_state"], "waitlisted",
        "attendance does not release event capacity: {queued:?}"
    );
    let explicit = telemetry_support::call(
        &f.app,
        "POST",
        &format!("/api/v1/event-missions/{}/register", f.attachment),
        Some(&token),
        None,
        Some(&json!({"slot_id":f.slot}).to_string()),
    )
    .await;
    assert_eq!(explicit.0, StatusCode::CONFLICT, "{explicit:?}");
    let left_queue = telemetry_support::call(
        &f.app,
        "DELETE",
        &format!("/api/v1/event-missions/{}/register", f.attachment),
        Some(&token),
        None,
        None,
    )
    .await;
    assert_eq!(left_queue.0, StatusCode::OK, "{left_queue:?}");
    assert_eq!(withdraw(&f).await.0, StatusCode::OK);
    let accepted = telemetry_support::call(
        &f.app,
        "POST",
        &format!("/api/v1/event-missions/{}/register", f.attachment),
        Some(&f.token),
        None,
        Some(&json!({"slot_id":f.slot}).to_string()),
    )
    .await;
    assert_eq!(accepted.0, StatusCode::OK, "{accepted:?}");
    assert_eq!(accepted.1["state"], "attended");
    assert_eq!(accepted.1["reservation_state"], "registered");
    assert_eq!(accepted.1["attendance_state"], "attended");
    assert_response_contract(&accepted.1);
    assert_eq!(
        snapshot(&f).await,
        (
            "registered".into(),
            Some("attended".into()),
            Some(f.slot),
            1
        )
    );
}

fn assert_response_contract(value: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../contracts/definitions/reservation-response.schema.json"
    ))
    .expect("the text decodes as JSON");
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("the contract schema compiles into a validator");
    assert!(validator.is_valid(value));
    let generated: contract_schema_types::operations::reservation_response::ReservationResponse =
        serde_json::from_value(value.clone())
            .expect("the JSON value decodes into the expected type");
    assert_eq!(
        serde_json::to_value(generated).expect("the value serialises to JSON"),
        *value
    );
    let backend: api_operations::models::reservation_response::ReservationResponse =
        serde_json::from_value(value.clone())
            .expect("the JSON value decodes into the expected type");
    assert_eq!(
        serde_json::to_value(backend).expect("the value serialises to JSON"),
        *value
    );
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../../contracts/fixtures/api_goldens/POST__event-missions__register.json"
    ))
    .expect("the text decodes as JSON");
    let mut normalized = value.clone();
    normalized["slot_id"] = golden["slot_id"].clone();
    assert_eq!(
        normalized, golden,
        "live response and frontend golden agree apart from fixture UUID"
    );
    for key in ["state", "reservation_state", "attendance_state", "slot_id"] {
        let mut missing = value.clone();
        missing
            .as_object_mut()
            .expect("the response contract document is a JSON object")
            .remove(key);
        assert!(!validator.is_valid(&missing), "missing {key}");
    }
    for invalid in [
        json!("no_show"),
        json!("unknown"),
        json!(null),
        json!(0),
        json!(false),
    ] {
        let mut changed = value.clone();
        changed["reservation_state"] = invalid;
        assert!(!validator.is_valid(&changed));
    }
    let mut unknown = value.clone();
    unknown["unexpected"] = json!(true);
    assert!(!validator.is_valid(&unknown));
}
