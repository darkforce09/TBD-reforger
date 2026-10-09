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

/// A redirect URL on the same host as `Config::for_tests`'s `frontend_url`, so the
/// CSRF tests exercise the aligned path.
const ALIGNED_REDIRECT: &str = "http://localhost:8080/api/v1/auth/discord/callback";
