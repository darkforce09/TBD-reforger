//! Unit coverage for the `Retry-After` parser: the accepted spellings, the clamp, and the
//! default every unusable value falls back to.

use super::*;

#[test]
fn retry_after_parsing_and_clamp() {
    assert_eq!(parse_retry_after(Some("2")), Duration::from_secs(2));
    assert_eq!(parse_retry_after(Some("0.5")), Duration::from_millis(500));
    assert_eq!(parse_retry_after(Some("100")), MAX_429_BACKOFF); // clamped
    assert_eq!(parse_retry_after(None), DEFAULT_429_BACKOFF);
    assert_eq!(parse_retry_after(Some("garbage")), DEFAULT_429_BACKOFF);
}
