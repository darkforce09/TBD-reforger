//! Telemetry domain — game-server match ingest plus its read surfaces, the [`dashboard`] summary
//! and the [`leaderboards`] tables. The same-named `telemetry.rs` is glob re-exported so
//! `handlers::telemetry::*` paths hold.

// Deliberate inception: the domain keeps its same-named root handler file so every
// `handlers::telemetry::…` path resolves through the glob re-export below.
#[allow(clippy::module_inception)]
mod telemetry;
pub use self::telemetry::*;

pub mod dashboard;
pub mod leaderboards;
