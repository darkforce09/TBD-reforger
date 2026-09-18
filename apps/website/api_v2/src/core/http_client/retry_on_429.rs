//! Bounded retry for `429 Too Many Requests`. Retries honor `Retry-After` (fractional seconds),
//! clamped so a hostile rate-limit cannot park a request indefinitely.

use std::time::Duration;

use reqwest::{RequestBuilder, Response, StatusCode};

const MAX_429_ATTEMPTS: u32 = 3;
const DEFAULT_429_BACKOFF: Duration = Duration::from_secs(1);
const MAX_429_BACKOFF: Duration = Duration::from_secs(5);

/// Send `build()`'s request, retrying up to 3 times while the response is 429.
/// The request is rebuilt each attempt (fresh body). The final 429 is returned to
/// the caller, whose normal non-2xx handling surfaces it.
pub async fn send_with_retry_on_429<F>(build: F) -> reqwest::Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    let mut attempt = 1;
    loop {
        let resp = build().send().await?;
        if resp.status() != StatusCode::TOO_MANY_REQUESTS || attempt == MAX_429_ATTEMPTS {
            return Ok(resp);
        }
        let wait = parse_retry_after(
            resp.headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
        );
        attempt += 1;
        tokio::time::sleep(wait).await;
    }
}

/// Convert a `Retry-After` value (seconds, possibly fractional) into a bounded wait.
pub(crate) fn parse_retry_after(v: Option<&str>) -> Duration {
    match v.and_then(|s| s.parse::<f64>().ok()) {
        Some(secs) if secs >= 0.0 => {
            let d = Duration::from_secs_f64(secs);
            if d > MAX_429_BACKOFF {
                MAX_429_BACKOFF
            } else {
                d
            }
        }
        _ => DEFAULT_429_BACKOFF,
    }
}

#[cfg(test)]
#[path = "tests/retry_on_429.rs"]
mod tests;
