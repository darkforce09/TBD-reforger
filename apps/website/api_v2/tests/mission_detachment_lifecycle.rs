//! HTTP lifecycle operations distinguish retained participation from active event attachments.

use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{sync::Mutex, time::timeout};
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};

mod common;
mod telemetry_support;

static CATALOG_RACES: Mutex<()> = Mutex::const_new(());
const DEADLINE: Duration = Duration::from_secs(10);

struct Fixture {
    state: AppState,
    app: Router,
    token: String,
    actor: String,
    title: String,
    event: Uuid,
    mission: Uuid,
    starts_at: String,
}

impl Fixture {
    async fn new() -> Self {
        let url =
            common::require_test_database_url().expect("mission lifecycle requires PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let state = AppState::new(pool, Config::for_tests(url, "mission-detachment"));
        let actor = format!("mission-detachment-{}", Uuid::new_v4());
        let token = common::access_token(
            &state,
            "mission_detachment_lifecycle",
            &actor,
            "admin",
            true,
        )
        .await;
        let app = http_router::router(state.clone());
        let title = format!("Retained mission {}", Uuid::new_v4());
        let created = telemetry_support::call(
            &app,
            "POST",
            "/api/v1/missions",
            Some(&token),
            None,
            Some(
                &json!({"title":title,"terrain":"everon","game_mode":"pve_coop","max_players":2})
                    .to_string(),
            ),
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED, "{created:?}");
        let mission = created.1["id"].as_str().unwrap().parse().unwrap();
        let starts_at = (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339();
        let created = telemetry_support::call(&app, "POST", "/api/v1/events", Some(&token), None,
            Some(&json!({"name_override":"Detachment fixture","start_time":starts_at,"status":"open"}).to_string())).await;
        assert_eq!(created.0, StatusCode::CREATED, "{created:?}");
        let event = created.1["id"].as_str().unwrap().parse().unwrap();
        Self {
            state,
            app,
            token,
            actor,
            title,
            event,
            mission,
            starts_at,
        }
    }

    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        telemetry_support::call(
            &self.app,
            method,
            uri,
            Some(&self.token),
            None,
            body.as_ref().map(Value::to_string).as_deref(),
        )
        .await
    }

    async fn attach(&self) -> (StatusCode, Value) {
        self.call("POST", &format!("/api/v1/events/{}/missions", self.event), Some(json!({
            "mission_id":self.mission,"start_time":self.starts_at,
            "orbat":[{"faction":"USA","squad":"Alpha","callsign":"A","slots":[{"role":"Rifleman"}]}]
        }))).await
    }

    async fn attach_with_attendance(&self) -> (Uuid, Uuid) {
        let attached = self.attach().await;
        assert_eq!(attached.0, StatusCode::CREATED, "{attached:?}");
        let attachment: Uuid = attached.1["id"].as_str().unwrap().parse().unwrap();
        let slot: Uuid =
            sqlx::query_scalar("SELECT id FROM orbat_slots WHERE event_mission_id = $1")
                .bind(attachment)
                .fetch_one(&self.state.pool)
                .await
                .unwrap();
        let registered = self
            .call(
                "POST",
                &format!("/api/v1/event-missions/{attachment}/register"),
                Some(json!({"slot_id":slot})),
            )
            .await;
        assert_eq!(registered.0, StatusCode::OK, "{registered:?}");
        let registration: Uuid = sqlx::query_scalar(
            "SELECT id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(attachment)
        .bind(&self.actor)
        .fetch_one(&self.state.pool)
        .await
        .unwrap();
        let arma: String = sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(&self.actor)
            .fetch_one(&self.state.pool)
            .await
            .unwrap();
        let ingested = telemetry_support::call(&self.app, "POST", "/api/v1/ingest/match-results", None, Some(telemetry_support::SVC),
            Some(&json!({"match":{"source_match_id":Uuid::new_v4().to_string(),"event_id":self.event,"mission_id":self.mission,"outcome":"success"},
                "players":[{"arma_id":arma,"source_event_id":"participation","role_played":"Rifleman","kills":4,"deaths":1}]}).to_string())).await;
        assert_eq!(ingested.0, StatusCode::OK, "{ingested:?}");
        (attachment, registration)
    }

    async fn detach(&self, attachment: Uuid) {
        let removed = self
            .call(
                "DELETE",
                &format!("/api/v1/events/{}/missions/{attachment}", self.event),
                None,
            )
            .await;
        assert_eq!(removed.0, StatusCode::NO_CONTENT, "{removed:?}");
    }

    async fn lifecycle(&self, delete: bool) -> (StatusCode, Value) {
        self.call(
            if delete { "DELETE" } else { "PATCH" },
            &format!("/api/v1/missions/{}", self.mission),
            (!delete).then(|| json!({"status":"archived"})),
        )
        .await
    }

    async fn mission_row(&self) -> Value {
        sqlx::query_scalar("SELECT to_jsonb(m) FROM missions m WHERE id = $1")
            .bind(self.mission)
            .fetch_one(&self.state.pool)
            .await
            .unwrap()
    }

    async fn evidence_counts(&self, action: Option<&str>) -> (i64, i64) {
        sqlx::query_as(
            "SELECT count(*), count(p.audit_id) FROM audit_logs a
             LEFT JOIN audit_publication_pending p ON p.audit_id = a.id
             WHERE a.target_type = 'mission' AND a.target_id = $1
               AND ($2::text IS NULL OR (a.action = $2 AND a.actor_id = $3))",
        )
        .bind(self.mission.to_string())
        .bind(action)
        .bind(&self.actor)
        .fetch_one(&self.state.pool)
        .await
        .unwrap()
    }

    async fn history(&self, attachment: Uuid) -> Value {
        sqlx::query_scalar(
            "SELECT jsonb_build_object(
             'attachment', (SELECT to_jsonb(em) FROM event_missions em WHERE em.id = $1),
             'registrations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM event_registrations r WHERE r.event_mission_id = $1),
             'reservation_history', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.id) FROM event_registration_history h
                 JOIN event_registrations r ON r.id = h.registration_id WHERE r.event_mission_id = $1),
             'participation', (SELECT jsonb_agg(to_jsonb(p) ORDER BY p.registration_id, p.match_id, p.arma_id)
                 FROM event_registration_participation p JOIN event_registrations r ON r.id = p.registration_id WHERE r.event_mission_id = $1),
             'slots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM orbat_slots s WHERE s.event_mission_id = $1),
             'matches', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM matches m WHERE m.event_id = $2 AND m.mission_id = $3),
             'results', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM match_player_stats s JOIN matches m ON m.id = s.match_id WHERE m.event_id = $2 AND m.mission_id = $3),
             'versions', (SELECT jsonb_agg(to_jsonb(v) ORDER BY v.id) FROM mission_versions v WHERE v.mission_id = $3),
             'authored_audit', (SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM audit_logs a
                 WHERE (a.target_id = $1::text AND a.action = 'event.mission_removed')
                    OR (a.target_id = $2::text AND a.action = 'event.mission_attached')))"
        ).bind(attachment).bind(self.event).bind(self.mission).fetch_one(&self.state.pool).await.unwrap()
    }

    async fn service_history(&self) -> Value {
        let response = self.call("GET", "/api/v1/me/deployments", None).await;
        assert_eq!(response.0, StatusCode::OK, "{response:?}");
        assert_eq!(response.1["upcoming"], json!([]));
        let history = response.1["service_history"].clone();
        assert_eq!(history.as_array().unwrap().len(), 1);
        assert_eq!(history[0]["operation"], self.title);
        history
    }
}

