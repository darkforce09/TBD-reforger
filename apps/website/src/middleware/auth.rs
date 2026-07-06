//! Auth as axum extractors — Rust port of `auth.go` + `authz.go`.
//!
//! [`AuthUser`] = `RequireAuth` (Bearer JWT → identity). The role-gated newtypes
//! ([`LeaderUser`], [`MissionMakerUser`], [`AdminUser`]) = `RequireMinRole`.
//! [`ServiceAuth`] = `RequireServiceToken` (constant-time `X-Service-Token`).

use std::sync::Arc;

use axum::Json;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::header;
use axum::http::request::Parts;

use crate::auth::{Manager, constant_time_equal};
use crate::config::Config;
use crate::middleware::{json_error, role_rank};

type Rejection = (StatusCode, Json<serde_json::Value>);

/// A validated bearer identity (mirrors the `discord_id`/`role`/`arma_linked`
/// context keys set by Go's `RequireAuth`).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub discord_id: String,
    pub role: String,
    pub arma_linked: bool,
}

impl<S> FromRequestParts<S> for AuthUser
where
    Arc<Manager>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let Some(token) = header.strip_prefix("Bearer ").map(str::trim) else {
            return Err(json_error(StatusCode::UNAUTHORIZED, "missing bearer token"));
        };
        let jm = Arc::<Manager>::from_ref(state);
        match jm.parse(token) {
            Ok(claims) => Ok(AuthUser {
                discord_id: claims.sub,
                role: claims.role,
                arma_linked: claims.arma_linked,
            }),
            Err(_) => Err(json_error(
                StatusCode::UNAUTHORIZED,
                "invalid or expired token",
            )),
        }
    }
}

/// Build a role-gated extractor newtype requiring at least `$min`.
macro_rules! role_gate {
    ($name:ident, $min:literal) => {
        #[doc = concat!("`RequireMinRole(\"", $min, "\")` — wraps an authenticated ", $min, "+ user.")]
        #[derive(Debug, Clone)]
        pub struct $name(pub AuthUser);

        impl<S> FromRequestParts<S> for $name
        where
            Arc<Manager>: FromRef<S>,
            S: Send + Sync,
        {
            type Rejection = Rejection;

            async fn from_request_parts(
                parts: &mut Parts,
                state: &S,
            ) -> Result<Self, Self::Rejection> {
                let user = AuthUser::from_request_parts(parts, state).await?;
                if role_rank(&user.role) >= role_rank($min) {
                    Ok($name(user))
                } else {
                    Err(json_error(StatusCode::FORBIDDEN, "insufficient role"))
                }
            }
        }
    };
}

role_gate!(LeaderUser, "leader");
role_gate!(MissionMakerUser, "mission_maker");
role_gate!(AdminUser, "admin");

/// `RequireServiceToken` — game-server ingest guarded by a shared secret in the
/// `X-Service-Token` header, compared in constant time.
#[derive(Debug, Clone, Copy)]
pub struct ServiceAuth;

impl<S> FromRequestParts<S> for ServiceAuth
where
    Arc<Config>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cfg = Arc::<Config>::from_ref(state);
        let got = parts
            .headers
            .get("x-service-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if cfg.service_token.is_empty() || !constant_time_equal(got, &cfg.service_token) {
            return Err(json_error(
                StatusCode::UNAUTHORIZED,
                "invalid service token",
            ));
        }
        Ok(ServiceAuth)
    }
}
