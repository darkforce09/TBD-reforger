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
///
/// **`username` is deliberately required — do not add `#[serde(default)]` to it (T-319).**
/// This is the T-185 door one field over, in a service rather than a handler, and the
/// difference matters: a malformed *upstream* response is not a client error, so there is no
/// 400 to return and no request to reject. The only lever here is whether the body decodes.
///
/// Discord's user object always carries `username` — it is required and non-nullable, and
/// there is no such thing as a Discord account without one. So a 200 body that lacks it is
/// not "this user has no username", it is **not a user object**: a gateway or CDN answering
/// 200 with something else, or an API shape change. Defaulted, that decoded cleanly into
/// `DiscordUser { username: "" }`, and because [`Self::display_name`] falls back to
/// `username` and [`Self::handle`] is built from it, [`crate::handlers::oauth`] bound two
/// empty strings into `users.username` and `users.discord_handle`.
///
/// **The right answer is to fail the login, not to patch the value.** Keeping the stored
/// name would need a `COALESCE` in the oauth upsert, which is the wrong place to encode
/// "the profile was junk" — and it would still let the junk profile mint a session. Failing
/// the decode routes the whole callback down its existing `Err` path
/// (`fetch_user` → `err("discord_unreachable")`), which writes nothing at all: no user row,
/// no session, no audit entry. The user retries; a transient blip costs one login.
///
/// **"It self-heals on the next login" is only half true, and the wrong half is the one that
/// lasts.** Verified against the live schema: `users.username`/`discord_handle` do heal, via
/// `ON CONFLICT (discord_id) DO UPDATE SET username = EXCLUDED.username`. But the same
/// callback then writes an `auth.login` row with `actor_name = ''` and the message
/// `" signed in via Discord"`, and `audit_logs` is append-only — this crate contains zero
/// `UPDATE audit_logs`. The user row recovers; the audit trail keeps an anonymous login
/// forever, and every action taken during the blank window is logged under an empty actor.
///
/// Like `GuildMember::roles` this does **not** get `null_default` either — `"username": null`
/// is malformed for a user object, so failing closed is right. (It already failed pre-fix,
/// since `#[serde(default)]` covers a missing field but not an explicit `null`; the hole was
/// only ever the *absent* case.) An explicit `""` still decodes: that is a stated answer, not
/// silence, and the same line `GuildMember` draws between an absent `roles` and `[]`.
#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default, deserialize_with = "null_default")]
    pub global_name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub discriminator: String,
    #[serde(default, deserialize_with = "null_default")]
    pub avatar: String,
}

