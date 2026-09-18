//! Runtime configuration read from the environment at boot.
//!
//! `DATABASE_URL` and `JWT_SECRET` are always required (hard-fail). Outside development,
//! `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, and `DISCORD_REDIRECT_URL` are required too,
//! so blank OAuth credentials cannot load and then surface at first use as
//! `oauth_unconfigured` / `discord_unreachable` — a misconfiguration wearing the costume of an
//! outage. A `.env` file is loaded when present, but is optional.
//!
//! `DISCORD_BOT_TOKEN` and `GAME_AGENT_SOCKET` are optional: empty means "not configured".
//! Each is read only through an accessor ([`Config::require_discord_bot_token`],
//! [`Config::require_game_agent_socket`]) that turns "unset" into a named error at the point of
//! use, instead of an empty `Bot ` header or a `Path::new("")` that connects to nothing. A
//! variable is added here together with the code that reads it, so this file cannot accumulate
//! settings that look configured and do nothing.
//!
//! The four `TBD_DB_POOL_*` knobs are read at pool open by
//! [`DbPoolConfig::from_env`], not by [`Config::load`]: `Config` carries no field nobody
//! consumes, and the binary hands the connector a URL rather than a `Config`. A malformed value
//! is a [`ConfigError::MalformedValue`] naming the variable.
//!
//! [`DbPoolConfig::from_env`]: crate::core::database::connection_pool::DbPoolConfig::from_env
//! [`server_infrastructure::handlers::rcon_console::send_rcon`]: crate::server_infrastructure::handlers::rcon_console::send_rcon

pub mod proxy_network;

use std::env;
use std::path::Path;

use proxy_network::parse_trusted_proxies;

/// Default body cap for `POST /missions/:id/versions` (256 MB).
const DEFAULT_MISSION_VERSION_MAX_BODY_BYTES: i64 = 256 << 20;

/// All runtime settings for the API.
#[derive(Debug, Clone)]
pub struct Config {
    // Server
    pub port: String,
    /// `"development"` | `"production"`.
    pub env: String,
    /// Reverse-proxy addresses/CIDRs whose `X-Forwarded-For` is trusted (empty = trust none).
    ///
    /// The consumer is [`crate::core::middleware::RateLimitState`]: behind a loopback reverse proxy
    /// every public client shares one `ConnectInfo` peer, so without this list they would share
    /// one rate-limit bucket. Entries are validated at boot by [`proxy_network::ProxyNet::parse`] — an
    /// unparseable entry is a boot failure, not a silently-ignored line that leaves the operator
    /// believing the header is honoured.
    ///
    /// Empty is the default and means the header is ignored **entirely**. That is deliberate:
    /// `X-Forwarded-For` is client-controllable, and a rate-limit key any client can forge is
    /// worse than one everybody shares — shared means everyone is limited together, forgeable
    /// means nobody is limited at all.
    pub trusted_proxies: Vec<String>,

    // Frontend integration
    pub frontend_url: String,
    pub allowed_origins: Vec<String>,
    /// The Leptos SPA `dist/` to serve statically (with COOP/COEP + SPA fallback). Empty
    /// = don't serve a SPA (dev uses `trunk serve`; the API is API-only).
    pub spa_dist_dir: String,
    /// The map-assets dir served at `/map-assets` when a SPA is served (the editor's DEM /
    /// basemap / world chunks). Empty defaults to `../../assets_v2/terrains` relative to the CWD.
    pub map_assets_dir: String,
    /// The glyph dir served at `/map-assets/glyphs` (the tactical marker atlas, shared by every
    /// terrain). Empty defaults to `../../assets_v2/glyphs` relative to the CWD.
    pub glyph_assets_dir: String,

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

    /// Absolute path of the host control agent socket, e.g.
    /// `/run/user/1000/tbd-reforger-agent.sock` (the systemd unit renders it as
    /// `%t/tbd-reforger-agent.sock`). **Empty = no transport**, and `send_rcon` keeps answering
    /// 503 rather than pretending. Read through [`Config::require_game_agent_socket`].
    pub game_agent_socket: String,
}

/// Configuration load error — a required variable was empty or unusable.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    /// Set, non-empty, and still cannot work. Rejected at boot rather than at
    /// first use, where it would surface as a *remote* error and read as an outage.
    #[error("{0} is malformed: {1}")]
    Malformed(&'static str, &'static str),
    /// Same rule as [`Self::Malformed`], for a **list** variable where the reason is useless
    /// without the offending entry: `TRUSTED_PROXIES` can hold a dozen entries and "is malformed"
    /// would send the operator reading all of them. Carries the entry verbatim.
    #[error("{0} entry {1:?} is malformed: {2}")]
    MalformedEntry(&'static str, String, &'static str),
    /// A scalar variable that is set but does not parse (the `TBD_DB_POOL_*` knobs). Carries the
    /// value verbatim, `{:?}`-quoted, for the reason [`Self::MalformedEntry`] does (`"5m"`).
    #[error("{0} value {1:?} is malformed: {2}")]
    MalformedValue(&'static str, String, &'static str),
}

