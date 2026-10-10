//! Discord OAuth2 login and callback.
//!
//! `discord_login` sets a 10-min httpOnly `oauth_state` CSRF cookie and 307-redirects
//! to Discord consent. `discord_callback` validates state (constant-time), exchanges
//! the code, registers the account (`account_registration`), syncs roles, and 302-redirects to
//! the SPA callback with the tokens in the URL fragment — or to an error reason on any failure.
//! A callback query string that does not decode (a repeated `code` or `state`) carries no usable
//! code, so it redirects with `missing_code` like an absent one, never with an API error body.
//!
//! **Role-sync invariant.** Roles are only ever written when Discord actually answered.
//! An unreachable Discord preserves the verified snapshot. Session issuance evaluates its
//! age and any audited grace extension without treating transport failure as departure.

use api_identifiers::{DiscordGuildId, DiscordUserId};
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
// Split rather than `{HeaderMap, HeaderValue, StatusCode, header}`: the two rustfmt style
// editions in play disagree on where a lowercase module sorts inside a brace list, and the
// merged form is stable under only one of them. Split, both agree.
use axum::http::header;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use crate::services::account_registration::{AccountProfile, register_account};
use crate::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh, record_membership_failure,
};
use crate::services::session_issuance::{issue_session, redirect_auth_error, session_redirect};
use crate::services::user_lookup::load_user;
use api_caller_identity::arma_identity_link::arma_id_is_linked;
use api_discord::discord_client::GuildMember;
use api_discord::error::Error as DiscordError;
use api_foundation::error_handling::error_causes::message_with_causes;
use api_http_layer::authentication_primitives;
use api_state::AppState;

use super::oauth_host_guard::{
    OAUTH_STATE_CLEAR, callback_csrf_reject, reject_login_on_host_mismatch,
};

/// Query params on the OAuth callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// The authorization code Discord hands back; empty when absent.
    #[serde(default)]
    pub code: String,
    /// The CSRF state echoed back by Discord; empty when absent.
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
    query: Result<Query<CallbackQuery>, QueryRejection>,
) -> Response {
    let fe = &state.cfg.frontend_url;
    let Ok(Query(q)) = query else {
        return with_set_cookie(redirect_auth_error(fe, "missing_code"), OAUTH_STATE_CLEAR);
    };
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
    // Register the account from the fresh Discord profile; `account_registration` is the write
    // boundary of the profile columns, the avatar URL guard included. The role follows below,
    // from the membership observation.
    let (username, handle, avatar_url) = (du.display_name(), du.handle(), du.avatar_url());
    let profile = AccountProfile {
        discord_id: &du.id,
        username: &username,
        discord_handle: &handle,
        avatar_url: &avatar_url,
    };
    if register_account(&state.pool, &profile).await.is_err() {
        return err("server_error");
    }

    let lease = match claim_membership_refresh(
        &state.pool,
        &du.id,
        &state.cfg.discord_guild_id,
        true,
    )
    .await
    {
        Ok(lease) => lease,
        Err(_) => return err("server_error"),
    };
    let snapshot = if guild_configured(&state.cfg.discord_guild_id) {
        classify_member_lookup(
            &du.id,
            state.discord.fetch_guild_member(&tok.access_token).await,
        )
    } else {
        RoleSnapshot::Unavailable
    };
    if let Some(lease) = lease {
        let persisted = match &snapshot {
            RoleSnapshot::Authoritative(roles) => accept_membership_observation(
                &state.pool,
                &lease,
                Some(&GuildMember {
                    nick: String::new(),
                    roles: roles.clone(),
                }),
                &state.cfg.discord_guild_id,
            )
            .await
            .map(|_| ()),
            RoleSnapshot::Nonmember => accept_membership_observation(
                &state.pool,
                &lease,
                None,
                &state.cfg.discord_guild_id,
            )
            .await
            .map(|_| ()),
            RoleSnapshot::Unavailable => {
                record_membership_failure(
                    &state.pool,
                    &lease,
                    chrono::Duration::seconds(60),
                    "OAuth membership unavailable",
                )
                .await
            }
        };
        if persisted.is_err() {
            return err("server_error");
        }
    }

    // Profile facts supply the callback link flag; session issuance rechecks account authority.
    let Ok(Some(fresh)) = load_user(&state.pool, &du.id).await else {
        return err("server_error");
    };
    if fresh.is_banned {
        return err("banned");
    }
    let arma_linked = arma_id_is_linked(&fresh.arma_id);

    let Ok((access, exp, refresh)) = issue_session(&state, &du.id).await else {
        return err("server_error");
    };

    with_set_cookie(
        session_redirect(fe, &access, &refresh, exp, arma_linked),
        OAUTH_STATE_CLEAR,
    )
}

