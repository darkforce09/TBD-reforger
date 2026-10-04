//! The rate-limit retry: a `429` answer is waited out and sent again, a bounded number of times.
//!
//! **Role:** decides whether an answer is retried and how long to wait first
//! ([`rate_limit_wait_s`]), and runs that loop around any credential-free send
//! ([`send_with_rate_limit_retry`]).
//! **Position:** the anonymous reads of `super::public_reads` (browser build) and the app's offline
//! saved-copy reads send through it; the sleep is passed in, so the loop compiles and is tested
//! natively.
//! **Signals & state:** none; the attempt counter lives in one call.
//! **Invariants:** at most [`RATE_LIMIT_ATTEMPTS`] sends per call; only a `429` that is not the
//! last attempt waits and retries; the wait is the answer's `Retry-After` whole seconds, or
//! [`DEFAULT_RETRY_AFTER_S`] when absent or unreadable, capped at [`MAX_RETRY_AFTER_S`]; the last
//! answer — a `429` included — goes back to the caller unchanged; a send failure ends the loop at
//! once.

use std::future::Future;

/// How many times one call sends before the last answer goes back to the caller.
pub const RATE_LIMIT_ATTEMPTS: u32 = 3;

/// The wait, in seconds, when a `429` carries no readable `Retry-After`.
pub const DEFAULT_RETRY_AFTER_S: u32 = 2;

/// The longest wait, in seconds, whatever `Retry-After` asks for.
pub const MAX_RETRY_AFTER_S: u32 = 60;

/// The seconds to wait before sending again after the answer with `status` and `retry_after`
/// on the zero-based `attempt`, or `None` when that answer goes back to the caller.
pub fn rate_limit_wait_s(status: u16, attempt: u32, retry_after: Option<&str>) -> Option<u32> {
    if status != 429 || attempt + 1 >= RATE_LIMIT_ATTEMPTS {
        return None;
    }
    let asked = retry_after
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_RETRY_AFTER_S);
    Some(asked.min(MAX_RETRY_AFTER_S))
}

/// Sends through `send` until an answer is not a retried `429`, sleeping through `sleep_s`
/// between attempts; `status_and_retry_after` reads an answer's status and `Retry-After` header.
///
/// Generic over the send's failure type, so each caller keeps its own: the public reads send with
/// the crate's [`crate::Error`].
///
/// # Errors
///
/// The first failure `send` returns.
pub async fn send_with_rate_limit_retry<R, Failure, Send, Sent, Sleep, Slept>(
    mut send: Send,
    status_and_retry_after: impl Fn(&R) -> (u16, Option<String>),
    mut sleep_s: Sleep,
) -> Result<R, Failure>
where
    Send: FnMut() -> Sent,
    Sent: Future<Output = Result<R, Failure>>,
    Sleep: FnMut(u32) -> Slept,
    Slept: Future<Output = ()>,
{
    let mut attempt = 0;
    loop {
        let answer = send().await?;
        let (status, retry_after) = status_and_retry_after(&answer);
        match rate_limit_wait_s(status, attempt, retry_after.as_deref()) {
            Some(wait) => {
                sleep_s(wait).await;
                attempt += 1;
            }
            None => return Ok(answer),
        }
    }
}

#[cfg(test)]
#[path = "tests/rate_limit_retry.rs"]
mod tests;
