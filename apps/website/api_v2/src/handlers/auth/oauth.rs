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
use crate::config::Config;
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

/// `FRONTEND_URL` and `DISCORD_REDIRECT_URL` named different cookie hosts (T-303).
///
/// Carries both values so every diagnostic can name what to change, rather than
/// asserting a mismatch the reader then has to go and find.
#[derive(Debug, PartialEq, Eq)]
struct OauthHostMismatch {
    frontend_host: String,
    redirect_host: String,
}

/// The host a cookie set by this URL would be scoped to — no scheme, no port, no path.
///
/// Ports are deliberately dropped: cookies are **not** port-scoped, so `:3000` vs `:8080`
/// is not a mismatch and flagging it would make the guard cry wolf on every correct dev
/// setup. `url::Url` (already a dependency, used by `services::discord`) handles IPv6
/// brackets, userinfo and case-normalisation, which a hand-rolled split would not.
///
/// `None` for a value that is blank or does not parse. That is fail-open **by design and
/// only here**: a blank `DISCORD_REDIRECT_URL` is the "Discord not configured" state, which
/// `authorize_url` already reports as `oauth_unconfigured` and which `Config::validate`
/// hard-fails in production — claiming "host mismatch" over it would replace an accurate
/// error with a wrong one.
fn cookie_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    url::Url::parse(trimmed)
        .ok()?
        .host_str()
        .map(str::to_ascii_lowercase)
}

/// Compare the two configured hosts. `Some` only when both parse **and** differ.
///
/// **Why this is a real invariant and not a style preference.** The `oauth_state` cookie is
/// set with no `Domain` attribute, so it is *host-only*: the browser will send it back to
/// exactly the host that set it and to nothing else. `localhost` and `127.0.0.1` are two
/// different hosts to a browser even though they are one machine — there is no "same
/// site" relationship between them. In development the SPA on `:3000` proxies `/api` to the
/// API on `:8080`, so the browser is on `FRONTEND_URL`'s host when `discord_login` sets the
/// cookie; Discord then returns it to `DISCORD_REDIRECT_URL`'s host. Different hosts ⇒ the
/// cookie is not sent ⇒ [`callback_csrf_reject`] sees an empty `oauth_state` and answers
/// `invalid_state`, which reads as CSRF tampering rather than as the config fault it is.
fn oauth_host_mismatch(frontend_url: &str, redirect_url: &str) -> Option<OauthHostMismatch> {
    let frontend_host = cookie_host(frontend_url)?;
    let redirect_host = cookie_host(redirect_url)?;
    (frontend_host != redirect_host).then_some(OauthHostMismatch {
        frontend_host,
        redirect_host,
    })
}

