//! Why an access token or the durable rate limiter failed.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** [`crate::authentication_primitives::Manager`] refuses a token it cannot sign or
//! accept with [`Error::AccessToken`]; [`crate::middleware::durable_ratelimit::PgRateLimiter`]
//! reports a bucket table it cannot reach with [`Error::RateLimitStore`]. The outbound retry
//! ([`crate::http_client::retry_on_429`]) hands back `reqwest`'s own error untouched, so the
//! client that called it classifies a transport failure by its own typed error.
//! **Signals & state:** none; plain data.
//! **Invariants:** each variant is the transparent library error, so its message and its causes
//! read exactly as that library renders them; no variant carries a token or a secret.

/// Why an access token or the durable rate limiter failed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An access token could not be signed, or was refused: a bad signature, a non-HMAC
    /// algorithm, a missing or wrong issuer or audience, an expired or not yet valid token, or an
    /// empty subject or session.
    #[error(transparent)]
    AccessToken(#[from] jsonwebtoken::errors::Error),
    /// The durable rate limiter's Postgres bucket table could not be read or written.
    #[error(transparent)]
    RateLimitStore(#[from] sqlx::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
