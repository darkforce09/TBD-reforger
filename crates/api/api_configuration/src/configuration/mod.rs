//! Runtime configuration read from the environment at boot.
//!
//! `DATABASE_URL` and `JWT_SECRET` are always required (hard-fail). Outside development,
//! `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, and `DISCORD_REDIRECT_URL` are required too,
//! so blank OAuth credentials cannot load and then surface at first use as
//! `oauth_unconfigured` / `discord_unreachable` — a misconfiguration wearing the costume of an
//! outage. The settings file (`deploy/api.env`, or the file `TBD_API_ENV_FILE` names) is loaded
//! when present, but is optional.
//!
//! `DISCORD_BOT_TOKEN` is optional: empty means "not configured". It is read only through
//! [`Config::require_discord_bot_token`], which turns "unset" into a named error at the point of
//! use instead of an empty `Bot ` header. A variable is added here together with the code that
//! reads it, so this file cannot accumulate settings that look configured and do nothing.
//!
//! The four `TBD_DB_POOL_*` knobs are read at pool open by `api_database`'s
//! `DbPoolConfig::from_env`, not by [`Config::load`]: `Config` carries no field nobody
//! consumes, and the binary hands the connector a URL rather than a `Config`. A malformed value
//! is a [`ConfigError::MalformedValue`] naming the variable.
//!
//! The four directory settings `UPLOAD_DIR`, `EQUIPMENT_DATA_DIR`, `MAP_ASSETS_DIR` and
//! `GLYPH_ASSETS_DIR` default, in development only, to folders of the checkout, joined onto the
//! checkout root the `repository_root` walk finds from the working directory; a development boot
//! outside any checkout with one of them unset is a [`ConfigError::CheckoutRootNotFound`].

mod development_directories;
pub mod proxy_network;
pub mod settings_file;

use api_identifiers::{DiscordClientId, DiscordGuildId};
use std::env;
use std::path::Path;

use development_directories::{
    CheckoutDirectory, EQUIPMENT_DATA, GLYPH_ASSETS, MAP_ASSETS, UPLOAD,
};
use proxy_network::parse_trusted_proxies;

/// Default body cap for `POST /missions/:id/versions` (256 MB).
const DEFAULT_MISSION_VERSION_MAX_BODY_BYTES: i64 = 256 << 20;

/// All runtime settings for the API.
#[derive(Debug, Clone)]
pub struct Config {
    // Server
    /// `PORT`: the TCP port the server listens on.
    pub port: String,
    /// `"development"` | `"production"`.
    pub env: String,
    /// Reverse-proxy addresses/CIDRs whose `X-Forwarded-For` is trusted (empty = trust none).
    ///
    /// The consumer is the API's rate limiting middleware state (`RateLimitState`): behind a
    /// loopback reverse proxy every public client shares one `ConnectInfo` peer, so without this
    /// list they would share one rate-limit bucket. Entries are validated at boot by
    /// [`proxy_network::ProxyNet::parse`] — an unparseable entry is a boot failure, not a
    /// silently-ignored line that leaves the operator believing the header is honoured.
    ///
    /// Empty is the default and means the header is ignored **entirely**. That is deliberate:
    /// `X-Forwarded-For` is client-controllable, and a rate-limit key any client can forge is
    /// worse than one everybody shares — shared means everyone is limited together, forgeable
    /// means nobody is limited at all.
    pub trusted_proxies: Vec<String>,

    // Frontend integration
    /// `FRONTEND_URL`: the single-page app's origin, where the OAuth callback lands.
    pub frontend_url: String,
    /// `ALLOWED_ORIGINS`: the origins the CORS layer admits.
    pub allowed_origins: Vec<String>,
    /// The Leptos SPA `dist/` to serve statically (with COOP/COEP + SPA fallback). Empty
    /// = don't serve a SPA (dev uses `trunk serve`; the API is API-only).
    pub spa_dist_dir: String,
    /// `MAP_ASSETS_DIR`: the terrain tree served at `/map-assets` (the editor's DEM, basemap and
    /// world chunks). In development an unset value is the checkout's `assets/terrains`; outside
    /// development it is required and must be absolute — see
    /// `deploy/systemd/tbd-website-api.service`.
    pub map_assets_dir: String,
    /// `GLYPH_ASSETS_DIR`: the glyph tree served at `/map-assets/glyphs` (the tactical marker
    /// atlas, shared by every terrain). In development an unset value is the checkout's
    /// `assets/glyphs`; outside development it is required and must be absolute.
    pub glyph_assets_dir: String,

