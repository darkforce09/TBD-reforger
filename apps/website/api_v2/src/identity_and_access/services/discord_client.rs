//! Discord OAuth2 + guild-member HTTP client.
//!
//! Hand-rolled over `reqwest`. The 429 retry is bounded and honours `Retry-After`, so a
//! rate-limited login backs off instead of hammering Discord. The rustls ring provider is
//! installed once, so HTTPS works without the aws-lc-rs C build.

use std::sync::Once;
use std::time::Duration;

use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::Deserialize;

use super::discord_user_profile::DiscordUser;

/// Production Discord API base (overridable for tests).
pub const DEFAULT_DISCORD_API: &str = "https://discord.com/api/v10";
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

/// Deserialize a field tolerating JSON `null` (→ the type's default). Discord sends `null`
/// for e.g. a member with no server nickname or a user with no custom avatar, and serde's
/// `#[serde(default)]` alone covers only a *missing* field, not an explicit `null`.
pub(super) fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
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

/// Required roles distinguish a complete membership response from a malformed error envelope.
/// An empty array is valid membership with no roles; absent or null roles are rejected.
#[derive(Debug, Deserialize)]
pub struct GuildMember {
    #[serde(default, deserialize_with = "null_default")]
    pub nick: String,
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
            let error: serde_json::Value = resp.json().await?;
            if error.get("code").and_then(|value| value.as_i64()) == Some(10007) {
                return Ok(None);
            }
            anyhow::bail!("discord: membership verification unavailable");
        }
        Ok(Some(decode_2xx(resp).await?))
    }

    /// A 404 is a departure only when Discord identifies the missing object as a member.
    pub async fn fetch_member_with_bot(
        &self,
        token: &str,
        guild_id: &str,
        discord_id: &str,
    ) -> Result<Option<GuildMember>, super::discord_rest_reconciliation::MembershipLookupFailure>
    {
        use super::discord_rest_reconciliation::MembershipLookupFailure as Failure;
        if token.is_empty() || guild_id.is_empty() || discord_id.is_empty() {
            return Err(Failure::unavailable("Discord bot or guild unconfigured"));
        }
        let url = format!(
            "{}/guilds/{}/members/{}",
            self.api_base, guild_id, discord_id
        );
        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Bot {token}"))
            .header(
                "User-Agent",
                "TBD-Reforger/2 (Discord membership reconciliation)",
            )
            .send()
            .await
            .map_err(|_| Failure::unavailable("Discord transport unavailable"))?;
        let status = response.status();
        let retry_header = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<f64>().ok());
        let value: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
        if status == StatusCode::TOO_MANY_REQUESTS {
            let valid = |n: &f64| n.is_finite() && *n >= 0.0 && *n <= 604800.0;
            let seconds = value
                .get("retry_after")
                .and_then(|v| v.as_f64())
                .filter(valid)
                .or(retry_header.filter(valid))
                .unwrap_or(60.0);
            return Err(Failure {
                reason: "Discord rate limited",
                rate_limited: true,
                retry_after: chrono::Duration::milliseconds(
                    (seconds * 1000.0).ceil().max(1000.0) as i64
                ),
            });
        }
        if status == StatusCode::NOT_FOUND
            && value.get("code").and_then(|v| v.as_i64()) == Some(10007)
        {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(Failure::unavailable(
                "Discord membership verification unavailable",
            ));
        }
        serde_json::from_value(value)
            .map(Some)
            .map_err(|_| Failure::unavailable("Discord returned malformed membership data"))
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
            let requested = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok());
            if requested.is_none_or(|seconds| {
                !seconds.is_finite() || seconds > MAX_429_BACKOFF.as_secs_f64()
            }) {
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
/// body snippet so the failure names what Discord actually said.
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
        Some(secs) if secs.is_finite() && secs >= 0.0 => {
            let d = Duration::from_secs_f64(secs.min(MAX_429_BACKOFF.as_secs_f64()));
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
#[path = "tests/discord_client.rs"]
mod tests;
