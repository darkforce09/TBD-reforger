//! The database-backed half of observability and durable rate limiting; the in-process half
//! (metrics registry, exposition text, `/healthz` going red) lives beside `src/router.rs`.
//!
//! 1. `/healthz` is green against a migrated database, its migration check reports the applied
//!    count, and `/metrics` reports a live database (`tbd_db_up 1`, non-zero pool gauges).
//! 2. `PgRateLimiter` refuses at the limit and still refuses after a restart, which is the claim
//!    behind the word "durable"; the in-memory `IpLimiter` is exercised beside it.

use crate::common;

use std::time::Duration;

use api_configuration::configuration::Config;

use api_http_layer::middleware::IpLimiter;
use api_http_layer::middleware::durable_ratelimit::{
    PgRateLimiter, RATE_LIMIT_BUCKETS_DDL, bucket_key,
};
use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// Connect with a short acquire timeout — a wedged pool must fail this suite fast rather
/// than sit on the production 30 s budget.
async fn pool_for(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
        .expect("connect the per-binary test database")
}

fn app_with(pool: PgPool) -> Router {
    router(api_server::composition::application_state(
        pool,
        Config::for_tests("postgres://unused", "observability-secret"),
    ))
}

/// The operator bearer `Config::for_tests` installs as the observability token.
const OBSERVABILITY_BEARER: (&str, &str) = ("authorization", "Bearer test-observability-token");

/// One GET of `uri` with at most one credential header.
async fn call(app: &Router, uri: &str, credential: Option<(&str, &str)>) -> (StatusCode, String) {
    let mut b = Request::builder().method("GET").uri(uri);
    if let Some((name, value)) = credential {
        b = b.header(name, value);
    }
    let resp = app
        .clone()
        .oneshot(b.body(Body::empty()).expect("request"))
        .await
        .expect("router call");
    let status = resp.status();
    let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// Value of one exposition line. Matching the whole line rather than `contains` is
/// deliberate: `contains("tbd_db_up")` is true of the `# HELP` comment, so a registry that
/// emitted no sample at all would still satisfy a `contains` assertion.
fn value(body: &str, prefix: &str) -> Option<f64> {
    body.lines()
        .find(|l| l.starts_with(prefix))?
        .rsplit(' ')
        .next()?
        .parse()
        .ok()
}

/// Runs the DDL exactly once per binary.
///
/// `CREATE TABLE IF NOT EXISTS` is **not** concurrency-safe against itself: three of these
/// tests run at once, and two of them racing produced
/// `23505 duplicate key ... pg_type_typname_nsp_index` — the IF-NOT-EXISTS check and the
/// catalog insert are not atomic. That is a property of the DDL, not of the limiter, and
/// serialising here keeps it from reading as a limiter failure.
static BUCKET_TABLE: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// Ensure the table [`PgRateLimiter`] needs. **This DDL is the deliverable for the
/// migration owner** — `RATE_LIMIT_BUCKETS_DDL` is a `const` in
/// `api_http_layer::middleware::durable_ratelimit` precisely so the bytes proven here and the bytes in
/// `migrations/0021_rate_limit_buckets.sql` cannot drift.
async fn ensure_bucket_table(pool: &PgPool) {
    BUCKET_TABLE
        .get_or_init(|| async {
            sqlx::raw_sql(RATE_LIMIT_BUCKETS_DDL)
                .execute(pool)
                .await
                .expect("create rate_limit_buckets");
        })
        .await;
}

// ───────────────────────────── health + scrape, live ─────────────────────────────

/// `/healthz` green against a migrated database, and the migration check reading real
/// numbers rather than reporting a constant.
///
/// The red half — both checks `down`, 503 — is `router::tests::healthz_goes_red_when_the_
/// database_is_unreachable`. Neither is worth anything without the other: a probe that is
/// always green and a probe that is always red are the same defect.
///
/// The detail sits behind the observability bearer, so this reads the probe *with* the
/// token. The public shape (`{"status": …}` and nothing else) is asserted against a dead pool by
/// `router::tests::healthz_discloses_nothing_to_an_unauthenticated_caller` and against a live one by
/// [`healthz_public_shape_is_status_only_against_a_live_database`] below — a probe that discloses
/// nothing only because it is failing would prove nothing.
#[tokio::test]
async fn healthz_is_green_and_metrics_see_a_live_database() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!(
            "skip: TEST_DATABASE_URL unset — healthz_is_green_and_metrics_see_a_live_database"
        );
        return;
    };
    let pool = pool_for(&url).await;
    api_database::migrate(&pool).await.expect("migrate");
    let app = app_with(pool);

    let (st, body) = call(&app, "/healthz", Some(OBSERVABILITY_BEARER)).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
    assert_eq!(v["status"], "ok", "legacy status string preserved");
    assert_eq!(v["checks"]["database"]["status"], "up");
    assert_eq!(v["checks"]["migrations"]["status"], "up");
    // Not `> 0`: an `applied` that is merely positive is satisfied by a hard-coded 1, and
    // "a tool reporting success over an input it never examined" is the defect this
    // program is built around. Pin it to the migration directory the database was built
    // from, so the check has to be reading `_sqlx_migrations` to pass.
    let migrations_folder = repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("the repository root above the API crate")
    .join("crates/api/api_database/migrations");
    let on_disk = std::fs::read_dir(migrations_folder)
        .expect("read migrations dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("sql"))
        .count() as i64;
    assert!(
        on_disk > 0,
        "no migration files found — this pin would be vacuous"
    );
    assert_eq!(
        v["checks"]["migrations"]["applied"].as_i64(),
        Some(on_disk),
        "healthz reports {:?} applied migrations but {on_disk} exist on disk — the check is \
         not reading _sqlx_migrations: {body}",
        v["checks"]["migrations"]["applied"]
    );
    assert_eq!(v["checks"]["migrations"]["failed"], 0);
    assert!(v["uptime_seconds"].is_number());

    // The same gauges that read 0 against a dead pool must now read the live world.
    let (st, m) = call(&app, "/metrics", Some(OBSERVABILITY_BEARER)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        value(&m, "tbd_db_up"),
        Some(1.0),
        "db_up stuck at 0 against a live database — the gauge is not a probe:\n{m}"
    );
    let idle = value(&m, "tbd_db_pool_connections{state=\"idle\"}").expect("idle gauge");
    let busy = value(&m, "tbd_db_pool_connections{state=\"in_use\"}").expect("in_use gauge");
    assert!(
        idle + busy > 0.0,
        "pool gauges are both zero after real queries:\n{m}"
    );
    // And the health traffic above is in the counters, by status.
    assert_eq!(
        value(
            &m,
            "tbd_http_requests_total{method=\"GET\",route=\"/healthz\",status=\"200\"}"
        ),
        Some(1.0),
        "a real 200 did not land in the counter:\n{m}"
    );
}

