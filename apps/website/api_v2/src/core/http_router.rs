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
use axum::routing::{get, post};
use sqlx::PgPool;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::core::application_state::AppState;
use crate::core::configuration::Config;
use crate::core::observability::health_probe::{healthz, service_token_matches};
use crate::core::observability::metrics_exposition::metrics_scrape;
use crate::core::observability::metrics_registry::Registry;
use crate::core::observability::request_observer::observe;
use crate::{handlers, middleware};

/// The `/api/v1` route tree. Auth tiers are enforced per-handler by the extractor
/// each takes (`AuthUser`, the role-gated newtypes, `ServiceAuth`). Grows per phase.
fn api_routes(dev: bool, version_limit: usize) -> Router<AppState> {
    let mut r = Router::new()
        .route("/auth/discord/login", get(handlers::oauth::discord_login))
        .route(
            "/auth/discord/callback",
            get(handlers::oauth::discord_callback),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/logout", post(handlers::auth::logout))
        .route(
            "/me",
            get(handlers::me::get_me).patch(handlers::me::update_me),
        )
        .route(
            "/me/link",
            post(handlers::me::create_link_code).delete(handlers::me::unlink),
        )
        .route("/me/link/status", get(handlers::me::link_status))
        .route(
            "/ingest/link-confirm",
            post(handlers::me::ingest_link_confirm),
        )
        // Content reads (member tier via each handler's AuthUser extractor).
        .route(
            "/announcements",
            get(handlers::announcements::list_announcements),
        )
        .route(
            "/announcements/{id}",
            get(handlers::announcements::get_announcement),
        )
        .route("/wiki", get(handlers::wiki::list_wiki))
        .route(
            "/wiki/{slug}",
            get(handlers::wiki::get_wiki_page).put(handlers::wiki::upsert_wiki_page),
        )
        .route(
            "/vehicle-database",
            get(handlers::wiki::list_vehicles).post(handlers::wiki::create_vehicle),
        )
        // Admin writes (create / replace / delete / set-current). Auth tier is per-handler via
        // AdminUser — same pattern as /wiki/{slug} PUT and /vehicle-database POST.
        .route(
            "/modpacks",
            get(handlers::modpacks::list_modpacks).post(handlers::modpacks::create_modpack),
        )
        .route(
            "/modpacks/current",
            get(handlers::modpacks::get_current_modpack),
        )
        .route(
            "/modpacks/{id}",
            axum::routing::put(handlers::modpacks::replace_modpack)
                .delete(handlers::modpacks::delete_modpack),
        )
        .route(
            "/modpacks/{id}/set-current",
            post(handlers::modpacks::set_current_modpack),
        )
        // The admin server-CRUD write side. `create_server` / `update_server` /
        // `deactivate_server` each carry an `@route` tag, and GO-7 — the `@route`-vs-router check
        // in `cargo xtask verify route-tags` — reads this file to prove every one of those tags
        // names a door that is actually in the wall.
        //
        // The tier is per-handler (`AdminUser`), so it travels with the handler rather than with
        // the registration. Admin and NOT `MissionMakerUser`: a `servers` row is infrastructure,
        // not mission content, and the neighbours split on exactly that line — `/factions` writes
        // are `MissionMakerUser` because a faction is authored content, while `/modpacks`,
        // `/wiki/{slug}`, `/vehicle-database` and `/admin/servers/{id}/rcon` are all `AdminUser`.
        // This row carries the `inet` + port that RCON dials and that the game-server ingest path
        // keys off, so it belongs with the second group.
        //
        // The writes live at `/servers`, not `/admin/servers`: that is what the `@route` tags
        // claim, and it is what the crate already does for every other admin write on a resource
        // the whole authenticated site can read. `/admin/*` is reserved for resources only an admin
        // may READ at all (users, audit-logs, leave-requests, rcon).
        .route(
            "/servers",
            get(handlers::servers::list_servers).post(handlers::servers::create_server),
        )
        .route(
            "/servers/{id}",
            axum::routing::patch(handlers::servers::update_server)
                .delete(handlers::servers::deactivate_server),
        )
        .route(
            "/servers/{id}/status",
            get(handlers::servers::get_server_status),
        )
        .route(
            "/servers/{id}/status/stream",
            get(handlers::leaderboards::stream_server_status),
        )
        .route("/registry", get(handlers::registry::list_registry))
        .route(
            "/registry/compat",
            get(handlers::registry::list_registry_compat),
        )
        .route(
            "/factions",
            get(handlers::factions::list_factions).post(handlers::factions::create_faction),
        )
        .route(
            "/factions/{id}",
            get(handlers::factions::get_faction)
                .put(handlers::factions::update_faction)
                .delete(handlers::factions::delete_faction),
        )
        .route("/dashboard", get(handlers::dashboard::get_dashboard))
        .route(
            "/leaderboards",
            get(handlers::leaderboards::get_leaderboards),
        )
        .route(
            "/users/{discordId}/stats",
            get(handlers::leaderboards::get_user_stats),
        )
        .route(
            "/me/deployments",
            get(handlers::deployments::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(handlers::deployments::list_my_leave).post(handlers::deployments::submit_leave),
        )
        // Admin: LOA review + audit console.
        .route(
            "/admin/leave-requests",
            get(handlers::deployments::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(handlers::deployments::review_leave),
        )
        .route("/admin/audit-logs", get(handlers::audit::list_audit_logs))
        .route(
            "/admin/audit-logs/stream",
            get(handlers::audit::stream_audit_logs),
        )
        .route(
            "/admin/audit-logs/export.csv",
            get(handlers::audit::export_audit_logs_csv),
        )
        // Mission library + editor.
        .route(
            "/missions",
            get(handlers::missions::list_missions).post(handlers::missions::create_mission),
        )
        .route(
            "/missions/{id}",
            get(handlers::missions::get_mission)
                .patch(handlers::missions::update_mission)
                .delete(handlers::missions::delete_mission),
        )
        .route(
            "/missions/{id}/submit",
            post(handlers::missions::submit_mission),
        )
        .route(
            "/missions/{id}/versions",
            // The version POST carries the compiled editor payload (hundreds of MB) —
            // override the global 1 MB body cap for this route only (Go: per-route BodyLimit).
            post(handlers::missions::create_version).layer(DefaultBodyLimit::max(version_limit)),
        )
        .route(
            "/missions/{id}/versions/{vid}",
            get(handlers::missions::get_version),
        )
        // Re-point current_version_id at a prior mission_versions row (rollback tip).
        .route(
            "/missions/{id}/versions/{vid}/set-current",
            post(handlers::missions::set_current_version),
        )
        .route(
            "/missions/{id}/armory",
            get(handlers::missions::get_armory).put(handlers::missions::set_armory),
        )
        .route(
            "/missions/{id}/bookmark",
            post(handlers::missions::bookmark_mission).delete(handlers::missions::remove_bookmark),
        )
        .route(
            "/missions/{id}/export",
            get(handlers::missions::export_mission),
        )
        .route(
            "/missions/{id}/compiled",
            get(handlers::missions::get_compiled_mission),
        )
        // Events (campaign) + ORBAT + registration.
        .route(
            "/events",
            get(handlers::events::list_events).post(handlers::events::create_event),
        )
        .route(
            "/events/{id}",
            get(handlers::events::get_event)
                .patch(handlers::events::update_event)
                .delete(handlers::events::delete_event),
        )
        .route(
            "/events/{id}/missions",
            post(handlers::events::add_event_mission),
        )
        .route(
            "/events/{id}/missions/{emid}",
            axum::routing::delete(handlers::events::remove_event_mission),
        )
        .route(
            "/event-missions/{emid}/orbat",
            get(handlers::events::get_orbat),
        )
        .route(
            "/event-missions/{emid}/register",
            post(handlers::events::register_for_event_mission)
                .delete(handlers::events::withdraw_from_event_mission),
        )
        .route(
            "/event-missions/{emid}/slots/{slotId}/assign",
            axum::routing::put(handlers::events::assign_slot).delete(handlers::events::clear_slot),
        )
        .route(
            "/event-missions/{emid}/squads/reserve",
            post(handlers::events::reserve_squad),
        )
        .route(
            "/event-missions/{emid}/squads/release",
            post(handlers::events::release_squad),
        )
        .route("/members", get(handlers::events::search_members))
        // Game-server telemetry ingest (service-token).
        .route(
            "/ingest/server-status",
            post(handlers::telemetry::ingest_server_status),
        )
        .route(
            "/ingest/match-results",
            post(handlers::telemetry::ingest_match_results),
        )
        // Game-server reads (service-token). Deliberately NOT the member-tier `/missions`
        // + `/event-missions/{emid}/orbat` handlers: both are scoped to the CALLING USER
        // (owner/bookmark filters, the caller's own registration state) and a service
        // token has no "me" — see the handler docs.
        .route(
            "/ingest/missions",
            get(handlers::missions::ingest_list_missions),
        )
        .route(
            "/ingest/events/{id}/roster",
            get(handlers::events::ingest_event_roster),
        )
        // Admin — personnel + server control.
        .route("/admin/users", get(handlers::admin::list_users))
        .route(
            "/admin/users/{discordId}",
            axum::routing::patch(handlers::admin::update_user),
        )
        .route(
            "/admin/users/{discordId}/ban",
            post(handlers::admin::ban_user).delete(handlers::admin::unban_user),
        )
        .route(
            "/admin/users/{discordId}/warnings",
            post(handlers::admin::issue_warning),
        )
        .route("/admin/roles/sync", post(handlers::admin::resync_roles))
        .route("/admin/servers/{id}/rcon", post(handlers::admin::send_rcon))
        // Corpus-wide default-override instrumentation. Lives under `/admin/*` because it is
        // an aggregate only an admin reads (not per-mission content); the handler is in `missions`
        // because it queries `mission_versions`. Tier via the per-handler `AdminUser` extractor.
        .route(
            "/admin/mission-default-overrides",
            get(handlers::missions::mission_default_overrides),
        )
        // Approvals.
        .route("/approvals", get(handlers::approvals::list_approvals))
        .route(
            "/approvals/{id}/approve",
            post(handlers::approvals::approve_mission),
        )
        .route(
            "/approvals/{id}/reject",
            post(handlers::approvals::reject_mission),
        )
        // Field tools — mortar + inject.
        .route(
            "/fire-missions/solve",
            post(handlers::field_tools::solve_fire),
        )
        .route("/fire-missions", post(handlers::field_tools::save_fire))
        .route(
            "/events/{id}/fire-missions",
            get(handlers::field_tools::list_event_fire_missions),
        )
        .route(
            "/missions/{id}/inject",
            post(handlers::field_tools::inject_mission),
        )
        // CMS — announcements + uploads.
        .route(
            "/cms/announcements",
            get(handlers::cms::list_cms_announcements).post(handlers::cms::create_announcement),
        )
        .route(
            "/cms/announcements/{id}",
            axum::routing::patch(handlers::cms::update_announcement)
                .delete(handlers::cms::delete_announcement),
        )
        .route(
            "/cms/announcements/{id}/push-discord",
            post(handlers::cms::push_announcement_discord),
        )
        .route(
            "/cms/uploads",
            post(handlers::cms::upload_image)
                .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
        );
    if dev {
        // Development-only login shortcut (also re-guards on env in-handler).
        r = r.route("/auth/dev-login", get(handlers::dev::dev_login));
    }
    r
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
        .nest("/api/v1", api_routes(dev, version_limit))
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
    // `middleware::RateLimitState`.
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
    // `middleware/ratelimit.rs`'s `/map-assets` section for the full argument, and
    // `tests/t630_map_assets_exempt.rs` for the proof that the routes above this line still refuse.
    //
    // Moving the `nest_service` below back above this layer silently re-arms the defect; that is
    // why the order is asserted by `middleware::ratelimit::tests::
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
