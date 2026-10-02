use super::{access_token_session_id, same_session, MAX_ACCESS_TOKEN_BYTES};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::json;

const SESSION_ID: &str = "735c12fa-129a-45bc-a789-f49f00a312de";

fn token(payload: &[u8]) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(payload);
    let signature = URL_SAFE_NO_PAD.encode([7u8; 32]);
    format!("{header}.{payload}.{signature}")
}

#[test]
fn session_identity_reads_actual_claim_format_without_authenticating_it() {
    let claims = json!({
        "iss":"tbd-reforger", "aud":"tbd-website", "sub":"123456789",
        "sid":SESSION_ID, "role":"admin", "arma_linked":true,
        "iat":1, "exp":2, "unknown":{"future_claim":true}
    });
    assert_eq!(
        access_token_session_id(&token(claims.to_string().as_bytes())),
        Some(SESSION_ID.into()),
        "expired claims and a fabricated signature still provide untrusted correlation only"
    );
}

#[test]
fn session_identity_normalizes_uppercase_uuid_hex() {
    let claims = json!({"sid": SESSION_ID.to_uppercase()});
    assert_eq!(
        access_token_session_id(&token(claims.to_string().as_bytes())),
        Some(SESSION_ID.into())
    );
}

#[test]
fn session_identity_rejects_absent_null_wrong_type_and_nil_identifiers() {
    for claims in [
        json!({}),
        json!({"sid":null}),
        json!({"sid":false}),
        json!({"sid":123}),
        json!({"sid":[SESSION_ID]}),
        json!({"sid":{"value":SESSION_ID}}),
        json!({"sid":"00000000-0000-0000-0000-000000000000"}),
        json!([]),
        json!(null),
    ] {
        assert_eq!(
            access_token_session_id(&token(claims.to_string().as_bytes())),
            None,
            "{claims}"
        );
    }
}

#[test]
fn session_identity_rejects_noncanonical_uuid_shapes_and_non_ascii_hex() {
    for sid in [
        "",
        "735c12fa129a45bca789f49f00a312de",
        "{735c12fa-129a-45bc-a789-f49f00a312de}",
        "735c12fa_129a-45bc-a789-f49f00a312de",
        "735c12fa-129a-45bc-a789-f49f00a312dg",
        "735c12fa-129a-45bc-a789-f49f00a312d ",
        "735c12fa-129a-45bc-a789-f49f00a312dé",
        "735c12fa-129a-45bc-a789-f49f00a312de ",
    ] {
        let claims = json!({"sid":sid});
        assert_eq!(
            access_token_session_id(&token(claims.to_string().as_bytes())),
            None,
            "{sid:?}"
        );
    }
    for position in 0..36 {
        let mut invalid = SESSION_ID.as_bytes().to_vec();
        invalid[position] = if matches!(position, 8 | 13 | 18 | 23) {
            b'0'
        } else {
            b'-'
        };
        let sid = String::from_utf8(invalid).unwrap();
        let claims = json!({"sid":sid});
        assert_eq!(
            access_token_session_id(&token(claims.to_string().as_bytes())),
            None,
            "position {position}"
        );
    }
}

#[test]
fn session_identity_requires_exactly_three_nonempty_segments_and_valid_payload() {
    let payload = URL_SAFE_NO_PAD.encode(json!({"sid":SESSION_ID}).to_string());
    for malformed in [
        String::new(),
        format!("header.{payload}"),
        format!("header.{payload}.signature.extra"),
        format!(".{payload}.signature"),
        format!("header.{payload}."),
        "header..signature".into(),
        "header.***.signature".into(),
        "header.////.signature".into(),
        "header.e30=.signature".into(),
        token(b"not JSON"),
        token(&[0xff, 0xfe]),
    ] {
        assert_eq!(access_token_session_id(&malformed), None, "{malformed}");
    }
}

#[test]
fn session_identity_enforces_the_whole_token_byte_limit_inclusively() {
    let payload = URL_SAFE_NO_PAD.encode(json!({"sid":SESSION_ID}).to_string());
    let prefix = format!("header.{payload}.");
    let at_limit = format!(
        "{prefix}{}",
        "a".repeat(MAX_ACCESS_TOKEN_BYTES - prefix.len())
    );
    assert_eq!(at_limit.len(), MAX_ACCESS_TOKEN_BYTES);
    assert_eq!(access_token_session_id(&at_limit), Some(SESSION_ID.into()));
    assert_eq!(access_token_session_id(&format!("{at_limit}a")), None);
}

#[test]
fn same_session_requires_two_present_nonempty_equal_identifiers() {
    assert!(same_session(Some(SESSION_ID), Some(SESSION_ID)));
    for pair in [
        (None, None),
        (None, Some(SESSION_ID)),
        (Some(SESSION_ID), None),
        (Some(""), Some("")),
        (Some(""), Some(SESSION_ID)),
        (Some(SESSION_ID), Some("")),
        (Some(SESSION_ID), Some("another-session")),
    ] {
        assert!(!same_session(pair.0, pair.1), "{pair:?}");
    }
}
