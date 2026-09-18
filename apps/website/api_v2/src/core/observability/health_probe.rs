//! `GET /healthz` — the database and migration-state probe, and the bounded `SELECT 1`
//! that `/metrics` shares with it.

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::response::Json;
use serde_json::json;
use sqlx::PgPool;

use super::metrics_registry::Registry;
use crate::config::Config;

/// Budget for the health/scrape database probe. Long enough for a loaded server, short
/// enough that a wedged pool reports `down` instead of holding the probe open (the pool's
/// own acquire timeout is 30 s — a health check that inherits it is a hung health check).
const DB_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// `SELECT 1`, bounded. Returns how long it took, or the failure text.
pub(super) async fn probe_db(pool: &PgPool) -> (bool, Duration, Option<String>) {
    let started = Instant::now();
    match tokio::time::timeout(DB_PROBE_TIMEOUT, sqlx::query("SELECT 1").execute(pool)).await {
        Ok(Ok(_)) => (true, started.elapsed(), None),
        Ok(Err(e)) => (false, started.elapsed(), Some(e.to_string())),
        Err(_) => (
            false,
            started.elapsed(),
            Some(format!("timed out after {DB_PROBE_TIMEOUT:?}")),
        ),
    }
}

/// Constant-time `X-Service-Token` comparison, without the 401.
///
/// [`crate::middleware::ServiceAuth`] is the extractor for routes that must *refuse* an
/// unauthenticated caller. `/healthz` must not: a load balancer or container orchestrator probes
/// it with no credentials and has to get a usable answer, so an absent or wrong token downgrades
/// the payload rather than rejecting the request. Same fail-closed rule as the extractor
/// otherwise — an unconfigured `SERVICE_TOKEN` matches nothing, so a deployment that never set one
/// can never serve the detail.
pub(crate) fn service_token_matches(cfg: &Config, headers: &axum::http::HeaderMap) -> bool {
    let got = headers
        .get("x-service-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    !cfg.service_token.is_empty() && crate::auth::constant_time_equal(got, &cfg.service_token)
}

/// Liveness/readiness probe.
///
/// Two independent checks, **either of which can take the whole probe red** — a health check
/// that cannot fail is worse than none:
///
/// * `database` — a bounded `SELECT 1`. Red when the database is down, refusing
///   connections, or slower than [`DB_PROBE_TIMEOUT`].
/// * `migrations` — red when `_sqlx_migrations` is unreadable (the schema was never
///   migrated, so the process is serving a database it does not match) or when any row
///   records a failed migration. This is the state `db::migrate` guarantees on boot and that
///   nothing else re-checks afterwards.
///
/// # Two payloads, one status (the reason `detailed` exists)
///
/// This route is **unauthenticated and published through Caddy**
/// (`scripts/deploy/Caddyfile.website:27`). The full report names the exact build, how recently
/// the process restarted, the connection-pool depth and the migration count — for example
/// `version=0.1.0  uptime=396  pool={connections:5, idle:4}  migrations.applied=18`. Served to any
/// caller who finds the URL, that is reconnaissance handed over for free.
///
/// Putting auth in front of the whole route is the wrong fix: `/healthz` is probed **without
/// credentials** by `scripts/platform/preflight.sh:145`, `scripts/deploy/Caddyfile.website:27`,
/// `.github/workflows/editor-gates.yml:95` and
/// `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests.rs:2714`, and it stays open
/// for exactly that reason while `/metrics` sits behind `X-Service-Token`.
///
/// So the split is by **payload**, never by status code:
///
/// * **Public** (`detailed == false`) — `{"status": "ok" | "unavailable"}` and the 200/503 split.
///   That is everything a prober reads: `curl -fsS` only looks at the code, and `preflight.sh`
///   compares the code. Nothing about the build, the uptime, the pool or the schema is disclosed.
/// * **`X-Service-Token`** (`detailed == true`) — the full report: `version`, `uptime_seconds`,
///   per-check `status`/`latency_ms`/`error`, the applied/failed migration counts and the pool
///   gauges. An operator's tooling sees the same fields, names and values it always did; it just
///   has to present the token `/metrics` already requires.
///
/// The top-level `status` string keeps its two legacy values in **both** shapes, and so does the
/// 200/503 split, because that pair is the actual contract every prober depends on.
pub(crate) async fn healthz(
    reg: &Registry,
    pool: &PgPool,
    detailed: bool,
) -> (StatusCode, Json<serde_json::Value>) {
    let (db_up, db_ping, db_err) = probe_db(pool).await;

    // Only meaningful if the database answered at all; skip the second round trip otherwise
    // and report the same cause rather than a confusing second timeout.
    let (mig_ok, mig_applied, mig_failed, mig_err) = if db_up {
        match tokio::time::timeout(
            DB_PROBE_TIMEOUT,
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT count(*) FILTER (WHERE success), \
                        count(*) FILTER (WHERE NOT success) \
                 FROM _sqlx_migrations",
            )
            .fetch_one(pool),
        )
        .await
        {
            Ok(Ok((applied, failed))) => (failed == 0, applied, failed, None),
            Ok(Err(e)) => (false, 0, 0, Some(e.to_string())),
            Err(_) => (
                false,
                0,
                0,
                Some(format!("timed out after {DB_PROBE_TIMEOUT:?}")),
            ),
        }
    } else {
        (false, 0, 0, Some("database unavailable".to_owned()))
    };

    let healthy = db_up && mig_ok;
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let status = if healthy { "ok" } else { "unavailable" };
    if !detailed {
        // The whole public payload. Every prober listed in this function's doc reads the code,
        // and `status` is here only because it is the one field documented as legacy contract.
        // Nothing derived from the build, the process or the schema.
        return (code, Json(json!({ "status": status })));
    }
    (
        code,
        Json(json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": reg.uptime().as_secs(),
            "checks": {
                "database": {
                    "status": if db_up { "up" } else { "down" },
                    "latency_ms": db_ping.as_millis() as u64,
                    "error": db_err,
                },
                "migrations": {
                    "status": if mig_ok { "up" } else { "down" },
                    "applied": mig_applied,
                    "failed": mig_failed,
                    "error": mig_err,
                },
            },
            "pool": {
                "connections": pool.size(),
                "idle": pool.num_idle(),
            },
        })),
    )
}
