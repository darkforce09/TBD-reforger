//! T-280 — the database-backed half of the observability / durable-rate-limiting slice.
//!
//! # Why this file exists rather than `#[cfg(test)]` in `src/app.rs`
//!
//! Everything that can be proven in-process (the metrics registry, the exposition text,
//! the cardinality cap, `/healthz` going red) lives in `src/app.rs` next to the code.
//! What is here needs a **real** database, and `common::t542_no_raw_test_database_url_reads_outside_common`
//! (T-542 / T-558) forbids `src/**` from reading `TEST_DATABASE_URL` at all — a rule that
//! exists because an in-crate DB test once read the operator's base URL raw and could have
//! run against live `tbd_reforger`. So the DB half comes here and goes through
//! [`common::require_test_database_url`], which provisions this binary its own database.
//!
//! # What it proves
//!
//! 1. `/healthz` is **green** against a migrated database and its migration check reports a
//!    real applied count — the other half of the red case pinned in `src/app.rs`.
//! 2. `/metrics` reports a live database (`tbd_db_up 1`, non-zero pool gauges) — the same
//!    gauges that read 0 against a dead pool in `src/app.rs`.
//! 3. `PgRateLimiter` **refuses at the limit and still refuses after a restart**, which is
//!    the entire claim behind the word "durable"; the in-memory `IpLimiter` is exercised
//!    beside it to pin the defect it replaces.
//! 4. The limit is shared **across processes** (two pools, one budget).
//! 5. It is a token bucket, not a lockout: refusal expires, and `prune` is garbage
//!    collection rather than an amnesty.

use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use website_api::app::durable_ratelimit::{PgRateLimiter, RATE_LIMIT_BUCKETS_DDL, bucket_key};
use website_api::config::Config;
use website_api::middleware::IpLimiter;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

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
    app::router(AppState::new(
        pool,
        Config::for_tests("postgres://unused", "t280-observability"),
    ))
}

async fn call(app: &Router, uri: &str, service_token: bool) -> (StatusCode, String) {
    let mut b = Request::builder().method("GET").uri(uri);
    if service_token {
        b = b.header("x-service-token", "test-service-token");
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
/// migration owner** — `RATE_LIMIT_BUCKETS_DDL` is a `const` in `src/app.rs` precisely so
/// the bytes proven here and the bytes that eventually land in
/// `apps/website/api/migrations/` cannot drift.
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
/// The red half — both checks `down`, 503 — is `app::tests::healthz_goes_red_when_the_
/// database_is_unreachable`. Neither is worth anything without the other: a probe that is
/// always green and a probe that is always red are the same defect.
///
/// **T-580** moved the detail behind `X-Service-Token`, so this now reads the probe *with* the
/// token. The public shape (`{"status": …}` and nothing else) is asserted against a dead pool by
/// `app::tests::healthz_discloses_nothing_to_an_unauthenticated_caller` and against a live one by
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
    db::migrate(&pool).await.expect("migrate");
    let app = app_with(pool);

    let (st, body) = call(&app, "/healthz", true).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
    assert_eq!(v["status"], "ok", "legacy status string preserved");
    assert_eq!(v["checks"]["database"]["status"], "up");
    assert_eq!(v["checks"]["migrations"]["status"], "up");
    // Not `> 0`: an `applied` that is merely positive is satisfied by a hard-coded 1, and
    // "a tool reporting success over an input it never examined" is the defect this
    // program is built around. Pin it to the migration directory the database was built
    // from, so the check has to be reading `_sqlx_migrations` to pass.
    let on_disk =
        std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations"))
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
    let (st, m) = call(&app, "/metrics", true).await;
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

/// T-580 — against a **live, healthy** database the public probe still discloses nothing.
///
/// The dead-pool half lives in `src/app.rs`. Both are needed: a `/healthz` that reveals nothing
/// because it is 503-ing on every check has not been fixed, it has been broken, and this is the
/// case where there is real detail to leak (a real version, a real uptime, real pool gauges and a
/// real migration count — the four fields wave 69's verifier actually measured off the public
/// route).
#[tokio::test]
async fn healthz_public_shape_is_status_only_against_a_live_database() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset — healthz_public_shape_is_status_only…");
        return;
    };
    let pool = pool_for(&url).await;
    db::migrate(&pool).await.expect("migrate");
    let app = app_with(pool);

    let (st, body) = call(&app, "/healthz", false).await;
    // The prober contract: 200 + `status`. `curl -fsS` (preflight.sh, editor-gates.yml,
    // smokes.rs) and Caddy's health handler read exactly this much.
    assert_eq!(st, StatusCode::OK, "{body}");
    let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
    assert_eq!(v["status"], "ok");
    assert_eq!(
        v.as_object().expect("object").keys().collect::<Vec<_>>(),
        vec!["status"],
        "public /healthz must carry exactly one field: {body}"
    );
    for leak in [
        "version",
        "uptime",
        "pool",
        "migration",
        "latency",
        "checks",
    ] {
        assert!(
            !body.contains(leak),
            "public /healthz leaked {leak:?}: {body}"
        );
    }

    // …and the token still gets the real numbers off the same live database, so this is a
    // relocation and not a deletion.
    let (st, detail) = call(&app, "/healthz", true).await;
    assert_eq!(st, StatusCode::OK, "{detail}");
    let d: serde_json::Value = serde_json::from_str(&detail).expect("healthz json");
    assert_eq!(d["checks"]["database"]["status"], "up", "{detail}");
    assert!(
        d["checks"]["migrations"]["applied"].as_i64().unwrap_or(0) > 0,
        "{detail}"
    );
    assert!(
        d["pool"]["connections"].as_u64().unwrap_or(0) > 0,
        "{detail}"
    );
}

