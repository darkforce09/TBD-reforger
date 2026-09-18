//! HTTP handlers grouped by domain; the `/api/v1` route tree is assembled in
//! [`crate::core::http_router`].
//!
//! Where a domain directory carries a file of its own name (`telemetry/telemetry.rs`, …) the
//! domain's `mod.rs` glob re-exports it, and the `pub use` façade below restores every other
//! module at a flat path, so `handlers::leaderboards::get_leaderboards` and friends resolve from
//! one place.

pub mod events;
pub mod missions;
pub mod telemetry;

pub use self::missions::{approvals, registry};
pub use self::telemetry::{dashboard, deployments, field_tools, leaderboards};
