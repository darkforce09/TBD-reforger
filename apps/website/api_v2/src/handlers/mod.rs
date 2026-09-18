//! HTTP handlers for the domains that have not yet moved to their own top-level module; the
//! `/api/v1` route tree is assembled in [`crate::core::http_router`].
//!
//! One domain lives here: [`telemetry`]. Because its directory carries a file of its own name
//! (`telemetry/telemetry.rs`), the domain's `mod.rs` glob re-exports it, and the `pub use`
//! façade below restores every other module at a flat path, so
//! `handlers::leaderboards::get_leaderboards` and friends resolve from one place.

pub mod telemetry;

pub use self::telemetry::{dashboard, deployments, field_tools, leaderboards};
