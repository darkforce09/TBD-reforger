//! Runtime configuration from environment — Rust port of `internal/config`.
//!
//! 16 env vars; `DATABASE_URL` and `JWT_SECRET` are required (hard-fail). A `.env`
//! file is loaded if present but optional.

use std::env;

/// Default body cap for `POST /missions/:id/versions` (256 MB), matching Go.
const DEFAULT_MISSION_VERSION_MAX_BODY_BYTES: i64 = 256 << 20;

/// All runtime settings for the API.
#[derive(Debug, Clone)]
pub struct Config {
    // Server
    pub port: String,
    /// `"development"` | `"production"`.
    pub env: String,
    /// Reverse-proxy CIDRs whose `X-Forwarded-For` is trusted (empty = trust none).
    pub trusted_proxies: Vec<String>,

    // Frontend integration
    pub frontend_url: String,
    pub allowed_origins: Vec<String>,

    // Database
    pub database_url: String,

    // Mission editor — body cap for the versions POST only.
    pub mission_version_max_body_bytes: i64,

    // Auth
    pub jwt_secret: String,
    pub jwt_access_ttl_min: i64,

    // Discord OAuth2 + role sync
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_redirect_url: String,
    pub discord_guild_id: String,
    pub discord_bot_token: String,
    pub discord_webhook_url: String,

    // Game-server ingest authentication
    pub service_token: String,
}

/// Configuration load error — a required variable was empty.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
}

impl Config {
    /// Read configuration from the environment, applying dev defaults. Loads a
    /// `.env` if present. Hard-fails if `DATABASE_URL` or `JWT_SECRET` is empty.
    pub fn load() -> Result<Self, ConfigError> {
        // best-effort: .env is optional; real config comes from the environment.
        let _ = dotenvy::dotenv();

        let frontend_url = get_env("FRONTEND_URL", "http://localhost:5173");
        let cfg = Config {
            port: get_env("PORT", "8080"),
            env: get_env("APP_ENV", "production"),
            trusted_proxies: split_csv(&env::var("TRUSTED_PROXIES").unwrap_or_default()),
            allowed_origins: split_csv(&get_env("ALLOWED_ORIGINS", &frontend_url)),
            frontend_url,
            database_url: env::var("DATABASE_URL").unwrap_or_default(),
            mission_version_max_body_bytes: get_env_int(
                "MISSION_VERSION_MAX_BODY_BYTES",
                DEFAULT_MISSION_VERSION_MAX_BODY_BYTES,
            ),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_default(),
            jwt_access_ttl_min: get_env_int("JWT_ACCESS_TTL_MIN", 15),
            discord_client_id: env::var("DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_client_secret: env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_url: env::var("DISCORD_REDIRECT_URL").unwrap_or_default(),
            discord_guild_id: env::var("DISCORD_GUILD_ID").unwrap_or_default(),
            discord_bot_token: env::var("DISCORD_BOT_TOKEN").unwrap_or_default(),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").unwrap_or_default(),
            service_token: env::var("SERVICE_TOKEN").unwrap_or_default(),
        };

        if cfg.database_url.is_empty() {
            return Err(ConfigError::Missing("DATABASE_URL"));
        }
        if cfg.jwt_secret.is_empty() {
            return Err(ConfigError::Missing("JWT_SECRET"));
        }
        Ok(cfg)
    }

    /// Body cap (bytes) for `POST /missions/:id/versions`, falling back to 256 MB.
    pub fn mission_version_body_limit(&self) -> i64 {
        if self.mission_version_max_body_bytes > 0 {
            self.mission_version_max_body_bytes
        } else {
            DEFAULT_MISSION_VERSION_MAX_BODY_BYTES
        }
    }

    /// True when running in development mode (enables dev-login, non-Secure cookies).
    pub fn is_development(&self) -> bool {
        self.env == "development"
    }

    /// Minimal config for tests + harnesses: development env, dev CORS origin, the
    /// given DB URL + JWT secret, a non-empty service token, blank Discord creds.
    pub fn for_tests(database_url: impl Into<String>, jwt_secret: impl Into<String>) -> Self {
        Self {
            port: "0".into(),
            env: "development".into(),
            trusted_proxies: Vec::new(),
            frontend_url: "http://localhost:5173".into(),
            allowed_origins: vec!["http://localhost:5173".into()],
            database_url: database_url.into(),
            mission_version_max_body_bytes: DEFAULT_MISSION_VERSION_MAX_BODY_BYTES,
            jwt_secret: jwt_secret.into(),
            jwt_access_ttl_min: 15,
            discord_client_id: String::new(),
            discord_client_secret: String::new(),
            discord_redirect_url: String::new(),
            discord_guild_id: String::new(),
            discord_bot_token: String::new(),
            discord_webhook_url: String::new(),
            service_token: "test-service-token".into(),
        }
    }
}

fn get_env(key: &str, fallback: &str) -> String {
    match env::var(key) {
        Ok(v) if !v.is_empty() => v,
        _ => fallback.to_string(),
    }
}

/// Parse a comma-separated env value into a trimmed, non-empty list.
fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect()
}

fn get_env_int(key: &str, fallback: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}
