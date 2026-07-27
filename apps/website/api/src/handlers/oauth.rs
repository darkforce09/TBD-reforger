//! Discord OAuth2 login/callback — Rust port of the OAuth half of `handlers/auth.go`.
//!
//! `discord_login` sets a 10-min httpOnly `oauth_state` CSRF cookie and 307-redirects
//! to Discord consent. `discord_callback` validates state (constant-time), exchanges
//! the code, upserts the user, syncs roles, and 302-redirects to the SPA callback with
//! the tokens in the URL fragment — or to an error reason on any failure.
//!
//! **Role-sync invariant (T-185).** Roles are only ever written when Discord actually
//! answered. See [`RoleSnapshot`] — an unreachable Discord must leave the stored
//! snapshot and the user's tier untouched, because losing the snapshot is permanent.

use axum::body::Body;
use axum::extract::{Query, State};
// Split rather than `{HeaderMap, HeaderValue, StatusCode, header}`: the two rustfmt style
// editions in play disagree on where a lowercase module sorts inside a brace list, and the
// merged form is stable under only one of them. Split, both agree.
use axum::http::header;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use crate::auth;
use crate::handlers::auth::{
    arma_id_is_linked, issue_session, redirect_auth_error, session_redirect,
};
use crate::handlers::load_user;
use crate::models::AuditSeverity;
use crate::services;
use crate::services::discord::GuildMember;
// T-405 — `users.avatar_url` is public tier; guarded at this write boundary like every other URL
// column (T-391's `is_http_url`).
use crate::services::text::is_http_url;
use crate::state::AppState;

/// Query params on the OAuth callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub state: String,
}

/// `GET /api/v1/auth/discord/login` — start the OAuth2 flow.
///
/// @route GET /api/v1/auth/discord/login
pub async fn discord_login(State(state): State<AppState>) -> Response {
    let st = auth::random_token(16);
    match state.discord.authorize_url(&st) {
        Ok(url) => {
            let secure = if state.cfg.is_development() {
                ""
            } else {
                "; Secure"
            };
            let cookie =
                format!("oauth_state={st}; Path=/; Max-Age=600; HttpOnly; SameSite=Lax{secure}");
            Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT) // 307, like Go
                .header(header::LOCATION, url)
                .header(header::SET_COOKIE, cookie)
                .body(Body::empty())
                .expect("redirect response")
        }
        // Blank client_id → surface the misconfig through the SPA, not Discord.
        Err(_) => redirect_auth_error(&state.cfg.frontend_url, "oauth_unconfigured"),
    }
}

/// Clear the CSRF `oauth_state` cookie. Every callback response that has decided
/// the cookie is missing/invalid — or that has successfully consumed it — must
/// emit this. T-248: the early `missing_code` / `invalid_state` returns used to
/// skip it and leave the ten-minute cookie live for replay.
/// Exact Set-Cookie value for clearing `oauth_state`. Pub so ITs can assert
/// byte-equality (T-480) — soft `contains("Path=/")` greened a wrong Path=/api.
pub const OAUTH_STATE_CLEAR: &str = "oauth_state=; Path=/; Max-Age=0; HttpOnly";

/// CSRF pre-check for the Discord callback. `Some(resp)` is a finished error
/// redirect that already clears `oauth_state`; `None` means state matched and
/// the caller may proceed (and must still clear the cookie on every exit).
fn callback_csrf_reject(fe: &str, q: &CallbackQuery, headers: &HeaderMap) -> Option<Response> {
    if q.code.is_empty() || q.state.is_empty() {
        return Some(with_set_cookie(
            redirect_auth_error(fe, "missing_code"),
            OAUTH_STATE_CLEAR,
        ));
    }
    let cookie_state = read_cookie(headers, "oauth_state").unwrap_or_default();
    if cookie_state.is_empty() || !auth::constant_time_equal(&q.state, &cookie_state) {
        return Some(with_set_cookie(
            redirect_auth_error(fe, "invalid_state"),
            OAUTH_STATE_CLEAR,
        ));
    }
    None
}

