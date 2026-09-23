//! Untrusted session correlation for browser concurrency; never credential authentication.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

const MAX_ACCESS_TOKEN_BYTES: usize = 16 * 1024;

/// Read a nonnil UUID session identifier from a bounded JWT-shaped string.
/// This does not verify the signature, algorithm, issuer, audience, expiry, or authority.
/// Only authenticated server responses establish access; this identifier correlates browser work.
pub fn access_token_session_id(token: &str) -> Option<String> {
    if token.len() > MAX_ACCESS_TOKEN_BYTES {
        return None;
    }
    let mut segments = token.split('.');
    let header = segments.next()?;
    let payload = segments.next()?;
    let signature = segments.next()?;
    if header.is_empty() || payload.is_empty() || signature.is_empty() || segments.next().is_some()
    {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let session_id = claims.get("sid")?.as_str()?;
    if session_id.len() != 36 {
        return None;
    }
    let mut nonzero = false;
    for (index, byte) in session_id.bytes().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return None;
            }
        } else {
            if !byte.is_ascii_hexdigit() {
                return None;
            }
            nonzero |= byte != b'0';
        }
    }
    nonzero.then(|| session_id.to_ascii_lowercase())
}

/// Match present correlation identifiers; two absent or empty identifiers never establish a session.
pub fn same_session(current: Option<&str>, expected: Option<&str>) -> bool {
    matches!((current, expected), (Some(current), Some(expected)) if !current.is_empty() && current == expected)
}

#[cfg(test)]
#[path = "tests/session_identity.rs"]
mod tests;
