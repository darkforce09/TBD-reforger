//! Shared application state injected into handlers and middleware.
//!
//! `FromRef` impls let axum extractors pull one sub-state — the pool, the config, the JWT
//! manager, the SSE hub, the Discord client, the webhook client — without a handler having to
//! take (and therefore depend on) the whole struct.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use crate::community_content::services::discord_webhook::WebhookService;
use crate::core::authentication_primitives::Manager;
use crate::core::configuration::Config;
use crate::core::middleware::IpLimiter;
use crate::core::realtime_hub::Hub;
use crate::identity_and_access::services::discord_client::DiscordService;

/// Everything shared across the HTTP layer. Cheap to clone (all `Arc`/pool handles).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Arc<Config>,
    pub jwt: Arc<Manager>,
    /// Normalized (trailing-slash-trimmed) CORS allow-list.
    pub cors_origins: Arc<HashSet<String>>,
    pub rl_global: Arc<IpLimiter>,
    pub rl_strict: Arc<IpLimiter>,
    /// In-process SSE pub/sub hub (server-status fan-out).
    pub hub: Arc<Hub>,
    /// Discord OAuth2 + guild-member client.
    pub discord: Arc<DiscordService>,
    /// Announcement → Discord webhook.
    pub webhook: Arc<WebhookService>,
}

impl AppState {
    /// Build state from an open pool + loaded config. Rate limiters are sized global
    /// 20 req/s burst 40, strict 1 req/s burst 10.
    pub fn new(pool: PgPool, cfg: Config) -> Self {
        let jwt = Manager::new(&cfg.jwt_secret, cfg.jwt_access_ttl_min);
        let discord = DiscordService::new(
            cfg.discord_client_id.clone(),
            cfg.discord_client_secret.clone(),
            cfg.discord_redirect_url.clone(),
            cfg.discord_guild_id.clone(),
        );
        let webhook = WebhookService::new(cfg.discord_webhook_url.clone());
        let cors_origins = cfg
            .allowed_origins
            .iter()
            .map(|o| o.trim_end_matches('/').to_string())
            .collect();
        Self {
            pool,
            cors_origins: Arc::new(cors_origins),
            jwt: Arc::new(jwt),
            rl_global: Arc::new(IpLimiter::new(20, 40)),
            rl_strict: Arc::new(IpLimiter::new(1, 10)),
            hub: Arc::new(Hub::new()),
            discord: Arc::new(discord),
            webhook: Arc::new(webhook),
            cfg: Arc::new(cfg),
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