    // Runtime storage — what the API writes. Never inside the source tree in production.
    /// `UPLOAD_DIR`: the directory the CMS thumbnail upload writes into and `/uploads` serves
    /// from. In development an unset value is the checkout's gitignored
    /// `assets/scratch/api/uploads`; outside development it is required and must be absolute,
    /// because the process working directory is a deployment detail and the checkout is what the
    /// deploy rsyncs with `--delete` — see `deploy/systemd/tbd-website-api.service`.
    pub upload_dir: String,

    /// `EQUIPMENT_DATA_DIR`: the original published exports and the rebuildable viewer indexes.
    /// In development an unset value is the checkout's gitignored `assets/equipment`; outside
    /// development an unset value leaves the datasets unconfigured and a set one must be absolute.
    pub equipment_data_dir: String,
    /// Optional Workbench publication root checked by the import worker.
    pub equipment_export_source_dir: Option<String>,

    // Database
    /// `DATABASE_URL`: the Postgres connection string; required.
    pub database_url: String,

    // Mission editor — body cap for the versions POST only.
    /// `MISSION_VERSION_MAX_BODY_BYTES`: the body cap of the mission versions POST.
    pub mission_version_max_body_bytes: i64,

    // Auth
    /// `JWT_SECRET`: the HS256 key access tokens are signed with; required.
    pub jwt_secret: String,
    /// `JWT_ACCESS_TTL_MIN`: the lifetime of an access token, in minutes.
    pub jwt_access_ttl_min: i64,

    // Discord OAuth2 + role sync
    /// `DISCORD_CLIENT_ID`: the OAuth2 application id; required outside development.
    pub discord_client_id: DiscordClientId,
    /// `DISCORD_CLIENT_SECRET`: the OAuth2 application secret; required outside development.
    pub discord_client_secret: String,
    /// `DISCORD_REDIRECT_URL`: the OAuth2 redirect URL; required outside development.
    pub discord_redirect_url: String,
    /// `DISCORD_GUILD_ID`: the community guild whose members and roles the API reads.
    pub discord_guild_id: DiscordGuildId,
    /// `DISCORD_BOT_TOKEN`: the bot token of the guild reads; empty means not configured, read
    /// through [`Config::require_discord_bot_token`].
    pub discord_bot_token: String,
    /// `DISCORD_WEBHOOK_URL`: the announcement webhook; empty means announcements are not pushed.
    pub discord_webhook_url: String,

    /// Operator bearer token of `/metrics` and the detailed `/healthz`
    /// (the API's `observability_auth`); empty serves neither.
    pub observability_token: String,
}

/// Configuration load error — a required variable was empty or unusable.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A required variable is unset or empty.
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
    /// A directory setting is unset in development, and its default lies under the checkout root,
    /// which the walk from the working directory did not find. A development process started
    /// outside the checkout names its directories itself; the default never falls back to a path
    /// relative to the working directory.
    #[error("{0} is unset, and its development default needs the checkout root: {1}")]
    CheckoutRootNotFound(&'static str, repository_root::Error),
}

