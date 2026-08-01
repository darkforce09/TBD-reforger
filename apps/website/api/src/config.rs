//! Runtime configuration from environment — Rust port of `internal/config`.
//!
//! 19 env vars (measured: 12 `env::var` + 5 `get_env` + 2 `get_env_int`; the
//! header said 16 through T-279 and was already stale at 18 before T-595 added
//! `GAME_AGENT_SOCKET`). `DATABASE_URL` and `JWT_SECRET` are always required
//! (hard-fail). In non-development, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`,
//! and `DISCORD_REDIRECT_URL` are also required (T-248 / T-430) so blank OAuth
//! creds cannot load config and later surface as `oauth_unconfigured` /
//! `discord_unreachable`. A `.env` file is loaded if present but optional.
//!
//! `DISCORD_BOT_TOKEN` (T-279) is **optional** — no consumer reads it yet, so
//! requiring it would fail boot for a feature that does not exist. It is read
//! through [`Config::require_discord_bot_token`], which makes "unset" a named
//! error rather than an empty `Bot ` header; a set-but-whitespace-bearing token
//! is rejected at boot because it can never authenticate.
//!
//! `GAME_AGENT_SOCKET` (T-595) is **optional and lands with its consumer** —
//! [`handlers::admin::send_rcon`] reads it through
//! [`Config::require_game_agent_socket`] on every request. T-279's rule is that
//! a var is added *with* the code that reads it, precisely so this file cannot
//! accumulate another `DISCORD_BOT_TOKEN`: a setting nobody consumes, which
//! looks configured and does nothing.
//!
//! [`handlers::admin::send_rcon`]: crate::handlers::admin::send_rcon

use std::env;
use std::net::IpAddr;
use std::path::Path;

/// Default body cap for `POST /missions/:id/versions` (256 MB), matching Go.
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
    /// **T-625 — this is now read.** Through T-624 it was parsed out of `TRUSTED_PROXIES` and
    /// consulted by nothing, which is why every public client behind `Caddyfile.website` shared
    /// one rate-limit bucket: `middleware::ratelimit` keyed on the `ConnectInfo` peer, and behind
    /// a loopback reverse proxy that peer is Caddy for everyone. The consumer is
    /// [`crate::middleware::RateLimitState`], and the entries are validated at boot by
    /// [`ProxyNet::parse`] — an unparseable entry is a boot failure, not a silently-ignored line
    /// that leaves the operator believing the header is honoured.
    ///
    /// Empty is the default and means the header is ignored **entirely**. That is deliberate:
    /// `X-Forwarded-For` is client-controllable, and a rate-limit key any client can forge is
    /// worse than one everybody shares — shared means everyone is limited together, forgeable
    /// means nobody is limited at all.
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

    /// T-595 — absolute path of T-289's host control agent socket, e.g.
    /// `/run/user/1000/tbd-reforger-agent.sock` (the systemd unit renders it as
    /// `%t/tbd-reforger-agent.sock`). **Empty = no transport**, and
    /// `send_rcon` keeps answering 503 rather than pretending. Read through
    /// [`Config::require_game_agent_socket`].
    pub game_agent_socket: String,
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
    /// Same rule as [`Self::Malformed`], for a **list** variable where the reason is useless
    /// without the offending entry: `TRUSTED_PROXIES` can hold a dozen entries and "is malformed"
    /// would send the operator reading all of them. Carries the entry verbatim (T-625).
    #[error("{0} entry {1:?} is malformed: {2}")]
    MalformedEntry(&'static str, String, &'static str),
}

