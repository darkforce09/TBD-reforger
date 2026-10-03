//! The reading the browser bench writes into `<pre data-ballistics-agreement>`, as the gate
//! decodes it.
//!
//! **Role:** the typed shape of the bench's JSON reading and its strict decoder.
//! **Position:** mirrors `AgreementReport` and `AgreementCaseReport` of
//! `apps/frontend/src/workspaces/debug/ballistics_agreement/agreement_report.rs`; fed by
//! [`super::browser_session`], consumed by [`super::case_verdict`].
//! **Signals & state:** none.
//! **Invariants:** decoding refuses unknown keys and missing required keys, so a drift between
//! the bench's shape and this mirror fails the gate instead of dropping a field; `f64` values decode exactly (the
//! crate's `serde_json` parses floats with correct rounding), and every `f64` also travels as its
//! bit pattern.

use std::collections::BTreeMap;

use crate::Result;
use crate::error::ResultExt;
use ballistics_agreement_cases::AgreementCaseId;
use ballistics_model::CatalogId;
use fire_mission_planning::fire_mission::{FireMissionInputs, FireMissionSolution};
use serde::{Deserialize, Serialize};

/// One run of the bench.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchReading {
    /// Seed of the case lattice.
    pub seed: u64,
    /// Number of cases requested.
    pub count: usize,
    /// Catalog the cases were drawn over and solved against.
    pub catalog_id: CatalogId,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// Solver revision of the wasm build.
    pub solver_revision: String,
    /// Every case, in draw order.
    pub cases: Vec<BenchCase>,
}

/// One case as the browser solved it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchCase {
    /// `<seed as 16 hex digits>_<index as 4 digits>`.
    pub case_id: AgreementCaseId,
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

/// Decodes the text of the bench's `<pre>`.
///
/// # Errors
///
/// The decoder's message when the text is not a complete reading.
pub fn decode_bench_reading(text: &str) -> Result<BenchReading> {
    serde_json::from_str(text).context("the bench reading does not decode")
}
