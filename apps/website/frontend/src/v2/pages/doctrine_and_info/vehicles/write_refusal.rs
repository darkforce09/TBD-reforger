//! What an administrator is told when a vehicle write is refused.
//!
//! **Role:** words a refused create, replace or delete from the status and body the backend sent.
//! **Position:** read by the form dialog and the delete confirmation after a write fails; the
//! sentence is shown inside the dialog that sent the write.
//! **Signals & state:** none; pure over the refusal.
//! **Invariants:** a `400` shows the backend's own sentence, which names the field and the rule it
//! broke. A `413` (`details.code = request_too_large`, or a proxy's bare `413`), a `403`, a `404`, a
//! `401` and a request that never produced a readable answer each get a fixed sentence, because
//! their bodies say nothing the administrator can act on. Any other status shows the backend's
//! sentence, or the caller's fallback when it sent none.

use crate::v2::core::api::client::ApiRefusal;

/// The `details.code` the backend names an oversized request body with.
const REQUEST_TOO_LARGE_CODE: &str = "request_too_large";

/// The sentence shown for `refusal`; `fallback` is used when the backend sent no sentence of its
/// own for a status that shows one.
pub(super) fn refusal_sentence(refusal: &ApiRefusal, fallback: &str) -> String {
    if refusal.status == 413 || refusal.code() == Some(REQUEST_TOO_LARGE_CODE) {
        return "The vehicle is too large to send. Shorten its fields and try again.".to_owned();
    }
    match refusal.status {
        0 => "The platform could not be reached, or its answer could not be read. Reload the \
              page to see whether the change landed before trying again."
            .to_owned(),
        401 => "Your session has ended. Sign in again, then retry.".to_owned(),
        403 => "Only administrators can change the vehicle database.".to_owned(),
        404 => "This vehicle is no longer in the database. Reload the page to see the current \
                list."
            .to_owned(),
        _ => refusal.message_or(fallback),
    }
}

#[cfg(test)]
#[path = "tests/write_refusal.rs"]
mod tests;
