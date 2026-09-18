//! HTTP application assembly — the router + global middleware chain. Shared by the
//! `api` binary and the test/differential harnesses so they exercise one router.
//!
//! [`router`] owns the layer order, which is load-bearing: the metrics middleware sits outside
//! the panic-catcher and the rate limiter so a 500-from-panic and a 429-from-throttle are both
//! counted, and the `/map-assets` mount sits *below* the rate-limit layer so terrain streaming is
//! never limiter-bound. Both seams are commented at their call sites and pinned by tests.

use std::sync::Arc;

use axum::Router;
use axum::extract::{DefaultBodyLimit, State};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::get;
use sqlx::PgPool;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::core::application_state::AppState;
use crate::core::configuration::Config;
use crate::core::middleware;
use crate::core::observability::health_probe::{healthz, service_token_matches};
use crate::core::observability::metrics_exposition::metrics_scrape;
use crate::core::observability::metrics_registry::Registry;
use crate::core::observability::request_observer::observe;

/// The `/api/v1` route tree, assembled from the eight domain route tables. Each domain owns one
/// `pub fn routes` listing its own registrations; this function only merges them, so the path a
/// caller reaches is the path written in the domain table, unprefixed by anything but the
/// `/api/v1` nest applied below.
///
/// Auth tiers are enforced per-handler by the extractor each takes (`AuthUser`, the role-gated
/// newtypes, `ServiceAuth`), not by the merge order.
fn api_v1_routes(dev: bool, version_limit: usize) -> Router<AppState> {
    Router::new()
        .merge(crate::identity_and_access::routes(dev))
        .merge(crate::operations::routes())
        .merge(crate::missions::routes(version_limit))
        .merge(crate::server_infrastructure::routes())
        .merge(crate::administration::routes())
        .merge(crate::match_telemetry::routes())
        .merge(crate::command_center::routes())
        .merge(crate::community_content::routes())
}

