//! PostgreSQL barriers verify atomic lifecycle evidence and schedule-lock ordering.

use sqlx::{AssertSqlSafe, PgPool};
use std::time::Duration;
use tokio::{sync::Mutex, time::timeout};
use uuid::Uuid;
use website_api::{core::database, operations::services::event_lifecycle_sweep::sweep_once};

mod common;

// A sweep operates on every event in this binary's scratch database.
static SWEEP_TESTS: Mutex<()> = Mutex::const_new(());
const DEADLINE: Duration = Duration::from_secs(10);

struct Fixture {
    pool: PgPool,
    author: String,
}

impl Fixture {
    async fn new() -> Self {
        let url = common::require_test_database_url().expect("lifecycle tests require PostgreSQL");
        let pool = database::connect(&url).await.unwrap();
        database::migrate(&pool).await.unwrap();
        let author = format!("lifecycle-{}", Uuid::new_v4());
        common::seed_user(
            &pool,
            &author,
            "Lifecycle fixture author",
            &common::unique_arma("lifecycle"),
            "admin",
        )
        .await;
        Self { pool, author }
    }

    async fn event(&self, status: &str, hours_from_now: i32) -> Uuid {
        let id = Uuid::new_v4();
        self.event_with_id(id, status, hours_from_now).await;
        id
    }

    async fn event_with_id(&self, id: Uuid, status: &str, hours_from_now: i32) {
        sqlx::query(
            "INSERT INTO events(id, name_override, start_time, status, created_by, created_at, updated_at)
             VALUES ($1, 'Lifecycle transaction fixture', now() + make_interval(hours => $2),
                     $3::event_status, $4, now(), now())",
        )
        .bind(id)
        .bind(hours_from_now)
        .bind(status)
        .bind(&self.author)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    async fn attachment(&self, event: Uuid, hours_from_now: i32, deleted: bool) -> Uuid {
        let mission: Uuid = sqlx::query_scalar(
            "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, created_at)
             VALUES ('Lifecycle horizon fixture', $1, 'everon', 'pve_coop', 16, now()) RETURNING id",
        )
        .bind(&self.author)
        .fetch_one(&self.pool)
        .await
        .unwrap();
        sqlx::query_scalar(
            "INSERT INTO event_missions(event_id, mission_id, start_time, created_at, deleted_at)
             VALUES ($1, $2, now() + make_interval(hours => $3), now(),
                     CASE WHEN $4 THEN now() ELSE NULL END) RETURNING id",
        )
        .bind(event)
        .bind(mission)
        .bind(hours_from_now)
        .bind(deleted)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    async fn status(&self, event: Uuid) -> String {
        sqlx::query_scalar("SELECT status::text FROM events WHERE id = $1")
            .bind(event)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn records(&self, event: Uuid) -> Vec<(String, String, String, Option<String>, bool)> {
        sqlx::query_as(
            "SELECT a.action, a.message, a.actor_name, a.actor_id,
                    EXISTS(SELECT 1 FROM audit_publication_pending p WHERE p.audit_id = a.id)
             FROM audit_logs a WHERE a.target_type = 'event' AND a.target_id = $1
               AND a.action IN ('event.auto_live', 'event.auto_completed') ORDER BY a.id",
        )
        .bind(event.to_string())
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn snapshot(&self, event: Uuid) -> serde_json::Value {
        sqlx::query_scalar("SELECT to_jsonb(e) FROM events e WHERE id = $1")
            .bind(event)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn total_evidence(&self) -> (i64, i64) {
        sqlx::query_as(
            "SELECT (SELECT count(*) FROM audit_logs),
                    (SELECT count(*) FROM audit_publication_pending)",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }
}

async fn sweep(pool: &PgPool) -> (Vec<Uuid>, Vec<Uuid>) {
    timeout(DEADLINE, sweep_once(pool))
        .await
        .expect("lifecycle sweep must finish within the test deadline")
        .expect("lifecycle transaction must succeed")
}

async fn wait_for_event_lock(pool: &PgPool, blocker_pid: i32) {
    timeout(DEADLINE, async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))
                   AND query LIKE '%FROM events%')",
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
    .expect("sweep must reach the controlled event-row lock barrier");
}

fn assert_required_evidence(
    records: &[(String, String, String, Option<String>, bool)],
    expected_actions: &[&str],
) {
    assert_eq!(
        records
            .iter()
            .map(|record| record.0.as_str())
            .collect::<Vec<_>>(),
        expected_actions,
        "each legal transition has exactly one ordered audit record"
    );
    for (_, message, actor_name, actor_id, pending) in records {
        assert!(!message.is_empty());
        assert_eq!(actor_name, "system");
        assert!(actor_id.is_none(), "the sweep invents no account author");
        assert!(
            *pending,
            "each committed audit has durable pending delivery"
        );
    }
}

async fn install_failure(f: &Fixture, event: Uuid, publication: bool) -> String {
    let name = format!("lifecycle_failure_{}", Uuid::new_v4().simple());
    // Identifiers and the interpolated target come only from UUID formatting.
    let (table, predicate) = if publication {
        (
            "audit_publication_pending",
            format!(
                "EXISTS(SELECT 1 FROM audit_logs WHERE id = NEW.audit_id
                 AND target_id = '{event}' AND action = 'event.auto_completed')"
            ),
        )
    } else {
        (
            "audit_logs",
            format!("NEW.target_id = '{event}' AND NEW.action = 'event.auto_completed'"),
        )
    };
    sqlx::raw_sql(AssertSqlSafe(format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF {predicate} THEN RAISE EXCEPTION 'injected lifecycle evidence failure'; END IF;
         RETURN NEW; END $$;
         CREATE TRIGGER {name} BEFORE INSERT ON {table}
         FOR EACH ROW EXECUTE FUNCTION {name}();"
    )))
    .execute(&f.pool)
    .await
    .unwrap();
    name
}

async fn remove_failure(f: &Fixture, name: &str, publication: bool) {
    let table = if publication {
        "audit_publication_pending"
    } else {
        "audit_logs"
    };
    sqlx::raw_sql(AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}();"
    )))
    .execute(&f.pool)
    .await
    .unwrap();
}

