//! The observed outcome of one fleet action, in the shape the command ledger records.

use serde_json::{Map, Value};

/// The ledger accepts failure reasons of 1 to 512 bytes.
pub const FAILURE_REASON_MAX_BYTES: usize = 512;

const TRUNCATION_MARK: &str = "...";
const UNNAMED_FAILURE: &str = "the action failed";

/// What an executor observed after acting: success with an outcome object, or failure with a
/// reason and, when something was observed, an outcome as well. A success never carries a
/// failure reason and a failure always carries one.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionVerdict {
    succeeded: bool,
    outcome: Option<Map<String, Value>>,
    failure_reason: Option<String>,
}

impl ActionVerdict {
    pub fn success(outcome: Map<String, Value>) -> Self {
        Self {
            succeeded: true,
            outcome: Some(outcome),
            failure_reason: None,
        }
    }

    /// A failure. The reason is trimmed and cut to [`FAILURE_REASON_MAX_BYTES`] on a character
    /// boundary; an empty reason reads "the action failed".
    pub fn failure(reason: &str, outcome: Option<Map<String, Value>>) -> Self {
        Self {
            succeeded: false,
            outcome,
            failure_reason: Some(bounded_failure_reason(reason)),
        }
    }

    pub fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub fn outcome(&self) -> Option<&Map<String, Value>> {
        self.outcome.as_ref()
    }

    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// The fields of the ledger's result report: succeeded, outcome, failure reason.
    pub fn into_parts(self) -> (bool, Option<Map<String, Value>>, Option<String>) {
        (self.succeeded, self.outcome, self.failure_reason)
    }
}

fn bounded_failure_reason(reason: &str) -> String {
    let reason = reason.trim();
    if reason.is_empty() {
        return UNNAMED_FAILURE.to_owned();
    }
    if reason.len() <= FAILURE_REASON_MAX_BYTES {
        return reason.to_owned();
    }
    let mut end = FAILURE_REASON_MAX_BYTES - TRUNCATION_MARK.len();
    while !reason.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATION_MARK}", reason[..end].trim_end())
}

#[cfg(test)]
#[path = "tests/action_verdict.rs"]
mod tests;
