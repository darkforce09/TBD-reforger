//! HS256 access-token issuance and verification.
//!
//! Validation covers the signature and the expiry and refuses any non-HMAC algorithm, so a
//! token re-signed with `alg: none` or an asymmetric algorithm can never be accepted against
//! the shared secret. Audience and issuer are carried but not enforced.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

const ISSUER: &str = "tbd-reforger";
const DEFAULT_TTL_MIN: i64 = 15;

/// Access-token payload: `sub` is the Discord ID, plus the cached web role and the
/// Arma-identity link flag the frontend reads without a round trip.
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
        // Only the signing method (HMAC) and the expiry are validated — not audience.
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
#[path = "tests/jwt_manager.rs"]
mod tests;