// ───────────────────────────── durable rate limiting ─────────────────────────────

/// **The durability claim.** Refuse at the limit; still refuse after a restart.
///
/// The in-memory limiter is exercised first, in the same test, to pin the defect T-280
/// exists for: a fresh `IpLimiter` hands the same caller a full bucket, which is exactly
/// what a process restart does today.
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
    let key = format!(
        "{}-{}",
        bucket_key("t280-restart", ip),
        uuid::Uuid::new_v4()
    );
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

/// Cross-process: two limiters on independent pools spend **one** budget. This is the
/// second half of the ticket's "cannot scale past one process".
#[tokio::test]
async fn pg_limiter_is_shared_across_processes() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset — pg_limiter_is_shared_across_processes");
        return;
    };
    let key = format!("t280-shared|{}", uuid::Uuid::new_v4());
    let p1 = pool_for(&url).await;
    let p2 = pool_for(&url).await;
    ensure_bucket_table(&p1).await;
    let a = PgRateLimiter::new(p1.clone(), 0, 4);
    let b = PgRateLimiter::new(p2.clone(), 0, 4);

    let mut allowed = 0;
    for _ in 0..4 {
        if a.check(&key).await.expect("process a") {
            allowed += 1;
        }
        if b.check(&key).await.expect("process b") {
            allowed += 1;
        }
    }
    assert_eq!(
        allowed, 4,
        "burst 4 must be spent ONCE across both processes — 8 would mean each process got \
         its own bucket, which is the defect"
    );

    sqlx::query("DELETE FROM public.rate_limit_buckets WHERE bucket_key = $1")
        .bind(&key)
        .execute(&p1)
        .await
        .expect("cleanup");
    p1.close().await;
    p2.close().await;
}

/// A limiter that never forgives is a ban. Refusal has to expire, and `prune` must be
/// garbage collection (a pruned bucket starts full, which is what "no row" already means).
#[tokio::test]
async fn pg_limiter_refills_over_time_and_prunes() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset — pg_limiter_refills_over_time_and_prunes");
        return;
    };
    let key = format!("t280-refill|{}", uuid::Uuid::new_v4());
    let pool = pool_for(&url).await;
    ensure_bucket_table(&pool).await;
    // 5 tokens/s, burst 1 — one token per 200 ms.
    //
    // The rate is chosen against the round trip, not for looks: at 100/s a single
    // ~10 ms query to Postgres refills a whole token, and the "immediately refused"
    // assertion below failed for that reason on the first run. 200 ms per token leaves the
    // round trip two orders of magnitude short while the sleep clears it comfortably.
    let l = PgRateLimiter::new(pool.clone(), 5, 1);

    assert!(l.check(&key).await.expect("first"), "the burst token");
    assert!(
        !l.check(&key).await.expect("second"),
        "immediately refused — the bucket is empty"
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        l.check(&key).await.expect("third"),
        "300 ms at 5/s must have refilled — refusal is not permanent"
    );

    assert_eq!(
        l.prune(Duration::ZERO).await.expect("prune"),
        1,
        "prune removed no row, so the sweeper does nothing and the table grows forever"
    );
    assert!(
        l.check(&key).await.expect("after prune"),
        "a pruned bucket starts full again — pruning is GC, not a lockout"
    );

    sqlx::query("DELETE FROM public.rate_limit_buckets WHERE bucket_key = $1")
        .bind(&key)
        .execute(&pool)
        .await
        .expect("cleanup");
    pool.close().await;
}
