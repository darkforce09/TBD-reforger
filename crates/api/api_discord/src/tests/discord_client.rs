use super::*;
use api_foundation::error_handling::error_causes::message_with_causes;

#[test]
fn authorize_url_has_params() {
    let s = DiscordService::new(
        "cid".into(),
        "sec".into(),
        "https://app/cb".into(),
        "g1".into(),
    );
    let u = s.authorize_url("st8").unwrap();
    assert!(u.contains("client_id=cid"), "{u}");
    assert!(u.contains("response_type=code"));
    assert!(u.contains("state=st8"));
    assert!(u.contains("scope=identify"));
}

#[test]
fn retry_after_parsing_and_clamp() {
    assert_eq!(parse_retry_after(Some("2")), Duration::from_secs(2));
    assert_eq!(parse_retry_after(Some("0.5")), Duration::from_millis(500));
    assert_eq!(parse_retry_after(Some("100")), MAX_429_BACKOFF); // clamped
    assert_eq!(parse_retry_after(None), DEFAULT_429_BACKOFF);
    assert_eq!(parse_retry_after(Some("garbage")), DEFAULT_429_BACKOFF);
}

/// Wrap a body in a real 200 `reqwest::Response` so the assertions below run through the
/// exact `decode_2xx` call production uses, not a stand-in `serde_json::from_str`.
fn ok_response(body: &'static str) -> Response {
    Response::from(
        axum::http::Response::builder()
            .status(200)
            .body(body)
            .expect("build 200 response"),
    )
}

#[tokio::test]
async fn a_200_without_roles_fails_to_decode() {
    // `#[serde(default)]` on `roles` would mean a 200 carrying anything that simply lacks the
    // field — a proxy's JSON error envelope, a truncated gateway response — decodes happily
    // into `roles: []`. The caller cannot tell that apart from Discord saying "no roles", so
    // it demotes the user and DELETEs the stored snapshot that `resync_all_roles` would have
    // restored from. Failing the decode is what routes it to the Err → Unavailable →
    // write-nothing path instead.
    let err = decode_2xx::<GuildMember>(ok_response(r#"{"code":0,"message":"502 Bad Gateway"}"#))
        .await
        .expect_err("a 200 body with no `roles` field must not decode");
    // The cause chain — reqwest's own Display is just "error decoding response body", and the
    // serde reason we care about sits underneath it.
    let chain = message_with_causes(&err);
    assert!(
        chain.contains("missing field `roles`"),
        "the decode error should name the missing field, got: {chain}"
    );
}
