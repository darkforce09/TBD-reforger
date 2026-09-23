use super::*;
use proptest::prelude::*;
use serde_json::{Value, json};
use uuid::Uuid;

const SECRET: &str = "jwt-validation-test-secret";

fn valid_claims() -> Claims {
    let now = Utc::now().timestamp();
    Claims {
        role: "enlisted".into(),
        arma_linked: false,
        sub: "123".into(),
        iss: ISSUER.into(),
        aud: AUDIENCE.into(),
        sid: Uuid::from_u128(1),
        iat: now - 60,
        exp: now + 3600,
    }
}

fn sign<T: Serialize>(claims: &T) -> String {
    encode(
        &Header::new(Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

#[test]
fn issue_and_parse_round_trip() {
    let manager = Manager::new(SECRET, 15);
    let session_id = Uuid::from_u128(123);
    let (token, expiry) = manager
        .issue_access("123", session_id, "admin", true)
        .unwrap();
    let claims = manager.parse(&token).unwrap();
    assert_eq!(claims.sub, "123");
    assert_eq!(claims.sid, session_id);
    assert_eq!(claims.role, "admin");
    assert!(claims.arma_linked);
    assert_eq!(claims.iss, "tbd-reforger");
    assert_eq!(claims.aud, "tbd-website");
    assert_eq!(claims.exp, expiry.timestamp());
    assert_eq!(claims.exp - claims.iat, 15 * 60);
    assert!(expiry > Utc::now());
}

#[test]
fn parse_rejects_wrong_secret() {
    let (token, _) = Manager::new("secret-a", 15)
        .issue_access("1", Uuid::from_u128(1), "enlisted", false)
        .unwrap();
    assert!(Manager::new("secret-b", 15).parse(&token).is_err());
}

#[test]
fn parse_rejects_expired_and_exact_expiry_boundary() {
    let manager = Manager::new(SECRET, 15);
    for seconds_past_expiry in [0, 1, 59, 3600] {
        let mut claims = valid_claims();
        claims.exp = Utc::now().timestamp() - seconds_past_expiry;
        claims.iat = claims.exp - 60;
        assert!(
            manager.parse(&sign(&claims)).is_err(),
            "expiry is exclusive with zero leeway: {seconds_past_expiry}s past expiry"
        );
    }
}

#[test]
fn parse_rejects_invalid_issuance_timeline() {
    let manager = Manager::new(SECRET, 15);
    let now = Utc::now().timestamp();
    for (issued_at, expires_at) in [
        (now + 3600, now + 7200),
        (now, now),
        (now, now - 1),
        (i64::MAX - 1, i64::MAX),
    ] {
        let mut claims = valid_claims();
        claims.iat = issued_at;
        claims.exp = expires_at;
        assert!(manager.parse(&sign(&claims)).is_err());
    }
}

#[test]
fn parse_rejects_missing_or_null_required_claims() {
    let manager = Manager::new(SECRET, 15);
    for field in [
        "sub",
        "iss",
        "aud",
        "sid",
        "iat",
        "exp",
        "role",
        "arma_linked",
    ] {
        let mut missing = serde_json::to_value(valid_claims()).unwrap();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            manager.parse(&sign(&missing)).is_err(),
            "missing {field} must be rejected"
        );

        let mut null = serde_json::to_value(valid_claims()).unwrap();
        null[field] = Value::Null;
        assert!(
            manager.parse(&sign(&null)).is_err(),
            "null {field} must be rejected"
        );
    }
}

#[test]
fn parse_rejects_empty_identity_and_malformed_session_id() {
    let manager = Manager::new(SECRET, 15);
    let mut empty_subject = valid_claims();
    empty_subject.sub.clear();
    assert!(manager.parse(&sign(&empty_subject)).is_err());

    for invalid_session in [
        json!(Uuid::nil()),
        json!("not-a-uuid"),
        json!(""),
        json!(123),
    ] {
        let mut claims = serde_json::to_value(valid_claims()).unwrap();
        claims["sid"] = invalid_session;
        assert!(manager.parse(&sign(&claims)).is_err());
    }
}

#[test]
fn parse_rejects_other_hmac_algorithms_and_unsigned_tokens() {
    let manager = Manager::new(SECRET, 15);
    let claims = valid_claims();
    for algorithm in [Algorithm::HS384, Algorithm::HS512] {
        let token = encode(
            &Header::new(algorithm),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        assert!(manager.parse(&token).is_err());
    }

    let signed = sign(&claims);
    let payload = signed.split('.').nth(1).unwrap();
    // The header is base64url({"alg":"none"}); the signature is absent.
    let unsigned = format!("eyJhbGciOiJub25lIn0.{payload}.");
    assert!(manager.parse(&unsigned).is_err());
}

#[test]
fn parse_accepts_distinct_nonempty_session_identifiers_without_collapsing_them() {
    let manager = Manager::new(SECRET, 15);
    for session_id in [Uuid::from_u128(1), Uuid::from_u128(u128::MAX)] {
        let (token, _) = manager
            .issue_access("123", session_id, "guest", false)
            .unwrap();
        let claims = manager.parse(&token).unwrap();
        assert_eq!(claims.sid, session_id);
        assert_eq!(claims.sub, "123");
        assert_eq!(claims.role, "guest");
    }
}

proptest! {
    #[test]
    fn jwt_issuer_or_audience_mutation_always_rejects(
        suffix in "[a-zA-Z0-9_-]{0,64}",
        mutate_issuer in any::<bool>(),
    ) {
        let mut claims = valid_claims();
        let incorrect = format!("untrusted-{suffix}");
        if mutate_issuer {
            claims.iss = incorrect;
        } else {
            claims.aud = incorrect;
        }
        prop_assert!(Manager::new(SECRET, 15).parse(&sign(&claims)).is_err());
    }

    #[test]
    fn jwt_round_trip_preserves_session_and_subject(
        session_bits in 1_u128..=u128::MAX,
        subject in "[0-9]{1,20}",
        linked in any::<bool>(),
    ) {
        let manager = Manager::new(SECRET, 15);
        let session_id = Uuid::from_u128(session_bits);
        let (token, _) = manager.issue_access(&subject, session_id, "guest", linked).unwrap();
        let parsed = manager.parse(&token).unwrap();
        prop_assert_eq!(parsed.sid, session_id);
        prop_assert_eq!(parsed.sub, subject);
        prop_assert_eq!(parsed.arma_linked, linked);
    }

    #[test]
    fn jwt_expiry_mutation_cannot_use_clock_leeway(seconds_expired in 0_i64..31_536_000_i64) {
        let mut claims = valid_claims();
        claims.exp = Utc::now().timestamp() - seconds_expired;
        claims.iat = claims.exp - 60;
        prop_assert!(Manager::new(SECRET, 15).parse(&sign(&claims)).is_err());
    }
}
