//! The ballistics agreement bench's reading: every drawn case solved, with the bit pattern of
//! every `f64` it holds.
//!
//! **Role:** solves one drawn agreement case with [`solve_fire_mission`] from the inputs the map
//! engine's [`fire_mission_inputs`] maps it to, and records the case as an
//! [`AgreementCaseReport`]; the whole run is an [`AgreementReport`], serialised into the bench's
//! `<pre>`.
//! **Position:** the pure half of [`super`], called by the browser half once per case. The
//! case-to-inputs mapping, the lead summary and the bit walk are the map engine's
//! (`map_engine::data::scenario::ballistics::agreement_cases`), shared with the native
//! agreement gate of `tools/developer_tools/` (`browser_testing::ballistics_agreement`), which
//! decodes this shape and solves the same cases natively.
//! **Signals & state:** none; pure functions over a borrowed catalog.
//! **Invariants:**
//! - A case's inputs are [`fire_mission_inputs`] of it against the catalog handed in.
//! - `bit_patterns` is [`case_bit_patterns`] of the inputs and the solution: the JSON pointer of
//!   every `f64` leaf of `{"inputs", "solution"}` to its IEEE 754 bits as 16 lowercase
//!   hexadecimal digits, so an exact comparison never depends on a decimal round trip.
//! - `lead_recommended_rings` and `lead_time_of_flight_s` restate the lead gun's recommended
//!   charge and that row's time of flight; both are `None` when nothing solves.

use std::collections::BTreeMap;

use map_engine::data::scenario::ballistics::agreement_cases::{
    case_bit_patterns, fire_mission_inputs, lead_summary, AgreementCase,
};
use map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;
use map_engine::data::scenario::ballistics::fire_mission::{
    solve_fire_mission, FireMissionInputs, FireMissionSolution, SOLVER_REVISION,
};
use serde::Serialize;

/// One run of the bench.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgreementReport {
    /// Seed of the case lattice.
    pub seed: u64,
    /// Number of cases requested.
    pub count: usize,
    /// Catalog the cases are drawn over and solved against.
    pub catalog_id: String,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// Solver revision of the build that solved the cases.
    pub solver_revision: String,
    /// Every case, in draw order.
    pub cases: Vec<AgreementCaseReport>,
}

/// One solved case.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgreementCaseReport {
    /// `<seed as 16 hex digits>_<index as 4 digits>`.
    pub case_id: String,
    /// The inputs the case was solved from.
    pub inputs: FireMissionInputs,
    /// The solution; `None` when the assembler refused the case.
    pub solution: Option<FireMissionSolution>,
    /// The assembler's refusal sentence; `None` when the case solved.
    pub refusal: Option<String>,
    /// Rings of the lead gun's recommended charge.
    pub lead_recommended_rings: Option<u32>,
    /// Time of flight of that charge, seconds.
    pub lead_time_of_flight_s: Option<f64>,
    /// JSON pointer of every `f64` of `{"inputs", "solution"}` to its bits in hexadecimal.
    pub bit_patterns: BTreeMap<String, String>,
}

/// Solves one drawn case and records it.
pub fn case_report(catalog: &BallisticsCatalog, case: &AgreementCase) -> AgreementCaseReport {
    let inputs = fire_mission_inputs(catalog, case);
    let (solution, refusal) = match solve_fire_mission(catalog, &inputs) {
        Ok(solution) => (Some(solution), None),
        Err(refused) => (None, Some(refused.to_string())),
    };
    let (lead_recommended_rings, lead_time_of_flight_s) = lead_summary(solution.as_ref());
    let bit_patterns = case_bit_patterns(&inputs, solution.as_ref());
    AgreementCaseReport {
        case_id: case.case_id.clone(),
        inputs,
        solution,
        refusal,
        lead_recommended_rings,
        lead_time_of_flight_s,
        bit_patterns,
    }
}

/// The run of `count` cases drawn from `seed` over `catalog`, from its solved `cases`.
pub fn assemble_report(
    catalog: &BallisticsCatalog,
    seed: u64,
    count: usize,
    cases: Vec<AgreementCaseReport>,
) -> AgreementReport {
    AgreementReport {
        seed,
        count,
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        solver_revision: SOLVER_REVISION.to_string(),
        cases,
    }
}

#[cfg(test)]
#[path = "tests/agreement_report.rs"]
mod tests;
