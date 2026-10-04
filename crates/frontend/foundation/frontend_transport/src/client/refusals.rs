//! The refusal reader: a backend answer kept whole, decoded into its data or its refusal.
//!
//! **Role:** decodes the answer of the refusal-keeping verbs — a 2xx body into the asked-for type,
//! any other status into the [`Error`] carrying the `details` object the refusal body names its
//! reason in — for the routes whose callers branch on that reason: a registration refused by an
//! access policy is told apart from one refused because the operation is full, and a stale access
//! revision from any other conflict.
//! **Position:** the failure half of the refusal-keeping verbs in the request module. A page reads
//! the reason through [`Error::code`] and [`Error::detail`], and the sentence through
//! [`Error::message_or`].
//! **Signals & state:** none. Every function is pure over its arguments.
//! **Invariants:** the status keeps the meanings [`Error::from_status`] gives it — `0` for a
//! request that never reached the backend or an answer that could not be read, `401` for a session
//! that is over. Only an object `details` is kept as the reason; an array of findings is folded into
//! the message instead, exactly as [`super::error_body_message`] folds it for every other request,
//! so both kinds of refusal still read as prose.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{Error, Result};

/// Decode an answer the backend gave — its status, and its body when that parsed as JSON.
///
/// A 2xx body becomes `T`; one that does not deserialise into `T` is the unreadable answer the
/// other verbs report as [`Error::Transport`]. Any other status becomes the refusal its body
/// describes.
///
/// # Errors
///
/// [`Error::Transport`] for an unreadable 2xx body, and [`Error::from_error_body`] for any other
/// status.
pub fn decode_answer<T: DeserializeOwned>(status: u16, body: Option<Value>) -> Result<T> {
    if (200..300).contains(&status) {
        body.and_then(|body| serde_json::from_value(body).ok())
            .ok_or(Error::Transport)
    } else {
        Err(Error::from_error_body(status, body.as_ref()))
    }
}

#[cfg(test)]
#[path = "tests/refusals.rs"]
mod tests;
