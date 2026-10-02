use super::*;

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
fn authorize_url_requires_client_id() {
    let s = DiscordService::new(String::new(), "x".into(), "y".into(), "z".into());
    assert!(s.authorize_url("s").is_err());
}

#[test]
fn retry_after_parsing_and_clamp() {
    assert_eq!(parse_retry_after(Some("2")), Duration::from_secs(2));
    assert_eq!(parse_retry_after(Some("0.5")), Duration::from_millis(500));
    assert_eq!(parse_retry_after(Some("100")), MAX_429_BACKOFF); // clamped
    assert_eq!(parse_retry_after(None), DEFAULT_429_BACKOFF);
    assert_eq!(parse_retry_after(Some("garbage")), DEFAULT_429_BACKOFF);
}

#[test]
fn null_fields_deserialize_to_their_defaults() {
    // Discord sends null for a member with no nickname / a user with no avatar. A null nick
    // that failed the `GuildMember` parse would collapse into empty roles, and the login
    // would then resolve the wrong web role.
    let m: GuildMember =
        serde_json::from_str(r#"{"nick":null,"roles":["1517285898817896559"]}"#).unwrap();
    assert_eq!(m.nick, "");
    assert_eq!(m.roles, ["1517285898817896559"]);

    let u: DiscordUser = serde_json::from_str(
        r#"{"id":"7","username":"sam","global_name":null,"discriminator":"0","avatar":null}"#,
    )
    .unwrap();
    assert_eq!(u.username, "sam");
    assert_eq!(u.global_name, "");
    assert_eq!(u.avatar, "");
    assert_eq!(u.display_name(), "sam");
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
    // `{:#}` walks anyhow's cause chain — reqwest's own Display is just "error decoding
    // response body", and the serde reason we care about sits underneath it.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("missing field `roles`"),
        "the decode error should name the missing field, got: {chain}"
    );
}

#[tokio::test]
async fn a_200_profile_without_a_username_fails_to_decode() {
    // The same shape one struct over. `#[serde(default)]` on `username` would mean a 200
    // carrying anything that merely lacks the field decodes into `username: ""`, and the
    // oauth callback binds `display_name()`/`handle()` — both empty in that state — into
    // `users.username`/`users.discord_handle`. Failing the decode routes it to the Err →
    // `discord_unreachable` → write-nothing path.
    let err = decode_2xx::<DiscordUser>(ok_response(r#"{"id":"7","avatar":"a1"}"#))
        .await
        .expect_err("a 200 profile with no `username` must not decode");
    // `{:#}` walks anyhow's cause chain — reqwest's Display is only "error decoding response
    // body"; the serde reason we care about sits underneath.
    let chain = format!("{err:#}");
    assert!(
        chain.contains("missing field `username`"),
        "the decode error should name the missing field, got: {chain}"
    );
}

#[tokio::test]
async fn a_200_profile_with_a_null_username_fails_to_decode() {
    // Pins the half that was never broken, so a later "let's be tolerant like the other
    // fields" pass cannot quietly reopen it by reaching for `null_default`. `null` is
    // malformed for a user object; it must stay an error, exactly as on `GuildMember::roles`.
    decode_2xx::<DiscordUser>(ok_response(r#"{"id":"7","username":null}"#))
        .await
        .expect_err("an explicit null username must not decode");
}

#[tokio::test]
async fn a_200_profile_with_an_empty_username_decodes() {
    // Absent must fail; explicitly empty must not. Same line `GuildMember` draws between a
    // missing `roles` and `[]` — silence is the bug, a stated value is an answer.
    let u = decode_2xx::<DiscordUser>(ok_response(
        r#"{"id":"7","username":"","global_name":"Dave"}"#,
    ))
    .await
    .expect("an explicit empty username is a stated value");
    assert_eq!(u.username, "");
    assert_eq!(u.display_name(), "Dave");
}

#[tokio::test]
async fn a_200_with_an_empty_roles_array_decodes() {
    // Absent must fail; empty must not. Discord sends `"roles": []` for a real member who
    // holds no roles, and that has to stay a decodable, authoritative answer.
    let m = decode_2xx::<GuildMember>(ok_response(r#"{"nick":"B","roles":[]}"#))
        .await
        .expect("an explicit empty roles array is a valid answer");
    assert!(m.roles.is_empty());
}
