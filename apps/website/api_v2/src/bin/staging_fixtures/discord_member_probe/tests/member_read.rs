//! Unit tests for what a member read's answer means and the rate-limit headers it reports.

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;

use super::{MemberOutcome, RateLimitHeaders, classify_answer};

#[test]
fn staging_fixtures_member_read_classifies_like_the_api_lookup() {
    assert_eq!(
        classify_answer(200, &json!({"roles": ["1", "2"], "nick": null}), None),
        MemberOutcome::Member {
            roles: vec!["1".to_owned(), "2".to_owned()]
        }
    );
    assert_eq!(
        classify_answer(200, &json!({"roles": []}), None),
        MemberOutcome::Member { roles: Vec::new() }
    );
    assert_eq!(
        classify_answer(
            404,
            &json!({"code": 10007, "message": "Unknown Member"}),
            None
        ),
        MemberOutcome::Nonmember
    );
    for (status, body) in [
        (200, json!({"roles": null})),
        (200, json!({"roles": [1]})),
        (404, json!({"code": 10004, "message": "Unknown Guild"})),
        (401, json!({})),
        (403, json!({})),
        (500, json!(null)),
    ] {
        assert!(
            matches!(
                classify_answer(status, &body, None),
                MemberOutcome::Unavailable { .. }
            ),
            "{status} {body}"
        );
    }
}

#[test]
fn staging_fixtures_member_read_takes_the_retry_delay_from_the_body_then_the_header() {
    assert_eq!(
        classify_answer(
            429,
            &json!({"retry_after": 0.412, "global": false}),
            Some(9000)
        ),
        MemberOutcome::RateLimited {
            retry_after_ms: Some(412),
            global: false
        }
    );
    assert_eq!(
        classify_answer(429, &json!({"global": true}), Some(2000)),
        MemberOutcome::RateLimited {
            retry_after_ms: Some(2000),
            global: true
        }
    );
    assert_eq!(
        classify_answer(429, &json!({"retry_after": -1.0}), None),
        MemberOutcome::RateLimited {
            retry_after_ms: None,
            global: false
        }
    );
}

#[test]
fn staging_fixtures_member_read_reports_the_rate_limit_headers() {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-ratelimit-bucket", "abcd1234"),
        ("x-ratelimit-limit", "5"),
        ("x-ratelimit-remaining", "0"),
        ("x-ratelimit-reset-after", "0.941"),
        ("x-ratelimit-scope", "user"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    assert_eq!(
        RateLimitHeaders::from_headers(&headers),
        RateLimitHeaders {
            bucket: Some("abcd1234".to_owned()),
            limit: Some(5),
            remaining: Some(0),
            reset_after_ms: Some(941),
            scope: Some("user".to_owned()),
        }
    );
    let mut malformed = HeaderMap::new();
    malformed.insert("x-ratelimit-remaining", HeaderValue::from_static("-1"));
    malformed.insert("x-ratelimit-reset-after", HeaderValue::from_static("NaN"));
    assert_eq!(
        RateLimitHeaders::from_headers(&malformed),
        RateLimitHeaders::default()
    );
}