/// `GET /api/v1/auth/discord/callback` — complete the flow.
///
/// @route GET /api/v1/auth/discord/callback
pub async fn discord_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let fe = &state.cfg.frontend_url;
    if let Some(reject) = callback_csrf_reject(fe, &q, &headers) {
        return reject;
    }
    // State is valid — every response from here clears the cookie (Go clears here too).
    let err = |reason: &str| with_set_cookie(redirect_auth_error(fe, reason), OAUTH_STATE_CLEAR);

    let Ok(tok) = state.discord.exchange_code(&q.code).await else {
        return err("discord_unreachable");
    };
    let Ok(du) = state.discord.fetch_user(&tok.access_token).await else {
        return err("discord_unreachable");
    };
    // Member roles drive the web role — but only when Discord actually answered.
    let snapshot = if guild_configured(&state.cfg.discord_guild_id) {
        classify_member_lookup(
            &du.id,
            state.discord.fetch_guild_member(&tok.access_token).await,
        )
    } else {
        tracing::error!(
            discord_id = %du.id,
            "DISCORD_GUILD_ID is not configured — skipping role sync; \
             stored Discord roles and web role left unchanged"
        );
        RoleSnapshot::Unavailable
    };

    // **T-405 — the write boundary for `users.avatar_url`, the highest-exposure column of the
    // group.** It is public tier (anyone who can trigger a login writes it), and it reaches an
    // `<img src>` on four SPA surfaces — leaderboards, the layout chrome, settings and the event
    // hub — so it is read by far more of the platform than the admin-tier columns.
    //
    // `avatar_url()` now refuses to build a URL out of an `id`/`avatar` that is not a bare path
    // segment (T-405, `services::discord`), so in practice this second check is belt to that
    // brace. It is here anyway because the two guard different things and can fail independently:
    // that one asserts "Discord's strings did not escape the path", this one asserts "whatever
    // ended up in this variable is an http(s) URL". A future edit that adds a config-driven CDN
    // base, or swaps in a different identity provider, moves the first guarantee without touching
    // the second — and this is the column where finding that out late is most expensive.
    //
    // Falls back to `""` instead of 400-ing, because this is an OAuth callback: refusing a login
    // over a cosmetic field would turn a bad avatar into an outage. `""` is the column's existing
    // "no avatar" value and every reader already handles it.
    let avatar_url = du.avatar_url();
    let avatar_url = if is_http_url(&avatar_url) {
        avatar_url
    } else {
        if !avatar_url.is_empty() {
            tracing::warn!(
                discord_id = %du.id,
                "discarded a non-http(s) avatar URL built from Discord's profile response"
            );
        }
        String::new()
    };

    // Upsert the user from the fresh Discord profile (role is set separately below).
    let upsert = sqlx::query(
        "INSERT INTO users \
         (discord_id, username, discord_handle, avatar_url, arma_character, is_banned, ban_reason, \
          last_login_at, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, '', false, '', now(), now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET \
          username = EXCLUDED.username, discord_handle = EXCLUDED.discord_handle, \
          avatar_url = EXCLUDED.avatar_url, last_login_at = EXCLUDED.last_login_at, updated_at = now()",
    )
    .bind(&du.id)
    .bind(du.display_name())
    .bind(du.handle())
    .bind(&avatar_url)
    .execute(&state.pool)
    .await;
    if upsert.is_err() {
        return err("server_error");
    }

    // Only a real answer from Discord may touch roles. `sync_roles` DELETEs every
    // `user_discord_roles` row for this user before re-inserting, so calling it with a
    // stand-in empty vec is what erased admins on a transient failure (T-185).
    if let Some(role_ids) = snapshot.ids_to_persist() {
        let Ok(role) = services::role_sync::sync_roles(&state.pool, &du.id, role_ids).await else {
            return err("server_error");
        };
        if sqlx::query("UPDATE users SET role = $1, updated_at = now() WHERE discord_id = $2")
            .bind(role)
            .bind(&du.id)
            .execute(&state.pool)
            .await
            .is_err()
        {
            return err("server_error");
        }
    }

    // Reload for current ban + Arma-link state — and for the role, which is either the
    // one just synced above or the untouched stored one when Discord was unreachable.
    let Ok(Some(fresh)) = load_user(&state.pool, &du.id).await else {
        return err("server_error");
    };
    if fresh.is_banned {
        return err("banned");
    }
    let arma_linked = arma_id_is_linked(&fresh.arma_id);

    let Ok((access, exp, refresh)) =
        issue_session(&state, &du.id, fresh.role.as_str(), arma_linked).await
    else {
        return err("server_error");
    };

    // A skipped sync is a degraded login, not a normal one: surface it where admins
    // actually look, not only in the process log.
    if snapshot.ids_to_persist().is_none() {
        services::write_audit(
            &state.pool,
            AuditSeverity::Warn,
            Some(&du.id),
            &fresh.username,
            "auth.role_sync_skipped",
            &format!(
                "Discord roles unavailable at login — kept {} for {}",
                fresh.role.as_str(),
                fresh.username
            ),
            "user",
            &du.id,
        )
        .await;
    }

    services::write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(&du.id),
        &fresh.username,
        "auth.login",
        &format!("{} signed in via Discord", fresh.username),
        "user",
        &du.id,
    )
    .await;

    with_set_cookie(
        session_redirect(fe, &access, &refresh, exp, arma_linked),
        OAUTH_STATE_CLEAR,
    )
}

