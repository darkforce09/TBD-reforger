//! Runtime configuration from environment — Rust port of `internal/config`.
//!
//! 16 env vars; `DATABASE_URL` and `JWT_SECRET` are always required (hard-fail).
//! In non-development, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, and
//! `DISCORD_REDIRECT_URL` are also required (T-248 / T-430) so blank OAuth
//! creds cannot load config and later surface as `oauth_unconfigured` /
//! `discord_unreachable`. A `.env` file is loaded if present but optional.
//!
//! `DISCORD_BOT_TOKEN` (T-279) is **optional** — no consumer reads it yet, so
//! requiring it would fail boot for a feature that does not exist. It is read
//! through [`Config::require_discord_bot_token`], which makes "unset" a named
//! error rather than an empty `Bot ` header; a set-but-whitespace-bearing token
//! is rejected at boot because it can never authenticate.

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
    /// T-159.29 — the Leptos SPA `dist/` to serve statically (with COOP/COEP + SPA fallback). Empty
    /// = don't serve a SPA (dev uses `trunk serve`; the API is API-only). Set at the cutover flip.
    pub spa_dist_dir: String,
    /// T-159.29 — the map-assets dir served at `/map-assets` when a SPA is served (the editor's DEM
    /// / basemap / world chunks). Empty defaults to `../../packages/map-assets` relative to the CWD.
    pub map_assets_dir: String,

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

/// Configuration load error — a required variable was empty or unusable.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    /// Set, non-empty, and still cannot work. Rejected at boot rather than at
    /// first use, where it would surface as a *remote* error and read as an
    /// outage (T-279) — the same disguise class as T-248 / T-481 / T-484.
    #[error("{0} is malformed: {1}")]
    Malformed(&'static str, &'static str),
}

impl Config {
    /// Read configuration from the environment, applying dev defaults. Loads a
    /// `.env` if present. Hard-fails if `DATABASE_URL` or `JWT_SECRET` is empty;
    /// in production also hard-fails on blank Discord client id/secret/redirect
    /// (T-248 / T-430).
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
            spa_dist_dir: env::var("SPA_DIST_DIR").unwrap_or_default(),
            map_assets_dir: env::var("MAP_ASSETS_DIR").unwrap_or_default(),
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

