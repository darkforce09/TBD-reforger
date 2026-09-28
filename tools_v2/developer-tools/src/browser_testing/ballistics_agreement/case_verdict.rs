//! The judgement of the browser's reading against the native solves.
//!
//! **Role:** judges every case of a [`BenchReading`] against the [`NativeCase`] drawn at the same
//! index, and the reading as a whole against the run that was asked for, into an
//! [`AgreementVerdict`] and the lines the gate prints.
//! **Position:** fed by [`super::bench_reading`] and [`super::native_reference`]; called by
//! [`super::run`], which prints [`AgreementVerdict::lines`] and exits with
//! [`AgreementVerdict::exit_code`].
//! **Signals & state:** none; pure functions.
//! **Invariants:**
//! - A case agrees when its id is the native id, its inputs are the native inputs bit for bit,
//!   its bit patterns restate its own values, its lead-gun summary restates its own solution, and
//!   its solution matches the native one within the fire-mission tolerances
//!   ([`ANGLE_TOLERANCE_MILS`] weapon mils, [`TIME_TOLERANCE_S`] seconds) through
//!   [`compare_solutions`], or both sides refuse with the same sentence.
//! - A case is bit-identical when, beyond agreeing, every `f64` bit pattern equals the native one.
//! - The run passes only with no run-level failure, at least one case, as many cases as asked
//!   for, and every case agreeing; zero cases never pass.

use std::collections::BTreeMap;

use website_map_engine::data::scenario::ballistics::agreement_cases::{
    case_bit_patterns, lead_summary,
};
use website_map_engine::data::scenario::ballistics::fire_mission_comparison::{
    ANGLE_TOLERANCE_MILS, TIME_TOLERANCE_S, compare_solutions,
};

use super::bench_reading::{BenchCase, BenchReading};
use super::native_reference::NativeCase;

/// Prefix of every printed case name.
pub const CASE_PREFIX: &str = "ballistics_wasm_agreement_";
/// Marker the gate prints before its tally.
pub const RUN_MARKER: &str = "ballistics-wasm-agreement";

/// What the run was asked to compare.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestedRun {
    /// Seed of the case lattice.
    pub seed: u64,
    /// Number of cases.
    pub count: usize,
    /// Catalog id the cases are drawn over.
    pub catalog_id: String,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// Solver revision of the native build.
    pub solver_revision: String,
}

/// One case's judgement.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseVerdict {
    /// The case id (the native id at that index).
    pub case_id: String,
    /// Why the case does not agree; empty when it does.
    pub failures: Vec<String>,
    /// Whether every `f64` equals the native one bit for bit.
    pub bit_identical: bool,
}

impl CaseVerdict {
    /// Whether the case agrees.
    pub fn agrees(&self) -> bool {
        self.failures.is_empty()
    }
}

/// The whole run's judgement.
#[derive(Clone, Debug, PartialEq)]
pub struct AgreementVerdict {
    /// Failures of the run as a whole (identity, count, provenance).
    pub run_failures: Vec<String>,
    /// Every judged case, in draw order.
    pub cases: Vec<CaseVerdict>,
}

impl AgreementVerdict {
    /// A run that failed before any case could be judged.
    pub fn failed(cause: String) -> Self {
        Self {
            run_failures: vec![cause],
            cases: Vec::new(),
        }
    }

    /// Whether the run passes.
    pub fn passed(&self) -> bool {
        self.run_failures.is_empty()
            && !self.cases.is_empty()
            && self.cases.iter().all(CaseVerdict::agrees)
    }

    /// The process exit code: 0 when the run passes, else 1.
    pub fn exit_code(&self) -> u8 {
        u8::from(!self.passed())
    }

    /// The printed report: one `case … ok|FAILED` line per case with its causes indented, the
    /// run failures, the bit-identity count and the tally.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for case in &self.cases {
            let status = if case.agrees() { "ok" } else { "FAILED" };
            lines.push(format!("case {CASE_PREFIX}{} ... {status}", case.case_id));
            lines.extend(case.failures.iter().map(|failure| format!("    {failure}")));
        }
        lines.extend(
            self.run_failures
                .iter()
                .map(|failure| format!("run failure: {failure}")),
        );
        let identical = self.cases.iter().filter(|case| case.bit_identical).count();
        let agreeing = self.cases.iter().filter(|case| case.agrees()).count();
        let total = self.cases.len();
        lines.push(format!("bit-identical cases: {identical}/{total}"));
        let tally = if self.passed() { "PASS" } else { "FAIL" };
        lines.push(format!("{RUN_MARKER}: {tally} {agreeing}/{total}"));
        lines
    }
}