/// Build the application: `/healthz`, `/metrics`, `/api/v1/*`, static `/uploads`, the optional
/// Leptos SPA + `/map-assets`, and the global middleware chain (outermost first: request-id →
/// logging → **metrics** → recovery → CORS → body-limit → rate-limit).
///
/// **The chain is not uniform at its innermost link.** `/map-assets` gets every layer in that list
/// except the last: it is mounted below the `rate_limit` layer and is therefore never seen by the
/// limiter. Everything else, including the SPA fallback and the other `ServeDir` (`/uploads`), is
/// above it. The seam is commented in full at the call site.
///
/// The metrics registry is created here, once per router, and shared by the `observe`
/// middleware, `/metrics` and `/healthz` — see [`Registry`] for why it is not a `static`.
pub fn router(state: AppState) -> Router {
    let dev = state.cfg.is_development();
    let version_limit = state.cfg.mission_version_body_limit() as usize;
    let registry = Arc::new(Registry::new());

    // `/metrics` and `/healthz` need the registry, which is not an `AppState` field, so both are
    // closures over the `Arc`.
    let reg_metrics = registry.clone();
    let reg_health = registry.clone();
    let mut r = Router::new()
        // Public callers get `{"status": …}` and the 200/503 split, nothing else. The detail is
        // behind the same `X-Service-Token` that gates `/metrics`. See [`healthz`].
        .route(
            "/healthz",
            get(
                move |State(pool): State<PgPool>,
                      State(cfg): State<Arc<Config>>,
                      headers: axum::http::HeaderMap| {
                    let reg = reg_health.clone();
                    async move {
                        let detailed = service_token_matches(&cfg, &headers);
                        healthz(&reg, &pool, detailed).await
                    }
                },
            ),
        )
        // Scraping is gated on the SAME `X-Service-Token` the game-server ingest uses, and
        // `ServiceAuth` fails closed when `SERVICE_TOKEN` is unset — so an unconfigured
        // deployment answers 401, never a public dump of route templates and latencies.
        .route(
            "/metrics",
            get(
                move |_: middleware::ServiceAuth, State(pool): State<PgPool>| {
                    let reg = reg_metrics.clone();
                    async move { metrics_scrape(&reg, &pool).await }
                },
            ),
        )
        .nest("/api/v1", api_v1_routes(dev, version_limit))
        .nest_service("/uploads", ServeDir::new("uploads"));

    // Always serve `/map-assets` (Trunk proxies here in dev; production SPA cutover uses the same
    // path). Gating this behind SPA_DIST_DIR left the editor with 404s for DEM/sat/world under
    // `cargo xtask mk leptos` + `cargo xtask mk rust-api`.
    //
    // The *mount* is deliberately deferred to below the rate-limit layer. Only the directory is
    // resolved here.
    let map_assets = if state.cfg.map_assets_dir.is_empty() {
        "../../../packages/map-assets".to_string()
    } else {
        state.cfg.map_assets_dir.clone()
    };

    // Serve the Leptos SPA statically when SPA_DIST_DIR is set (unset in dev, where `trunk serve`
    // owns the SPA). A no-extension path falls back to index.html
    // (client routing). The SPA document is a rate-limited route like any other, so it is
    // registered here, above the layer.
    if !state.cfg.spa_dist_dir.is_empty() {
        let dist = state.cfg.spa_dist_dir.clone();
        let index = format!("{dist}/index.html");
        r = r.fallback_service(ServeDir::new(dist).fallback(ServeFile::new(index)));
    }

    // The rate limiter's own state: `AppState` (the in-memory L1 limiters) plus the durable
    // Postgres L2 built on the same pool. Not folded into `AppState` — see
    // `core::middleware::rate_limiting::RateLimitState`.
    //
    // ─────────────────────────────────────────────────────────────────────────────────────────
    // **THIS LINE IS THE RATE-LIMIT SEAM.** `Router::layer` wraps the routes registered
    // *above* it and nothing registered below. Everything the API actually answers for — the
    // whole of `/api/v1`, `/healthz`, `/metrics`, `/uploads`, and the SPA fallback — is above,
    // and stays limited. `/map-assets` is mounted immediately below, and is therefore outside
    // this middleware: the limiter is never asked about a map asset rather than being asked and
    // told to say yes.
    //
    // That is the point. Measured on the live stack while the mount sat above this layer,
    // 145,858 of the 145,861 `429`s this limiter had ever issued were `/map-assets`, and none
    // were `/auth/` or `/ingest/`. A cold Mission Creator boot needs 951 distinct files from this
    // mount — no request-per-second ceiling both clears that and refuses anything a scraper would
    // do differently, because the resource here is bytes and the meter counts requests. See
    // `core/middleware/rate_limiting.rs`'s module header for the full argument, and
    // `tests/t630_map_assets_exempt.rs` for the proof that the routes above this line still refuse.
    //
    // Moving the `nest_service` below back above this layer silently re-arms the defect; that is
    // why the order is asserted by `core::middleware::rate_limiting::tests::
    // the_exempt_mount_is_registered_below_the_rate_limit_layer` as well as behaviourally.
    // ─────────────────────────────────────────────────────────────────────────────────────────
    r = r.layer(from_fn_with_state(
        middleware::RateLimitState::new(state.clone()),
        middleware::rate_limit,
    ));

    r = r.nest_service(
        middleware::RATE_LIMIT_EXEMPT_MOUNT,
        ServeDir::new(map_assets),
    );

    // Cross-origin isolation (COOP `same-origin` + COEP `credentialless`) mirrors the Trunk/gate
    // headers so the wasm SharedArrayBuffer path stays available. Applied after **both** the SPA
    // fallback and the map-asset mount, so one header set covers both sides of the limiter seam.
    // It also lands on a 429 body, which is inert (COOP/COEP are document-scoped) and is the price
    // of not maintaining two copies of these two layers.
    if !state.cfg.spa_dist_dir.is_empty() {
        use axum::http::header::{HeaderName, HeaderValue};
        use tower_http::set_header::SetResponseHeaderLayer;

        r = r
            .layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static("cross-origin-opener-policy"),
                HeaderValue::from_static("same-origin"),
            ))
            .layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static("cross-origin-embedder-policy"),
                HeaderValue::from_static("credentialless"),
            ));
    }

    r.layer(DefaultBodyLimit::max(middleware::MAX_JSON_BODY))
        .layer(from_fn_with_state(state.clone(), middleware::cors))
        .layer(CatchPanicLayer::new())
        // OUTSIDE the panic-catcher and the rate limiter, INSIDE the logger. That position
        // is the whole point: inside `CatchPanicLayer` a panicking handler would never
        // reach `record` and the 500 would go uncounted, and inside `rate_limit` a throttled
        // request would never reach it either — `tbd_http_rate_limited_total` would be a
        // series that can only ever read 0. Both are checked by the tests below.
        .layer(from_fn_with_state(registry, observe))
        .layer(from_fn(middleware::logging))
        .layer(from_fn(middleware::request_id))
        .with_state(state)
}

#[cfg(test)]
#[path = "tests/http_router.rs"]
mod tests;
