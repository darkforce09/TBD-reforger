//! HTTP handlers for the domains that have not yet moved to their own top-level module; the
//! `/api/v1` route tree is assembled in [`crate::core::http_router`].
//!
//! One domain lives here: [`telemetry`], the dashboard and leaderboard read surfaces. The
//! `pub use` façade below restores its modules at a flat path, so
//! `handlers::leaderboards::get_leaderboards` and friends resolve from one place.

pub mod telemetry;

pub use self::telemetry::{dashboard, leaderboards};
