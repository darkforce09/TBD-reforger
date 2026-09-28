use super::*;

fn claims(token: &str) -> serde_json::Value {
    let segments: Vec<&str> = token.split('.').collect();
    assert_eq!(segments.len(), 3, "a JWT has three segments: {token}");
    assert!(
        segments.iter().all(|segment| !segment.is_empty()),
        "no segment is empty: {token}"
    );
    serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(segments[1])
            .expect("base64url payload"),
    )
    .expect("JSON payload")
}

#[test]
fn every_rotation_names_the_same_gate_session() {
    let first = claims(&gate_access_token("arsenal"));
    let second = claims(&gate_access_token("outliner"));
    assert_eq!(first["sid"], GATE_SESSION_ID);
    assert_eq!(second["sid"], GATE_SESSION_ID);
    assert_eq!(first["sub"], "arsenal");
    assert_ne!(gate_access_token("arsenal"), gate_access_token("outliner"));
}

#[test]
fn the_gate_session_id_is_a_nonnil_hyphenated_uuid() {
    let bytes = GATE_SESSION_ID.as_bytes();
    assert_eq!(bytes.len(), 36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            assert_eq!(*byte, b'-', "hyphen at {index}");
        } else {
            assert!(byte.is_ascii_hexdigit(), "hex digit at {index}");
        }
    }
    assert!(
        GATE_SESSION_ID
            .bytes()
            .any(|byte| byte != b'0' && byte != b'-')
    );
    assert_eq!(GATE_SESSION_ID, GATE_SESSION_ID.to_ascii_lowercase());
}

#[test]
fn the_refresh_answer_is_a_complete_bearer_token_pair() {
    let answer = gate_refresh_answer("arsenal", "rt-seed", "2030-01-01T00:00:00Z");
    let mut keys: Vec<&str> = answer
        .as_object()
        .expect("the refresh answer is a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["access_token", "expires_at", "refresh_token", "token_type"]
    );
    assert_eq!(answer["token_type"], "Bearer");
    assert_eq!(answer["refresh_token"], "rt-seed");
    assert_eq!(answer["expires_at"], "2030-01-01T00:00:00Z");
    let access_token = answer["access_token"]
        .as_str()
        .expect("a string access token");
    assert_eq!(access_token, gate_access_token("arsenal"));
    assert_eq!(claims(access_token)["sid"], GATE_SESSION_ID);
}
