//! Discord OAuth2 + guild-member client — Rust port of `services/discord.go`.
//!
//! Hand-rolled over `reqwest` (Go used raw `net/http` — no oauth2 lib). Bounded 429
//! retry honoring `Retry-After` mirrors `httpretry.go`. The rustls ring provider is
//! installed once so HTTPS works without the aws-lc-rs C build.

use std::sync::Once;
use std::time::Duration;

use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::Deserialize;

/// Production Discord API base (overridable for tests).
pub const DEFAULT_DISCORD_API: &str = "https://discord.com/api";
const OAUTH_SCOPES: &str = "identify guilds.members.read";
const MAX_429_ATTEMPTS: u32 = 3;
const DEFAULT_429_BACKOFF: Duration = Duration::from_secs(1);
const MAX_429_BACKOFF: Duration = Duration::from_secs(5);

static TLS_INIT: Once = Once::new();
fn ensure_tls_provider() {
    TLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Deserialize a field tolerating JSON `null` (→ the type's default), matching Go's
/// `encoding/json`, where a non-pointer field left `null` keeps its zero value. Discord
/// sends `null` for e.g. a member with no server nickname or a user with no custom
/// avatar; serde's `#[serde(default)]` alone only covers a *missing* field, not `null`.
fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// Thin client for the OAuth2 + member-roles endpoints.
#[derive(Clone)]
pub struct DiscordService {
    client_id: String,
    client_secret: String,
    redirect_url: String,
    guild_id: String,
    api_base: String,
    http: Client,
}

/// OAuth2 token-exchange payload.
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: i64,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub scope: String,
}

/// The subset of `/users/@me` we use.
#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default, deserialize_with = "null_default")]
    pub global_name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub discriminator: String,
    #[serde(default, deserialize_with = "null_default")]
    pub avatar: String,
}

impl DiscordUser {
    /// Prefer the new global display name, falling back to username.
    pub fn display_name(&self) -> String {
        if self.global_name.is_empty() {
            self.username.clone()
        } else {
            self.global_name.clone()
        }
    }

    /// Classic `name#1234`, or just the username for the new unique-username system.
    pub fn handle(&self) -> String {
        if self.discriminator.is_empty() || self.discriminator == "0" {
            self.username.clone()
        } else {
            format!("{}#{}", self.username, self.discriminator)
        }
    }

    /// CDN avatar URL, or `""` if the user has no custom avatar.
    pub fn avatar_url(&self) -> String {
        if self.avatar.is_empty() {
            String::new()
        } else {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png",
                self.id, self.avatar
            )
        }
    }
}

/// The subset of the guild-member object we use (role snowflakes + nick).
#[derive(Debug, Deserialize)]
pub struct GuildMember {
    #[serde(default, deserialize_with = "null_default")]
    pub nick: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

impl DiscordService {
    /// Construct the client with production defaults + a 10s timeout.
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_url: String,
        guild_id: String,
    ) -> Self {
        ensure_tls_provider();
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("build reqwest client");
        Self {
            client_id,
            client_secret,
            redirect_url,
            guild_id,
            api_base: DEFAULT_DISCORD_API.to_string(),
            http,
        }
    }

    /// Override the API base (used by tests with a mock server).
    pub fn set_api_base(&mut self, base: &str) {
        self.api_base = base.trim_end_matches('/').to_string();
    }

    /// Build the consent URL. Fails when `client_id` is unconfigured — redirecting to
    /// Discord with an empty client_id strands the user on an opaque error page.
    pub fn authorize_url(&self, state: &str) -> anyhow::Result<String> {
        if self.client_id.is_empty() {
            anyhow::bail!("discord: client_id not configured");
        }
        let q = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("response_type", "code")
            .append_pair("scope", OAUTH_SCOPES)
            .append_pair("state", state)
            .finish();
        Ok(format!("{}/oauth2/authorize?{}", self.api_base, q))
    }

