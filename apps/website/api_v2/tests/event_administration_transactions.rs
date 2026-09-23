//! Real event writes conserve participant capacity, schedules, authority and retained history.
use axum::{Router, http::StatusCode};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use std::time::Duration as Timeout;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};
mod common;
mod telemetry_support;

struct Fixture {
    state: AppState,
    app: Router,
    token: String,
    actor: String,
    other: String,
    event: Uuid,
    attachments: Vec<Uuid>,
    initial: DateTime<Utc>,
}
async fn fixture() -> Fixture {
    let url = common::require_test_database_url().expect("isolated PostgreSQL required");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "event-administration"));
    let actor = format!("event-admin-{}", Uuid::new_v4());
    let other = format!("event-other-{}", Uuid::new_v4());
    let token = common::access_token(
        &state,
        "event_administration_transactions",
        &actor,
        "admin",
        true,
    )
    .await;
    common::access_token(
        &state,
        "event_administration_transactions",
        &other,
        "enlisted",
        true,
    )
    .await;
    let (event, initial): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO events(name_override, start_time, status, max_slots, created_by)
        VALUES ('Transactional event', statement_timestamp() + interval '2 hours', 'open', 4, $1) RETURNING id, start_time")
        .bind(&actor).fetch_one(&state.pool).await.unwrap();
    let mut attachments = Vec::new();
    for index in 0..3 {
        let mission: Uuid = sqlx::query_scalar(
            "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
            VALUES ('Event transaction', $1, 'everon', 'pve_coop', 8, 'live') RETURNING id",
        )
        .bind(&actor)
        .fetch_one(&state.pool)
        .await
        .unwrap();
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
        .unwrap();
        let slot: Uuid = sqlx::query_scalar("INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to)
            VALUES ($1, 'USA', 'Alpha', 'Rifleman', 0, $2) RETURNING id")
            .bind(attachment).bind(&actor).fetch_one(&state.pool).await.unwrap();
        let mut fixture = state.pool.begin().await.unwrap();
        let allocation = common::participant_allocation(&mut fixture, attachment, &actor).await;
        sqlx::query("INSERT INTO event_registrations(event_mission_id, discord_id, slot_id, attendance_state, legacy_attendance_state, allocation_id)
            VALUES ($1, $2, $3, 'attended', 'attended', $4)")
            .bind(attachment).bind(&actor).bind(slot).bind(allocation).execute(&mut *fixture).await.unwrap();
        fixture.commit().await.unwrap();
        attachments.push(attachment);
    }
    let app = http_router::router(state.clone());
    Fixture {
        state,
        app,
        token,
        actor,
        other,
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
        .unwrap()
}
async fn register_other(f: &Fixture) {
    let mut fixture = f.state.pool.begin().await.unwrap();
    let allocation = common::participant_allocation(&mut fixture, f.attachments[0], &f.other).await;
    sqlx::query(
        "INSERT INTO event_registrations(event_mission_id, discord_id, allocation_id) VALUES ($1, $2, $3)
        ON CONFLICT(event_mission_id, discord_id) DO UPDATE SET reservation_state = 'registered',
        allocation_id = EXCLUDED.allocation_id",
    )
    .bind(f.attachments[0])
    .bind(&f.other)
    .bind(allocation)
    .execute(&mut *fixture)
    .await
    .unwrap();
    fixture.commit().await.unwrap();
}
async fn await_blocked(f: &Fixture, blocker: i32) {
    tokio::time::timeout(Timeout::from_secs(10), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("request must reach the held parent lock");
}

#[tokio::test]
async fn capacity_reduction_counts_distinct_allocated_participants_and_preserves_uncapped_zero() {
    let f = fixture().await;
    let response = request(&f, "PATCH", Some(json!({"max_slots":1}))).await;
    assert_eq!(
        response.0,
        StatusCode::OK,
        "one participant across two missions: {response:?}"
    );
    register_other(&f).await;
    let before = schedule(&f).await;
    let refused = request(
        &f,
        "PATCH",
        Some(json!({"max_slots":1,"start_time":f.initial + Duration::days(1)})),
    )
    .await;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{refused:?}");
    assert_eq!(
        schedule(&f).await,
        before,
        "conflict must reject the entire PATCH"
    );
    assert_eq!(
        request(&f, "PATCH", Some(json!({"max_slots":2}))).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&f, "PATCH", Some(json!({"max_slots":0}))).await.0,
        StatusCode::OK
    );
    for invalid in [
        json!(-1),
        json!(257),
        json!(i64::MAX),
        json!(false),
        json!("2"),
    ] {
        assert_eq!(
            request(&f, "PATCH", Some(json!({"max_slots":invalid})))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let other_token = common::access_token(
        &f.state,
        "event_administration_transactions",
        &f.other,
        "enlisted",
        true,
    )
    .await;
    let released = telemetry_support::call(
        &f.app,
        "DELETE",
        &format!("/api/v1/event-missions/{}/register", f.attachments[0]),
        Some(&other_token),
        None,
        None,
    )
    .await;
    assert_eq!(released.0, StatusCode::OK);
    assert_eq!(
        request(&f, "PATCH", Some(json!({"max_slots":1}))).await.0,
        StatusCode::OK
    );
    let history: (String, i64) = sqlx::query_as(
        "SELECT reservation_state::text,
        (SELECT count(*) FROM event_registration_history h WHERE h.registration_id = r.id)
        FROM event_registrations r WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(f.attachments[0])
    .bind(&f.other)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(history, ("withdrawn".into(), 2));
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

#[test]
fn generated_reschedules_preserve_exact_utc_offsets_and_normalize_microseconds_once() {
    use chrono::Timelike;
    use proptest::prelude::*;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    common::property_evidence::run_property(
        "event_schedule_offset_conservation",
        24,
        &proptest::collection::vec((-3600_i64..3600, 0_u32..1000), 1..5),
        |operations| {
            runtime.block_on(async {
                let f = fixture().await;
                let original = schedule(&f).await;
                for (seconds, nanos) in operations {
                    let requested = f.initial
                        + Duration::seconds(seconds)
                        + Duration::nanoseconds(i64::from(nanos));
                    let normalized = requested
                        .with_nanosecond(requested.nanosecond() / 1000 * 1000)
                        .unwrap();
                    let response =
                        request(&f, "PATCH", Some(json!({"start_time":requested}))).await;
                    prop_assert_eq!(response.0, StatusCode::OK, "{:?}", response);
                    let stored: DateTime<Utc> =
                        serde_json::from_value(response.1["start_time"].clone()).unwrap();
                    prop_assert_eq!(stored, normalized);
                    for (id, actual) in schedule(&f).await {
                        let prior = original.iter().find(|row| row.0 == id).unwrap().1;
                        let expected = if id == f.attachments[2] {
                            prior
                        } else {
                            normalized + (prior - f.initial)
                        };
                        prop_assert_eq!(actual, expected);
                    }
                }
                Ok(())
            })
        },
    );
}

#[tokio::test]
async fn event_schedule_range_error_rolls_back_all_dependent_changes() {
    let f = fixture().await;
    let before = schedule(&f).await;
    for timestamp in [
        "0000-01-01T00:00:00Z",
        "9999-12-31T23:59:59Z",
        "2027-01-01T23:59:60Z",
    ] {
        let response = request(
            &f,
            "PATCH",
            Some(json!({"start_time":timestamp,"name_override":"Rejected"})),
        )
        .await;
        assert_eq!(
            response.0,
            StatusCode::BAD_REQUEST,
            "{timestamp}: {response:?}"
        );
        assert_eq!(schedule(&f).await, before);
    }
    assert_eq!(
        request(&f, "GET", None).await.1["name_override"],
        json!("Transactional event")
    );
}

#[tokio::test]
async fn event_patch_audit_failure_rolls_back_schedule_capacity_and_statistics() {
    let f = fixture().await;
    let before = schedule(&f).await;
    let trigger = format!("fail_event_{}", Uuid::new_v4().simple());
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.target_id = '{}' AND NEW.action = 'event.updated' THEN RAISE EXCEPTION 'injected required audit failure'; END IF;
        RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE INSERT ON audit_logs FOR EACH ROW EXECUTE FUNCTION {trigger}();", f.event)))
        .execute(&f.state.pool).await.unwrap();
    let payload = json!({"start_time":f.initial - Duration::hours(3),"max_slots":1,"name_override":"Changed"});
    let failed = request(&f, "PATCH", Some(payload.clone())).await;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{failed:?}");
    assert_eq!(schedule(&f).await, before);
    let row: (i64, String, f64) = sqlx::query_as(
        "SELECT e.max_slots, e.name_override, u.attendance_rate::float8 FROM events e
        JOIN users u ON u.discord_id = e.created_by WHERE e.id = $1",
    )
    .bind(f.event)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(row, (4, "Transactional event".into(), 0.0));
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {trigger} ON audit_logs; DROP FUNCTION {trigger}();"
    )))
    .execute(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(request(&f, "PATCH", Some(payload)).await.0, StatusCode::OK);
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs a JOIN audit_publication_pending o ON o.audit_id = a.id
        WHERE a.target_id = $1 AND a.action = 'event.updated'",
    )
    .bind(f.event.to_string())
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(
        audits, 1,
        "failed transaction must not leave duplicate or missing delivery entries"
    );
}

#[tokio::test]
async fn capacity_patch_waiter_rechecks_the_committed_participant_allocation() {
    let f = fixture().await;
    let mut owner = f.state.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
        .bind(f.event)
        .fetch_one(&mut *owner)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *owner)
        .await
        .unwrap();
    let patch = request(&f, "PATCH", Some(json!({"max_slots":1})));
    let allocation = async {
        await_blocked(&f, pid).await;
        let allocation =
            common::participant_allocation(&mut owner, f.attachments[0], &f.other).await;
        sqlx::query("INSERT INTO event_registrations(event_mission_id, discord_id, allocation_id) VALUES ($1,$2,$3)")
            .bind(f.attachments[0])
            .bind(&f.other)
            .bind(allocation)
            .execute(&mut *owner)
            .await
            .unwrap();
        owner.commit().await.unwrap();
    };
    let (response, ()) = tokio::time::timeout(Timeout::from_secs(20), async {
        tokio::join!(patch, allocation)
    })
    .await
    .unwrap();
    assert_eq!(
        response.0,
        StatusCode::CONFLICT,
        "capacity must include the writer it waited for: {response:?}"
    );
}

#[tokio::test]
async fn event_patch_waiter_rechecks_ban_and_terminal_status() {
    for ban in [true, false] {
        let f = fixture().await;
        let mut owner = f.state.pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
            .bind(f.event)
            .fetch_one(&mut *owner)
            .await
            .unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *owner)
            .await
            .unwrap();
        let patch = request(
            &f,
            "PATCH",
            Some(json!({"status":"open","name_override":"Stale request"})),
        );
        let revoke = async {
            await_blocked(&f, pid).await;
            if ban {
                sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
                    .bind(&f.actor)
                    .execute(&mut *owner)
                    .await
                    .unwrap();
            } else {
                sqlx::query("UPDATE events SET status = 'cancelled' WHERE id = $1")
                    .bind(f.event)
                    .execute(&mut *owner)
                    .await
                    .unwrap();
            }
            owner.commit().await.unwrap();
        };
        let (response, ()) = tokio::time::timeout(Timeout::from_secs(20), async {
            tokio::join!(patch, revoke)
        })
        .await
        .unwrap();
        assert_eq!(
            response.0,
            if ban {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::CONFLICT
            },
            "{response:?}"
        );
        let name: String = sqlx::query_scalar("SELECT name_override FROM events WHERE id = $1")
            .bind(f.event)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
        assert_eq!(name, "Transactional event");
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

#[tokio::test]
async fn completed_events_remain_terminal_when_schedules_or_attachments_change_before_or_after_sweep()
 {
    use website_api::operations::services::event_lifecycle_sweep::sweep_once;
    for swept in [false, true] {
        for action in ["schedule", "attach", "restore"] {
            let f = fixture().await;
            sqlx::query("UPDATE events SET start_time = statement_timestamp() - interval '10 hours' WHERE id = $1")
                .bind(f.event).execute(&f.state.pool).await.unwrap();
            sqlx::query("UPDATE event_missions SET start_time = statement_timestamp() - interval '10 hours' WHERE event_id = $1")
                .bind(f.event).execute(&f.state.pool).await.unwrap();
            if swept {
                let (started, completed) = sweep_once(&f.state.pool).await.unwrap();
                assert!(
                    started.contains(&f.event) && completed.contains(&f.event),
                    "the swept branch must execute both transitions"
                );
            }
            let stored: String =
                sqlx::query_scalar("SELECT status::text FROM events WHERE id = $1")
                    .bind(f.event)
                    .fetch_one(&f.state.pool)
                    .await
                    .unwrap();
            assert_eq!(
                stored,
                if swept { "completed" } else { "open" },
                "paired cases require distinct persisted starting states"
            );
            let before = request(&f, "GET", None).await;
            assert_eq!(before.1["status"], json!("completed"));
            let response = if action == "schedule" {
                request(&f, "PATCH", Some(json!({"start_time":f.initial}))).await
            } else {
                let mission: Uuid = if action == "restore" {
                    sqlx::query_scalar("SELECT mission_id FROM event_missions WHERE id = $1")
                        .bind(f.attachments[2])
                        .fetch_one(&f.state.pool)
                        .await
                        .unwrap()
                } else {
                    sqlx::query_scalar("INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
                        VALUES ('Terminal history', $1, 'everon', 'pve_coop', 1, 'live') RETURNING id")
                        .bind(&f.actor).fetch_one(&f.state.pool).await.unwrap()
                };
                telemetry_support::call(
                    &f.app,
                    "POST",
                    &format!("/api/v1/events/{}/missions", f.event),
                    Some(&f.token),
                    None,
                    Some(
                        &json!({"mission_id":mission,"start_time":f.initial,
                        "orbat":[{"faction":"USA","squad":"Alpha","slots":[{"role":"Rifleman"}]}]})
                        .to_string(),
                    ),
                )
                .await
            };
            assert!(response.0.is_success(), "{swept}/{action}: {response:?}");
            let after = request(&f, "GET", None).await;
            assert_eq!(after.1["status"], json!("completed"), "{swept}/{action}");
            let edges: Vec<String> = sqlx::query_scalar(
                "SELECT action FROM audit_logs WHERE target_id = $1
                AND action IN ('event.auto_live','event.auto_completed') ORDER BY id",
            )
            .bind(f.event.to_string())
            .fetch_all(&f.state.pool)
            .await
            .unwrap();
            assert_eq!(edges, ["event.auto_live", "event.auto_completed"]);
        }
    }
}