/// One trusted reverse proxy: a bare address (`127.0.0.1`) or a CIDR block (`10.0.0.0/8`).
///
/// # Why this is hand-rolled rather than `ipnet`
///
/// Two functions — parse and "does this address fall inside" — over `[u8; 4]` / `[u8; 16]`. A
/// dependency for that would be more supply chain than arithmetic, and the arithmetic is unit
/// tested below.
///
/// # The rules, and what each one refuses
///
/// * **A bare address is a single host** (`/32`, `/128`). It is *not* silently widened to the
///   surrounding network, which is the classic way a trusted-proxy list ends up trusting a whole
///   datacentre.
/// * **A CIDR must be written as its network address.** `10.0.0.5/8` is refused rather than
///   quietly read as `10.0.0.0/8`, because that reading trusts 16 million addresses the operator
///   did not type. The error names the form to write instead.
/// * **IPv4-mapped IPv6 is canonicalised** (`::ffff:127.0.0.1` → `127.0.0.1`), on both the
///   configured address and the address being tested. A dual-stack listener hands axum the mapped
///   form, and without this an operator who correctly wrote `127.0.0.1` would get *no* match —
///   silently falling back to the shared-bucket behaviour they were trying to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyNet {
    base: IpAddr,
    prefix_len: u8,
}

impl ProxyNet {
    /// Parse one `TRUSTED_PROXIES` entry. `Err` carries a reason fit to print at boot.
    pub fn parse(entry: &str) -> Result<Self, &'static str> {
        let entry = entry.trim();
        let Some((addr, len)) = entry.split_once('/') else {
            // Bare address: exactly this host. Canonicalised so either spelling of a mapped
            // IPv4 address matches a peer that arrives in either spelling.
            let base = entry
                .parse::<IpAddr>()
                .map_err(|_| "not an IP address or CIDR block")?
                .to_canonical();
            return Ok(Self {
                prefix_len: full_prefix_len(&base),
                base,
            });
        };
        // With an explicit prefix the family is the one the operator wrote — canonicalising here
        // would turn `::ffff:10.0.0.0/104` into an IPv4 base carrying an IPv6 prefix length.
        // Write IPv4 proxies in IPv4 form.
        let base = addr
            .parse::<IpAddr>()
            .map_err(|_| "the part before `/` is not an IP address")?;
        let prefix_len = len
            .parse::<u8>()
            .map_err(|_| "the part after `/` is not a prefix length")?;
        if prefix_len > full_prefix_len(&base) {
            return Err("prefix length is longer than the address family allows");
        }
        if !host_bits_are_zero(&base, prefix_len) {
            return Err(
                "host bits are set — write the network address (e.g. `10.0.0.0/8`, not \
                 `10.0.0.5/8`), so the entry cannot trust more than it says",
            );
        }
        Ok(Self { base, prefix_len })
    }

    /// True when `ip` falls inside this network.
    ///
    /// A mismatched family is `false`, never a panic and never a match: an IPv6 peer does not
    /// belong to an IPv4 proxy's network however the two are spelled.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.base, ip.to_canonical()) {
            (IpAddr::V4(base), IpAddr::V4(ip)) => {
                prefix_eq(&base.octets(), &ip.octets(), self.prefix_len)
            }
            (IpAddr::V6(base), IpAddr::V6(ip)) => {
                prefix_eq(&base.octets(), &ip.octets(), self.prefix_len)
            }
            _ => false,
        }
    }
}

/// Parse every entry. `Err` names the first bad one — `(entry, why)`.
///
/// Used twice on purpose: once by [`Config::validate`] so a typo is a boot failure, and once by
/// [`crate::middleware::RateLimitState::new`] so the middleware holds parsed networks rather than
/// re-parsing strings per request.
pub fn parse_trusted_proxies(entries: &[String]) -> Result<Vec<ProxyNet>, (String, &'static str)> {
    entries
        .iter()
        .map(|e| ProxyNet::parse(e).map_err(|why| (e.clone(), why)))
        .collect()
}