/// Is `s` safe to interpolate as **one path segment** of a CDN URL? **T-405.**
///
/// Non-empty and `[A-Za-z0-9_]` only. That excludes every character that could end the segment or
/// the path — `/`, `\`, `.`, `?`, `#`, `%`, `@`, `:` — and every control character and space along
/// with them, so the value cannot move the URL anywhere its author did not intend. See
/// [`DiscordUser::avatar_url`] for why the rule is a character class rather than Discord's exact
/// documented formats.
fn is_cdn_path_segment(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl DiscordUser {
    /// Prefer the new global display name, falling back to username.
    ///
    /// **T-371 — the selection test is `trim().is_empty()`, not `is_empty()`.** This is a
    /// *choice between two fields*, not a guard that should trim the winner. Discord's
    /// `global_name` of `"   "` is present and non-empty, so the old `is_empty()` branch
    /// stored it verbatim into `users.username` on every oauth login upsert. T-366 closed the
    /// *read* side (whitespace no longer produces an anonymous audit line); this closes the
    /// *write* side by making the selection test meaningful — whitespace-only `global_name`
    /// falls through to `username`. A name that survives (`"  Dave  "`) is returned exactly as
    /// Discord sent it: padding is cosmetic; namelessness is not. Same lever T-319 established
    /// for this struct — a malformed upstream is not a client error, so the only control after
    /// decode is what we select.
    pub fn display_name(&self) -> String {
        if self.global_name.trim().is_empty() {
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

    /// CDN avatar URL, or `""` if the user has no custom avatar **or if Discord handed us an `id`
    /// or `avatar` that is not a bare path segment**.
    ///
    /// # T-405 — the trust boundary, now stated and enforced
    ///
    /// This `format!`s two strings straight out of an HTTP response into a URL path. The stored
    /// result is public-tier (`handlers::oauth` writes it to `users.avatar_url` on every login)
    /// and reaches an `<img src>` on four SPA pages. Before this, nothing checked either string,
    /// so an `avatar` of `../../evil` walked the URL out of `/avatars/` entirely, and one
    /// containing `?`, `#` or `@` re-pointed it by query, fragment or authority — while still
    /// *looking* like a `cdn.discordapp.com` link to anyone reading the database.
    ///
    /// The trust boundary here is "we trust Discord's API", and that is very probably fine. It was
    /// also **undocumented and unenforced**, which are two different problems from "wrong": an
    /// assumption nobody wrote down cannot be reviewed, and one nothing checks is indistinguishable
    /// from an assumption that has quietly stopped holding — a compromised or spoofed token
    /// endpoint, a proxy, a future Discord format change, or a test double. Enforcing it costs one
    /// character-class check and converts a silent trust into a loud one.
    ///
    /// The rule is **`[A-Za-z0-9_]` only**, which is deliberately looser than Discord's documented
    /// shapes (snowflakes are decimal; avatar hashes are 32 hex characters, optionally `a_`-
    /// prefixed when animated). Pinning the exact shapes would be a stricter guard and a worse
    /// one: it buys nothing extra — every character that could escape the path segment is already
    /// excluded — and it would start silently blanking real avatars the day Discord widens its
    /// hash format. What is excluded is the part that matters: `/`, `.`, `?`, `#`, `%`, `@`, `:`,
    /// `\` and whitespace.
    ///
    /// Failing to `""` rather than panicking or erroring, because the only caller is an OAuth
    /// callback: a login must not fail over a cosmetic field. `""` is this column's existing
    /// "no avatar" value and every reader already handles it.
    pub fn avatar_url(&self) -> String {
        if self.avatar.is_empty()
            || !is_cdn_path_segment(&self.id)
            || !is_cdn_path_segment(&self.avatar)
        {
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
///
/// **`roles` is deliberately required — do not add `#[serde(default)]` to it (T-185).**
/// Every other field on these payloads defaults because a missing one is cosmetic. This one
/// is not: it is the authorization snapshot. A default here is not "no data", it decodes as
/// *"Discord affirmatively told us this user holds no roles"*, and
/// [`crate::handlers::oauth`] acts on that by DELETEing every stored `user_discord_roles`
/// row and dropping the user to enlisted. Because `resync_all_roles` rebuilds from that same
/// table, the demotion is unrecoverable.
///
/// The status code alone does not protect us. A gateway, proxy, or CDN that answers **200**
/// with a JSON error envelope produces a body that parses fine and simply lacks `roles` —
/// which is exactly how the T-185 role-wipe came back after `RoleSnapshot` had closed the
/// transport-failure door. Requiring the field turns that body into a decode error, so it
/// travels the `Err` → `RoleSnapshot::Unavailable` path and writes nothing.
///
/// This deliberately does **not** get `null_default` either: `"roles": null` is malformed for
/// a member object, so failing closed is right. An empty list still round-trips as `[]`,
/// which is a genuine answer and must keep demoting.
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

    /// **T-371 — whitespace-only `global_name` must not win field selection.**
    ///
    /// Pre-fix `global_name.is_empty()` let `"   "` win and oauth stored it verbatim into
    /// `users.username`. The fix is the *selection test* (`trim().is_empty()` → fall through
    /// to `username`), not trimming a meaningful winner. Tab/newline/NBSP also fall through
    /// because `str::trim` is Unicode White_Space.
    #[test]
    fn whitespace_only_global_name_falls_through_to_username() {
        let mk = |global_name: &str| DiscordUser {
            id: "7".into(),
            username: "sam".into(),
            global_name: global_name.into(),
            discriminator: "0".into(),
            avatar: String::new(),
        };
        assert_eq!(mk("").display_name(), "sam");
        assert_eq!(mk("   ").display_name(), "sam");
        assert_eq!(mk("\t\n").display_name(), "sam");
        assert_eq!(mk("\u{00A0}").display_name(), "sam"); // NBSP
        // Meaningful name still wins — returned exactly as Discord sent it (no display trim).
        assert_eq!(mk("Dave").display_name(), "Dave");
        assert_eq!(mk("  Dave  ").display_name(), "  Dave  ");
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

    /// Wrap a body in a real 200 `reqwest::Response` so the assertions below run through the
    /// exact `decode_2xx` call production uses, not a stand-in `serde_json::from_str`.
    fn ok_response(body: &'static str) -> Response {
        Response::from(
            axum::http::Response::builder()
                .status(200)
                .body(body)
                .expect("build 200 response"),
        )
    }

    #[tokio::test]
    async fn a_200_without_roles_fails_to_decode() {
        // THE T-185 RESURRECTION. `#[serde(default)]` on `roles` meant a 200 carrying anything
        // that simply lacks the field — a proxy's JSON error envelope, a truncated gateway
        // response — decoded happily into `roles: []`. The caller cannot tell that apart from
        // Discord saying "no roles", so it demotes the user and DELETEs the stored snapshot
        // that `resync_all_roles` would have restored from. Failing the decode is what routes
        // it to the Err → Unavailable → write-nothing path instead.
        let err =
            decode_2xx::<GuildMember>(ok_response(r#"{"code":0,"message":"502 Bad Gateway"}"#))
                .await
                .expect_err("a 200 body with no `roles` field must not decode");
        // `{:#}` walks anyhow's cause chain — reqwest's own Display is just "error decoding
        // response body", and the serde reason we care about sits underneath it.
        let chain = format!("{err:#}");
        assert!(
            chain.contains("missing field `roles`"),
            "the decode error should name the missing field, got: {chain}"
        );
    }

    #[tokio::test]
    async fn a_200_profile_without_a_username_fails_to_decode() {
        // T-319, and the T-185 shape one struct over. `#[serde(default)]` on `username` meant a
        // 200 carrying anything that merely lacks the field decoded into `username: ""`, and the
        // oauth callback binds `display_name()`/`handle()` — both empty in that state — into
        // `users.username`/`users.discord_handle`. Measured pre-fix on `{"id":"7","avatar":"a1"}`:
        // decode Ok, display_name() "", handle() "". Failing the decode is what routes it to the
        // Err → `discord_unreachable` → write-nothing path.
        let err = decode_2xx::<DiscordUser>(ok_response(r#"{"id":"7","avatar":"a1"}"#))
            .await
            .expect_err("a 200 profile with no `username` must not decode");
        // `{:#}` walks anyhow's cause chain — reqwest's Display is only "error decoding response
        // body"; the serde reason we care about sits underneath.
        let chain = format!("{err:#}");
        assert!(
            chain.contains("missing field `username`"),
            "the decode error should name the missing field, got: {chain}"
        );
    }

    #[tokio::test]
    async fn a_200_profile_with_a_null_username_fails_to_decode() {
        // Pins the half that was never broken, so a later "let's be tolerant like the other
        // fields" pass cannot quietly reopen it by reaching for `null_default`. `null` is
        // malformed for a user object; it must stay an error, exactly as on `GuildMember::roles`.
        decode_2xx::<DiscordUser>(ok_response(r#"{"id":"7","username":null}"#))
            .await
            .expect_err("an explicit null username must not decode");
    }

    #[tokio::test]
    async fn a_200_profile_with_an_empty_username_decodes() {
        // Absent must fail; explicitly empty must not. Same line `GuildMember` draws between a
        // missing `roles` and `[]` — silence is the bug, a stated value is an answer.
        let u = decode_2xx::<DiscordUser>(ok_response(
            r#"{"id":"7","username":"","global_name":"Dave"}"#,
        ))
        .await
        .expect("an explicit empty username is a stated value");
        assert_eq!(u.username, "");
        assert_eq!(u.display_name(), "Dave");
    }

    #[tokio::test]
    async fn a_200_with_an_empty_roles_array_decodes() {
        // Absent must fail; empty must not. Discord sends `"roles": []` for a real member who
        // holds no roles, and that has to stay a decodable, authoritative answer.
        let m = decode_2xx::<GuildMember>(ok_response(r#"{"nick":"B","roles":[]}"#))
            .await
            .expect("an explicit empty roles array is a valid answer");
        assert!(m.roles.is_empty());
    }

    /// **T-405 — the CDN path-segment trust boundary.**
    ///
    /// `avatar_url()` interpolates two strings from Discord's HTTP response into a URL path. Every
    /// input below is what that response would have to contain for the resulting URL to point
    /// somewhere other than `cdn.discordapp.com/avatars/<id>/<hash>.png`. Before T-405 each one
    /// produced exactly that misdirected URL, stored public-tier, rendered in an `<img src>`.
    #[test]
    fn a_hostile_avatar_hash_cannot_walk_the_url_off_the_cdn_path() {
        for (id, avatar, why) in [
            (
                "7",
                "../../evil",
                "parent-directory traversal out of /avatars/",
            ),
            ("7", "..", "bare parent directory"),
            ("7", "a/b", "an extra path segment"),
            ("7", "x?y=z", "everything after it becomes a query string"),
            ("7", "x#frag", "everything after it becomes a fragment"),
            ("7", "x%2f..%2fevil", "percent-encoded separator"),
            ("7", "x@evil.com", "re-points the authority once combined"),
            ("7", "x\\y", "backslash, which WHATWG folds to a slash"),
            ("7", "x y", "space"),
            ("7", "x.png", "a dot ends the segment early"),
            // The `id` half is interpolated too, and nothing checked it either.
            ("../../evil", "abc", "traversal through the id"),
            ("7/../..", "abc", "traversal through the id"),
            ("7?x=y", "abc", "query injection through the id"),
            ("", "abc", "empty id yields a doubled slash"),
        ] {
            let u = DiscordUser {
                id: id.to_string(),
                username: "n".into(),
                global_name: String::new(),
                discriminator: String::new(),
                avatar: avatar.to_string(),
            };
            assert_eq!(
                u.avatar_url(),
                "",
                "id={id:?} avatar={avatar:?} built a URL despite {why}"
            );
        }
    }

    /// The other half, and the half that keeps the guard alive: a guard that blanks every real
    /// avatar gets reverted by whoever ships next.
    #[test]
    fn real_discord_avatars_still_build_a_cdn_url() {
        let mk = |id: &str, avatar: &str| DiscordUser {
            id: id.to_string(),
            username: "n".into(),
            global_name: String::new(),
            discriminator: String::new(),
            avatar: avatar.to_string(),
        };
        // A real snowflake and a real 32-hex avatar hash.
        assert_eq!(
            mk("80351110224678912", "8342729096ea3675442027381ff50dfe").avatar_url(),
            "https://cdn.discordapp.com/avatars/80351110224678912/8342729096ea3675442027381ff50dfe.png"
        );
        // Animated avatars carry the `a_` prefix — the underscore is why the class is not
        // `is_ascii_alphanumeric` alone.
        assert_eq!(
            mk("80351110224678912", "a_8342729096ea3675442027381ff50dfe").avatar_url(),
            "https://cdn.discordapp.com/avatars/80351110224678912/a_8342729096ea3675442027381ff50dfe.png"
        );
        // No custom avatar stays the empty string it always was — unchanged by T-405.
        assert_eq!(mk("80351110224678912", "").avatar_url(), "");
    }

    /// The guard is deliberately a character class, not Discord's documented formats. Pinned so a
    /// later "tighten this up" pass has to argue with a test rather than silently start blanking
    /// real avatars the day Discord widens its hash alphabet.
    #[test]
    fn the_rule_is_a_character_class_not_a_format() {
        assert!(is_cdn_path_segment("abc123"));
        assert!(is_cdn_path_segment("a_b_C9"));
        assert!(is_cdn_path_segment("ZZZ"));
        // Not hex, not a snowflake, not 32 characters — and deliberately still accepted.
        assert!(is_cdn_path_segment("zzzz"));
        assert!(!is_cdn_path_segment(""));
        for bad in [
            "a.b", "a/b", "a\\b", "a?b", "a#b", "a%b", "a@b", "a:b", "a b", "a\tb",
        ] {
            assert!(!is_cdn_path_segment(bad), "accepted {bad:?}");
        }
    }
}