async fn evidence_failure_rolls_back_and_retry_is_exactly_once(publication: bool) {
    let f = Fixture::new().await;
    let event = f.event("scheduled", -30).await;
    let before = f.snapshot(event).await;
    let evidence_before = f.total_evidence().await;
    let trigger = install_failure(&f, event, publication).await;
    // The failure occurs on the second legal edge, after auto_live's audit and outbox insert.
    let rejected = timeout(DEADLINE, sweep_once(&f.pool)).await;
    remove_failure(&f, &trigger, publication).await;
    let error = rejected
        .expect("injected failure must finish")
        .expect_err("required evidence failure must reject the entire sweep");
    assert!(
        error
            .to_string()
            .contains("injected lifecycle evidence failure")
    );
    assert_eq!(
        f.snapshot(event).await,
        before,
        "no status or timestamp escapes rollback"
    );
    assert_eq!(
        f.total_evidence().await,
        evidence_before,
        "neither audit nor outbox rows escape rollback"
    );
    assert!(f.records(event).await.is_empty());

    let (started, completed) = sweep(&f.pool).await;
    assert!(started.contains(&event));
    assert!(completed.contains(&event));
    assert_eq!(f.status(event).await, "completed");
    let records = f.records(event).await;
    assert_required_evidence(&records, &["event.auto_live", "event.auto_completed"]);
    assert!(records[0].1.contains("scheduled → live"));
    let (started, completed) = sweep(&f.pool).await;
    assert!(!started.contains(&event));
    assert!(!completed.contains(&event));
    assert_eq!(
        f.records(event).await,
        records,
        "retry creates no duplicate evidence"
    );
    f.pool.close().await;
}

#[tokio::test]
async fn lifecycle_audit_failure_rolls_back_status_and_outbox_then_retry_is_exactly_once() {
    let _guard = SWEEP_TESTS.lock().await;
    evidence_failure_rolls_back_and_retry_is_exactly_once(false).await;
}

