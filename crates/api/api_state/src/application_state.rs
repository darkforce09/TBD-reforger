//! Shared application state injected into handlers and middleware.
//!
//! **Role:** the one dependency container of the HTTP layer: the pool, the configuration, the
//! token manager, the session authority, the CORS allow-list, the rate limiters, the SSE hub, the
//! metrics registry, the Discord and webhook clients and the equipment datasets.
//! **Position:** above the HTTP layer, the configuration, the Discord clients and the equipment
//! datasets, below the domains. The services that need a concrete implementation (the session
//! authority, the Discord and webhook clients, the equipment datasets) are built by the API's
//! composition root (`api_server::composition::application_state`) and injected through
//! [`AppState::new`], so this file names no domain.
//! **Signals & state:** every field is an `Arc` or a pool handle, so a clone shares them all; the
//! rate limiters and the metrics registry are the mutable state every request shares.
//! **Invariants:** `FromRef` implementations let an extractor or a middleware take one sub-state
//! (the pool, the config, the token manager, the hub, a client, the session authority, the CORS
//! allow-list, the rate-limit state) without depending on the whole struct.

use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use api_configuration::configuration::Config;
use api_discord::discord_client::DiscordService;
use api_discord::discord_webhook::WebhookService;
use api_equipment_datasets::EquipmentDatasets;
use api_http_layer::authentication_primitives::Manager;
use api_http_layer::authentication_primitives::session_authority::SessionAuthority;
use api_http_layer::middleware::cross_origin::CorsOrigins;
use api_http_layer::middleware::{IpLimiter, RateLimitState};
use api_http_layer::observability::metrics_registry::Registry;
use api_http_layer::realtime_hub::Hub;

/// Everything shared across the HTTP layer. Cheap to clone (all `Arc`/pool handles).
#[derive(Clone)]
pub struct AppState {
    /// The gameplay and diagnostic equipment datasets the debug viewer reads.
    pub equipment_data: Arc<EquipmentDatasets>,
    /// The Postgres pool every handler and worker runs its statements on.
    pub pool: PgPool,
    /// The configuration loaded at boot.
    pub cfg: Arc<Config>,
    /// The access token manager, keyed by the configured secret and lifetime.
    pub jwt: Arc<Manager>,
    /// The authority that decides a session's current permissions for the `AuthUser` extractor.
    pub session_authority: Arc<dyn SessionAuthority>,
    /// Normalized (trailing-slash-trimmed) CORS allow-list.
    pub cors_origins: CorsOrigins,
    /// The in-memory limiters, the durable strict tier and the trusted proxies of the
    /// rate-limit middleware.
    pub rate_limits: RateLimitState,
    /// In-process SSE pub/sub hub (server-status fan-out).
    pub hub: Arc<Hub>,
    /// The Prometheus metrics of this state: the router's middleware, `/metrics` and `/healthz`
    /// and the background workers all record into and read this one registry.
    pub metrics_registry: Arc<Registry>,
    /// Discord OAuth2 + guild-member client.
    pub discord: Arc<DiscordService>,
    /// Announcement → Discord webhook.
    pub webhook: Arc<WebhookService>,
}

impl AppState {
    /// Build state from an open pool, the loaded config and the injected services. Rate limiters
    /// are sized global 20 req/s burst 40, strict 1 req/s burst 10.
    pub fn new(
        pool: PgPool,
        cfg: Arc<Config>,
        session_authority: Arc<dyn SessionAuthority>,
        discord: Arc<DiscordService>,
        webhook: Arc<WebhookService>,
        equipment_data: Arc<EquipmentDatasets>,
    ) -> Self {
        let jwt = Manager::new(&cfg.jwt_secret, cfg.jwt_access_ttl_min);
        let rate_limits = RateLimitState::new(
            pool.clone(),
            &cfg.trusted_proxies,
            Arc::new(IpLimiter::new(20, 40)),
            Arc::new(IpLimiter::new(1, 10)),
        );
        Self {
            equipment_data,
            session_authority,
            cors_origins: CorsOrigins::from_configured(&cfg.allowed_origins),
            rate_limits,
            pool,
            jwt: Arc::new(jwt),
            hub: Arc::new(Hub::new()),
            metrics_registry: Arc::new(Registry::new()),
            discord,
            webhook,
            cfg,
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(s: &AppState) -> Self {
        s.pool.clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(s: &AppState) -> Self {
        s.cfg.clone()
    }
}

impl FromRef<AppState> for Arc<Manager> {
    fn from_ref(s: &AppState) -> Self {
        s.jwt.clone()
    }
}

impl FromRef<AppState> for Arc<Hub> {
    fn from_ref(s: &AppState) -> Self {
        s.hub.clone()
    }
}

impl FromRef<AppState> for Arc<DiscordService> {
    fn from_ref(s: &AppState) -> Self {
        s.discord.clone()
    }
}

impl FromRef<AppState> for Arc<WebhookService> {
    fn from_ref(s: &AppState) -> Self {
        s.webhook.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SessionAuthority> {
    fn from_ref(state: &AppState) -> Self {
        state.session_authority.clone()
    }
}

impl FromRef<AppState> for CorsOrigins {
    fn from_ref(state: &AppState) -> Self {
        state.cors_origins.clone()
    }
}

impl FromRef<AppState> for RateLimitState {
    fn from_ref(state: &AppState) -> Self {
        state.rate_limits.clone()
    }
}
