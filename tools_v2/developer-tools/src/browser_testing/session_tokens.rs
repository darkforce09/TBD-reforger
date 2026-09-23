//! The access tokens the gate harness answers `/api/v1/auth/refresh` with.
//!
//! The SPA reads the session an access token belongs to from the token's `sid` claim. A rotation
//! into a token of another session, or into a token naming no session while one is held, ends the
//! current session: the SPA advances its session generation, abandons every request the earlier
//! generation started and forgets the loaded profile. The API's access tokens are JWTs that keep
//! their session's `sid` across rotations, so the harness's tokens do too: unsigned JWT-shaped
//! strings whose payload names [`GATE_SESSION_ID`], with a `sub` label that tells the answering
//! fixture apart in a trace.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;

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

#[cfg(test)]
#[path = "tests/session_tokens.rs"]
mod tests;