#[tokio::test]
async fn lifecycle_outbox_failure_rolls_back_status_and_audit_then_retry_is_exactly_once() {
    let _guard = SWEEP_TESTS.lock().await;
    evidence_failure_rolls_back_and_retry_is_exactly_once(true).await;
}

#[tokio::test]
async fn lifecycle_rechecks_committed_start_and_mission_horizon_after_event_lock_wait() {
    let _guard = SWEEP_TESTS.lock().await;
    let f = Fixture::new().await;
    for extend_mission in [false, true] {
        let original_status = if extend_mission { "live" } else { "open" };
        let event = f.event(original_status, -30).await;
        let attachment = f.attachment(event, -30, false).await;
        let mut edit = f.pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
            .bind(event)
            .fetch_one(&mut *edit)
            .await
            .unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *edit)
            .await
            .unwrap();
        let (outcome, ()) = timeout(DEADLINE * 2, async {
            tokio::join!(sweep_once(&f.pool), async {
                wait_for_event_lock(&f.pool, pid).await;
                if extend_mission {
                    // The event row itself is unchanged: the old statement snapshot still has
                    // the old child time, so locking the parent alone cannot refresh this decision.
                    sqlx::query("UPDATE event_missions SET start_time = clock_timestamp() + interval '3 days' WHERE id = $1")
                        .bind(attachment).execute(&mut *edit).await.unwrap();
                } else {
                    sqlx::query("UPDATE events SET start_time = clock_timestamp() + interval '3 days' WHERE id = $1")
                        .bind(event).execute(&mut *edit).await.unwrap();
                }
                edit.commit().await.unwrap();
            })
        })
        .await
        .expect("schedule race must complete after the barrier is released");
        let (started, completed) = outcome.unwrap();
        assert!(
            !started.contains(&event),
            "postponement cancels a stale start decision"
        );
        assert!(
            !completed.contains(&event),
            "postponement cancels a stale completion decision"
        );
        assert_eq!(f.status(event).await, original_status);
        assert!(f.records(event).await.is_empty());
    }
    f.pool.close().await;
}

#[tokio::test]
async fn lifecycle_status_updates_admit_an_uncommitted_telemetry_foreign_key_lock() {
    let _guard = SWEEP_TESTS.lock().await;
    let f = Fixture::new().await;
    let event = f.event("locked", -30).await;
    let mut telemetry = f.pool.begin().await.unwrap();
    let match_id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches(event_id, source_match_id, outcome, started_at, created_at)
         VALUES ($1, $2, 'pending', now(), now()) RETURNING id",
    )
    .bind(event)
    .bind(format!("lifecycle-fk-{}", Uuid::new_v4()))
    .fetch_one(&mut *telemetry)
    .await
    .unwrap();
    // The explicit lock also pins the compatibility assertion if the FK becomes deferred.
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR KEY SHARE")
        .bind(event)
        .fetch_one(&mut *telemetry)
        .await
        .unwrap();
    let outcome = timeout(DEADLINE, sweep_once(&f.pool)).await;
    telemetry.commit().await.unwrap();
    let (started, completed) = outcome
        .expect("sweep must commit while telemetry retains its event KEY SHARE lock")
        .unwrap();
    assert!(started.contains(&event));
    assert!(completed.contains(&event));
    assert_eq!(f.status(event).await, "completed");
    assert_required_evidence(
        &f.records(event).await,
        &["event.auto_live", "event.auto_completed"],
    );
    let stored: Uuid = sqlx::query_scalar("SELECT event_id FROM matches WHERE id = $1")
        .bind(match_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        stored, event,
        "telemetry commits with its event association intact"
    );
    f.pool.close().await;
}

