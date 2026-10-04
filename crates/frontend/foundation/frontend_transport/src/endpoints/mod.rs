//! One typed call per backend route of operation access, registration, machine credentials, mission
//! reviews, fleet commands, mission deployments, fleet scenarios and the game-server registry.
//!
//! **Role:** names each route's path, request body and answer in one place, so a page calls
//! `put_event_access_policy(store, event, &change)` rather than assembling a path, a body and a
//! verb of its own.
//! **Position:** between the pages and the request verbs, and built only on those verbs, so every
//! call here shares their bearer injection, single flight and single retry.
//! **Signals & state:** none.
//! **Invariants:** every path builder is pure and compiled into the native build, so the tests hold
//! each one against the backend's route tables; the calls themselves are browser-only, like the
//! verbs under them. A path segment or query value taken from data — a faction or squad name, an
//! account id, a revocation reason, a terrain key, a server id — is percent-encoded, because a
//! squad name may carry a space or a slash. Every call answers the crate's [`Error`](crate::Error)
//! on failure; a change keeps the structured reason its refusal body names, since its callers
//! branch on it.

pub mod event_access_administration;
pub mod event_registration;
pub mod fleet_commands;
pub mod fleet_scenarios;
pub mod machine_credentials;
pub mod mission_deployments;
pub mod mission_reviews;
pub mod server_registry;

/// Percent-encode one path segment or query value: every byte but the RFC 3986 unreserved
/// characters (`A-Z a-z 0-9 - . _ ~`) becomes `%XX`, so the backend's router decodes exactly the
/// text that was encoded.
pub fn encode_path_segment<Segment: AsRef<str> + ?Sized>(segment: &Segment) -> String {
    let segment = segment.as_ref();
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte))
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// A serialisable request body as the JSON value the verbs send.
///
/// The bodies here are plain structs of strings, numbers and lists, which always serialise; the
/// failure arm reports the unreadable answer rather than sending a body nobody built.
#[cfg(target_arch = "wasm32")]
fn json_body<T: serde::Serialize>(body: &T) -> crate::Result<serde_json::Value> {
    serde_json::to_value(body).map_err(|_| crate::Error::Transport)
}

#[cfg(test)]
#[path = "tests/endpoints.rs"]
mod tests;