async fn detach_then_lifecycle_preserves_history(delete: bool) {
    let f = Fixture::new().await;
    let (attachment, registration) = f.attach_with_attendance().await;
    let registered_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT registered_at FROM event_registrations WHERE id = $1")
            .bind(registration)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    f.detach(attachment).await;
    let before = f.history(attachment).await;
    let service_before = f.service_history().await;
    let authored_before = f.mission_row().await;
    let response = f.lifecycle(delete).await;
    assert_eq!(
        response.0,
        if delete {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::OK
        },
        "{response:?}"
    );
    let after = f.mission_row().await;
    assert_eq!(
        f.evidence_counts(Some(lifecycle_action(delete))).await,
        (1, 1)
    );
    if delete {
        assert!(!after["deleted_at"].is_null());
        assert_eq!(
            f.call("GET", &format!("/api/v1/missions/{}", f.mission), None)
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    } else {
        assert_eq!(response.1["status"], "archived");
        assert_eq!(after["status"], "archived");
        assert!(after["deleted_at"].is_null());
    }
    for field in [
        "id",
        "author_id",
        "title",
        "current_version_id",
        "created_at",
    ] {
        assert_eq!(
            after[field], authored_before[field],
            "mission authorship field {field} remains factual"
        );
    }
    assert_eq!(
        f.history(attachment).await,
        before,
        "lifecycle writes preserve every retained reference and participation fact"
    );
    assert_eq!(
        f.service_history().await,
        service_before,
        "historical HTTP records keep their mission name"
    );
    let retained: (
        String,
        String,
        Option<Uuid>,
        chrono::DateTime<chrono::Utc>,
        i64,
    ) = sqlx::query_as(
        "SELECT reservation_state::text, attendance_state::text, slot_id, registered_at,
         (SELECT count(*) FROM event_registration_participation WHERE registration_id = $1)
         FROM event_registrations WHERE id = $1",
    )
    .bind(registration)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(
        retained,
        (
            "withdrawn".into(),
            "attended".into(),
            None,
            registered_at,
            1
        )
    );
    let hidden = f
        .call(
            "GET",
            &format!("/api/v1/event-missions/{attachment}/orbat"),
            None,
        )
        .await;
    assert_eq!(hidden.0, StatusCode::NOT_FOUND);
    let event = f
        .call("GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(event.0, StatusCode::OK);
    assert_eq!(event.1["missions"], json!([]));
    f.state.pool.close().await;
}

#[tokio::test]
async fn detached_mission_archives_without_rewriting_attendance_or_authorship() {
    let _guard = CATALOG_RACES.lock().await;
    detach_then_lifecycle_preserves_history(false).await;
}

#[tokio::test]
async fn detached_mission_soft_deletes_without_hiding_its_historical_name_or_attendance() {
    let _guard = CATALOG_RACES.lock().await;
    detach_then_lifecycle_preserves_history(true).await;
}

#[tokio::test]
async fn active_upcoming_attachment_still_blocks_archive_and_delete_atomically() {
    let _guard = CATALOG_RACES.lock().await;
    let f = Fixture::new().await;
    let (attachment, _) = f.attach_with_attendance().await;
    let before = f.mission_row().await;
    let history = f.history(attachment).await;
    for delete in [false, true] {
        let response = f.lifecycle(delete).await;
        assert_eq!(response.0, StatusCode::CONFLICT, "{response:?}");
        assert_eq!(f.mission_row().await, before);
        assert_eq!(f.history(attachment).await, history);
    }
    f.state.pool.close().await;
}

#[tokio::test]
async fn deleted_parent_event_does_not_block_catalog_lifecycle_or_erase_retained_history() {
    let _guard = CATALOG_RACES.lock().await;
    for delete in [false, true] {
        let f = Fixture::new().await;
        let (attachment, _) = f.attach_with_attendance().await;
        let removed = f
            .call("DELETE", &format!("/api/v1/events/{}", f.event), None)
            .await;
        assert_eq!(removed.0, StatusCode::NO_CONTENT, "{removed:?}");
        let retained_child: bool = sqlx::query_scalar(
            "SELECT em.deleted_at IS NULL AND e.deleted_at IS NOT NULL
             FROM event_missions em JOIN events e ON e.id = em.event_id WHERE em.id = $1",
        )
        .bind(attachment)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert!(
            retained_child,
            "parent removal retains its historical attachment row"
        );
        let detach = f
            .call(
                "DELETE",
                &format!("/api/v1/events/{}/missions/{attachment}", f.event),
                None,
            )
            .await;
        assert_eq!(
            detach.0,
            StatusCode::NOT_FOUND,
            "a removed parent has no operational detach route"
        );
        let before = f.history(attachment).await;
        let service_before = f.service_history().await;
        let response = f.lifecycle(delete).await;
        assert_eq!(
            response.0,
            if delete {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::OK
            },
            "{response:?}"
        );
        assert_eq!(f.history(attachment).await, before);
        assert_eq!(f.service_history().await, service_before);
        f.state.pool.close().await;
    }
}

async fn wait_for_catalog_lock(pool: &sqlx::PgPool, blocker_pid: i32) {
    timeout(DEADLINE, async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname = current_database()
                 AND $1 = ANY(pg_blocking_pids(pid)) AND query LIKE '%FROM missions%')",
            )
            .bind(blocker_pid)
            .fetch_one(pool)
            .await
            .unwrap();
            if blocked {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("HTTP request must reach the controlled catalog-mission lock");
}

fn lifecycle_action(delete: bool) -> &'static str {
    if delete {
        "mission.delete_authorized"
    } else {
        "mission.archive"
    }
}

#[tokio::test]
async fn archive_and_delete_recheck_an_attachment_committed_during_the_catalog_lock_wait() {
    let _guard = CATALOG_RACES.lock().await;
    for delete in [false, true] {
        let f = Fixture::new().await;
        let mut attachment = f.state.pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
            .bind(f.event)
            .fetch_one(&mut *attachment)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM missions WHERE id = $1 FOR NO KEY UPDATE")
            .bind(f.mission)
            .fetch_one(&mut *attachment)
            .await
            .unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *attachment)
            .await
            .unwrap();
        let (response, id) = timeout(DEADLINE * 2, async {
            tokio::join!(f.lifecycle(delete), async {
                wait_for_catalog_lock(&f.state.pool, pid).await;
                let id: Uuid = sqlx::query_scalar(
                    "INSERT INTO event_missions(event_id, mission_id, start_time)
                     VALUES ($1, $2, clock_timestamp() + interval '3 days') RETURNING id",
                )
                .bind(f.event)
                .bind(f.mission)
                .fetch_one(&mut *attachment)
                .await
                .unwrap();
                attachment.commit().await.unwrap();
                id
            })
        })
        .await
        .expect("catalog race must finish after the attachment commits");
        assert_eq!(response.0, StatusCode::CONFLICT, "{response:?}");
        let stored: bool =
            sqlx::query_scalar("SELECT deleted_at IS NULL FROM event_missions WHERE id = $1")
                .bind(id)
                .fetch_one(&f.state.pool)
                .await
                .unwrap();
        assert!(stored);
        assert_eq!(f.mission_row().await["status"], "draft");
        assert!(f.mission_row().await["deleted_at"].is_null());
        f.state.pool.close().await;
    }
}

