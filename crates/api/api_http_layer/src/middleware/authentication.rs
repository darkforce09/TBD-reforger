//! Authentication and role authorization, expressed as axum extractors.
//!
//! [`AuthUser`] validates a Bearer JWT into an identity. The role-gated newtypes
//! ([`LeaderUser`], [`MissionMakerUser`], [`AdminUser`]) additionally require a minimum rank.

use api_identifiers::DiscordUserId;
use std::sync::Arc;

use crate::authentication_primitives::session_authority::SessionAuthority;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::header;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use crate::authentication_primitives::{Claims, Manager};
use crate::middleware::{json_error, role_rank};
use api_configuration::configuration::Config;

/// A validated bearer identity: the Discord id, the role it carries, and whether the account has
/// a linked Arma identity.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// The member's Discord user id, the token's `sub`.
    pub discord_id: DiscordUserId,
    /// The effective web role the session authority answered with (`guest` to `admin`).
    pub role: String,
    /// True when the account has a linked Arma identity.
    pub arma_linked: bool,
    /// The verified claims of the bearer token, its session id and expiry among them.
    pub session_claims: Claims,
    /// True when the Discord membership snapshot behind the role is older than its freshness
    /// window.
    pub membership_stale: bool,
    /// True when a membership grace override currently decides the member's role.
    pub membership_override_active: bool,
    /// True when the member may grant or revoke membership grace overrides.
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
