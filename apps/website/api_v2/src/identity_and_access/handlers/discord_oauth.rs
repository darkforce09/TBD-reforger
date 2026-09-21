//! Discord OAuth2 login and callback.
//!
//! `discord_login` sets a 10-min httpOnly `oauth_state` CSRF cookie and 307-redirects
//! to Discord consent. `discord_callback` validates state (constant-time), exchanges
//! the code, upserts the user, syncs roles, and 302-redirects to the SPA callback with
//! the tokens in the URL fragment — or to an error reason on any failure.
//!
//! **Role-sync invariant.** Roles are only ever written when Discord actually answered.
//! See [`RoleSnapshot`] — an unreachable Discord must leave the stored snapshot and the
//! user's tier untouched, because losing the snapshot is permanent.

use axum::body::Body;
use axum::extract::{Query, State};
// Split rather than `{HeaderMap, HeaderValue, StatusCode, header}`: the two rustfmt style
// editions in play disagree on where a lowercase module sorts inside a brace list, and the
// merged form is stable under only one of them. Split, both agree.
use axum::http::header;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::core::application_state::AppState;
use crate::core::authentication_primitives;
// `users.avatar_url` is public tier; guarded at this write boundary like every other URL
// column.
use crate::core::text::http_url_guard::is_http_url;
use crate::identity_and_access::services::discord_client::GuildMember;
use crate::identity_and_access::services::discord_role_sync;
use crate::identity_and_access::services::session_issuance::{
    arma_id_is_linked, issue_session, redirect_auth_error, session_redirect,
};
use crate::identity_and_access::services::user_lookup::load_user;