#[tokio::test]
async fn attachment_and_restoration_recheck_catalog_deletion_or_archive_after_lock_wait() {
    let _guard = CATALOG_RACES.lock().await;
    for restore in [false, true] {
        for delete in [false, true] {
            let f = Fixture::new().await;
            let retained = if restore {
                let (attachment, _) = f.attach_with_attendance().await;
                f.detach(attachment).await;
                Some((attachment, f.history(attachment).await))
            } else {
                None
            };
            let mut lifecycle = f.state.pool.begin().await.unwrap();
            sqlx::query("SELECT id FROM missions WHERE id = $1 FOR NO KEY UPDATE")
                .bind(f.mission)
                .fetch_one(&mut *lifecycle)
                .await
                .unwrap();
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *lifecycle)
                .await
                .unwrap();
            let (response, ()) = timeout(DEADLINE * 2, async {
                tokio::join!(f.attach(), async {
                    wait_for_catalog_lock(&f.state.pool, pid).await;
                    let query = if delete {
                        "UPDATE missions SET deleted_at = clock_timestamp() WHERE id = $1"
                    } else {
                        "UPDATE missions SET status = 'archived' WHERE id = $1"
                    };
                    sqlx::query(query)
                        .bind(f.mission)
                        .execute(&mut *lifecycle)
                        .await
                        .unwrap();
                    lifecycle.commit().await.unwrap();
                })
            })
            .await
            .expect("attachment race must finish after catalog lifecycle commit");
            assert_eq!(
                response.0,
                if delete {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::CONFLICT
                },
                "restore={restore}, delete={delete}: {response:?}"
            );
            let active: i64 = sqlx::query_scalar("SELECT count(*) FROM event_missions WHERE event_id = $1 AND mission_id = $2 AND deleted_at IS NULL")
                .bind(f.event).bind(f.mission).fetch_one(&f.state.pool).await.unwrap();
            assert_eq!(
                active, 0,
                "catalog lifecycle cannot leave a newly active attachment"
            );
            if let Some((attachment, before)) = retained {
                assert_eq!(
                    f.history(attachment).await,
                    before,
                    "rejected restoration preserves retained facts"
                );
            }
            f.state.pool.close().await;
        }
    }
}