impl Config {
    /// Read configuration from the environment, applying dev defaults. Loads a
    /// `.env` if present. Hard-fails if `DATABASE_URL` or `JWT_SECRET` is empty;
    /// outside development also hard-fails on blank Discord client id/secret/redirect.
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
            glyph_assets_dir: env::var("GLYPH_ASSETS_DIR").unwrap_or_default(),
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
            game_agent_socket: env::var("GAME_AGENT_SOCKET").unwrap_or_default(),
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
        // Outside development, blank Discord OAuth fields would load and later surface as
        // `oauth_unconfigured` / `discord_unreachable` — misconfiguration disguised as an
        // outage. Development keeps blank Discord so `Config::for_tests` and dev-login work.
        if !self.is_development() {
            // Trim before Missing: `" "` is not empty to `is_empty()` but is not a configured
            // Discord OAuth value either. Reject trim-empty; do not store a trimmed value.
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
        // The bot token is not required (see `require_discord_bot_token`), so empty stays legal
        // in every env and means "the bot is not configured". A *non-empty* token carrying
        // whitespace can never authenticate: it goes out as `Authorization: Bot <token>`, and a
        // stray `\n` from a copy-paste or a secrets manager either makes the header invalid
        // (reqwest refuses to build it) or earns a flat 401 from Discord. Both read as "Discord
        // is down" at the call site instead of "your token has a newline in it". Real tokens are
        // `.`-joined base64url segments and contain no whitespace, so this rejects only lies.
        if !self.discord_bot_token.is_empty()
            && self.discord_bot_token.contains(char::is_whitespace)
        {
            return Err(ConfigError::Malformed(
                "DISCORD_BOT_TOKEN",
                "contains whitespace",
            ));
        }
        // The agent socket is optional (empty = no transport, which `send_rcon` reports honestly
        // as 503). A *set* value that cannot work dies at boot, not at 03:00 when an admin clicks
        // Restart. Two rules, both for failures that would otherwise arrive disguised as "the
        // game server is unreachable":
        //
        // 1. Leading/trailing whitespace — the copy-paste / secrets-manager newline.
        //    `"/run/user/1000/x.sock\n"` is a plausible `.env` value and `connect(2)` on it is
        //    ENOENT, which reads as an unreachable agent. Inner spaces are legal in a path and
        //    are NOT rejected.
        // 2. Not absolute — `UnixStream::connect` resolves a relative path against the API
        //    process's CWD, a systemd/launcher detail nobody sets deliberately. It would either
        //    miss (ENOENT) or, worse, hit a different socket than intended.
        if !self.game_agent_socket.is_empty() {
            if self.game_agent_socket != self.game_agent_socket.trim() {
                return Err(ConfigError::Malformed(
                    "GAME_AGENT_SOCKET",
                    "has leading or trailing whitespace",
                ));
            }
            if !Path::new(&self.game_agent_socket).is_absolute() {
                return Err(ConfigError::Malformed(
                    "GAME_AGENT_SOCKET",
                    "must be an absolute path",
                ));
            }
        }
        // `TRUSTED_PROXIES` decides whether a client-supplied header is believed, so a typo in it
        // must not be survivable. Unset stays legal and means "trust none"; a *set* entry that
        // does not parse dies here rather than being skipped at request time, where the operator
        // would see a running API and a header that is quietly still ignored.
        if let Err((entry, why)) = parse_trusted_proxies(&self.trusted_proxies) {
            return Err(ConfigError::MalformedEntry("TRUSTED_PROXIES", entry, why));
        }
        Ok(self)
    }

    /// True when a Discord bot token is configured at all.
    ///
    /// An unconfigured integration must be distinguishable from a broken one, or the
    /// misconfiguration hides inside whatever the remote call happens to return.
    pub fn discord_bot_configured(&self) -> bool {
        !self.discord_bot_token.is_empty()
    }

    /// The bot token, or a named [`ConfigError::Missing`] when it is unset.
    ///
    /// This is the only supported way to read `DISCORD_BOT_TOKEN`. The raw field is an empty
    /// `String` when unconfigured, and the failure mode that matters is a caller sending
    /// `Authorization: Bot ` with nothing after it: Discord answers 401, and a 401 from Discord
    /// is indistinguishable from a revoked token or a real outage. Going through this accessor
    /// turns "unset" into a named error at the point of use instead.
    ///
    /// Validation guarantees the returned value is non-empty and whitespace-free.
    pub fn require_discord_bot_token(&self) -> Result<&str, ConfigError> {
        if !self.discord_bot_configured() {
            return Err(ConfigError::Missing("DISCORD_BOT_TOKEN"));
        }
        Ok(&self.discord_bot_token)
    }

    /// True when a game-agent socket path is configured at all.
    ///
    /// Same shape as [`Self::discord_bot_configured`]: an unconfigured channel must be
    /// distinguishable from a broken one, or "nobody set this up" hides inside whatever
    /// `connect(2)` happens to return.
    pub fn game_agent_configured(&self) -> bool {
        !self.game_agent_socket.is_empty()
    }

    /// The agent socket path, or a named [`ConfigError::Missing`] when unset.
    ///
    /// The **only** supported way to read `GAME_AGENT_SOCKET`. The raw field is `""` when
    /// unconfigured, and `Path::new("")` is a real path that `UnixStream::connect` answers with
    /// ENOENT — indistinguishable at the call site from a game host whose agent has crashed.
    /// Going through this accessor turns "nobody configured a transport" into a named condition
    /// the handler can report as such.
    ///
    /// Validation guarantees the returned path is absolute and free of surrounding whitespace.
    pub fn require_game_agent_socket(&self) -> Result<&Path, ConfigError> {
        if !self.game_agent_configured() {
            return Err(ConfigError::Missing("GAME_AGENT_SOCKET"));
        }
        Ok(Path::new(&self.game_agent_socket))
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
            glyph_assets_dir: String::new(),
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
            // Unconfigured by default: a test that wants the RCON transport stands up its
            // own socket and sets this, so no suite can accidentally reach a real agent.
            game_agent_socket: String::new(),
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
#[path = "tests/configuration.rs"]
mod tests;
