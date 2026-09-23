//! HTTP and PostgreSQL barriers verify attendance cannot create or restore reservations.
use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use std::time::Duration;
use uuid::Uuid;
use website_api::{
    core::{application_state::AppState, configuration::Config, database, http_router},
    identity_and_access::services::{
        identity_linking::{confirm_identity, unlink_identity},
        link_code_issuance::issue_link_code,
        session_authorization::authorize_session,
    },
};
mod common;
mod telemetry_support;

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
}
async fn fixture() -> Fixture {
    let url = common::require_test_database_url().expect("scratch PostgreSQL required");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "reservation-attendance"));
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
        .unwrap();
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
        VALUES ('Attendance mission', $1, 'everon', 'pve_coop', 2, 'live') RETURNING id",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, created_by, max_slots)
        VALUES ('Attendance event', now() + interval '1 hour', 'open', $1, 1) RETURNING id",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let attachment: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time)
        VALUES ($1, $2, now() + interval '1 hour') RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let slot: Uuid = sqlx::query_scalar(
        "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to)
        VALUES ($1, 'USA', 'Alpha', 'Rifleman', 0, $2) RETURNING id",
    )
    .bind(attachment)
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let mut fixture = state.pool.begin().await.unwrap();
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
    .unwrap();
    fixture.commit().await.unwrap();
    let app = http_router::router(state.clone());
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
    }
}
async fn results(f: &Fixture, outcome: &str, event: Uuid) -> (StatusCode, Value) {
    telemetry_support::call(&f.app, "POST", "/api/v1/ingest/match-results", None, Some(telemetry_support::SVC),
        Some(&json!({"match": {"source_match_id": f.source, "outcome": outcome, "event_id": event, "mission_id": f.mission},
            "players": [{"arma_id": f.arma, "source_event_id": "one", "role_played": "rifleman"}]}).to_string())).await
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
    .unwrap()
}
async fn moved_event(f: &Fixture) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, created_by)
        VALUES ('Corrected event', now(), 'open', $1) RETURNING id",
    )
    .bind(&f.actor)
    .fetch_one(&f.state.pool)
    .await
    .unwrap()
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
    confirm_identity(&f.state, &code, &f.arma, "Player")
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
#[tokio::test]
async fn withdrawal_and_finalization_serialize_at_the_account_barrier() {
    let f = fixture().await;
    let mut blocker = f.state.pool.begin().await.unwrap();
    sqlx::query("SELECT discord_id FROM users WHERE discord_id = $1 FOR NO KEY UPDATE")
        .bind(&f.actor)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let barrier = tokio::sync::Barrier::new(3);
    let report = async {
        barrier.wait().await;
        results(&f, "success", f.event).await
    };
    let release = async {
        barrier.wait().await;
        withdraw(&f).await
    };
    let unlock = async {
        barrier.wait().await;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let blocked: i64 = sqlx::query_scalar(
                    "WITH RECURSIVE blocked(pid) AS (SELECT $1::integer UNION
                    SELECT activity.pid FROM pg_stat_activity activity JOIN blocked
                    ON blocked.pid = ANY(pg_blocking_pids(activity.pid))
                    WHERE activity.datname = current_database())
                    SELECT count(*) FROM blocked WHERE pid <> $1",
                )
                .bind(pid)
                .fetch_one(&f.state.pool)
                .await
                .unwrap();
                if blocked >= 2 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("both production transactions must reach the held account lock");
        blocker.commit().await.unwrap();
    };
    let (report, release, ()) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(report, release, unlock)
    })
    .await
    .expect("attendance and withdrawal complete without deadlock");
    assert_eq!(report.0, StatusCode::OK, "{report:?}");
    assert_eq!(release.0, StatusCode::OK, "{release:?}");
    assert_eq!(
        snapshot(&f).await,
        ("withdrawn".into(), Some("attended".into()), None, 1)
    );
}
#[tokio::test]
async fn late_aggregate_failure_rolls_back_participation_and_retry_recovers_once() {
    let f = fixture().await;
    let trigger = format!("reject_attendance_{}", Uuid::new_v4().simple());
    assert!(
        f.actor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    );
    let statement = format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        RAISE EXCEPTION 'injected aggregate failure'; RETURN NEW; END $$;
        CREATE TRIGGER {trigger} BEFORE UPDATE OF total_deployments ON users
        FOR EACH ROW WHEN (NEW.discord_id = '{}') EXECUTE FUNCTION {trigger}();",
        f.actor
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(statement))
        .execute(&f.state.pool)
        .await
        .unwrap();
    let before = snapshot(&f).await;
    let failed = results(&f, "success", f.event).await;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {trigger} ON users; DROP FUNCTION {trigger}();"
    )))
    .execute(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{failed:?}");
    assert_eq!(snapshot(&f).await, before);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM matches WHERE source_match_id = $1")
        .bind(&f.source)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    for _ in 0..2 {
        assert_eq!(results(&f, "success", f.event).await.0, StatusCode::OK);
    }
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
        "../../../../contracts_v2/definitions/reservation-response.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    assert!(validator.is_valid(value));
    let generated: website_api::operations::models::generated::reservation_response::ReservationResponse = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(generated).unwrap(), *value);
    let backend: website_api::operations::models::reservation_response::ReservationResponse =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(backend).unwrap(), *value);
    let golden: Value = serde_json::from_str(include_str!(
        "../../frontend/tests/fixtures/api/POST__event-missions__register.json"
    ))
    .unwrap();
    let mut normalized = value.clone();
    normalized["slot_id"] = golden["slot_id"].clone();
    assert_eq!(
        normalized, golden,
        "live response and frontend golden agree apart from fixture UUID"
    );
    for key in ["state", "reservation_state", "attendance_state", "slot_id"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(key);
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

#[test]
fn generated_attendance_corrections_preserve_independent_reservations() {
    use proptest::prelude::*;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    common::property_evidence::run_property(
        "attendance_reservation_independence",
        24,
        &(0_u8..4, proptest::collection::vec(0_u8..4, 1..6)),
        |(reservation, operations)| {
            runtime.block_on(async {
            let f = fixture().await;
            let reservation = ["registered", "waitlisted", "withdrawn", "legacy_unknown"][reservation as usize];
            let slot = (reservation == "registered").then_some(f.slot);
            // Reservation state, seat and allocation change together, as every writer does.
            let mut setup = f.state.pool.begin().await.unwrap();
            sqlx::query("UPDATE event_registrations SET reservation_state = $2::registration_state, slot_id = $3,
                allocation_id = CASE WHEN $2 IN ('registered', 'legacy_unknown') THEN allocation_id END WHERE id = $1")
                .bind(f.registration).bind(reservation).bind(slot).execute(&mut *setup).await.unwrap();
            if slot.is_none() {
                sqlx::query("UPDATE orbat_slots SET assigned_to = NULL WHERE id = $1")
                    .bind(f.slot).execute(&mut *setup).await.unwrap();
            }
            sqlx::query("UPDATE event_participant_allocations SET released_at = clock_timestamp(),
                release_reason = 'fixture_reservation_state' WHERE event_id = $1 AND discord_id = $2
                AND released_at IS NULL AND $3 NOT IN ('registered', 'legacy_unknown')")
                .bind(f.event).bind(&f.actor).bind(reservation).execute(&mut *setup).await.unwrap();
            setup.commit().await.unwrap();
            let other = moved_event(&f).await;
            let mut finalized = false;
            let mut attended = false;
            for operation in operations {
                let outcome = match operation {0 => "pending", 3 => "aborted", _ => "success"};
                let event = if operation == 2 {other} else {f.event};
                let response = results(&f, outcome, event).await;
                if finalized && operation == 0 {
                    prop_assert_eq!(response.0, StatusCode::CONFLICT);
                } else {
                    prop_assert_eq!(response.0, StatusCode::OK, "{:?}", response);
                    finalized |= operation != 0;
                    attended = finalized && event == f.event;
                }
                let actual = snapshot(&f).await;
                prop_assert_eq!(actual.0.as_str(), reservation);
                prop_assert_eq!(actual.1.as_deref(), attended.then_some("attended"));
                prop_assert_eq!(actual.2, slot);
                prop_assert_eq!(actual.3, i64::from(attended));
            }
            Ok(())
        })
        },
    );
}

#[tokio::test]
async fn correction_retracts_old_signup_attribution_after_identity_moves() {
    let f = fixture().await;
    assert_eq!(results(&f, "success", f.event).await.0, StatusCode::OK);
    let original_user = authorize_session(
        &f.state.pool,
        &f.state.cfg,
        &f.state.jwt.parse(&f.token).unwrap(),
    )
    .await
    .unwrap();
    unlink_identity(&f.state, &original_user).await.unwrap();
    assert_eq!(snapshot(&f).await.1.as_deref(), Some("attended"));
    let new_actor = format!("new-owner-{}", Uuid::new_v4());
    let access = common::access_token(
        &f.state,
        "reservation_attendance_transactions",
        &new_actor,
        "enlisted",
        false,
    )
    .await;
    let new_user = authorize_session(
        &f.state.pool,
        &f.state.cfg,
        &f.state.jwt.parse(&access).unwrap(),
    )
    .await
    .unwrap();
    let code = issue_link_code(&f.state, &new_user).await.unwrap().0;
    confirm_identity(&f.state, &code, &f.arma, "New owner")
        .await
        .unwrap();
    assert_eq!(
        snapshot(&f).await.1.as_deref(),
        Some("attended"),
        "current attribution cannot erase factual signup participation"
    );
    let other = moved_event(&f).await;
    assert_eq!(results(&f, "success", other).await.0, StatusCode::OK);
    assert_eq!(
        snapshot(&f).await,
        ("registered".into(), None, Some(f.slot), 0)
    );
}

#[tokio::test]
async fn removed_mission_hides_operational_views_and_restores_without_erasing_history() {
    let f = fixture().await;
    assert_eq!(results(&f, "success", f.event).await.0, StatusCode::OK);
    let removed = telemetry_support::call(
        &f.app,
        "DELETE",
        &format!("/api/v1/events/{}/missions/{}", f.event, f.attachment),
        Some(&f.token),
        None,
        None,
    )
    .await;
    assert_eq!(removed.0, StatusCode::NO_CONTENT, "{removed:?}");
    assert_eq!(
        snapshot(&f).await,
        ("withdrawn".into(), Some("attended".into()), None, 1)
    );
    let hidden = telemetry_support::call(
        &f.app,
        "GET",
        &format!("/api/v1/event-missions/{}/orbat", f.attachment),
        Some(&f.token),
        None,
        None,
    )
    .await;
    assert_eq!(hidden.0, StatusCode::NOT_FOUND);
    let hub = telemetry_support::call(
        &f.app,
        "GET",
        &format!("/api/v1/events/{}", f.event),
        Some(&f.token),
        None,
        None,
    )
    .await;
    assert_eq!(hub.0, StatusCode::OK);
    assert_eq!(hub.1["missions"], json!([]));
    let orders = telemetry_support::call(
        &f.app,
        "GET",
        "/api/v1/me/deployments",
        Some(&f.token),
        None,
        None,
    )
    .await;
    assert_eq!(orders.0, StatusCode::OK);
    assert_eq!(orders.1["upcoming"], json!([]));
    let mut restore = json!({"mission_id": f.mission, "start_time": (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
        "orbat": [{"faction": "USA", "squad": "Alpha", "slots": [{"role": "Medic"}]}]});
    let mismatch = telemetry_support::call(
        &f.app,
        "POST",
        &format!("/api/v1/events/{}/missions", f.event),
        Some(&f.token),
        None,
        Some(&restore.to_string()),
    )
    .await;
    assert_eq!(
        mismatch.0,
        StatusCode::CONFLICT,
        "different authored data cannot be silently discarded: {mismatch:?}"
    );
    restore["orbat"][0]["slots"][0]["role"] = json!("Rifleman");
    let restored = telemetry_support::call(
        &f.app,
        "POST",
        &format!("/api/v1/events/{}/missions", f.event),
        Some(&f.token),
        None,
        Some(&restore.to_string()),
    )
    .await;
    assert_eq!(restored.0, StatusCode::CREATED, "{restored:?}");
    assert_eq!(restored.1["id"], json!(f.attachment));
    assert_eq!(
        snapshot(&f).await,
        ("withdrawn".into(), Some("attended".into()), None, 1)
    );
    let original_slot: (String, Option<String>) =
        sqlx::query_as("SELECT role, assigned_to FROM orbat_slots WHERE id = $1")
            .bind(f.slot)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    assert_eq!(original_slot, ("Rifleman".into(), None));
    // Moving finalized history across the schedule boundary updates the cache before success.
    for (hours, expected) in [(-1, 100.0), (1, 0.0)] {
        let removed = telemetry_support::call(
            &f.app,
            "DELETE",
            &format!("/api/v1/events/{}/missions/{}", f.event, f.attachment),
            Some(&f.token),
            None,
            None,
        )
        .await;
        assert_eq!(removed.0, StatusCode::NO_CONTENT);
        restore["start_time"] =
            json!((chrono::Utc::now() + chrono::Duration::hours(hours)).to_rfc3339());
        let restored = telemetry_support::call(
            &f.app,
            "POST",
            &format!("/api/v1/events/{}/missions", f.event),
            Some(&f.token),
            None,
            Some(&restore.to_string()),
        )
        .await;
        assert_eq!(restored.0, StatusCode::CREATED, "{restored:?}");
        let stored: f64 =
            sqlx::query_scalar("SELECT attendance_rate::float8 FROM users WHERE discord_id = $1")
                .bind(&f.actor)
                .fetch_one(&f.state.pool)
                .await
                .unwrap();
        assert_eq!(
            stored, expected,
            "restoration must recompute affected accounts transactionally"
        );
        assert_eq!(
            snapshot(&f).await,
            ("withdrawn".into(), Some("attended".into()), None, 1)
        );
    }
}

#[tokio::test]
async fn registration_waiting_for_parent_rechecks_committed_mission_removal() {
    let f = fixture().await;
    let before = snapshot(&f).await;
    let mut removal = f.state.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
        .bind(f.event)
        .fetch_one(&mut *removal)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *removal)
        .await
        .unwrap();
    let barrier = tokio::sync::Barrier::new(2);
    let registration = async {
        barrier.wait().await;
        telemetry_support::call(
            &f.app,
            "POST",
            &format!("/api/v1/event-missions/{}/register", f.attachment),
            Some(&f.token),
            None,
            Some(&json!({"slot_id":f.slot}).to_string()),
        )
        .await
    };
    let commit = async {
        barrier.wait().await;
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let waiting: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                    WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))",
                )
                .bind(pid)
                .fetch_one(&f.state.pool)
                .await
                .unwrap();
                if waiting {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("registration must pass its initial lookup and reach the event lock");
        sqlx::query("UPDATE event_missions SET deleted_at = clock_timestamp() WHERE id = $1")
            .bind(f.attachment)
            .execute(&mut *removal)
            .await
            .unwrap();
        removal.commit().await.unwrap();
    };
    let (response, ()) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(registration, commit)
    })
    .await
    .expect("removal race terminates");
    assert_eq!(response.0, StatusCode::NOT_FOUND, "{response:?}");
    assert_eq!(
        snapshot(&f).await,
        before,
        "rejected stale lookup cannot alter signup or occupancy"
    );
}
