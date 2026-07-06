//! HTTP middleware — Rust port of `internal/middleware`.
//!
//! The global chain (applied in `bin/api.rs`, outermost first) mirrors Go:
//! request-id → logging → recovery → CORS → body-limit → rate-limit. Auth is
//! expressed as axum extractors ([`AuthUser`], the role-gated newtypes,
//! [`ServiceAuth`]) rather than route-group layers.

pub mod auth;
pub mod cors;
pub mod ratelimit;
pub mod request_id;

pub use auth::{AdminUser, AuthUser, LeaderUser, MissionMakerUser, ServiceAuth};
pub use cors::cors;
pub use ratelimit::{IpLimiter, rate_limit};
pub use request_id::{logging, request_id};

use axum::Json;
use axum::http::StatusCode;
use serde_json::json;

/// Default JSON body cap (1 MB), mirroring `MaxJSONBody`.
pub const MAX_JSON_BODY: usize = 1 << 20;
/// Multipart upload cap (6 MB), mirroring `MaxMultipartBody` (per-file 5 MB in-handler).
pub const MAX_MULTIPART_BODY: usize = 6 << 20;

/// Numeric role ordering — a higher role satisfies a lower requirement. Mirrors
/// `authz.go` `roleRank` (mission_maker outranks leader; preserved deliberately).
pub fn role_rank(role: &str) -> i32 {
    match role {
        "admin" => 4,
        "mission_maker" => 3,
        "leader" => 2,
        "enlisted" => 1,
        _ => 0,
    }
}

/// The canonical error envelope `{"error": msg}` used by rejections and handlers.
pub fn json_error(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(json!({ "error": msg })))
}