/// Bits in a full address of this family.
fn full_prefix_len(ip: &IpAddr) -> u8 {
    match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

/// True when the first `prefix_len` bits of `a` and `b` are equal.
fn prefix_eq(a: &[u8], b: &[u8], prefix_len: u8) -> bool {
    let whole = usize::from(prefix_len / 8);
    let rest = prefix_len % 8;
    if a[..whole] != b[..whole] {
        return false;
    }
    if rest == 0 {
        return true;
    }
    // Compare only the leading `rest` bits of the next byte.
    let mask = 0xffu8 << (8 - rest);
    (a[whole] ^ b[whole]) & mask == 0
}

/// True when every bit past `prefix_len` is zero — i.e. the address IS its network address.
fn host_bits_are_zero(ip: &IpAddr, prefix_len: u8) -> bool {
    fn check(octets: &[u8], prefix_len: u8) -> bool {
        let whole = usize::from(prefix_len / 8);
        let rest = prefix_len % 8;
        if rest != 0 && octets[whole] & (0xffu8 >> rest) != 0 {
            return false;
        }
        let tail = if rest == 0 { whole } else { whole + 1 };
        octets[tail..].iter().all(|b| *b == 0)
    }
    match ip {
        IpAddr::V4(v4) => check(&v4.octets(), prefix_len),
        IpAddr::V6(v6) => check(&v6.octets(), prefix_len),
    }
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
        // T-595 — the agent socket is optional (empty = no transport, which
        // `send_rcon` reports honestly as 503). But a *set* value that cannot
        // work must die at boot, not at 03:00 when an admin clicks Restart.
        //
        // Two rules, both for failures that would otherwise arrive disguised as
        // "the game server is unreachable":
        //
        // 1. Leading/trailing whitespace — the copy-paste / secrets-manager
        //    newline. `"/run/user/1000/x.sock\n"` is a perfectly plausible
        //    `.env` value and `connect(2)` on it is ENOENT, which this API would
        //    correctly-but-uselessly report as an unreachable agent. Inner
        //    spaces are legal in a path and are NOT rejected.
        // 2. Not absolute — `UnixStream::connect` resolves a relative path
        //    against the API process's CWD, which is a systemd/launcher detail
        //    nobody sets deliberately. It would either miss (ENOENT) or, worse,
        //    hit a different socket than intended.
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
        // T-625 — `TRUSTED_PROXIES` decides whether a client-supplied header is believed, so a
        // typo in it must not be survivable. Unset stays legal and means "trust none"; a *set*
        // entry that does not parse dies here rather than being skipped at request time, where
        // the operator would see a running API and a header that is quietly still ignored.
        if let Err((entry, why)) = parse_trusted_proxies(&self.trusted_proxies) {
            return Err(ConfigError::MalformedEntry("TRUSTED_PROXIES", entry, why));
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

    /// True when a game-agent socket path is configured at all (T-595).
    ///
    /// Same shape as [`Self::discord_bot_configured`]: an unconfigured channel must be
    /// distinguishable from a broken one, or "nobody set this up" hides inside whatever
    /// `connect(2)` happens to return.
    pub fn game_agent_configured(&self) -> bool {
        !self.game_agent_socket.is_empty()
    }

    /// The agent socket path, or a named [`ConfigError::Missing`] when unset.
    ///
    /// T-595 — the **only** supported way to read `GAME_AGENT_SOCKET`. The raw field is `""`
    /// when unconfigured, and `Path::new("")` is a real path that `UnixStream::connect`
    /// answers with ENOENT — indistinguishable at the call site from a game host whose agent
    /// has crashed. Going through this accessor turns "nobody configured a transport" into a
    /// named condition the handler can report as such.
    ///
    /// Validation guarantees the returned path is absolute and free of surrounding
    /// whitespace.
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
        let cfg = cfg
            .validate()
            .expect("development empty bot token must load");
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

    // ---- T-595 GAME_AGENT_SOCKET ----------------------------------------

    /// Unset is legal — there is no agent on a developer's box — but it must not
    /// read as a usable path. `Path::new("")` connects to nothing and reports
    /// ENOENT, which at the call site is indistinguishable from a dead agent.
    #[test]
    fn unset_agent_socket_loads_but_is_not_readable() {
        let cfg = production_base();
        assert!(cfg.game_agent_socket.is_empty());
        let cfg = cfg
            .validate()
            .expect("unset GAME_AGENT_SOCKET must not block boot");
        assert!(!cfg.game_agent_configured());
        match cfg.require_game_agent_socket() {
            Err(ConfigError::Missing("GAME_AGENT_SOCKET")) => {}
            other => panic!("expected Missing(GAME_AGENT_SOCKET), got {other:?}"),
        }
    }

    /// The real deployment value round-trips (`%t/tbd-reforger-agent.sock`
    /// expanded by systemd).
    #[test]
    fn configured_agent_socket_is_readable() {
        let mut cfg = production_base();
        cfg.game_agent_socket = "/run/user/1000/tbd-reforger-agent.sock".into();
        let cfg = cfg.validate().expect("an absolute socket path must load");
        assert!(cfg.game_agent_configured());
        assert_eq!(
            cfg.require_game_agent_socket().expect("readable"),
            Path::new("/run/user/1000/tbd-reforger-agent.sock")
        );
    }

    /// A trailing newline from a copy-paste or a secrets-manager read. Both
    /// `is_empty()` and `is_absolute()` pass it, so only the trim rule catches
    /// it — and uncaught it becomes ENOENT, reported to the operator as an
    /// unreachable game host rather than as a typo in their `.env`.
    #[test]
    fn agent_socket_with_surrounding_whitespace_is_rejected() {
        for bad in [
            "/run/user/1000/tbd-reforger-agent.sock\n",
            " /run/user/1000/tbd-reforger-agent.sock",
            "/run/user/1000/tbd-reforger-agent.sock\r\n",
            "\t/run/user/1000/tbd-reforger-agent.sock ",
        ] {
            let mut cfg = production_base();
            cfg.game_agent_socket = bad.into();
            match cfg.validate() {
                Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
                other => panic!("expected Malformed(GAME_AGENT_SOCKET) for {bad:?}, got {other:?}"),
            }
        }
    }

    /// A relative path resolves against the API process's CWD — a systemd detail
    /// nobody chose. Reject it at boot rather than connect somewhere unintended.
    #[test]
    fn relative_agent_socket_is_rejected() {
        for bad in [
            "tbd-reforger-agent.sock",
            "run/user/1000/tbd-reforger-agent.sock",
            "./agent.sock",
        ] {
            let mut cfg = production_base();
            cfg.game_agent_socket = bad.into();
            match cfg.validate() {
                Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
                other => panic!("expected Malformed(GAME_AGENT_SOCKET) for {bad:?}, got {other:?}"),
            }
        }
    }

    /// Inner spaces are legal in a filesystem path and must NOT be swept up by
    /// the trim rule — that would be a guard rejecting valid configuration,
    /// which is its own kind of lie.
    #[test]
    fn agent_socket_may_contain_inner_spaces() {
        let mut cfg = production_base();
        cfg.game_agent_socket = "/run/user/1000/tbd agent.sock".into();
        let cfg = cfg
            .validate()
            .expect("a path with an inner space is a legal path");
        assert!(cfg.game_agent_configured());
    }

    /// Development is not a hole: an unusable path is unusable everywhere.
    #[test]
    fn bad_agent_socket_is_rejected_in_development_too() {
        let mut cfg = Config::for_tests("postgres://x/x", "jwt-secret");
        cfg.game_agent_socket = "relative.sock".into();
        assert!(cfg.is_development());
        match cfg.validate() {
            Err(ConfigError::Malformed("GAME_AGENT_SOCKET", _)) => {}
            other => panic!("expected Malformed(GAME_AGENT_SOCKET) in dev, got {other:?}"),
        }
    }

    // ───────────────────────── T-625 — TRUSTED_PROXIES ─────────────────────────

    fn net(entry: &str) -> ProxyNet {
        ProxyNet::parse(entry).unwrap_or_else(|e| panic!("{entry:?} should parse: {e}"))
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().expect("test address")
    }

    /// A bare address is that host and **nothing else**. The failure this pins is the widening
    /// one: reading `10.0.0.1` as "the 10.0.0.0/8 this address sits in" would trust 16 million
    /// hosts on the strength of one line.
    #[test]
    fn a_bare_address_is_a_single_host() {
        let n = net("127.0.0.1");
        assert!(n.contains(ip("127.0.0.1")));
        assert!(!n.contains(ip("127.0.0.2")));
        assert!(!n.contains(ip("127.1.0.1")));

        let six = net("::1");
        assert!(six.contains(ip("::1")));
        assert!(!six.contains(ip("::2")));
    }

    /// CIDR membership, including a prefix that does not land on a byte boundary — the case a
    /// byte-wise comparison gets wrong in the permissive direction.
    #[test]
    fn cidr_membership_is_bitwise() {
        let n = net("10.0.0.0/8");
        assert!(n.contains(ip("10.0.0.1")));
        assert!(n.contains(ip("10.255.255.254")));
        assert!(!n.contains(ip("11.0.0.1")));

        let odd = net("192.168.4.0/22"); // 192.168.4.0 – 192.168.7.255
        assert!(odd.contains(ip("192.168.4.1")));
        assert!(odd.contains(ip("192.168.7.255")));
        assert!(!odd.contains(ip("192.168.8.0")));
        assert!(!odd.contains(ip("192.168.3.255")));

        let v6 = net("2001:db8::/32");
        assert!(v6.contains(ip("2001:db8::1")));
        assert!(!v6.contains(ip("2001:db9::1")));
    }

    /// `/0` trusts everything of that family — legal, and it must mean what it says rather than
    /// accidentally matching nothing (or the other family).
    #[test]
    fn a_zero_prefix_trusts_the_whole_family_and_only_that_family() {
        let all_v4 = net("0.0.0.0/0");
        assert!(all_v4.contains(ip("1.2.3.4")));
        assert!(all_v4.contains(ip("203.0.113.9")));
        assert!(!all_v4.contains(ip("2001:db8::1")));
    }

    /// An IPv4-mapped IPv6 peer is the IPv4 client it denotes. A dual-stack listener produces
    /// these, and without canonicalisation a correct `127.0.0.1` entry would match nothing —
    /// failing closed, but silently, over a configuration that is right.
    #[test]
    fn ipv4_mapped_addresses_canonicalise_on_both_sides() {
        assert!(net("127.0.0.1").contains(ip("::ffff:127.0.0.1")));
        assert!(net("::ffff:127.0.0.1").contains(ip("127.0.0.1")));
        assert!(net("10.0.0.0/8").contains(ip("::ffff:10.1.2.3")));
        assert!(!net("10.0.0.0/8").contains(ip("::ffff:11.1.2.3")));
    }

    /// Families do not cross.
    #[test]
    fn mismatched_families_never_match() {
        assert!(!net("127.0.0.1").contains(ip("::1")));
        assert!(!net("::1").contains(ip("127.0.0.1")));
    }

    /// A CIDR with host bits set is refused rather than silently masked. `10.0.0.5/8` read as
    /// `10.0.0.0/8` trusts a network the operator never typed — the widening this file exists to
    /// prevent, arriving as a typo instead of as a decision.
    #[test]
    fn a_cidr_with_host_bits_set_is_refused_not_masked() {
        let err = ProxyNet::parse("10.0.0.5/8").expect_err("host bits set must not parse");
        assert!(err.contains("host bits"), "unhelpful reason: {err}");
        assert!(ProxyNet::parse("10.0.0.0/8").is_ok());
        // …and the same rule inside a byte.
        assert!(ProxyNet::parse("192.168.5.0/22").is_err());
        assert!(ProxyNet::parse("192.168.4.0/22").is_ok());
    }

    /// Every other way an entry can be wrong is an error with a reason, never a `ProxyNet`.
    #[test]
    fn malformed_entries_are_rejected() {
        for bad in [
            "",
            "  ",
            "not-an-ip",
            "10.0.0.0/",
            "10.0.0.0/33",
            "::/129",
            "10.0.0.0/eight",
            "10.0.0.0/8/8",
            "127.0.0.1:8080",
            "example.com",
        ] {
            assert!(
                ProxyNet::parse(bad).is_err(),
                "{bad:?} must not parse as a trusted proxy"
            );
        }
    }

    /// Surrounding whitespace is the `.env` copy-paste, not a different proxy.
    #[test]
    fn entries_tolerate_surrounding_whitespace() {
        assert_eq!(net("  127.0.0.1  "), net("127.0.0.1"));
        assert_eq!(net(" 10.0.0.0/8 "), net("10.0.0.0/8"));
    }

    /// A bad entry is a **boot failure**, and the message names the entry. Silently dropping it
    /// would leave an operator who typed one CIDR wrong believing per-client keying is live.
    #[test]
    fn a_malformed_trusted_proxy_entry_fails_validation() {
        let mut cfg = production_base();
        cfg.trusted_proxies = vec!["127.0.0.1".into(), "10.0.0.5/8".into()];
        match cfg.validate() {
            Err(ConfigError::MalformedEntry("TRUSTED_PROXIES", entry, _)) => {
                assert_eq!(entry, "10.0.0.5/8", "the error must name the bad entry");
            }
            other => panic!("expected MalformedEntry(TRUSTED_PROXIES), got {other:?}"),
        }
    }

    /// Unset stays legal and means trust-none — the shipped default, unchanged.
    #[test]
    fn an_empty_trusted_proxy_list_is_valid_and_trusts_nothing() {
        let cfg = production_base().validate().expect("no proxies is legal");
        assert!(cfg.trusted_proxies.is_empty());
        assert_eq!(parse_trusted_proxies(&cfg.trusted_proxies).unwrap(), vec![]);
    }

    /// **The value the deployment actually ships.** `docker-compose.staging.yml` has carried
    /// `TRUSTED_PROXIES: ${TRUSTED_PROXIES:-127.0.0.1/32}` since before anything read it, so T-625
    /// turns that line from inert into load-bearing in two ways at once: it is now the setting that
    /// switches per-client keying on, **and** it is now parsed at boot, so a value this file
    /// refuses would stop the staging API from starting.
    ///
    /// Read out of the compose file rather than retyped, because a typed copy would agree with
    /// itself while the deployment shipped something else — which is the whole T-625/T-626 theme.
    #[test]
    fn the_shipped_staging_default_parses_and_matches_the_loopback_proxy() {
        const COMPOSE: &str = include_str!("../../docker-compose.staging.yml");
        let line = COMPOSE
            .lines()
            .find(|l| l.trim_start().starts_with("TRUSTED_PROXIES:"))
            .expect("docker-compose.staging.yml no longer sets TRUSTED_PROXIES");
        let shipped = line
            .split_once(":-")
            .and_then(|(_, rest)| rest.split_once('}'))
            .map(|(default, _)| default.trim())
            .expect("TRUSTED_PROXIES no longer has a `${VAR:-default}` default");
        let entries = split_csv(shipped);
        let nets = parse_trusted_proxies(&entries).unwrap_or_else(|(entry, why)| {
            panic!(
                "the staging compose default {shipped:?} does not parse — entry {entry:?}: {why}. \
                 The API would refuse to boot on staging."
            )
        });
        assert!(
            nets.iter().any(|n| n.contains(ip("127.0.0.1"))),
            "the staging default {shipped:?} does not cover the loopback address Caddy proxies \
             from, so X-Forwarded-For would still be ignored there"
        );
        assert!(
            !nets.iter().any(|n| n.contains(ip("203.0.113.9"))),
            "the staging default {shipped:?} trusts a public address"
        );
    }

    /// The whole list parses, in order, and a good list validates.
    #[test]
    fn a_well_formed_list_parses_and_validates() {
        let mut cfg = production_base();
        cfg.trusted_proxies = vec!["127.0.0.1".into(), "10.0.0.0/8".into(), "::1".into()];
        let cfg = cfg.validate().expect("well-formed list must validate");
        let nets = parse_trusted_proxies(&cfg.trusted_proxies).expect("parse");
        assert_eq!(nets.len(), 3);
        assert!(nets[0].contains(ip("127.0.0.1")));
        assert!(nets[1].contains(ip("10.9.9.9")));
        assert!(nets[2].contains(ip("::1")));
        assert!(!nets.iter().any(|n| n.contains(ip("203.0.113.1"))));
    }
}