/// Authoritative membership is distinct from verification failure and from an empty role list.
enum RoleSnapshot {
    /// Discord confirms membership. The role list may be empty.
    Authoritative(Vec<String>),
    /// Discord confirms the user is not a guild member.
    Nonmember,
    /// We could not ask Discord at all. The stored snapshot and the user's current
    /// tier must be left exactly as they are.
    Unavailable,
}

/// True when a guild id is actually set.
///
/// Blank leaves `DiscordService` requesting `/users/@me/guilds//member`; Discord answers
/// 404, and `fetch_guild_member` maps 404 to `Ok(None)` — "not a member". So a blank
/// `DISCORD_GUILD_ID` is a misconfiguration indistinguishable from a legitimate non-member,
/// and unguarded it enlists the entire community, one login at a time, without emitting a
/// single log line.
fn guild_configured(guild_id: &DiscordGuildId) -> bool {
    !guild_id.as_str().trim().is_empty()
}

/// Log a failed Discord call so a **config fault** and a **real outage** are
/// distinguishable, which the shared `#error=discord_unreachable` response code is not.
///
/// **How it discriminates, and why that is not string-matching.** The classification is by
/// *type*, not by parsing another module's message text. Every network failure inside the
/// Discord client arrives as the `Transport` variant of the client's typed error, carrying the
/// `reqwest::Error`; a Discord response that arrived and was rejected arrives as a variant with
/// no `reqwest::Error` in it at all (`decode_2xx`'s `DiscordStatus`, for one). So:
///   * `reqwest::Error` that is connect/timeout/request → nothing usable came back → OUTAGE.
///   * `reqwest::Error` that is a decode → a body arrived and did not parse → PROTOCOL
///     (a proxy serving an error envelope under a 2xx, typically).
///   * no `reqwest::Error` → Discord answered and said no → CONFIG.
///
/// The verdict is a hint, never the whole story, so the **full cause chain is logged
/// verbatim beside it** ([`message_with_causes`]) — the operator can always check the
/// classification against the evidence rather than trusting it. A guard whose output cannot be
/// audited is the shape this program exists to reject.
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
fn log_discord_call_failure(stage: &'static str, e: &DiscordError) {
    // The full cause chain, not just the outermost Display — a transport error's own Display is
    // a summary that discards the actual cause.
    let detail = message_with_causes(e);
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
fn classify_discord_failure(e: &DiscordError) -> (&'static str, &'static str) {
    let transport = match e {
        DiscordError::Transport(re) => Some(re),
        _ => None,
    };
    match transport {
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
             Pre-flight the credential pair with the curl in deploy/api.env.example.",
        ),
    }
}

/// Classify a `fetch_guild_member` outcome, logging loudly when Discord is unreachable.
///
/// `Ok(None)` is Discord's 404 for "not in this guild" — a real answer, so it is allowed
/// to demote. `Err` is not an answer at all and must change nothing.
fn classify_member_lookup(
    discord_id: &DiscordUserId,
    lookup: Result<Option<GuildMember>, DiscordError>,
) -> RoleSnapshot {
    match lookup {
        Ok(Some(m)) => RoleSnapshot::Authoritative(m.roles),
        Ok(None) => RoleSnapshot::Nonmember,
        Err(e) => {
            tracing::error!(
                discord_id = %discord_id,
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
