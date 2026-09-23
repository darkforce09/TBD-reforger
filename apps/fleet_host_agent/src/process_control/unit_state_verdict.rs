//! The verdict of a process action: the unit's state read back after the action, against the
//! state the action intends.

use std::time::Duration;

use serde_json::{Map, Value};

use super::ProcessAction;
use super::systemctl_runner::{ProgramFailure, ProgramOutput};
use crate::action_verdict::ActionVerdict;

/// Bytes of the verb's error output kept for the operator.
const ERROR_OUTPUT_MAX_BYTES: usize = 160;

/// The unit's LoadState and ActiveState as systemd reported them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedUnitState {
    pub load_state: String,
    pub active_state: String,
}

/// How the verb invocation itself ended. It is recorded for the operator and never decides the
/// verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemctlObservation {
    /// The program exited; `code` is `None` when a signal ended it.
    Exited {
        code: Option<i32>,
        error_output: String,
    },
    TimedOut {
        limit: Duration,
    },
    NotStarted {
        error: String,
    },
}

impl SystemctlObservation {
    pub(super) fn of(result: Result<ProgramOutput, ProgramFailure>) -> Self {
        match result {
            Ok(output) => Self::Exited {
                code: output.status.code(),
                error_output: error_output_excerpt(&output.stderr),
            },
            Err(ProgramFailure::TimedOut(limit)) => Self::TimedOut { limit },
            Err(ProgramFailure::NotStarted(error)) => Self::NotStarted {
                error: error.to_string(),
            },
        }
    }

    /// "exited with status 1: <first line of its error output>", "did not finish within 100s",
    /// and so on.
    pub fn describe(&self) -> String {
        match self {
            Self::Exited { code, error_output } => {
                let ending = match code {
                    Some(code) => format!("exited with status {code}"),
                    None => "was ended by a signal".to_owned(),
                };
                if error_output.is_empty() {
                    ending
                } else {
                    format!("{ending}: {error_output}")
                }
            }
            Self::TimedOut { limit } => format!("did not finish within {limit:?}"),
            Self::NotStarted { error } => format!("could not be started: {error}"),
        }
    }
}

/// Everything observed while performing one process action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessActionReport {
    pub action: ProcessAction,
    pub unit: String,
    pub systemctl: SystemctlObservation,
    /// The wait between the verb and the state read.
    pub dwell: Duration,
    /// The state read back, or why it could not be read.
    pub unit_state: Result<ObservedUnitState, String>,
}

impl ProcessActionReport {
    /// Succeeded only when the unit is loaded and in the state the action intends. A failure
    /// names the state observed; the outcome records the observation either way.
    pub fn verdict(&self) -> ActionVerdict {
        let outcome = self.outcome();
        let verb = self.action.systemctl_verb();
        let intended = self.action.intended_active_state();
        let invocation = format!("systemctl --user {verb} {}", self.systemctl.describe());
        match &self.unit_state {
            Err(problem) => ActionVerdict::failure(
                &format!(
                    "{verb} of {}: the unit state could not be read ({problem}); {invocation}",
                    self.unit
                ),
                Some(outcome),
            ),
            Ok(state) if state.load_state != "loaded" => ActionVerdict::failure(
                &format!(
                    "{verb} of {} cannot be confirmed: the unit is not loaded (LoadState={}); \
                     {invocation}",
                    self.unit, state.load_state
                ),
                Some(outcome),
            ),
            Ok(state) if state.active_state == intended => ActionVerdict::success(outcome),
            Ok(state) => ActionVerdict::failure(
                &format!(
                    "{verb} left {} {} instead of {intended}{}; {invocation}",
                    self.unit,
                    state.active_state,
                    self.dwell_phrase()
                ),
                Some(outcome),
            ),
        }
    }

    fn dwell_phrase(&self) -> String {
        if self.dwell.is_zero() {
            String::new()
        } else {
            format!(" after waiting {:?}", self.dwell)
        }
    }

    fn outcome(&self) -> Map<String, Value> {
        let mut outcome = Map::new();
        outcome.insert("unit".to_owned(), Value::from(self.unit.clone()));
        if let Ok(state) = &self.unit_state {
            outcome.insert(
                "load_state".to_owned(),
                Value::from(state.load_state.clone()),
            );
            outcome.insert(
                "active_state".to_owned(),
                Value::from(state.active_state.clone()),
            );
        }
        let dwell_milliseconds = u64::try_from(self.dwell.as_millis()).unwrap_or(u64::MAX);
        outcome.insert(
            "dwell_milliseconds".to_owned(),
            Value::from(dwell_milliseconds),
        );
        outcome.insert(
            "systemctl".to_owned(),
            Value::from(self.systemctl.describe()),
        );
        outcome
    }
}

/// The first line of the program's error output, cut to a short excerpt, with control
/// characters replaced.
fn error_output_excerpt(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let first_line = text.lines().map(str::trim).find(|line| !line.is_empty());
    let mut excerpt = String::new();
    for character in first_line.unwrap_or_default().chars() {
        if excerpt.len() + character.len_utf8() > ERROR_OUTPUT_MAX_BYTES {
            break;
        }
        excerpt.push(if character.is_control() {
            ' '
        } else {
            character
        });
    }
    excerpt
}

#[cfg(test)]
#[path = "tests/unit_state_verdict.rs"]
mod tests;