/// What the Discord guild-member lookup actually told us about a user's roles.
///
/// The distinction is the whole point of T-185. [`services::role_sync::sync_roles`]
/// DELETEs every `user_discord_roles` row for the user before re-inserting, then
/// resolves the web role from what it just wrote — so handing it an empty vec both
/// demotes the user to enlisted *and* destroys the snapshot. `resync_all_roles` reads
/// that same table, so once it is gone there is nothing left to restore from: a
/// two-second Discord timeout during one login permanently unmade an admin.
///
/// An empty role list may therefore only ever come from Discord genuinely saying "this
/// user has no roles" — never from a timeout, a 5xx, or an unconfigured guild id.
enum RoleSnapshot {
    /// Discord answered. These ids are authoritative; empty means a real non-member.
    Authoritative(Vec<String>),
    /// We could not ask Discord at all. The stored snapshot and the user's current
    /// tier must be left exactly as they are.
    Unavailable,
}

impl RoleSnapshot {
    /// The role ids to write, or `None` when nothing may be written.
    ///
    /// Do not paper over the `None` with a default — `unwrap_or_default()` on a failed
    /// lookup is precisely the bug this type exists to prevent.
    fn ids_to_persist(&self) -> Option<&[String]> {
        match self {
            RoleSnapshot::Authoritative(ids) => Some(ids),
            RoleSnapshot::Unavailable => None,
        }
    }
}

/// True when a guild id is actually set.
///
/// Blank leaves `DiscordService` requesting `/users/@me/guilds//member`; Discord answers
/// 404, and `fetch_guild_member` maps 404 to `Ok(None)` — "not a member". So a blank
/// `DISCORD_GUILD_ID` is a misconfiguration that is indistinguishable from a legitimate
/// non-member, and before T-185 it enlisted the entire community, one login at a time,
/// without emitting a single log line.
fn guild_configured(guild_id: &str) -> bool {
    !guild_id.trim().is_empty()
}

