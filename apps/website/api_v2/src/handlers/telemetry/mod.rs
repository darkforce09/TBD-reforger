//! Telemetry read surfaces: the [`dashboard`] summary and the [`leaderboards`] tables. The
//! game-server ingest that feeds them lives in [`crate::match_telemetry`].

pub mod dashboard;
pub mod leaderboards;