#[tokio::test]
async fn lifecycle_locks_all_transition_candidates_in_uuid_order() {
    let _guard = SWEEP_TESTS.lock().await;
    let f = Fixture::new().await;
    let mut ids = [Uuid::new_v4(), Uuid::new_v4()];
    ids.sort_unstable();
    let [first, second] = ids;
    // The lower UUID is in the completion phase; phase-by-phase locks invert this order.
    f.event_with_id(first, "live", -20).await;
    f.event_with_id(second, "scheduled", -30).await;
    let mut editor = f.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE")
        .bind(second)
        .fetch_one(&mut *editor)
        .await
        .unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *editor)
        .await
        .unwrap();
    let (outcome, ()) = timeout(DEADLINE * 2, async {
        tokio::join!(sweep_once(&f.pool), async {
            wait_for_event_lock(&f.pool, pid).await;
            let mut observer = f.pool.begin().await.unwrap();
            let lock = sqlx::query("SELECT id FROM events WHERE id = $1 FOR NO KEY UPDATE NOWAIT")
                .bind(first)
                .fetch_one(&mut *observer)
                .await;
            observer.rollback().await.unwrap();
            editor.commit().await.unwrap();
            let error =
                lock.expect_err("lower UUID must already be locked before waiting on higher UUID");
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("55P03")
            );
        })
    })
    .await
    .expect("ordered lifecycle sweep must finish after the editor releases its event");
    let (started, completed) = outcome.unwrap();
    assert!(!started.contains(&first));
    assert!(started.contains(&second));
    assert!(completed.contains(&first) && completed.contains(&second));
    assert!(completed.windows(2).all(|pair| pair[0] < pair[1]));
    assert_required_evidence(&f.records(first).await, &["event.auto_completed"]);
    assert_required_evidence(
        &f.records(second).await,
        &["event.auto_live", "event.auto_completed"],
    );
    f.pool.close().await;
}

#[tokio::test]
async fn lifecycle_preserves_legal_edges_and_latest_active_mission_six_hour_horizon() {
    let _guard = SWEEP_TESTS.lock().await;
    let f = Fixture::new().await;
    let mut starts = Vec::new();
    for status in ["scheduled", "open", "locked"] {
        starts.push((f.event(status, -1).await, status));
    }
    let long_past = f.event("scheduled", -30).await;
    let mission_extends = f.event("live", -30).await;
    f.attachment(mission_extends, -12, false).await;
    f.attachment(mission_extends, -1, false).await;
    let earlier_mission = f.event("live", -1).await;
    f.attachment(earlier_mission, -30, false).await;
    let deleted_mission = f.event("live", -30).await;
    f.attachment(deleted_mission, 72, true).await;
    let future = f.event("open", 72).await;
    let cancelled = f.event("cancelled", -30).await;
    let completed = f.event("completed", -30).await;
    let deleted_event = f.event("scheduled", -30).await;
    sqlx::query("UPDATE events SET deleted_at = now() WHERE id = $1")
        .bind(deleted_event)
        .execute(&f.pool)
        .await
        .unwrap();

    let (started, finished) = sweep(&f.pool).await;
    for (event, from) in starts {
        assert!(started.contains(&event));
        assert!(!finished.contains(&event));
        assert_eq!(f.status(event).await, "live");
        let records = f.records(event).await;
        assert_required_evidence(&records, &["event.auto_live"]);
        assert!(records[0].1.contains(&format!("{from} → live")));
    }
    assert!(started.contains(&long_past) && finished.contains(&long_past));
    assert_eq!(f.status(long_past).await, "completed");
    assert_required_evidence(
        &f.records(long_past).await,
        &["event.auto_live", "event.auto_completed"],
    );
    assert!(
        finished.contains(&deleted_mission),
        "removed missions do not extend the horizon"
    );
    assert_required_evidence(&f.records(deleted_mission).await, &["event.auto_completed"]);
    for (event, expected) in [
        (mission_extends, "live"),
        (earlier_mission, "live"),
        (future, "open"),
        (cancelled, "cancelled"),
        (completed, "completed"),
        (deleted_event, "scheduled"),
    ] {
        assert!(!started.contains(&event) && !finished.contains(&event));
        assert_eq!(f.status(event).await, expected);
        assert!(f.records(event).await.is_empty());
    }
    f.pool.close().await;
}
