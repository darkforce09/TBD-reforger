//! Real PostgreSQL locks and local HTTP verify REST lease, dispatch, and backoff coordination.
use axum::{Router, http::StatusCode, routing::get};
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Barrier, Mutex};
use website_api::{
    core::{application_state::AppState, configuration::Config, database},
    identity_and_access::services::discord_rest_reconciliation::{enroll_accounts, reconcile_one},
};
mod common;
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct Fixture {
    state: AppState,
    requests: Arc<AtomicUsize>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn fixture(status: StatusCode, body: &'static str) -> Fixture {
    let url = common::require_test_database_url().unwrap();
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    sqlx::query("DELETE FROM discord_membership_snapshots")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() WHERE singleton",
    )
    .execute(&pool)
    .await
    .unwrap();
    common::seed_user(&pool, "rest-member", "Member", "rest-arma", "enlisted").await;
    let mut cfg = Config::for_tests(url, "rest-coordination");
    cfg.discord_bot_token = "bot-token".into();
    let mut state = AppState::new(pool, cfg);
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = requests.clone();
    let app = Router::new().route(
        "/guilds/test-tbd-guild/members/rest-member",
        get(move || {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                (status, body)
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Arc::make_mut(&mut state.discord).set_api_base(&base);
    enroll_accounts(&state.pool, &state.cfg.discord_guild_id)
        .await
        .unwrap();
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id <> 'rest-member'")
        .execute(&state.pool)
        .await
        .unwrap();
    Fixture {
        state,
        requests,
        server,
    }
}

#[tokio::test]
async fn discord_rest_backoff_installed_while_account_is_locked_prevents_dispatch() {
    let _guard = LOCK.lock().await;
    let f = fixture(StatusCode::OK, r#"{"roles":[]}"#).await;
    let mut blocker = f.state.pool.begin().await.unwrap();
    sqlx::query("SELECT discord_id FROM users WHERE discord_id = 'rest-member' FOR UPDATE")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let state = f.state.clone();
    let worker = tokio::spawn(async move { reconcile_one(&state).await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)))")
                .bind(blocker_pid).fetch_one(&f.state.pool).await.unwrap();
            if blocked { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("worker must reach the controlled account-lock barrier");
    sqlx::query("UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() + interval '120 seconds' WHERE singleton")
        .execute(&f.state.pool).await.unwrap();
    blocker.commit().await.unwrap();
    assert!(!worker.await.unwrap());
    assert_eq!(f.requests.load(Ordering::SeqCst), 0);
    let released: bool = sqlx::query_scalar("SELECT lease_token IS NULL AND next_refresh_at > clock_timestamp() + interval '110 seconds' FROM discord_membership_snapshots WHERE discord_id = 'rest-member'")
        .fetch_one(&f.state.pool).await.unwrap();
    assert!(
        released,
        "an unstarted lease is explicitly rescheduled after the shared deadline"
    );
}

#[tokio::test]
async fn discord_rest_replicas_claim_one_due_member_once() {
    let _guard = LOCK.lock().await;
    let f = fixture(StatusCode::OK, r#"{"roles":[]}"#).await;
    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let (state, barrier) = (f.state.clone(), barrier.clone());
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            reconcile_one(&state).await.unwrap()
        }));
    }
    barrier.wait().await;
    let mut started = 0;
    for task in tasks {
        started += usize::from(task.await.unwrap());
    }
    assert_eq!(started, 1);
    assert_eq!(f.requests.load(Ordering::SeqCst), 1);
    let status: String = sqlx::query_scalar("SELECT membership_status FROM discord_membership_snapshots WHERE discord_id = 'rest-member'")
        .fetch_one(&f.state.pool).await.unwrap();
    assert_eq!(status, "member");
}

#[tokio::test]
async fn discord_rest_retry_after_uses_database_time_and_preserves_verified_membership() {
    let _guard = LOCK.lock().await;
    let f = fixture(
        StatusCode::TOO_MANY_REQUESTS,
        r#"{"retry_after":120.0,"global":true}"#,
    )
    .await;
    common::fixtures::seed_membership(
        &f.state.pool,
        "rest-member",
        &f.state.cfg.discord_guild_id,
        "admin",
    )
    .await;
    sqlx::query("UPDATE discord_membership_snapshots SET next_refresh_at = clock_timestamp() WHERE discord_id = 'rest-member'")
        .execute(&f.state.pool).await.unwrap();
    let before: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
    assert!(reconcile_one(&f.state).await.unwrap());
    let (member_deadline, shared_deadline, status): (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, String) =
        sqlx::query_as("SELECT s.next_refresh_at, r.next_request_at, s.membership_status FROM discord_membership_snapshots s CROSS JOIN discord_rest_schedule r WHERE s.discord_id = 'rest-member' AND r.singleton")
            .fetch_one(&f.state.pool).await.unwrap();
    assert!(member_deadline >= before + chrono::Duration::seconds(120));
    assert!(shared_deadline >= before + chrono::Duration::seconds(120));
    assert_eq!(status, "member");
    assert!(!reconcile_one(&f.state).await.unwrap());
    assert_eq!(f.requests.load(Ordering::SeqCst), 1);
}
