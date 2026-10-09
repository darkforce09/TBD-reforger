//! The plan and the report of the staging member load, without the engine that runs it.
//!
//! - **Role:** the committed workload's shape and the run's bindings with their checks
//!   ([`WorkloadPlan`], [`LoadRunPlan`], [`FixtureEvent`]), the compiled request mix, the pacing
//!   that decides how many member accounts a workload reaches, the source-address check, the
//!   records an exchange leaves, and the [`LoadReport`] folded from what every client recorded.
//! - **Position:** tier 1 of `tools/staging`. The load generator (`staging_load_generator`) runs a
//!   plan and assembles the report from its clients' outcomes; the xtask load procedure builds
//!   plans, checks addresses and judges reports through these types alone, and hands a plan to
//!   the `staging-load` executable as JSON.
//! - **Signals & state:** none; plain values and pure functions. Nothing here starts a runtime,
//!   opens a connection or reads a clock.
//! - **Invariants:** no async runtime and no HTTP client: a plan and a report cross the process
//!   boundary as JSON, and every check here runs before any request is sent.

pub mod client_outcome;
pub mod concurrency_census;
mod error;
pub mod identifiers;
pub mod latency_recording;
pub mod load_report;
pub mod pacing;
pub mod prelude;
pub mod process_boundary;
pub mod request_catalog;
pub mod run_settings;
pub mod source_addresses;
pub mod workload_plan;

pub use error::{Error, Result};
pub use load_report::{ClassSummary, LoadReport};
pub use pacing::reachable_member_accounts;
pub use process_boundary::{decode_plan, decode_report, encode_plan, encode_report};
pub use source_addresses::verify_source_addresses;
pub use workload_plan::{FixtureEvent, LoadRunPlan, WorkloadPlan};
