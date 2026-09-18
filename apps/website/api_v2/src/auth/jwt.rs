//! HS256 access-token issuance/verification — Rust port of `internal/auth/jwt.go`.
//!
//! The token *string* will not be byte-identical to the Go service's (claim
//! serialization order differs between libraries) — that is the documented
//! non-bit-exact surface #2. What matters and is preserved: HS256, the claim set,
//! signature+expiry validation, and rejection of non-HMAC algorithms.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const ISSUER: &str = "tbd-reforger";
const DEFAULT_TTL_MIN: i64 = 15;

/// Access-token payload: `sub` = Discord ID, plus the cached web role and Arma-link
/// flag. Mirrors Go's `Claims` embedding `jwt.RegisteredClaims`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub role: String,
    pub arma_linked: bool,
    pub sub: String,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
}

/// Signs and verifies HS256 access tokens.
#[derive(Clone)]
pub struct Manager {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    access_ttl: Duration,
}

impl Manager {
    /// Build a Manager with the given secret and access-token TTL (minutes; ≤0 → 15).
    pub fn new(secret: &str, access_ttl_min: i64) -> Self {
        let ttl = if access_ttl_min <= 0 {
            DEFAULT_TTL_MIN
        } else {
            access_ttl_min
        };
        // Go validates only signing method (HMAC) + expiry — not aud/iss.
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_aud = false;
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            validation,
            access_ttl: Duration::minutes(ttl),
        }
    }

    /// Mint a signed access token and return it with its expiry.
    pub fn issue_access(
        &self,
        discord_id: &str,
        role: &str,
        arma_linked: bool,
    ) -> Result<(String, DateTime<Utc>), jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let exp = now + self.access_ttl;
        let claims = Claims {
            role: role.to_string(),
            arma_linked,
            sub: discord_id.to_string(),
            iss: ISSUER.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)?;
        Ok((token, exp))
    }

    /// Verify signature + expiry (rejecting non-HMAC algorithms) and return claims.
    pub fn parse(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        Ok(decode::<Claims>(token, &self.decoding, &self.validation)?.claims)
    }
}

#[cfg(test)]
mod tests {
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
}
