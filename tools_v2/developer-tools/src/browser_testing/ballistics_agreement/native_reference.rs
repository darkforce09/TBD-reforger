//! The native half of the agreement: the same cases drawn, converted and solved on the host.
//!
//! **Role:** draws the agreement cases over a catalog, turns each into the fire-mission inputs
//! it stands for, solves them with the one assembler and records every `f64` of the inputs and
//! the solution by JSON pointer and bit pattern.
//! **Position:** called by [`super::run`] with the committed catalog; its [`NativeCase`]s are
//! the reference [`super::case_verdict`] judges the browser's reading against. The drawing, the
//! case-to-inputs mapping and the bit walk are the map engine's (`agreement_cases`,
//! `fire_mission_inputs`, `case_bit_patterns` of
//! `website_map_engine::data::scenario::ballistics::agreement_cases`), the same functions the
//! browser bench that produces the reading calls.
//! **Signals & state:** none; pure functions over a borrowed catalog.
//! **Invariants:** a native case's inputs are `fire_mission_inputs` of the drawn case and its
//! bit patterns are `case_bit_patterns` of those inputs and its solution (none for a refusal).

use std::collections::BTreeMap;

use website_map_engine::data::scenario::ballistics::agreement_cases::{
    AgreementCase, agreement_cases, case_bit_patterns, fire_mission_inputs,
};
use website_map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;
use website_map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionInputs, FireMissionSolution, solve_fire_mission,
};

/// One case solved on the host.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeCase {
    /// `<seed as 16 hex digits>_<index as 4 digits>`.
    pub case_id: String,
    /// The inputs the case was solved from.
    pub inputs: FireMissionInputs,
    /// The solution, or the assembler's refusal sentence.
    pub outcome: Result<FireMissionSolution, String>,
    /// JSON pointer of every `f64` of `{"inputs", "solution"}` to its bits in hexadecimal.
    pub bit_patterns: BTreeMap<String, String>,
}

impl NativeCase {
    /// The solution, when the case solved.
    pub fn solution(&self) -> Option<&FireMissionSolution> {
        self.outcome.as_ref().ok()
    }

    /// The refusal sentence, when the case was refused.
    pub fn refusal(&self) -> Option<&str> {
        self.outcome.as_ref().err().map(String::as_str)
    }
}

/// Draws `count` cases over `catalog` from `seed` and solves every one on the host.
pub fn native_cases(catalog: &BallisticsCatalog, seed: u64, count: usize) -> Vec<NativeCase> {
    agreement_cases(catalog, seed, count)
        .iter()
        .map(|case| native_case(catalog, case))
        .collect()
}

/// Solves one drawn case on the host.
pub fn native_case(catalog: &BallisticsCatalog, case: &AgreementCase) -> NativeCase {
    let inputs = fire_mission_inputs(catalog, case);
    let outcome = solve_fire_mission(catalog, &inputs).map_err(|refused| refused.to_string());
    let bit_patterns = case_bit_patterns(&inputs, outcome.as_ref().ok());
    NativeCase {
        case_id: case.case_id.clone(),
        inputs,
        outcome,
        bit_patterns,
    }
}

#[cfg(test)]
#[path = "../tests/ballistics_agreement/native_reference.rs"]
mod tests;