        cfg.validate()
    }

    /// Fail closed on required fields. Separated from [`Self::load`] so unit tests
    /// can exercise the production Discord guard without mutating process env.
    fn validate(self) -> Result<Self, ConfigError> {
        if self.database_url.is_empty() {
            return Err(ConfigError::Missing("DATABASE_URL"));
        }
        if self.jwt_secret.is_empty() {
            return Err(ConfigError::Missing("JWT_SECRET"));
        }
        // Production: blank Discord OAuth fields used to load config and later
        // surface as `oauth_unconfigured` / `discord_unreachable` — misconfig
        // disguised as an outage (T-248 secret/redirect; T-430 client id).
        // Development keeps blank Discord so `Config::for_tests` and dev-login work.
        if !self.is_development() {
            // T-481 / T-484 — trim before Missing: `" "` is not empty to `is_empty()`
            // but is not a configured Discord OAuth value. Reject trim-empty for
            // client id, secret, and redirect; do not store a trimmed value.
            if self.discord_client_id.trim().is_empty() {
                return Err(ConfigError::Missing("DISCORD_CLIENT_ID"));
            }
            if self.discord_client_secret.trim().is_empty() {
                return Err(ConfigError::Missing("DISCORD_CLIENT_SECRET"));
            }
            if self.discord_redirect_url.trim().is_empty() {
                return Err(ConfigError::Missing("DISCORD_REDIRECT_URL"));
            }
        }
        // T-279 — the bot token is NOT required (no consumer yet; see
        // `require_discord_bot_token`), so empty/absent stays legal in every env
        // and means "the bot is not configured". But a *non-empty* token that
        // carries whitespace can never authenticate: it goes out as the header
        // `Authorization: Bot <token>`, and a stray `\n` from a copy-paste or a
        // secrets manager either makes the header invalid (reqwest refuses to
        // build it) or earns a flat 401 from Discord. Both read as "Discord is
        // down" at the call site instead of "your token has a newline in it".
        // Discord bot tokens are `.`-joined base64url segments — no valid token
        // contains a whitespace character anywhere, so this rejects only lies.
        if !self.discord_bot_token.is_empty()
            && self.discord_bot_token.contains(char::is_whitespace)
        {
            return Err(ConfigError::Malformed(
                "DISCORD_BOT_TOKEN",
                "contains whitespace",
            ));
        }
        Ok(self)
    }

    /// True when a Discord bot token is configured at all.
    ///
    /// Mirrors `handlers::oauth::guild_configured` — an unconfigured integration
    /// must be distinguishable from a broken one, or the misconfiguration hides
    /// inside whatever the remote call happens to return.
    pub fn discord_bot_configured(&self) -> bool {
        !self.discord_bot_token.is_empty()
    }

    /// The bot token, or a named [`ConfigError::Missing`] when it is unset.
    ///
    /// T-279 — this is the ONLY supported way to read `DISCORD_BOT_TOKEN`. The
    /// raw field is an empty `String` when unconfigured, and the failure mode
    /// that matters is a caller sending `Authorization: Bot ` with nothing after
    /// it: Discord answers 401, and a 401 from Discord is indistinguishable from
    /// a revoked token or a real outage. Going through this accessor turns
    /// "unset" into a boot-shaped, named error at the point of use instead.
    ///
    /// Validation guarantees the returned value is non-empty and whitespace-free.
    pub fn require_discord_bot_token(&self) -> Result<&str, ConfigError> {
        if !self.discord_bot_configured() {
            return Err(ConfigError::Missing("DISCORD_BOT_TOKEN"));
        }
        Ok(&self.discord_bot_token)
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
            spa_dist_dir: String::new(),
            map_assets_dir: String::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn production_base() -> Config {
        let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
        cfg.env = "production".into();
        cfg.discord_client_id = "client-id".into();
        cfg.discord_client_secret = "secret".into();
        cfg.discord_redirect_url = "https://example.com/callback".into();
        cfg
    }

    #[test]
    fn production_rejects_blank_discord_client_id() {
        let mut cfg = production_base();
        cfg.discord_client_id.clear();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_ID), got {other:?}"),
        }
    }

    /// T-481 Class-R — production must reject whitespace-only `DISCORD_CLIENT_ID`.
    /// Pre-fix: `is_empty()` only → `" "` validates Ok and later surfaces as
    /// `oauth_unconfigured` instead of a boot-time Missing.
    #[test]
    fn production_rejects_whitespace_only_discord_client_id() {
        let mut cfg = production_base();
        cfg.discord_client_id = " ".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_ID), got {other:?}"),
        }
        // Tab / mixed whitespace are the same lie as a single space.
        cfg = production_base();
        cfg.discord_client_id = "\t  \n".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_ID")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_ID) for mixed ws, got {other:?}"),
        }
    }

    #[test]
    fn production_rejects_blank_discord_client_secret() {
        let mut cfg = production_base();
        cfg.discord_client_secret.clear();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_SECRET), got {other:?}"),
        }
    }

    /// T-484 Class-R — production must reject whitespace-only `DISCORD_CLIENT_SECRET`.
    /// Pre-fix: `is_empty()` only → `" "` validates Ok (same disguise class as
    /// pre-T-481 client id).
    #[test]
    fn production_rejects_whitespace_only_discord_client_secret() {
        let mut cfg = production_base();
        cfg.discord_client_secret = " ".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_SECRET), got {other:?}"),
        }
        cfg = production_base();
        cfg.discord_client_secret = "\t  \n".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_CLIENT_SECRET")) => {}
            other => panic!("expected Missing(DISCORD_CLIENT_SECRET) for mixed ws, got {other:?}"),
        }
    }

    #[test]
    fn production_rejects_blank_discord_redirect_url() {
        let mut cfg = production_base();
        cfg.discord_redirect_url.clear();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
            other => panic!("expected Missing(DISCORD_REDIRECT_URL), got {other:?}"),
        }
    }

    /// T-484 Class-R — production must reject whitespace-only `DISCORD_REDIRECT_URL`.
    /// Pre-fix: `is_empty()` only → `" "` validates Ok (same disguise class as
    /// pre-T-481 client id).
    #[test]
    fn production_rejects_whitespace_only_discord_redirect_url() {
        let mut cfg = production_base();
        cfg.discord_redirect_url = " ".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
            other => panic!("expected Missing(DISCORD_REDIRECT_URL), got {other:?}"),
        }
        cfg = production_base();
        cfg.discord_redirect_url = "\t  \n".into();
        match cfg.validate() {
            Err(ConfigError::Missing("DISCORD_REDIRECT_URL")) => {}
            other => panic!("expected Missing(DISCORD_REDIRECT_URL) for mixed ws, got {other:?}"),
        }
    }

    #[test]
    fn development_allows_blank_discord() {
        // Config::for_tests + local APP_ENV=development must keep blank Discord
        // for dev-login; only production fails closed.
        let cfg = Config::for_tests("postgres://x/x", "jwt-secret");
        assert!(cfg.is_development());
        assert!(cfg.discord_client_id.is_empty());
        assert!(cfg.discord_client_secret.is_empty());
        assert!(cfg.discord_redirect_url.is_empty());
        cfg.validate().expect("development blank Discord must load");
    }

    #[test]
    fn production_with_discord_creds_loads() {
        production_base()
            .validate()
            .expect("production with Discord client id+secret+redirect must load");
    }

    // ---- T-279 DISCORD_BOT_TOKEN ----------------------------------------

    /// Unset is legal (no consumer yet) but must NOT read as a usable token.
    /// The whole point of the accessor: `""` can never escape as a `Bot `
    /// header, it becomes a named error instead.
    #[test]
    fn unset_bot_token_loads_but_is_not_readable() {
        let cfg = production_base();
        assert!(cfg.discord_bot_token.is_empty());
        let cfg = cfg.validate().expect("unset bot token must not block boot");
        assert!(!cfg.discord_bot_configured());
        match cfg.require_discord_bot_token() {
            Err(ConfigError::Missing("DISCORD_BOT_TOKEN")) => {}
            other => panic!("expected Missing(DISCORD_BOT_TOKEN), got {other:?}"),
        }
    }

    /// `DISCORD_BOT_TOKEN=` in `.env` decodes to `Ok("")`, not absent — the live
    /// `.env` is exactly this today, so it must stay bootable in every env.
    #[test]
    fn empty_bot_token_is_unconfigured_in_development_too() {
        let cfg = Config::for_tests("postgres://x/x", "jwt-secret");
        assert!(cfg.is_development());
        let cfg = cfg.validate().expect("development empty bot token must load");
        assert!(!cfg.discord_bot_configured());
    }

    /// A real token round-trips and is readable.
    #[test]
    fn configured_bot_token_is_readable() {
        let mut cfg = production_base();
        cfg.discord_bot_token = "MTIzNDU2.Nzg5MA.abcdefGHIJKL-_".into();
        let cfg = cfg.validate().expect("a well-formed bot token must load");
        assert!(cfg.discord_bot_configured());
        assert_eq!(
            cfg.require_discord_bot_token().expect("readable"),
            "MTIzNDU2.Nzg5MA.abcdefGHIJKL-_"
        );
    }

    /// Whitespace-only is the T-481/T-484 class-R lie applied to the bot token:
    /// non-empty to `is_empty()`, unusable to Discord.
    #[test]
    fn whitespace_only_bot_token_is_rejected_at_boot() {
        for bad in [" ", "\t", "\n", "\t  \n"] {
            let mut cfg = production_base();
            cfg.discord_bot_token = bad.into();
            match cfg.validate() {
                Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
                other => panic!("expected Malformed(DISCORD_BOT_TOKEN) for {bad:?}, got {other:?}"),
            }
        }
    }

    /// The failure this actually prevents: a trailing newline from a copy-paste
    /// or a secrets-manager read. `is_empty()` and `trim().is_empty()` BOTH pass
    /// it, so only a whitespace-anywhere rule catches it — and uncaught it ships
    /// an invalid `Authorization` header that Discord answers with 401.
    #[test]
    fn bot_token_with_surrounding_or_inner_whitespace_is_rejected() {
        for bad in [
            "MTIzNDU2.Nzg5MA.abcdef\n",
            " MTIzNDU2.Nzg5MA.abcdef",
            "MTIzNDU2.Nzg5MA.abcdef\r\n",
            "MTIzNDU2 .Nzg5MA.abcdef",
        ] {
            let mut cfg = production_base();
            cfg.discord_bot_token = bad.into();
            match cfg.validate() {
                Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
                other => panic!("expected Malformed(DISCORD_BOT_TOKEN) for {bad:?}, got {other:?}"),
            }
        }
    }

    /// Development must not be a hole: an unusable token is unusable everywhere.
    /// (Unlike the OAuth trio, this rule is env-independent — a whitespace token
    /// is never a legitimate dev state, it is always a typo.)
    #[test]
    fn whitespace_bot_token_is_rejected_in_development_too() {
        let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
        cfg.discord_bot_token = " ".into();
        assert!(cfg.is_development());
        match cfg.validate() {
            Err(ConfigError::Malformed("DISCORD_BOT_TOKEN", _)) => {}
            other => panic!("expected Malformed(DISCORD_BOT_TOKEN) in dev, got {other:?}"),
        }
    }
}
