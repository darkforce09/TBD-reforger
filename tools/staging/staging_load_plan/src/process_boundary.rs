//! The JSON a plan and a report cross the process boundary in, between the xtask load procedure
//! and the `staging-load` executable.
//!
//! - **Role:** encodes a [`LoadRunPlan`] for the executable's standard input and decodes it there,
//!   and encodes the [`LoadReport`] the executable prints and decodes it in the procedure.
//! - **Position:** the xtask load procedure encodes plans and decodes reports; the load
//!   generator's command line decodes plans and encodes reports; both sides call these four
//!   functions, so they cannot disagree on the encoding.
//! - **Signals & state:** none; pure functions.
//! - **Invariants:** the encoding is compact JSON with fields in declaration order; decoding
//!   refuses unknown fields; encoding a decoded value reproduces the encoded text byte for byte.

use crate::error::{Error, Result};
use crate::load_report::LoadReport;
use crate::workload_plan::LoadRunPlan;

/// The plan as one line of compact JSON.
///
/// # Errors
///
/// The plan holds a value JSON cannot carry.
pub fn encode_plan(plan: &LoadRunPlan) -> Result<String> {
    serde_json::to_string(plan).map_err(|error| Error::Unencodable {
        what: "plan",
        error,
    })
}

/// The plan `text` encodes.
///
/// # Errors
///
/// `text` is not JSON of the plan's shape.
pub fn decode_plan(text: &str) -> Result<LoadRunPlan> {
    serde_json::from_str(text).map_err(|error| Error::Undecodable {
        what: "plan",
        error,
    })
}

/// The report as one line of compact JSON.
///
/// # Errors
///
/// The report holds a value JSON cannot carry.
pub fn encode_report(report: &LoadReport) -> Result<String> {
    serde_json::to_string(report).map_err(|error| Error::Unencodable {
        what: "report",
        error,
    })
}

/// The report `text` encodes; surrounding whitespace, such as the executable's final newline, is
/// ignored.
///
/// # Errors
///
/// `text` is not JSON of the report's shape.
pub fn decode_report(text: &str) -> Result<LoadReport> {
    serde_json::from_str(text.trim()).map_err(|error| Error::Undecodable {
        what: "report",
        error,
    })
}
