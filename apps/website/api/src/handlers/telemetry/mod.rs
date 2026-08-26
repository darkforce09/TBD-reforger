//! Telemetry domain — game-server ingest plus its read surfaces ([`servers`],
//! [`leaderboards`], [`dashboard`], [`deployments`]) and [`field_tools`]. T-934.15:
//! the flat files moved here unchanged; the same-named `telemetry.rs` is glob
//! re-exported so `handlers::telemetry::*` paths hold.

mod telemetry;
pub use self::telemetry::*;

pub mod dashboard;
pub mod deployments;
pub mod field_tools;
pub mod leaderboards;
pub mod servers;
