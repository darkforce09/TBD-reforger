use axum::http::{HeaderValue, header};

use super::*;

fn set_cookie_values(resp: &Response) -> Vec<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_string))
        .collect()
}

/// Exact equality to `OAUTH_STATE_CLEAR`. A soft
/// `contains("oauth_state=")/Max-Age=0/HttpOnly` passes a wrong `Path=/api`.
fn clears_oauth_state(resp: &Response) -> bool {
    set_cookie_values(resp)
        .iter()
        .any(|c| c.as_str() == OAUTH_STATE_CLEAR)
}

/// `missing_code` must clear the CSRF cookie — an early return that skips
/// `OAUTH_STATE_CLEAR` leaves the ten-minute cookie live for replay.
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

/// `invalid_state` (present query, absent/mismatched cookie) must clear too.
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

/* ───────────────── OAuth cookie-host alignment ───────────────── */

/// A redirect URL on the same host as `Config::for_tests`'s `frontend_url`, so the
/// CSRF tests above exercise the aligned path.
const ALIGNED_REDIRECT: &str = "http://localhost:8080/api/v1/auth/discord/callback";

/// The committed template, compiled in. Reading the file the repo actually ships is the
/// whole point — a constant restating the intended values would pass while `.env.example`
/// said something else.
const ENV_EXAMPLE: &str = include_str!("../../../../.env.example");

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

/// **The template pin.** The shipped configuration must not hand a fresh checkout a
/// setup whose first live Discord login is guaranteed to fail `invalid_state`.
///
/// This asserts against `.env.example`, never against the operator's gitignored `.env` —
/// a hand-patched local file makes a live login work on one machine while every new clone
/// stays broken, so testing it would be a check that examines the one input that cannot
/// fail.
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
fn a_split_host_pair_is_a_mismatch_and_the_aligned_pair_is_not() {
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

fn dev_cfg(frontend: &str, redirect: &str) -> Config {
    let mut cfg = Config::for_tests("postgres://x/x", "host-guard-secret");
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
    let cfg = Config::for_tests("postgres://x/x", "host-guard-secret");
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
