//! The access tokens and token-pair answers the gate harness answers `/api/v1/auth/refresh` with.
//!
//! The SPA reads the session an access token belongs to from the token's `sid` claim. A rotation
//! into a token of another session, or into a token naming no session while one is held, ends the
//! current session: the SPA advances its session generation, abandons every request the earlier
//! generation started and forgets the loaded profile. The API's access tokens are JWTs that keep
//! their session's `sid` across rotations, so the harness's tokens do too: unsigned JWT-shaped
//! strings whose payload names [`GATE_SESSION_ID`], with a `sub` label that tells the answering
//! fixture apart in a trace. The SPA refuses a refresh answer whose `token_type` is not `Bearer`,
//! so every canned refresh answer comes from [`gate_refresh_answer`], a complete Bearer pair.
//! [`admin_session_seed_script`] stores a signed-in session in `localStorage` before the app boots,
//! so an auth-gated page opens without a sign-in.
//!
//! **Role:** the unsigned access tokens and Bearer token-pair answers of a token refresh, and the
//! seeded admin session.
//! **Position:** the smokes and the offline mortar gate answer intercepted refresh requests with
//! them.
//! **Signals & state:** none; the seed reads the committed `GET__me.json` fixture per call.
//! **Invariants:** every token names the one gate session id.

use crate::error::ResultExt;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};

/// The one session every harness access token belongs to.
pub const GATE_SESSION_ID: &str = "6a7e5000-0000-4000-8000-00000000a001";

/// An access token of [`GATE_SESSION_ID`] labelled `label`. The signature segment is a fixed
/// placeholder: the SPA never verifies signatures, only the API does, and the harness stands in
/// for the API.
pub fn gate_access_token(label: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload =
        URL_SAFE_NO_PAD.encode(json!({ "sid": GATE_SESSION_ID, "sub": label }).to_string());
    format!("{header}.{payload}.gate-harness")
}

/// A complete `/api/v1/auth/refresh` answer: an access token of [`GATE_SESSION_ID`] labelled
/// `label` (see [`gate_access_token`]), the rotated `refresh_token`, its `expires_at` timestamp
/// and `token_type` `Bearer`, the one token type the SPA accepts.
pub fn gate_refresh_answer(label: &str, refresh_token: &str, expires_at: &str) -> Value {
    json!({
        "access_token": gate_access_token(label),
        "refresh_token": refresh_token,
        "expires_at": expires_at,
        "token_type": "Bearer"
    })
}

/// The `localStorage` script that seeds the `tbd-auth` session with the admin user of the
/// committed `contracts/fixtures/api_goldens/GET__me.json`, injected before the app boots
/// (the editor smokes, `gate render-check --seed-auth` and the doctor's liveness probe).
pub(crate) fn admin_session_seed_script() -> crate::Result<String> {
    let me_path = ::repository_root::find_repository_root()?
        .join("contracts/fixtures/api_goldens/GET__me.json");
    let me: Value =
        serde_json::from_str(&std::fs::read_to_string(&me_path).context("GET__me.json")?)?;
    let inner = serde_json::to_string(&json!({
        "state": {
            "refreshToken": "rt-seed",
            "user": me["user"],
            "expiresAt": "2026-01-01T00:00:00Z"
        },
        "version": 0
    }))?;
    Ok(format!(
        "localStorage.setItem('tbd-auth', {});",
        serde_json::to_string(&inner)?
    ))
}