#[tokio::test]
async fn lifecycle_required_actor_audit_failure_rolls_back_archive_delete_and_trigger_outbox() {
    let _guard = CATALOG_RACES.lock().await;
    for delete in [false, true] {
        let f = Fixture::new().await;
        let before = f.mission_row().await;
        let evidence_before = f.evidence_counts(None).await;
        let trigger = format!("catalog_audit_failure_{}", Uuid::new_v4().simple());
        let action = lifecycle_action(delete);
        // Only generated UUIDs and fixed action names enter this fixture's SQL text.
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
             IF NEW.target_id = '{}' AND NEW.action = '{action}' THEN
                 RAISE EXCEPTION 'injected catalog actor audit failure';
             END IF; RETURN NEW; END $$;
             CREATE TRIGGER {trigger} BEFORE INSERT ON audit_logs
             FOR EACH ROW EXECUTE FUNCTION {trigger}();",
            f.mission
        )))
        .execute(&f.state.pool)
        .await
        .unwrap();
        let rejected = timeout(DEADLINE, f.lifecycle(delete)).await;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "DROP TRIGGER {trigger} ON audit_logs; DROP FUNCTION {trigger}();"
        )))
        .execute(&f.state.pool)
        .await
        .unwrap();
        let rejected = rejected.expect("injected required-audit error must terminate");
        assert_eq!(
            rejected.0,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{rejected:?}"
        );
        assert_eq!(
            f.mission_row().await,
            before,
            "required evidence failure rolls back lifecycle state"
        );
        assert_eq!(
            f.evidence_counts(None).await,
            evidence_before,
            "trigger audit and outbox also roll back"
        );
        let retry = f.lifecycle(delete).await;
        assert_eq!(
            retry.0,
            if delete {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::OK
            },
            "{retry:?}"
        );
        assert_eq!(f.evidence_counts(Some(action)).await, (1, 1));
        let duplicate = f.lifecycle(delete).await;
        assert_eq!(
            duplicate.0,
            if delete {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::OK
            },
            "{duplicate:?}"
        );
        assert_eq!(
            f.evidence_counts(Some(action)).await,
            (1, 1),
            "a repeated transition creates no extra actor evidence"
        );
        f.state.pool.close().await;
    }
}

