//! The cookie-host alignment guard and the CSRF pre-check for the Discord callback.
//!
//! The `oauth_state` cookie is set with no `Domain` attribute, so it is **host-only**: the
//! browser returns it to exactly the host that set it and to nothing else. Everything in this
//! module exists because that one property decides whether a login can possibly succeed.

use axum::http::HeaderMap;
use axum::response::Response;

use crate::core::authentication_primitives;
use crate::core::configuration::Config;
use crate::identity_and_access::services::session_issuance::redirect_auth_error;

use super::discord_oauth::{CallbackQuery, read_cookie, with_set_cookie};

/// `FRONTEND_URL` and `DISCORD_REDIRECT_URL` name different cookie hosts.
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
/// setup. `url::Url` (already a dependency) handles IPv6 brackets, userinfo and
/// case-normalisation, which a hand-rolled split would not.
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
/// **Why this is a real invariant and not a style preference.** `localhost` and `127.0.0.1`
/// are two different hosts to a browser even though they are one machine — there is no "same
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

/// Emitted once per process for the production advisory, so a legitimately split-host
/// deployment gets the warning in its boot logs without an `ERROR` on every single login.
/// Repeating it forever is how operators learn to filter errors out.
static PROD_HOST_MISMATCH_WARNED: std::sync::Once = std::sync::Once::new();

/// Refuse to start a flow that is already guaranteed to fail — in development only.
///
/// **The asymmetry is deliberate.** The invariant is not "these two config values must
/// match"; it is "the host the browser is on when the flow starts must equal the redirect
/// host". In development those are the same thing, because the SPA proxies `/api` and the
/// browser is always on `FRONTEND_URL`. In a split-host production deployment (SPA on
/// `app.example.com`, API behind a proxy on `api.example.com`) the browser navigates to the
/// **API's** own origin to begin the flow, so the cookie is set on and returned to
/// `api.example.com` and the login works perfectly while the two config values disagree.
/// Blocking there would convert a working deployment into an outage. So: refuse where the
/// check is exact, warn where it is a heuristic.
///
/// The enforcement point is the login route rather than boot, because this fires at the exact
/// moment the broken path is exercised, with the cause named in the response itself.
///
/// Returns `Some(finished error redirect)` when the flow must not start.
pub(super) fn reject_login_on_host_mismatch(cfg: &Config) -> Option<Response> {
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
         apps/website/api_v2/.env so both use the SAME host string; ports may differ, hosts may not. \
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

/// Exact `Set-Cookie` value that clears the CSRF `oauth_state` cookie.
///
/// Every callback response — one that has decided the cookie is missing/invalid, and one that
/// has successfully consumed it — must emit this, or the ten-minute cookie stays live for
/// replay. Public so integration tests can assert byte-equality: a soft
/// `contains("Path=/")` passes a wrong `Path=/api`.
pub const OAUTH_STATE_CLEAR: &str = "oauth_state=; Path=/; Max-Age=0; HttpOnly";

/// CSRF pre-check for the Discord callback. `Some(resp)` is a finished error
/// redirect that already clears `oauth_state`; `None` means state matched and
/// the caller may proceed (and must still clear the cookie on every exit).
///
/// `redirect_url` is carried only for the host-mismatch diagnostic on the `invalid_state`
/// branch — it does not affect the decision. Pass `""` to skip the hint.
pub(super) fn callback_csrf_reject(
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
    if cookie_state.is_empty()
        || !authentication_primitives::constant_time_equal(&q.state, &cookie_state)
    {
        // `invalid_state` has two very different causes and reads as only one of them. A
        // MISSING cookie with mismatched config hosts is the config fault; a PRESENT-but-
        // different cookie is the genuine tamper/expiry shape. Say which was observed, so the
        // log does the discrimination the error code cannot.
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
                     apps/website/api_v2/.env."
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

#[cfg(test)]
#[path = "tests/oauth_host_guard.rs"]
mod tests;