/// Classify a `fetch_guild_member` outcome, logging loudly when Discord is unreachable.
///
/// `Ok(None)` is Discord's 404 for "not in this guild" — a real answer, so it is allowed
/// to demote. `Err` is not an answer at all and must change nothing.
fn classify_member_lookup(
    discord_id: &str,
    lookup: anyhow::Result<Option<GuildMember>>,
) -> RoleSnapshot {
    match lookup {
        Ok(Some(m)) => RoleSnapshot::Authoritative(m.roles),
        Ok(None) => RoleSnapshot::Authoritative(Vec::new()),
        Err(e) => {
            tracing::error!(
                discord_id,
                error = %e,
                "discord guild-member lookup failed — keeping the stored role snapshot"
            );
            RoleSnapshot::Unavailable
        }
    }
}

/// Read a cookie value by name from the request's `Cookie` header.
fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{name}=");
    raw.split(';')
        .map(str::trim)
        .find_map(|p| p.strip_prefix(&prefix).map(str::to_string))
}

/// Append a `Set-Cookie` header to a response.
fn with_set_cookie(mut resp: Response, cookie: &str) -> Response {
    if let Ok(hv) = HeaderValue::from_str(cookie) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(roles: &[&str]) -> GuildMember {
        GuildMember {
            nick: String::new(),
            roles: roles.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn transport_failure_writes_nothing() {
        // The T-185 regression: a Discord timeout collapsed into an empty role vec, which
        // sync_roles turned into "DELETE every stored role for this user" → Enlisted, with
        // no snapshot left for resync_all_roles to restore from. `None` here is the proof
        // that sync_roles — the only thing that deletes — is never reached on a failure.
        let snap = classify_member_lookup("42", Err(anyhow::anyhow!("connection reset by peer")));
        assert!(
            snap.ids_to_persist().is_none(),
            "an unreachable Discord must not write roles"
        );
    }

    #[test]
    fn real_non_member_still_demotes() {
        // Discord's 404 is an answer: the user genuinely holds no guild roles, so the
        // empty write (and the resulting Enlisted) is correct.
        let snap = classify_member_lookup("42", Ok(None));
        assert!(
            matches!(snap.ids_to_persist(), Some(ids) if ids.is_empty()),
            "a 404 non-member must still sync to no roles"
        );
    }

    #[test]
    fn member_roles_are_persisted_verbatim() {
        let snap = classify_member_lookup("42", Ok(Some(member(&["1517", "8899"]))));
        assert_eq!(
            snap.ids_to_persist().expect("authoritative"),
            ["1517", "8899"]
        );
    }

    /// Mirror the production decode seam for a 200: `decode_2xx` is
    /// `Ok(resp.json::<GuildMember>().await?)`, so a body that fails to deserialize leaves
    /// as `Err` and one that succeeds arrives as `Ok(Some(..))`. Deserializing here rather
    /// than hand-building a `GuildMember` is the point — the bug lived in the derive.
    fn lookup_from_200_body(body: &str) -> anyhow::Result<Option<GuildMember>> {
        Ok(Some(serde_json::from_str::<GuildMember>(body)?))
    }

    #[test]
    fn absent_roles_field_on_a_200_does_not_demote() {
        // T-185 shipped `RoleSnapshot` to stop a transport failure from erasing roles, but
        // left `#[serde(default)]` on `GuildMember::roles` — so a gateway or proxy serving a
        // JSON error envelope with a 200 status still deserialized to `roles: []`, became
        // Authoritative(vec![]), and sent sync_roles off to DELETE every stored role. Same
        // permanent damage as the original bug, through a different door. An absent field is
        // not an answer about this user's roles, and must reach the Unavailable branch.
        let snap = classify_member_lookup(
            "42",
            lookup_from_200_body(r#"{"code":0,"message":"502 Bad Gateway"}"#),
        );
        assert!(
            snap.ids_to_persist().is_none(),
            "a 200 whose body omits `roles` must not be read as an authoritative empty role list"
        );
    }

    #[test]
    fn explicitly_empty_roles_array_still_demotes() {
        // The other half of the contract, and the reason this is a serde fix rather than a
        // "treat empty as unavailable" fix: a guild member who genuinely holds no roles gets
        // `"roles": []` from Discord. That IS an answer, so it must still write — and the
        // resulting demotion to enlisted is correct, not a regression.
        let snap = classify_member_lookup("42", lookup_from_200_body(r#"{"nick":"B","roles":[]}"#));
        assert!(
            matches!(snap.ids_to_persist(), Some(ids) if ids.is_empty()),
            "an explicit `roles: []` is a real answer and must still sync to no roles"
        );
    }

    #[test]
    fn populated_roles_on_a_200_are_authoritative() {
        // Guard the happy path against an over-broad fix: tightening `roles` must not make
        // ordinary logins fall through to Unavailable and freeze everyone's role forever.
        let snap = classify_member_lookup(
            "42",
            lookup_from_200_body(r#"{"nick":null,"roles":["1517285898817896559"]}"#),
        );
        assert_eq!(
            snap.ids_to_persist().expect("authoritative"),
            ["1517285898817896559"]
        );
    }

    #[test]
    fn blank_guild_id_is_not_configured() {
        // A blank id must never reach Discord: the resulting 404 would read as Ok(None)
        // and demote every user who logs in.
        assert!(!guild_configured(""));
        assert!(!guild_configured("   "));
        assert!(guild_configured("1517285898817896559"));
    }

    fn set_cookie_values(resp: &Response) -> Vec<String> {
        resp.headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(str::to_string))
            .collect()
    }

    /// T-483 — exact equality to `OAUTH_STATE_CLEAR`. Soft
    /// `contains("oauth_state=")/Max-Age=0/HttpOnly` greened `Path=/api`.
    fn clears_oauth_state(resp: &Response) -> bool {
        set_cookie_values(resp)
            .iter()
            .any(|c| c.as_str() == OAUTH_STATE_CLEAR)
    }

    /// T-248 — `missing_code` must clear the CSRF cookie. Before the fix the early
    /// return skipped `OAUTH_STATE_CLEAR` and left the ten-minute cookie live.
    #[test]
    fn missing_code_clears_oauth_state_cookie() {
        let q = CallbackQuery {
            code: String::new(),
            state: String::new(),
        };
        let headers = HeaderMap::new();
        let resp = callback_csrf_reject("http://localhost:5173", &q, &headers)
            .expect("empty code/state must reject");
        assert!(
            clears_oauth_state(&resp),
            "missing_code must Set-Cookie oauth_state Max-Age=0; got {:?}",
            set_cookie_values(&resp)
        );
        let loc = resp.headers()[header::LOCATION].to_str().unwrap();
        assert!(loc.contains("error=missing_code"), "{loc}");
    }

    /// T-248 — `invalid_state` (present query, absent/mismatched cookie) must clear too.
    #[test]
    fn invalid_state_clears_oauth_state_cookie() {
        let q = CallbackQuery {
            code: "abc".into(),
            state: "xyz".into(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("oauth_state=other"),
        );
        let resp = callback_csrf_reject("http://localhost:5173", &q, &headers)
            .expect("mismatched state must reject");
        assert!(
            clears_oauth_state(&resp),
            "invalid_state must Set-Cookie oauth_state Max-Age=0; got {:?}",
            set_cookie_values(&resp)
        );
        let loc = resp.headers()[header::LOCATION].to_str().unwrap();
        assert!(loc.contains("error=invalid_state"), "{loc}");
    }

    /// Matching state is not a reject — the caller proceeds and clears on every exit.
    #[test]
    fn matching_state_is_not_a_csrf_reject() {
        let q = CallbackQuery {
            code: "abc".into(),
            state: "good".into(),
        };
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("oauth_state=good"));
        assert!(callback_csrf_reject("http://localhost:5173", &q, &headers).is_none());
    }
}
