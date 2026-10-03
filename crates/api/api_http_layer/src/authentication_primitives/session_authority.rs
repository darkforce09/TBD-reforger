//! Authorization boundary implemented by the identity domain and wired by the composition root.

use super::Claims;
use crate::middleware::AuthUser;
use api_foundation::error_handling::api_error::ApiError;
use futures::future::BoxFuture;

/// Turns the verified claims of a bearer token into the caller's current identity.
///
/// The token proves only who signed in and when; the implementation reads the persisted session
/// and the account, so a revoked session, a deleted or banned account or a changed role takes
/// effect on the next request rather than at token expiry.
pub trait SessionAuthority: Send + Sync {
    /// The caller behind `claims`, or the [`ApiError`] that refuses the request (401 for an
    /// expired or revoked session, 403 for a banned account).
    fn authorize(&self, claims: Claims) -> BoxFuture<'static, Result<AuthUser, ApiError>>;
}
