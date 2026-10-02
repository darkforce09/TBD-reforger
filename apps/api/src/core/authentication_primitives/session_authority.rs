//! Authorization boundary implemented by the identity domain and wired by the composition root.

use super::Claims;
use crate::core::{error_handling::api_error::ApiError, middleware::AuthUser};
use futures::future::BoxFuture;

pub trait SessionAuthority: Send + Sync {
    fn authorize(&self, claims: Claims) -> BoxFuture<'static, Result<AuthUser, ApiError>>;
}
