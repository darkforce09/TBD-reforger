//! The global request middleware chain and the primitives every layer in it shares.
//!
//! Chain order, outermost first: correlation id → access logging → panic recovery → CORS →
//! body limit → rate limiting. Authentication is deliberately **not** a layer: it is expressed
//! as axum extractors ([`AuthUser`], the role-gated newtypes, [`ServiceAuth`]), so the tier a
//! route requires travels with the handler rather than with a route group.

pub mod authentication;
pub mod client_identity;
pub mod cross_origin;
pub mod durable_ratelimit;
pub mod rate_limiting;
pub mod tracing_correlation;

pub use authentication::{AdminUser, AuthUser, LeaderUser, MissionMakerUser, ServiceAuth};
pub use cross_origin::cors;
pub use rate_limiting::{
    DURABLE_STRICT_BURST, DURABLE_STRICT_RPS, DURABLE_STRICT_SCOPE, IpLimiter,
    RATE_LIMIT_EXEMPT_MOUNT, RateLimitState, STRICT_PREFIXES, rate_limit,
};
pub use tracing_correlation::{logging, request_id};

use axum::Json;
use axum::http::StatusCode;
use serde_json::json;

/// Default JSON body cap (1 MB).
pub const MAX_JSON_BODY: usize = 1 << 20;
/// Multipart upload cap (6 MB); the per-file 5 MB cap is enforced in-handler.
pub const MAX_MULTIPART_BODY: usize = 6 << 20;

/// Numeric role ordering — a higher role satisfies a lower requirement.
///
/// `mission_maker` outranks `leader`: mission authorship is the broader grant on this platform,
/// and the ordering is deliberate rather than alphabetical.
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