    /// Swap an authorization code for an access token.
    pub async fn exchange_code(&self, code: &str) -> anyhow::Result<TokenResponse> {
        let url = format!("{}/oauth2/token", self.api_base);
        let form = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_url.as_str()),
        ];
        let resp = self.retry_429(|| self.http.post(&url).form(&form)).await?;
        let out: TokenResponse = decode_2xx(resp).await?;
        if out.access_token.is_empty() {
            anyhow::bail!("discord: empty access token");
        }
        Ok(out)
    }

    /// Retrieve the authenticated user's profile.
    pub async fn fetch_user(&self, access_token: &str) -> anyhow::Result<DiscordUser> {
        let url = format!("{}/users/@me", self.api_base);
        let resp = self
            .retry_429(|| self.http.get(&url).bearer_auth(access_token))
            .await?;
        decode_2xx(resp).await
    }

    /// Retrieve the caller's guild membership + roles. `None` (not an error) when the
    /// user is not in the guild (404), so login still succeeds for non-members.
    pub async fn fetch_guild_member(
        &self,
        access_token: &str,
    ) -> anyhow::Result<Option<GuildMember>> {
        let url = format!(
            "{}/users/@me/guilds/{}/member",
            self.api_base, self.guild_id
        );
        let resp = self
            .retry_429(|| self.http.get(&url).bearer_auth(access_token))
            .await?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(decode_2xx(resp).await?))
    }

    /// Send `build()`'s request, retrying bounded on 429 (rebuilding each attempt).
    async fn retry_429<F>(&self, build: F) -> anyhow::Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut attempt = 1;
        loop {
            let resp = build().send().await?;
            if resp.status() != StatusCode::TOO_MANY_REQUESTS || attempt == MAX_429_ATTEMPTS {
                return Ok(resp);
            }
            let wait = parse_retry_after(
                resp.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
            );
            attempt += 1;
            tokio::time::sleep(wait).await;
        }
    }
}

/// Decode a 2xx JSON response into `T`; non-2xx becomes an error carrying a bounded
/// body snippet (mirrors Go's `do`).
async fn decode_2xx<T: serde::de::DeserializeOwned>(resp: Response) -> anyhow::Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let body: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(4096)
            .collect();
        anyhow::bail!("discord: status {}: {}", status.as_u16(), body);
    }
    Ok(resp.json::<T>().await?)
}

/// Convert a `Retry-After` value (seconds, possibly fractional) into a bounded wait.
fn parse_retry_after(v: Option<&str>) -> Duration {
    match v.and_then(|s| s.parse::<f64>().ok()) {
        Some(secs) if secs >= 0.0 => {
            let d = Duration::from_secs_f64(secs);
            if d > MAX_429_BACKOFF {
                MAX_429_BACKOFF
            } else {
                d
            }
        }
        _ => DEFAULT_429_BACKOFF,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_has_params() {
        let s = DiscordService::new(
            "cid".into(),
            "sec".into(),
            "https://app/cb".into(),
            "g1".into(),
        );
        let u = s.authorize_url("st8").unwrap();
        assert!(u.contains("client_id=cid"), "{u}");
        assert!(u.contains("response_type=code"));
        assert!(u.contains("state=st8"));
        assert!(u.contains("scope=identify"));
    }

    #[test]
    fn authorize_url_requires_client_id() {
        let s = DiscordService::new(String::new(), "x".into(), "y".into(), "z".into());
        assert!(s.authorize_url("s").is_err());
    }

    #[test]
    fn user_derived_fields() {
        let modern = DiscordUser {
            id: "1".into(),
            username: "dave".into(),
            global_name: "Dave".into(),
            discriminator: "0".into(),
            avatar: "abc".into(),
        };
        assert_eq!(modern.display_name(), "Dave");
        assert_eq!(modern.handle(), "dave"); // discriminator "0" → username only
        assert_eq!(
            modern.avatar_url(),
            "https://cdn.discordapp.com/avatars/1/abc.png"
        );

        let legacy = DiscordUser {
            id: "2".into(),
            username: "bob".into(),
            global_name: String::new(),
            discriminator: "1234".into(),
            avatar: String::new(),
        };
        assert_eq!(legacy.display_name(), "bob");
        assert_eq!(legacy.handle(), "bob#1234");
        assert_eq!(legacy.avatar_url(), "");
    }

    #[test]
    fn retry_after_parsing_and_clamp() {
        assert_eq!(parse_retry_after(Some("2")), Duration::from_secs(2));
        assert_eq!(parse_retry_after(Some("0.5")), Duration::from_millis(500));
        assert_eq!(parse_retry_after(Some("100")), MAX_429_BACKOFF); // clamped
        assert_eq!(parse_retry_after(None), DEFAULT_429_BACKOFF);
        assert_eq!(parse_retry_after(Some("garbage")), DEFAULT_429_BACKOFF);
    }

    #[test]
    fn null_fields_deserialize_like_go() {
        // Discord sends null for a member with no nickname / a user with no avatar. Go's
        // encoding/json kept the zero value; serde must too. Regression: a null nick failed
        // GuildMember parse → empty roles → login resolved the wrong web role.
        let m: GuildMember =
            serde_json::from_str(r#"{"nick":null,"roles":["1517285898817896559"]}"#).unwrap();
        assert_eq!(m.nick, "");
        assert_eq!(m.roles, ["1517285898817896559"]);

        let u: DiscordUser = serde_json::from_str(
            r#"{"id":"7","username":"sam","global_name":null,"discriminator":"0","avatar":null}"#,
        )
        .unwrap();
        assert_eq!(u.username, "sam");
        assert_eq!(u.global_name, "");
        assert_eq!(u.avatar, "");
        assert_eq!(u.display_name(), "sam");
    }
}