/// Judges the browser's `reading` against the `native` cases of the `requested` run.
pub fn judge_reading(
    requested: &RequestedRun,
    native: &[NativeCase],
    reading: &BenchReading,
) -> AgreementVerdict {
    let mut run_failures = Vec::new();
    let mut expect = |what: &str, bench: String, native: String| {
        if bench != native {
            run_failures.push(format!(
                "{what}: the bench read {bench}, the gate asked {native}"
            ));
        }
    };
    expect("seed", reading.seed.to_string(), requested.seed.to_string());
    expect(
        "count",
        reading.count.to_string(),
        requested.count.to_string(),
    );
    expect(
        "catalog",
        reading.catalog_id.clone(),
        requested.catalog_id.clone(),
    );
    expect(
        "catalog version",
        reading.catalog_version.to_string(),
        requested.catalog_version.to_string(),
    );
    expect(
        "solver revision",
        reading.solver_revision.clone(),
        requested.solver_revision.clone(),
    );
    if native.is_empty() {
        run_failures.push("the catalog draws zero cases".to_string());
    }
    if native.len() != requested.count {
        run_failures.push(format!(
            "the gate drew {} cases for a count of {}",
            native.len(),
            requested.count
        ));
    }
    if reading.cases.len() != native.len() {
        run_failures.push(format!(
            "the bench solved {} cases, the gate {}",
            reading.cases.len(),
            native.len()
        ));
    }
    let cases = native
        .iter()
        .zip(&reading.cases)
        .map(|(native_case, bench_case)| judge_case(native_case, bench_case))
        .collect();
    AgreementVerdict {
        run_failures,
        cases,
    }
}

/// Judges one browser case against the native case drawn at the same index.
pub fn judge_case(native: &NativeCase, bench: &BenchCase) -> CaseVerdict {
    let mut failures = Vec::new();
    if bench.case_id != native.case_id {
        failures.push(format!("the bench case is {}", bench.case_id));
    }
    let restated = case_bit_patterns(&bench.inputs, bench.solution.as_ref());
    if restated != bench.bit_patterns {
        failures.push("the bench bit patterns do not restate its own values".to_string());
    }
    if bench.inputs != native.inputs
        || !same_patterns_under(&bench.bit_patterns, &native.bit_patterns, "/inputs/")
    {
        failures.push("the bench solved other inputs than the native draw".to_string());
    }
    let summary = (bench.lead_recommended_rings, bench.lead_time_of_flight_s);
    if !same_summary(summary, lead_summary(bench.solution.as_ref())) {
        failures.push("the bench lead-gun summary does not restate its own solution".to_string());
    }
    match (&bench.solution, &bench.refusal, &native.outcome) {
        (Some(bench_solution), None, Ok(native_solution)) => {
            let mils_per_circle = native_solution
                .guns
                .first()
                .map_or(0, |gun| gun.mils_per_circle);
            let comparison = compare_solutions(bench_solution, native_solution, mils_per_circle);
            for mismatch in comparison.mismatches.iter().take(4) {
                failures.push(format!(
                    "beyond {ANGLE_TOLERANCE_MILS} mil / {TIME_TOLERANCE_S} s: {mismatch:?}"
                ));
            }
            if comparison.mismatches.len() > 4 {
                failures.push(format!(
                    "… {} mismatches in all",
                    comparison.mismatches.len()
                ));
            }
        }
        (None, Some(bench_refusal), Err(native_refusal)) => {
            if bench_refusal != native_refusal {
                failures.push(format!(
                    "the bench refused with `{bench_refusal}`, the native solve with `{native_refusal}`"
                ));
            }
        }
        (bench_solution, bench_refusal, native_outcome) => failures.push(format!(
            "the bench carries {} solution and {} refusal; the native solve {}",
            if bench_solution.is_some() { "a" } else { "no" },
            if bench_refusal.is_some() { "a" } else { "no" },
            if native_outcome.is_ok() {
                "solved"
            } else {
                "refused"
            }
        )),
    }
    let bit_identical = failures.is_empty()
        && bench.bit_patterns == native.bit_patterns
        && bench.refusal.as_deref() == native.refusal();
    CaseVerdict {
        case_id: native.case_id.clone(),
        failures,
        bit_identical,
    }
}

fn same_patterns_under(
    bench: &BTreeMap<String, String>,
    native: &BTreeMap<String, String>,
    prefix: &str,
) -> bool {
    let under = |patterns: &BTreeMap<String, String>| {
        patterns
            .iter()
            .filter(|(pointer, _)| pointer.starts_with(prefix))
            .map(|(pointer, bits)| (pointer.clone(), bits.clone()))
            .collect::<Vec<_>>()
    };
    under(bench) == under(native)
}

fn same_summary(left: (Option<u32>, Option<f64>), right: (Option<u32>, Option<f64>)) -> bool {
    left.0 == right.0 && left.1.map(f64::to_bits) == right.1.map(f64::to_bits)
}

#[cfg(test)]
#[path = "../tests/ballistics_agreement/case_verdict.rs"]
mod tests;
