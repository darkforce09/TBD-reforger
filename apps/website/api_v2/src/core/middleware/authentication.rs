//! Authentication and role authorization, expressed as axum extractors.
//!
//! [`AuthUser`] validates a Bearer JWT into an identity. The role-gated newtypes
//! ([`LeaderUser`], [`MissionMakerUser`], [`AdminUser`]) additionally require a minimum rank.
//! [`ServiceAuth`] guards game-server ingest with a constant-time `X-Service-Token` comparison.

use std::sync::Arc;

use crate::core::authentication_primitives::session_authority::SessionAuthority;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::header;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use crate::core::authentication_primitives::{Claims, Manager, constant_time_equal};
use crate::core::configuration::Config;
use crate::core::middleware::{json_error, role_rank};

/// A validated bearer identity: the Discord id, the role it carries, and whether the account has
/// a linked Arma identity.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub discord_id: String,
    pub role: String,
    pub arma_linked: bool,
    pub session_claims: Claims,
    pub membership_stale: bool,
    pub membership_override_active: bool,
    pub can_manage_membership_override: bool,
}

impl<S> FromRequestParts<S> for AuthUser
where
    Arc<Manager>: FromRef<S>,
    Arc<Config>: FromRef<S>,
    Arc<dyn SessionAuthority>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let Some(token) = header.strip_prefix("Bearer ").map(str::trim) else {
            return Err(
                json_error(StatusCode::UNAUTHORIZED, "missing bearer token").into_response()
            );
        };
        let jm = Arc::<Manager>::from_ref(state);
        let claims = jm.parse(token).map_err(|_| {
            json_error(StatusCode::UNAUTHORIZED, "invalid or expired token").into_response()
        })?;
        Arc::<dyn SessionAuthority>::from_ref(state)
            .authorize(claims)
            .await
            .map_err(IntoResponse::into_response)
    }
}

/// Build a role-gated extractor newtype requiring at least `$min`.
macro_rules! role_gate {
    ($name:ident, $min:literal) => {
        #[doc = concat!("An authenticated user of rank ", $min, " or higher.")]
        #[derive(Debug, Clone)]
        pub struct $name(pub AuthUser);

        impl<S> FromRequestParts<S> for $name
        where
            Arc<Manager>: FromRef<S>,
            Arc<Config>: FromRef<S>,
            Arc<dyn SessionAuthority>: FromRef<S>,
            S: Send + Sync,
        {
            type Rejection = Response;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let user = AuthUser::from_request_parts(parts, state).await?;
                if role_rank(&user.role) >= role_rank($min) {
                    Ok($name(user))
                } else {
                    Err(json_error(StatusCode::FORBIDDEN, "insufficient role").into_response())
                }
            }
        }
    };
}

role_gate!(LeaderUser, "leader");
role_gate!(MissionMakerUser, "mission_maker");
role_gate!(AdminUser, "admin");

/// Game-server ingest guarded by a shared secret in the `X-Service-Token` header, compared in
/// constant time. An empty configured token refuses every request rather than accepting one.
#[derive(Debug, Clone, Copy)]
pub struct ServiceAuth;

impl<S> FromRequestParts<S> for ServiceAuth
where
    Arc<Config>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cfg = Arc::<Config>::from_ref(state);
        let got = parts
            .headers
            .get("x-service-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if cfg.service_token.is_empty() || !constant_time_equal(got, &cfg.service_token) {
            return Err(
                json_error(StatusCode::UNAUTHORIZED, "invalid service token").into_response(),
            );
        }
        Ok(ServiceAuth)
    }
}
