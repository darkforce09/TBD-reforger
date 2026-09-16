//! Role: how long to wait before re-reading a stored record whose read failed.
//! Position: `editing/persist` in the map engine.
//! Signals & state: none; a pure schedule over an attempt number.
//! Invariants: the delay doubles per attempt, so three attempts span 80 + 160 + 320 ms. Long
//! enough to ride out a transient store failure, short enough that an author waiting on their
//! draft does not notice — and bounded, because a read that keeps failing has to become a visible
//! lockout rather than an endless retry that silently disables saving.

/// The delay before retry `attempt`, in milliseconds. Attempts are 1-based: the first retry waits
/// 80 ms, the second 160 ms, the third 320 ms.
#[must_use]
pub fn backoff_before_attempt_ms(attempt: u8) -> i32 {
    80 * (1_i32 << u32::from(attempt.saturating_sub(1)))
}

#[cfg(test)]
#[path = "tests/record_read_retry.rs"]
mod tests;