impl Config {
    /// Read configuration from the environment, applying dev defaults. Loads the settings file
    /// ([`settings_file::load_settings_file`]) if present. Hard-fails if `DATABASE_URL` or `JWT_SECRET` is empty;
    /// outside development also hard-fails on blank Discord client id/secret/redirect and on an
    /// unset or relative upload, map or glyph directory.
    ///
    /// # Errors
    ///
    /// A [`ConfigError`] naming the first variable that is missing or unusable, or the directory
    /// setting whose development default finds no checkout root above the working directory.
    pub fn load() -> Result<Self, ConfigError> {
        // Best effort: the settings file is optional; the environment stays authoritative.
        let _ = settings_file::load_settings_file();

        let frontend_url = get_env("FRONTEND_URL", "http://localhost:5173");
        let app_env = get_env("APP_ENV", "production");
        let development = app_env == "development";
        let directory = |setting: CheckoutDirectory| {
            development_directories::resolve(
                setting,
                &env::var(setting.variable).unwrap_or_default(),
                development,
                repository_root::find_repository_root,
            )
        };
        let cfg = Config {
            port: get_env("PORT", "8080"),
            trusted_proxies: split_csv(&env::var("TRUSTED_PROXIES").unwrap_or_default()),
            allowed_origins: split_csv(&get_env("ALLOWED_ORIGINS", &frontend_url)),
            frontend_url,
            spa_dist_dir: env::var("SPA_DIST_DIR").unwrap_or_default(),
            map_assets_dir: directory(MAP_ASSETS)?,
            glyph_assets_dir: directory(GLYPH_ASSETS)?,
            upload_dir: directory(UPLOAD)?,
            equipment_data_dir: directory(EQUIPMENT_DATA)?,
            equipment_export_source_dir: env::var("EQUIPMENT_EXPORT_SOURCE_DIR")
                .ok()
                .filter(|s| !s.is_empty()),
            env: app_env,
            database_url: env::var("DATABASE_URL").unwrap_or_default(),
            mission_version_max_body_bytes: get_env_int(
                "MISSION_VERSION_MAX_BODY_BYTES",
                DEFAULT_MISSION_VERSION_MAX_BODY_BYTES,
            ),
            jwt_secret: env::var("JWT_SECRET").unwrap_or_default(),
            jwt_access_ttl_min: get_env_int("JWT_ACCESS_TTL_MIN", 15),
            discord_client_id: env::var("DISCORD_CLIENT_ID").unwrap_or_default().into(),
            discord_client_secret: env::var("DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_url: env::var("DISCORD_REDIRECT_URL").unwrap_or_default(),
            discord_guild_id: env::var("DISCORD_GUILD_ID").unwrap_or_default().into(),
            discord_bot_token: env::var("DISCORD_BOT_TOKEN").unwrap_or_default(),
            discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").unwrap_or_default(),
            observability_token: env::var("OBSERVABILITY_TOKEN").unwrap_or_default(),
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
            if self.discord_client_id.as_str().trim().is_empty() {
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
        // What the API writes and serves must come from where the operator said, never from a
        // directory that happens to be the process's CWD. Empty can only survive `load` outside
        // development (the development defaults fill it), and there it is a missing setting, not
        // a default.
        for (name, value) in [
            (UPLOAD.variable, self.upload_dir.as_str()),
            (MAP_ASSETS.variable, self.map_assets_dir.as_str()),
            (GLYPH_ASSETS.variable, self.glyph_assets_dir.as_str()),
        ] {
            if value.is_empty() {
                return Err(ConfigError::Missing(name));
            }
            check_directory(name, value, self.is_development())?;
        }
        for (name, value) in [
            (
                EQUIPMENT_DATA.variable,
                Some(self.equipment_data_dir.as_str()),
            ),
            (
                "EQUIPMENT_EXPORT_SOURCE_DIR",
                self.equipment_export_source_dir.as_deref(),
            ),
        ] {
            if let Some(value) = value.filter(|v| !v.is_empty()) {
                check_directory(name, value, self.is_development())?;
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
    /// given DB URL + JWT secret, a non-empty service token, blank Discord creds, runtime
    /// storage under a per-process temporary directory so no suite writes into the checkout, and
    /// the checkout's read-only terrain and glyph trees, found from this crate's manifest folder
    /// so a test resolves them from any working directory.
    ///
    /// # Panics
    ///
    /// When this crate's manifest folder lies outside a checkout, which a test build of the
    /// checkout never is.
    pub fn for_tests(database_url: impl Into<String>, jwt_secret: impl Into<String>) -> Self {
        let scratch = std::env::temp_dir().join(format!("api-tests-{}", std::process::id()));
        let checkout_directory = |setting: CheckoutDirectory| {
            development_directories::resolve(setting, "", true, || {
                repository_root::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
            })
            .expect("the test build's manifest folder lies inside the checkout")
        };
        Self {
            equipment_data_dir: scratch.join("equipment").display().to_string(),
            equipment_export_source_dir: None,
            port: "0".into(),
            env: "development".into(),
            trusted_proxies: Vec::new(),
            frontend_url: "http://localhost:5173".into(),
            allowed_origins: vec!["http://localhost:5173".into()],
            spa_dist_dir: String::new(),
            map_assets_dir: checkout_directory(MAP_ASSETS),
            glyph_assets_dir: checkout_directory(GLYPH_ASSETS),
            upload_dir: scratch.join("uploads").display().to_string(),
            database_url: database_url.into(),
            mission_version_max_body_bytes: DEFAULT_MISSION_VERSION_MAX_BODY_BYTES,
            jwt_secret: jwt_secret.into(),
            jwt_access_ttl_min: 15,
            discord_client_id: DiscordClientId::new(""),
            discord_client_secret: String::new(),
            discord_redirect_url: String::new(),
            discord_guild_id: DiscordGuildId::new("test-tbd-guild"),
            discord_bot_token: String::new(),
            discord_webhook_url: String::new(),
            observability_token: "test-observability-token".into(),
            // Unconfigured by default: a test that wants the RCON transport stands up its
            // own socket and sets this, so no suite can accidentally reach a real agent.
        }
    }
}

/// Refuses a set directory value with surrounding whitespace in every environment, and a relative
/// one outside development, naming the variable.
fn check_directory(name: &'static str, value: &str, development: bool) -> Result<(), ConfigError> {
    if value.trim() != value {
        return Err(ConfigError::Malformed(
            name,
            "has leading or trailing whitespace",
        ));
    }
    if !development && !Path::new(value).is_absolute() {
        return Err(ConfigError::Malformed(
            name,
            "must be an absolute path outside development",
        ));
    }
    Ok(())
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
