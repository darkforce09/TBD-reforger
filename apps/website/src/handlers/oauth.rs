//! Discord OAuth2 login/callback — Rust port of the OAuth half of `handlers/auth.go`.
//!
//! `discord_login` sets a 10-min httpOnly `oauth_state` CSRF cookie and 307-redirects
//! to Discord consent. `discord_callback` validates state (constant-time), exchanges
//! the code, upserts the user, syncs roles, and 302-redirects to the SPA callback with
//! the tokens in the URL fragment — or to an error reason on any failure.

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use serde::Deserialize;

use crate::auth;
use crate::handlers::auth::{issue_session, redirect_auth_error, session_redirect};
use crate::handlers::load_user;
use crate::models::AuditSeverity;
use crate::services;
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

/// `GET /api/v1/auth/discord/callback` — complete the flow.
///
/// @route GET /api/v1/auth/discord/callback
pub async fn discord_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let fe = &state.cfg.frontend_url;
    if q.code.is_empty() || q.state.is_empty() {
        return redirect_auth_error(fe, "missing_code");
    }
    let cookie_state = read_cookie(&headers, "oauth_state").unwrap_or_default();
    if cookie_state.is_empty() || !auth::constant_time_equal(&q.state, &cookie_state) {
        return redirect_auth_error(fe, "invalid_state");
    }
    // State is valid — every response from here clears the cookie (Go clears here too).
    const CLEAR: &str = "oauth_state=; Path=/; Max-Age=0; HttpOnly";
    let err = |reason: &str| with_set_cookie(redirect_auth_error(fe, reason), CLEAR);

    let Ok(tok) = state.discord.exchange_code(&q.code).await else {
        return err("discord_unreachable");
    };
    let Ok(du) = state.discord.fetch_user(&tok.access_token).await else {
        return err("discord_unreachable");
    };
    // Member roles drive the web role; tolerate non-members (None) as enlisted.
    let role_ids = state
        .discord
        .fetch_guild_member(&tok.access_token)
        .await
        .ok()
        .flatten()
        .map(|m| m.roles)
        .unwrap_or_default();

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
    .bind(du.avatar_url())
    .execute(&state.pool)
    .await;
    if upsert.is_err() {
        return err("server_error");
    }

    let Ok(role) = services::role_sync::sync_roles(&state.pool, &du.id, &role_ids).await else {
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

    // Reload for current ban + Arma-link state.
    let Ok(Some(fresh)) = load_user(&state.pool, &du.id).await else {
        return err("server_error");
    };
    if fresh.is_banned {
        return err("banned");
    }
    let arma_linked = fresh.arma_id.is_some();

    let Ok((access, exp, refresh)) =
        issue_session(&state, &du.id, role.as_str(), arma_linked).await
    else {
        return err("server_error");
    };

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
        CLEAR,
    )
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
