//! Telemetry domain — game-server ingest plus its read surfaces ([`servers`],
//! [`leaderboards`], [`dashboard`], [`deployments`]) and [`field_tools`]. T-934.15:
//! the flat files moved here unchanged; the same-named `telemetry.rs` is glob
//! re-exported so `handlers::telemetry::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// pre-T-934.15 `handlers::telemetry::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod telemetry;
pub use self::telemetry::*;

pub mod dashboard;
pub mod deployments;
pub mod field_tools;
pub mod leaderboards;
pub mod servers;