use super::oauth_host_guard::{
    OAUTH_STATE_CLEAR, callback_csrf_reject, reject_login_on_host_mismatch,
};

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
    // Refuse to start a flow that is already guaranteed to fail. See
    // `reject_login_on_host_mismatch` for why this is a refusal in development and only a
    // log line in production.
    if let Some(reject) = reject_login_on_host_mismatch(&state.cfg) {
        return reject;
    }
    let st = authentication_primitives::random_token(16);
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
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header(header::LOCATION, url)
                .header(header::SET_COOKIE, cookie)
                .body(Body::empty())
                .expect("redirect response")
        }
        // Blank client_id → surface the misconfig through the SPA, not Discord.
        Err(_) => redirect_auth_error(&state.cfg.frontend_url, "oauth_unconfigured"),
    }
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
    if let Some(reject) = callback_csrf_reject(fe, &state.cfg.discord_redirect_url, &q, &headers) {
        return reject;
    }
    // State is valid — every response from here clears the cookie.
    let err = |reason: &str| with_set_cookie(redirect_auth_error(fe, reason), OAUTH_STATE_CLEAR);

    // Both failure modes below emit the identical `#error=discord_unreachable`, so the error
    // itself must not be dropped: without the log a wrong DISCORD_CLIENT_SECRET is
    // indistinguishable from Discord being down. See `log_discord_call_failure`.
    let tok = match state.discord.exchange_code(&q.code).await {
        Ok(tok) => tok,
        Err(e) => {
            log_discord_call_failure("token_exchange", &e);
            return err("discord_unreachable");
        }
    };
    let du = match state.discord.fetch_user(&tok.access_token).await {
        Ok(du) => du,
        Err(e) => {
            log_discord_call_failure("fetch_user", &e);
            return err("discord_unreachable");
        }
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

    // **The write boundary for `users.avatar_url`, the highest-exposure column of the group.**
    // It is public tier (anyone who can trigger a login writes it), and it reaches an
    // `<img src>` on four SPA surfaces — leaderboards, the layout chrome, settings and the event
    // hub — so it is read by far more of the platform than the admin-tier columns.
    //
    // `avatar_url()` refuses to build a URL out of an `id`/`avatar` that is not a bare path
    // segment, so in practice this second check is belt to that brace. It is here anyway because
    // the two guard different things and can fail independently: that one asserts "Discord's
    // strings did not escape the path", this one asserts "whatever ended up in this variable is
    // an http(s) URL". A future edit that adds a config-driven CDN base, or swaps in a different
    // identity provider, moves the first guarantee without touching the second — and this is the
    // column where finding that out late is most expensive.
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
    // stand-in empty vec is what erases admins on a transient failure.
    if let Some(role_ids) = snapshot.ids_to_persist() {
        let Ok(role) = discord_role_sync::sync_roles(&state.pool, &du.id, role_ids).await else {
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
        write_audit(
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

    write_audit(
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
/// The distinction is the whole point of this type.
/// [`crate::identity_and_access::services::discord_role_sync::sync_roles`] DELETEs every
/// `user_discord_roles` row for the user before re-inserting, then resolves the web role from
/// what it just wrote — so handing it an empty vec both demotes the user to enlisted *and*
/// destroys the snapshot. `resync_all_roles` reads that same table, so once it is gone there
/// is nothing left to restore from: a two-second Discord timeout during one login permanently
/// unmakes an admin.
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
/// `DISCORD_GUILD_ID` is a misconfiguration indistinguishable from a legitimate non-member,
/// and unguarded it enlists the entire community, one login at a time, without emitting a
/// single log line.
fn guild_configured(guild_id: &str) -> bool {
    !guild_id.trim().is_empty()
}

/// Log a failed Discord call so a **config fault** and a **real outage** are
/// distinguishable, which the shared `#error=discord_unreachable` response code is not.
///
/// **How it discriminates, and why that is not string-matching.** The classification is by
/// *type*, not by parsing another module's message text. Every network failure inside the
/// Discord client surfaces a `reqwest::Error` somewhere in the `anyhow` source chain; a
/// Discord response that arrived and was rejected surfaces `decode_2xx`'s plain
/// `anyhow::bail!`, which has no `reqwest::Error` in it at all. So:
///   * `reqwest::Error` that is connect/timeout/request → nothing usable came back → OUTAGE.
///   * `reqwest::Error` that is a decode → a body arrived and did not parse → PROTOCOL
///     (a proxy serving an error envelope under a 2xx, typically).
///   * no `reqwest::Error` → Discord answered and said no → CONFIG.
///
/// The verdict is a hint, never the whole story, so the **full source chain is logged
/// verbatim beside it** (`{e:#}`) — the operator can always check the classification against
/// the evidence rather than trusting it. A guard whose output cannot be audited is the shape
/// this program exists to reject.
///
/// **Why no secret can reach this log.** The strongest guarantee is structural: this function
/// is handed a stage label and an error, and nothing else — `q.code` and `tok.access_token`
/// are not in scope here and are not passed. For the error text itself, all four things it
/// can contain are safe by construction:
///   1. `reqwest::Error` Display carries the request URL. Those URLs are `…/oauth2/token` and
///      `…/users/@me` — no query string. The client secret and the code travel in the POST
///      **form body** and the access token in the `Authorization` **header**; reqwest prints
///      neither bodies nor headers.
///   2. `decode_2xx` embeds a ≤4096-char snippet of a **non-2xx** body. A non-2xx OAuth
///      response cannot carry an access token — there is no token to issue when the call was
///      refused — so it is an `{"error":"invalid_client"}`-shaped envelope.
///   3. `"discord: empty access token"` — a literal, and notably not the token.
///   4. serde decode errors name the missing/unexpected field and a line/column offset, not
///      the document contents.
fn log_discord_call_failure(stage: &'static str, e: &anyhow::Error) {
    // Full anyhow chain, not just the outermost Display — a wrapped transport error is
    // otherwise reduced to a summary that discards the actual cause.
    let detail = format!("{e:#}");
    let (fault, guidance) = classify_discord_failure(e);
    tracing::error!(
        stage = %stage,
        fault = %fault,
        error = %detail,
        "discord {} failed [{}] — {}",
        stage,
        fault,
        guidance
    );
}

/// The verdict half of [`log_discord_call_failure`], split out so it is **testable**.
///
/// Left inline it would be a branch whose only output is a log line — unassertable,
/// therefore unfalsifiable. Returns `(fault, guidance)`.
fn classify_discord_failure(e: &anyhow::Error) -> (&'static str, &'static str) {
    match e.chain().find_map(|c| c.downcast_ref::<reqwest::Error>()) {
        Some(re) if re.is_timeout() || re.is_connect() || re.is_request() => (
            "outage",
            "no usable answer came back from Discord (network, DNS, TLS or Discord itself). \
             The configuration is NOT implicated — retrying is the correct response.",
        ),
        Some(_) => (
            "protocol",
            "Discord's response arrived but did not decode. Usually a proxy or gateway serving \
             an error envelope under a 2xx status, not a credential problem.",
        ),
        None => (
            "config",
            "Discord answered and REJECTED the call — the status and its body are in `error`. \
             401 invalid_client = wrong DISCORD_CLIENT_ID / DISCORD_CLIENT_SECRET; 400 \
             invalid_grant = wrong DISCORD_REDIRECT_URL, or a code already used or expired. \
             Pre-flight the credential pair with the curl in apps/website/api_v2/.env.example.",
        ),
    }
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
pub(super) fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{name}=");
    raw.split(';')
        .map(str::trim)
        .find_map(|p| p.strip_prefix(&prefix).map(str::to_string))
}

/// Append a `Set-Cookie` header to a response.
pub(super) fn with_set_cookie(mut resp: Response, cookie: &str) -> Response {
    if let Ok(hv) = HeaderValue::from_str(cookie) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    resp
}

#[cfg(test)]
#[path = "tests/discord_oauth.rs"]
mod tests;
