//! Network answers that a saved copy replaces: when a cache-backed request is answered from the
//! cache instead of the network.
//!
//! **Role:** decides, from what the network produced for a cache-backed request, whether the
//! saved copy answers instead ([`prefers_saved_copy`]), and names the header that marks such an
//! answer ([`SAVED_COPY_HEADER`]) and how its `Date` header reads to a person
//! ([`saved_on_from_date_header`]).
//! **Position:** the worker's fetch handler asks [`prefers_saved_copy`] after every network-first
//! fetch and marks the saved copy it answers with; the page's offline core asks the same
//! function after its own catalog and pack reads and reads the marker and the date back.
//! **Signals & state:** none; pure functions.
//! **Invariants:** an unreachable network and every `5xx` status (a proxy whose upstream is down
//! answers `502`, `503` or `504`) prefer the saved copy; every other status, `4xx` included, is
//! the server's real answer and is never masked; an opaque cross-origin answer (status `0`) is
//! not a failure.

/// The header the worker adds to a saved copy it answers with because the network failed; its
/// value is `"1"`. A stored response never carries it, because the worker adds it to the copy it
/// returns, not to the cache entry.
pub const SAVED_COPY_HEADER: &str = "x-served-from-offline-cache";

/// What the network produced for one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAnswer {
    /// A response with this status (`0` for an opaque cross-origin response).
    Status(u16),
    /// No response: the connection failed, was refused or was cut.
    Unreachable,
}

/// Whether `status` is a gateway or server failure (`500` to `599`).
pub fn is_gateway_or_server_failure(status: u16) -> bool {
    (500..=599).contains(&status)
}

/// Whether the saved copy answers in place of `answer`: an unreachable network or a gateway or
/// server failure; a success, a redirect and a `4xx` refusal are passed on unchanged.
pub fn prefers_saved_copy(answer: NetworkAnswer) -> bool {
    match answer {
        NetworkAnswer::Unreachable => true,
        NetworkAnswer::Status(status) => is_gateway_or_server_failure(status),
    }
}

/// The HTTP `Date` header of a saved copy (`Sun, 28 Sep 2026 14:05:09 GMT`), worded for a
/// person as `28 Sep 2026, 14:05 UTC`; `None` when the value is not an IMF-fixdate.
pub fn saved_on_from_date_header(date: &str) -> Option<String> {
    let parts: Vec<&str> = date.split_whitespace().collect();
    let [weekday, day, month, year, time, "GMT"] = parts.as_slice() else {
        return None;
    };
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let clock: Vec<&str> = time.split(':').collect();
    let well_formed = weekday.len() == 4
        && weekday.ends_with(',')
        && day.len() == 2
        && day.bytes().all(|b| b.is_ascii_digit())
        && MONTHS.contains(month)
        && year.len() == 4
        && year.bytes().all(|b| b.is_ascii_digit())
        && clock.len() == 3
        && clock
            .iter()
            .all(|field| field.len() == 2 && field.bytes().all(|b| b.is_ascii_digit()));
    well_formed.then(|| {
        format!(
            "{} {month} {year}, {}:{} UTC",
            day.trim_start_matches('0'),
            clock[0],
            clock[1]
        )
    })
}

#[cfg(test)]
#[path = "tests/network_fallback.rs"]
mod tests;
