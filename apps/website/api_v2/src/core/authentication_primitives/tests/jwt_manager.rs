use super::*;

#[test]
fn issue_and_parse_round_trip() {
    let m = Manager::new("secret", 15);
    let (tok, exp) = m.issue_access("123", "admin", true).unwrap();
    let c = m.parse(&tok).unwrap();
    assert_eq!(c.sub, "123");
    assert_eq!(c.role, "admin");
    assert!(c.arma_linked);
    assert_eq!(c.iss, ISSUER);
    assert!(exp > Utc::now());
}

#[test]
fn parse_rejects_wrong_secret() {
    let (tok, _) = Manager::new("secret-a", 15)
        .issue_access("1", "enlisted", false)
        .unwrap();
    assert!(Manager::new("secret-b", 15).parse(&tok).is_err());
}

#[test]
fn parse_rejects_expired() {
    let m = Manager::new("secret", 15);
    let past = Utc::now().timestamp() - 3600;
    let claims = Claims {
        role: "enlisted".into(),
        arma_linked: false,
        sub: "1".into(),
        iss: ISSUER.into(),
        iat: past - 60,
        exp: past,
    };
    let tok = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"secret"),
    )
    .unwrap();
    assert!(m.parse(&tok).is_err(), "expired token must be rejected");
}
