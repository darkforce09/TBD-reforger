//! The generic procedure engine every staging procedure runs on.
//!
//! **Role:** declares the step vocabulary ([`step`]), the procedure contract and plan checks
//! ([`procedure`]), the runner ([`runner`]) and its single observation (`probe_reading`), its
//! clock ([`clock`]), and the recorded run ([`recording`]) that binds a run to its receipt.
//!
//! **Position:** between the procedure modules, which supply plans, and the recorder in
//! `api_readiness_checks::operational_recording`, which judges the outcome.
//!
//! **Signals & state:** a run's state lives in the runner for the run's duration.
//!
//! **Invariants:** the engine never reads stdin; deadlines count from observed rows; every
//! observation is journaled; a missed deadline fails its case.

pub(crate) mod clock;
pub(crate) mod probe_reading;
pub(crate) mod procedure;
pub(crate) mod recording;
pub(crate) mod runner;
pub(crate) mod step;
