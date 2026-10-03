//! HS256 access-token issuance and verification.
//!
//! Validation covers the signature and the expiry and refuses any non-HMAC algorithm, so a
//! token re-signed with `alg: none` or an asymmetric algorithm cannot be accepted.
//! Issuer, audience, expiry and a nonempty persisted session identity are mandatory.

use api_identifiers::{AuthenticationSessionId, DiscordUserId};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::Result;

const AUDIENCE: &str = "tbd-website";
const ISSUER: &str = "tbd-reforger";
const DEFAULT_TTL_MIN: i64 = 15;

/// Access-token payload: `sub` is the Discord ID, plus the cached web role and the
/// Arma-identity link flag the frontend reads without a round trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// The web role current when the token was issued.
    pub role: String,
    /// True when the account had a linked Arma identity at issue.
    pub arma_linked: bool,
    /// The subject: the member's Discord user id.
    pub sub: DiscordUserId,
    /// The issuer, always `tbd-reforger`.
    pub iss: String,
    /// The audience, always `tbd-website`.
    pub aud: String,
    /// The persisted authentication session the token belongs to.
    pub sid: AuthenticationSessionId,
    /// Issued at, in Unix seconds.
    pub iat: i64,
    /// Expires at, in Unix seconds.
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
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_audience(&[AUDIENCE]);
        validation.leeway = 0;
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
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
        discord_id: &DiscordUserId,
        session_id: AuthenticationSessionId,
        role: &str,
        arma_linked: bool,
    ) -> Result<(String, DateTime<Utc>)> {
        let now = Utc::now();
        let exp = now + self.access_ttl;
        let claims = Claims {
            role: role.to_string(),
            arma_linked,
            sub: discord_id.clone(),
            iss: ISSUER.to_string(),
            aud: AUDIENCE.to_string(),
            sid: session_id,
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)?;
        Ok((token, exp))
    }

    /// Verify signature + expiry (rejecting non-HMAC algorithms) and return claims.
    pub fn parse(&self, token: &str) -> Result<Claims> {
        let claims = decode::<Claims>(token, &self.decoding, &self.validation)?.claims;
        if claims.sid.as_uuid().is_nil()
            || claims.sub.as_str().is_empty()
            || claims.iat > Utc::now().timestamp()
            || claims.iat >= claims.exp
            || claims.exp <= Utc::now().timestamp()
        {
            return Err(jsonwebtoken::errors::Error::from(
                jsonwebtoken::errors::ErrorKind::InvalidToken,
            )
            .into());
        }
        Ok(claims)
    }
}

#[cfg(test)]
#[path = "tests/jwt_manager.rs"]
mod tests;