/// `GET /api/v1/auth/discord/login` — start the OAuth2 flow.
///
/// @route GET /api/v1/auth/discord/login
pub async fn discord_login(State(state): State<AppState>) -> Response {
    // T-303 — refuse to start a flow that is already guaranteed to fail. See
    // `reject_login_on_host_mismatch` for why this is a refusal in development and only a
    // log line in production.
    if let Some(reject) = reject_login_on_host_mismatch(&state.cfg) {
        return reject;
    }
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

/// Emitted once per process for the production advisory, so a legitimately split-host
/// deployment gets the warning in its boot logs without an `ERROR` on every single login.
/// Repeating it forever is how operators learn to filter errors out.
static PROD_HOST_MISMATCH_WARNED: std::sync::Once = std::sync::Once::new();

/// T-303 — the host-alignment guard, and the deliberate asymmetry in how it is enforced.
///
/// **The ticket asked for a hard refusal to BOOT. This is a refusal to start the FLOW, in
/// development only, and that is a considered downgrade on both axes.** Reasons, in order:
///
/// 1. **A blanket refusal would be wrong, because the mismatch is legitimate in production.**
///    The invariant is not "these two config values must match" — it is "the host the browser
///    is on when the flow starts must equal the redirect host". In dev those are the same
///    thing, because Trunk proxies `/api` and the browser is always on `FRONTEND_URL`. In a
///    split-host production deployment (SPA on `app.example.com`, API behind a proxy on
///    `api.example.com`) the browser navigates to the **API's** own origin to begin the flow,
///    so the cookie is set on and returned to `api.example.com` and the login works perfectly
///    while the two config values disagree. Refusing to boot there would convert a working
///    deployment into an outage — a self-inflicted P0 strictly worse than the bad login this
///    ticket is about. So: refuse where the check is exact, warn where it is a heuristic.
///
/// 2. **The enforcement point is the login route, not `main`.** The check itself belongs to
///    OAuth and lives here; a boot-time call site would have to be added to `bin/api.rs` or
///    `config.rs`, both outside this slice. It is not a meaningful loss: in dev the API is
///    started constantly and nobody reads a clean boot log, whereas this fires at the exact
///    moment the broken path is exercised, with the cause named in the response itself. What
///    a boot refusal would additionally buy is catching it before the first login attempt —
///    reported as a one-line follow-up rather than smuggled in here.
///
/// Returns `Some(finished error redirect)` when the flow must not start.
fn reject_login_on_host_mismatch(cfg: &Config) -> Option<Response> {
    let mismatch = oauth_host_mismatch(&cfg.frontend_url, &cfg.discord_redirect_url)?;
    if !cfg.is_development() {
        PROD_HOST_MISMATCH_WARNED.call_once(|| {
            tracing::warn!(
                frontend_host = %mismatch.frontend_host,
                redirect_host = %mismatch.redirect_host,
                frontend_url = %cfg.frontend_url,
                discord_redirect_url = %cfg.discord_redirect_url,
                "FRONTEND_URL and DISCORD_REDIRECT_URL name different hosts. This is LEGITIMATE \
                 when the SPA and the API are served from different origins (the browser starts \
                 the flow on the API's origin, so the host-only oauth_state cookie still round \
                 trips) — ignore this if that is your topology. If the SPA instead proxies /api \
                 to this server, every login will fail with invalid_state and these two must be \
                 aligned. Logged once per process."
            );
        });
        return None;
    }
    tracing::error!(
        frontend_host = %mismatch.frontend_host,
        redirect_host = %mismatch.redirect_host,
        frontend_url = %cfg.frontend_url,
        discord_redirect_url = %cfg.discord_redirect_url,
        "REFUSING to start the Discord OAuth flow: FRONTEND_URL is on host '{}' but \
         DISCORD_REDIRECT_URL is on host '{}'. In development the SPA proxies /api to this API, \
         so the oauth_state cookie would be set on '{}' and Discord would return the browser to \
         '{}' — a different cookie host, so the cookie is never sent and the login fails \
         'invalid_state' (which looks like CSRF tampering, not a misconfiguration). FIX: edit \
         apps/website/api/.env so both use the SAME host string; ports may differ, hosts may not. \
         'localhost' and '127.0.0.1' are different hosts to a browser. Prefer changing \
         FRONTEND_URL — DISCORD_REDIRECT_URL must stay byte-identical to the Redirect registered \
         in the Discord Developer Portal.",
        mismatch.frontend_host,
        mismatch.redirect_host,
        mismatch.frontend_host,
        mismatch.redirect_host,
    );
    Some(redirect_auth_error(
        &cfg.frontend_url,
        "oauth_host_mismatch",
    ))
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
///
/// `redirect_url` is carried only for the T-303 diagnostic on the `invalid_state`
/// branch — it does not affect the decision. Pass `""` to skip the hint.
fn callback_csrf_reject(
    fe: &str,
    redirect_url: &str,
    q: &CallbackQuery,
    headers: &HeaderMap,
) -> Option<Response> {
    if q.code.is_empty() || q.state.is_empty() {
        return Some(with_set_cookie(
            redirect_auth_error(fe, "missing_code"),
            OAUTH_STATE_CLEAR,
        ));
    }
    let cookie_state = read_cookie(headers, "oauth_state").unwrap_or_default();
    if cookie_state.is_empty() || !auth::constant_time_equal(&q.state, &cookie_state) {
        // T-303 — `invalid_state` has two very different causes and reads as only one of
        // them. A MISSING cookie with mismatched config hosts is the config fault; a
        // PRESENT-but-different cookie is the genuine tamper/expiry shape. Say which was
        // observed, so the log does the discrimination the error code cannot.
        // The state values themselves are never logged: `q.state` is attacker-controlled
        // and `cookie_state` is the live CSRF secret. Only presence and length go out.
        if cookie_state.is_empty() {
            match oauth_host_mismatch(fe, redirect_url) {
                Some(m) => tracing::error!(
                    frontend_host = %m.frontend_host,
                    redirect_host = %m.redirect_host,
                    "invalid_state with NO oauth_state cookie, and FRONTEND_URL/\
                     DISCORD_REDIRECT_URL are on different hosts — this is almost certainly \
                     that misconfiguration, NOT CSRF tampering. The cookie is host-only, so it \
                     was set on one host and never sent to the other. Align the two hosts in \
                     apps/website/api/.env."
                ),
                None => tracing::warn!(
                    "invalid_state with no oauth_state cookie; config hosts agree, so the \
                     likely causes are >10 min at the Discord consent screen (Max-Age=600), \
                     cookies blocked in the browser, or a genuinely forged callback"
                ),
            }
        } else {
            tracing::warn!(
                cookie_state_len = cookie_state.len(),
                "invalid_state: an oauth_state cookie WAS sent but did not match the state \
                 Discord returned — this is the tamper/replay shape, not a config fault"
            );
        }
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
    if let Some(reject) = callback_csrf_reject(fe, &state.cfg.discord_redirect_url, &q, &headers) {
        return reject;
    }
    // State is valid — every response from here clears the cookie (Go clears here too).
    let err = |reason: &str| with_set_cookie(redirect_auth_error(fe, reason), OAUTH_STATE_CLEAR);

    // T-303 — these two used to be `let Ok(..) = .. else`, which DROPPED the error entirely.
    // Both failure modes then emitted the identical `#error=discord_unreachable` and nothing
    // in the log, so a wrong DISCORD_CLIENT_SECRET was indistinguishable from Discord being
    // down. See `log_discord_call_failure` for what makes them distinguishable now.
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

/// T-303 — log a failed Discord call so a **config fault** and a **real outage** are
/// distinguishable, which the shared `#error=discord_unreachable` response code is not.
///
/// **How it discriminates, and why that is not string-matching.** The classification is by
/// *type*, not by parsing another module's message text. Every network failure inside
/// [`services::discord`] surfaces a `reqwest::Error` somewhere in the `anyhow` source chain;
/// a Discord response that arrived and was rejected surfaces `decode_2xx`'s plain
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
///   4. serde decode errors name the missing/!unexpected field and a line/column offset, not
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
/// Left inline it would have been a branch whose only output is a log line — unassertable,
/// therefore unfalsifiable, therefore exactly the kind of check this program is named after.
/// Returns `(fault, guidance)`.
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
             Pre-flight the credential pair with the curl in apps/website/api/.env.example.",
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
        let resp = callback_csrf_reject("http://localhost:5173", ALIGNED_REDIRECT, &q, &headers)
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
        let resp = callback_csrf_reject("http://localhost:5173", ALIGNED_REDIRECT, &q, &headers)
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
        assert!(
            callback_csrf_reject("http://localhost:5173", ALIGNED_REDIRECT, &q, &headers).is_none()
        );
    }

    /* ───────────────── T-303 — OAuth cookie-host alignment ───────────────── */

    /// A redirect URL on the same host as `Config::for_tests`'s `frontend_url`, so the
    /// pre-existing CSRF tests exercise the aligned (non-T-303) path.
    const ALIGNED_REDIRECT: &str = "http://localhost:8080/api/v1/auth/discord/callback";

    /// The committed template, compiled in. Reading the file the repo actually ships is the
    /// whole point — a constant restating the intended values would pass while `.env.example`
    /// said something else, which is exactly how T-303 survived being "known" for days.
    const ENV_EXAMPLE: &str = include_str!("../../../.env.example");

    /// First non-comment `KEY=` assignment in a dotenv-style file.
    fn env_example_value(key: &str) -> String {
        let prefix = format!("{key}=");
        ENV_EXAMPLE
            .lines()
            .map(str::trim)
            .find(|l| l.starts_with(&prefix))
            .unwrap_or_else(|| panic!("{key} must be present in .env.example"))
            .split_once('=')
            .expect("checked by starts_with")
            .1
            .trim()
            .to_string()
    }

    /// **The T-303 regression pin.** The shipped template must not hand a fresh checkout a
    /// configuration whose first live Discord login is guaranteed to fail `invalid_state`.
    ///
    /// This asserts against `.env.example`, never against the operator's gitignored `.env` —
    /// that file was hand-patched on 2026-07-27 to unblock testing, so a live login on this
    /// machine works while every new clone stays broken. Testing the patched file would be a
    /// check that examines the one input that cannot fail.
    #[test]
    fn committed_env_example_uses_one_cookie_host() {
        let frontend = env_example_value("FRONTEND_URL");
        let redirect = env_example_value("DISCORD_REDIRECT_URL");
        assert_eq!(
            oauth_host_mismatch(&frontend, &redirect),
            None,
            ".env.example ships FRONTEND_URL={frontend} and DISCORD_REDIRECT_URL={redirect}. \
             The oauth_state cookie is host-only, so different hosts mean it is set on one and \
             never sent to the other, and the first live login fails 'invalid_state' — which \
             reads as CSRF tampering rather than as this misconfiguration. Ports may differ; \
             hosts may not."
        );
    }

    #[test]
    fn cookie_host_ignores_port_scheme_and_case() {
        // Cookies are not port-scoped, so :3000 and :8080 are the same cookie host.
        assert_eq!(
            cookie_host("http://localhost:3000").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            cookie_host("https://LocalHost:8080/api/v1").as_deref(),
            Some("localhost")
        );
        assert_eq!(
            cookie_host("http://127.0.0.1:3000").as_deref(),
            Some("127.0.0.1")
        );
    }

    #[test]
    fn cookie_host_is_none_for_blank_or_unparseable() {
        // Blank and garbage must NOT be reported as a host mismatch: a blank
        // DISCORD_REDIRECT_URL is the "Discord unconfigured" state, which authorize_url
        // already reports accurately as `oauth_unconfigured`.
        assert_eq!(cookie_host(""), None);
        assert_eq!(cookie_host("   "), None);
        assert_eq!(cookie_host("localhost:3000"), None); // no scheme → not a URL
    }

    #[test]
    fn the_t303_pair_is_a_mismatch_and_the_aligned_pair_is_not() {
        // Verbatim the values `.env.example` shipped on main before this slice.
        let m = oauth_host_mismatch(
            "http://127.0.0.1:3000",
            "http://localhost:8080/api/v1/auth/discord/callback",
        )
        .expect("127.0.0.1 vs localhost are different cookie hosts");
        assert_eq!(m.frontend_host, "127.0.0.1");
        assert_eq!(m.redirect_host, "localhost");
        // Same host, different ports → not a mismatch (cookies ignore the port).
        assert_eq!(
            oauth_host_mismatch("http://localhost:3000", ALIGNED_REDIRECT),
            None
        );
    }

    /* ──── T-303 — telling a bad secret apart from a Discord outage ──── */

    /// A wrong `DISCORD_CLIENT_SECRET` is the case the ticket names, and before this slice it
    /// was logged as nothing at all. Discord answers 401 and `decode_2xx` turns that into a
    /// plain `anyhow::bail!` with no `reqwest::Error` in the chain — so "no transport error"
    /// is the typed signal that Discord *answered and refused*, i.e. a CONFIG fault.
    #[test]
    fn a_rejected_credential_classifies_as_config_not_outage() {
        // Byte-shaped like services::discord::decode_2xx's real bail.
        let e = anyhow::anyhow!(
            "discord: status {}: {}",
            401,
            r#"{"error":"invalid_client"}"#
        );
        let (fault, _) = classify_discord_failure(&e);
        assert_eq!(
            fault, "config",
            "a 401 from Discord must not be reported as an outage — conflating them is the \
             whole defect T-303 exists to fix"
        );
    }

    /// A **real** `reqwest::Error` of the exact type production produces, from a refused
    /// loopback connection — nothing listens on port 1, and no external network is touched.
    /// A hand-built stand-in would be free to diverge from the type the classifier downcasts
    /// to, which is the one thing this must not do.
    ///
    /// The provider install mirrors `services::discord`'s own (discord.rs:23): the crate is
    /// built with reqwest's `rustls-no-provider`, so constructing a `Client` panics without
    /// it. Idempotent — `install_default` returns `Err` when one is already set.
    async fn loopback_transport_error(path: &str) -> reqwest::Error {
        let _ = rustls::crypto::ring::default_provider().install_default();
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:1{path}"))
            .send()
            .await
            .expect_err("nothing listens on loopback port 1")
    }

    /// The other half, and the one that keeps the classifier honest: without it, a
    /// classifier that returned "config" unconditionally would still pass the test above.
    #[tokio::test]
    async fn a_refused_connection_classifies_as_outage_not_config() {
        let e = anyhow::Error::new(loopback_transport_error("/oauth2/token").await);
        let (fault, _) = classify_discord_failure(&e);
        assert_eq!(
            fault, "outage",
            "a transport failure must not be reported as a config fault; the classifier would \
             be vacuous if every input returned the same verdict"
        );
    }

    /// The classification must survive `anyhow`'s `?` wrapping, which is how the error
    /// actually arrives from `retry_429` — a downcast of only the outermost error would
    /// silently relabel every wrapped outage as a config fault.
    #[tokio::test]
    async fn a_wrapped_transport_error_is_still_found_in_the_chain() {
        let e = anyhow::Error::new(loopback_transport_error("/users/@me").await)
            .context("discord: fetching the user profile");
        assert_eq!(classify_discord_failure(&e).0, "outage");
    }

    /* ──── T-584 — the no-secret guarantee, made able to FAIL ──── */

    /// The three values that must never reach a log: the authorization code, the access token
    /// and the client secret. Deliberately distinctive strings — a substring search for them
    /// cannot collide with anything reqwest, anyhow or `tracing` emits on its own.
    const CREDENTIAL_SENTINELS: [&str; 3] = [
        "invalid-authorization-code",
        "invalid-access-token",
        "invalid-client-secret",
    ];
    /// The detector both tests below are asserted with — ONE function, so the positive control
    /// proves the sensitivity of the exact code that returns the verdict.
    fn credential_in(haystack: &str) -> Option<&'static str> {
        CREDENTIAL_SENTINELS
            .into_iter()
            .find(|s| haystack.contains(s))
    }

    /// Everything [`log_discord_call_failure`] actually emits, captured as a `tracing` layer
    /// sees it — the whole event, not just the `error` field.
    ///
    /// This is what makes the guarantee cover the LOG rather than a private re-render of the
    /// error inside the test. The old test asserted over its own `format!("{:#}")`; a log site
    /// that grew a fourth field carrying a credential would not have moved that string at all.
    ///
    /// `with_default` installs the subscriber on THIS THREAD only, and `#[tokio::test]` runs a
    /// current-thread runtime, so there is no global subscriber and no bleed into the other
    /// tests `cargo test` runs in parallel.
    fn capture_call_failure_log(stage: &'static str, e: &anyhow::Error) -> String {
        #[derive(Clone, Default)]
        struct SharedBuf(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl std::io::Write for SharedBuf {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let buf = SharedBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer({
                let b = buf.clone();
                move || b.clone()
            })
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || log_discord_call_failure(stage, e));
        let bytes = buf.0.lock().unwrap().clone();
        String::from_utf8(bytes).expect("tracing writes UTF-8")
    }

    /// **The positive control, and the reason the guarantee below can fail at all.**
    ///
    /// Before T-584 this pair was a single test that asserted three strings were absent from a
    /// haystack they had never been put into. No rendering of `reqwest::Error` could have made
    /// those assertions fail — a "no credential leaked" verdict over an input that never
    /// contained one, which is the shape of defect this program is named after. The stated
    /// positive control (`contains("127.0.0.1:1")`) only ruled out an EMPTY haystack; it said
    /// nothing about whether the detector could see a credential that was really there.
    ///
    /// So: same `loopback_transport_error`, same `reqwest::Error` type, same
    /// `log_discord_call_failure`, same capture, same `credential_in` — the ONE difference is
    /// that the credential is a genuine input, carried in the failing URL's query string.
    /// Measured: reqwest renders the full URL, query included, so this is a real leak path and
    /// not a hypothetical one.
    ///
    /// If reqwest ever stops rendering the URL, or the capture silently stops capturing, or
    /// `credential_in` is weakened, THIS test goes red — and the guarantee below is exposed as
    /// vacuous instead of quietly becoming so.
    #[tokio::test]
    async fn a_credential_in_the_failing_url_does_reach_the_log() {
        for secret in CREDENTIAL_SENTINELS {
            let e = anyhow::Error::new(
                loopback_transport_error(&format!("/oauth2/token?leaked={secret}")).await,
            );
            let logged = capture_call_failure_log("token_exchange", &e);
            assert_eq!(
                credential_in(&logged),
                Some(secret),
                "the detector must SEE a credential that is genuinely in the log line; if it \
                 cannot, the no-leak assertion is vacuous. Captured: {logged}"
            );
        }
    }

    /// The guarantee itself, on the real path. `log_discord_call_failure` is handed only a
    /// stage label and the error — `q.code`, the access token and the client secret are not in
    /// scope at the log site, and the production call carries them in the POST **body**, which
    /// no `reqwest::Error` renders. This asserts that end-to-end over the emitted event.
    #[tokio::test]
    async fn the_logged_call_failure_carries_no_credential() {
        let e = anyhow::Error::new(loopback_transport_error("/oauth2/token").await);
        let logged = capture_call_failure_log("token_exchange", &e);
        assert_eq!(
            credential_in(&logged),
            None,
            "a logged Discord failure must never carry a credential. Captured: {logged}"
        );
        // Still the operator-facing contract: the line has to identify the failed call and its
        // verdict, or it is clean only because it is useless.
        assert!(
            logged.contains("127.0.0.1:1") && logged.contains("outage"),
            "the log line must still name the failed call and its fault class: {logged}"
        );
    }

    fn dev_cfg(frontend: &str, redirect: &str) -> Config {
        let mut cfg = Config::for_tests("postgres://x/x", "t303-secret");
        cfg.frontend_url = frontend.into();
        cfg.discord_redirect_url = redirect.into();
        cfg
    }

    #[test]
    fn development_refuses_to_start_the_flow_on_mismatched_hosts() {
        let cfg = dev_cfg("http://127.0.0.1:3000", ALIGNED_REDIRECT);
        assert!(cfg.is_development());
        let resp = reject_login_on_host_mismatch(&cfg)
            .expect("a dev config guaranteed to fail invalid_state must not start the flow");
        let loc = resp.headers()[header::LOCATION].to_str().unwrap();
        assert!(
            loc.contains("error=oauth_host_mismatch"),
            "the reason must name the cause rather than reuse an existing code — reusing one \
             is how invalid_state came to mean two different things: {loc}"
        );
    }

    #[test]
    fn development_with_aligned_hosts_starts_the_flow() {
        // The no-false-positive half. Without this the guard could be "always refuse",
        // which would pass the test above while breaking every correct setup.
        assert!(
            reject_login_on_host_mismatch(&dev_cfg("http://localhost:3000", ALIGNED_REDIRECT))
                .is_none()
        );
    }

    #[test]
    fn development_with_unconfigured_redirect_still_reaches_oauth_unconfigured() {
        // `Config::for_tests` leaves DISCORD_REDIRECT_URL blank, and tests/oauth_redirect.rs
        // depends on that path answering `oauth_unconfigured`. A blank redirect is not a host
        // mismatch, and this guard must not steal that error.
        let cfg = Config::for_tests("postgres://x/x", "t303-secret");
        assert!(cfg.discord_redirect_url.is_empty());
        assert!(reject_login_on_host_mismatch(&cfg).is_none());
    }

    #[test]
    fn production_split_host_is_allowed_to_boot_and_log_in() {
        // The justification for NOT making this a hard refusal. With the SPA on one origin and
        // the API behind a proxy on another, the browser starts the flow on the API's origin,
        // so the host-only cookie is set on and returned to that same origin and the login
        // works. Refusing here would turn a working deployment into an outage.
        let mut cfg = dev_cfg(
            "https://app.example.com",
            "https://api.example.com/api/v1/auth/discord/callback",
        );
        cfg.env = "production".into();
        assert!(!cfg.is_development());
        assert!(
            oauth_host_mismatch(&cfg.frontend_url, &cfg.discord_redirect_url).is_some(),
            "the hosts really do differ — this test would be vacuous if they did not"
        );
        assert!(
            reject_login_on_host_mismatch(&cfg).is_none(),
            "production must warn, not block: a split-host deployment is legitimate"
        );
    }
}