#[tokio::test]
async fn catalog_wait_revalidates_actor_ban_session_revocation_and_discord_demotion() {
    let _guard = CATALOG_RACES.lock().await;
    for delete in [false, true] {
        for invalidation in ["ban", "revocation", "demotion"] {
            let f = Fixture::new().await;
            let before = f.mission_row().await;
            let evidence_before = f.evidence_counts(None).await;
            let mut barrier = f.state.pool.begin().await.unwrap();
            sqlx::query("SELECT id FROM missions WHERE id = $1 FOR NO KEY UPDATE")
                .bind(f.mission)
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
            let (response, ()) = timeout(DEADLINE * 2, async {
                tokio::join!(f.lifecycle(delete), async {
                    wait_for_catalog_lock(&f.state.pool, pid).await;
                    sqlx::query("SELECT discord_id FROM users WHERE discord_id = $1 FOR NO KEY UPDATE")
                        .bind(&f.actor).fetch_one(&mut *barrier).await.unwrap();
                    let query = match invalidation {
                        "ban" => "UPDATE users SET is_banned = true WHERE discord_id = $1",
                        "revocation" => "UPDATE authentication_sessions SET revoked_at = clock_timestamp() WHERE discord_id = $1",
                        "demotion" => "DELETE FROM user_discord_roles WHERE discord_id = $1",
                        _ => unreachable!(),
                    };
                    sqlx::query(query).bind(&f.actor).execute(&mut *barrier).await.unwrap();
                    barrier.commit().await.unwrap();
                })
            }).await.expect("authority change race must terminate after the catalog lock releases");
            assert_eq!(
                response.0,
                if matches!(invalidation, "ban" | "revocation") {
                    StatusCode::UNAUTHORIZED
                } else {
                    StatusCode::FORBIDDEN
                },
                "delete={delete}, invalidation={invalidation}: {response:?}"
            );
            assert_eq!(f.mission_row().await, before);
            assert_eq!(f.evidence_counts(None).await, evidence_before);
            f.state.pool.close().await;
        }
    }
}
