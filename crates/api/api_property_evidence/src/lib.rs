//! The property run recorder of the API's property tests.
//!
//! **Role:** runs a proptest strategy with a fixed seed and case count, counts the checks that
//! completed, digests their inputs and prints the run's record ([`run_property`],
//! [`collect_property`], [`PropertyRun`]).
//! **Position:** dev-only: only `[dev-dependencies]` name this crate. The API's operations unit
//! tests and its property suites under `apps/api/tests/` call it.
//! **Signals & state:** none kept between runs.
//! **Invariants:** a run that executes zero cases, fails a check or completes fewer checks than
//! requested panics instead of producing a record; the same seed reproduces the same record.

pub mod prelude;
mod property_run;

pub use property_run::{PropertyRun, collect_property, run_property};