// ───────────────────────────── durable rate limiting ─────────────────────────────

/// **The durability claim.** Refuse at the limit; still refuse after a restart.
///
/// The in-memory limiter is exercised first, in the same test, to pin the defect the durable
/// tier exists for: a fresh `IpLimiter` hands the same caller a full bucket, which is exactly
/// what a process restart produces.
#[tokio::test]
async fn pg_limiter_refuses_at_the_limit_and_after_a_restart() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!(
            "skip: TEST_DATABASE_URL unset — pg_limiter_refuses_at_the_limit_and_after_a_restart"
        );
        return;
    };

    // ── the defect, as an assertion. 1 token/second, burst 3: four checks in the same
    //    microsecond, so no refill can rescue the fourth.
    let mem = IpLimiter::new(1, 3);
    let ip = "10.9.9.9".parse().expect("ip");
    assert!(
        mem.check(ip) && mem.check(ip) && mem.check(ip),
        "burst of 3"
    );
    assert!(!mem.check(ip), "in-memory limiter must refuse the 4th");
    let mem_after_restart = IpLimiter::new(1, 3); // a process restart IS a new limiter
    assert!(
        mem_after_restart.check(ip),
        "the in-memory limiter is expected to FORGET across a restart — if this ever fails, \
         the single-instance defect was fixed elsewhere and this test needs revisiting"
    );

    // ── the fix: same shape, state in Postgres.
    let key = format!("{}-{}", bucket_key("restart", ip), uuid::Uuid::new_v4());
    let pool_a = pool_for(&url).await;
    ensure_bucket_table(&pool_a).await;
    let a = PgRateLimiter::new(pool_a.clone(), 0, 3); // refill 0: only spending matters
    for i in 1..=3 {
        assert!(
            a.check(&key).await.expect("check"),
            "token {i} of the burst should have been granted"
        );
    }
    assert!(
        !a.check(&key).await.expect("check"),
        "the 4th must be refused AT the limit"
    );

    // Restart: drop the limiter, close the pool, open a brand-new one. Nothing in this
    // process survives — only the row does.
    drop(a);
    pool_a.close().await;
    drop(pool_a);

    let pool_b = pool_for(&url).await;
    let b = PgRateLimiter::new(pool_b.clone(), 0, 3);
    assert!(
        !b.check(&key).await.expect("check after restart"),
        "STILL refused after a simulated restart — this is the whole durability claim, and \
         it is the assertion the in-memory limiter above provably fails"
    );

    sqlx::query("DELETE FROM public.rate_limit_buckets WHERE bucket_key = $1")
        .bind(&key)
        .execute(&pool_b)
        .await
        .expect("cleanup");
    pool_b.close().await;
}
