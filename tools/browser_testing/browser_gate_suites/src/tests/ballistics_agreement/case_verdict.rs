//! Unit tests for the judgement of the browser's reading against the native solves.

use ballistics_model::catalog::BallisticsCatalog;
use fire_mission_planning::fire_mission::SOLVER_REVISION;
use serde_json::{Value, json};

use super::*;
use crate::ballistics_agreement::COMMITTED_CATALOG;
use crate::ballistics_agreement::bench_reading::decode_bench_reading;
use crate::ballistics_agreement::native_reference::native_cases;
use ::repository_layout::find_repository_root;

const SEED: u64 = 0x5EED_0000_0000_0003;
const COUNT: usize = 3;

fn committed_catalog() -> BallisticsCatalog {
    let path = find_repository_root()
        .expect("repository root")
        .join(COMMITTED_CATALOG);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "the committed catalog {} is unreadable: {error}",
            path.display()
        )
    });
    BallisticsCatalog::from_json_slice(&bytes).expect("the committed catalog decodes")
}

fn requested(catalog: &BallisticsCatalog, count: usize) -> RequestedRun {
    RequestedRun {
        seed: SEED,
        count,
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        solver_revision: SOLVER_REVISION.to_string(),
    }
}

/// The reading a faithful bench writes for `native`.
fn faithful_reading(run: &RequestedRun, native: &[NativeCase]) -> BenchReading {
    BenchReading {
        seed: run.seed,
        count: run.count,
        catalog_id: run.catalog_id.clone(),
        catalog_version: run.catalog_version,
        solver_revision: run.solver_revision.clone(),
        cases: native
            .iter()
            .map(|case| {
                let (lead_recommended_rings, lead_time_of_flight_s) = lead_summary(case.solution());
                BenchCase {
                    case_id: case.case_id.clone(),
                    inputs: case.inputs.clone(),
                    solution: case.solution().cloned(),
                    refusal: case.refusal().map(str::to_string),
                    lead_recommended_rings,
                    lead_time_of_flight_s,
                    bit_patterns: case.bit_patterns.clone(),
                }
            })
            .collect(),
    }
}

/// The reading as the gate receives it: serialised to the `<pre>` text and decoded back.
fn through_the_pre(reading: &BenchReading) -> BenchReading {
    let text = serde_json::to_string(reading).expect("serialises");
    decode_bench_reading(&text).expect("decodes")
}

/// Rewrites a case's bit patterns and lead summary from its own (edited) values.
fn restamp(case: &mut BenchCase) {
    case.bit_patterns = case_bit_patterns(&case.inputs, case.solution.as_ref());
    let (rings, time_of_flight_s) = lead_summary(case.solution.as_ref());
    case.lead_recommended_rings = rings;
    case.lead_time_of_flight_s = time_of_flight_s;
}

fn scale_floats(value: &mut Value, factor: f64) {
    match value {
        Value::Number(number) if number.is_f64() => {
            *value = json!(number.as_f64().expect("an f64") * factor);
        }
        Value::Array(items) => items.iter_mut().for_each(|item| scale_floats(item, factor)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|item| scale_floats(item, factor)),
        _ => {}
    }
}

fn fixture() -> (RequestedRun, Vec<NativeCase>) {
    let catalog = committed_catalog();
    let run = requested(&catalog, COUNT);
    let native = native_cases(&catalog, SEED, COUNT);
    (run, native)
}

#[test]
fn a_faithful_reading_passes_bit_identical() {
    let (run, native) = fixture();
    let verdict = judge_reading(
        &run,
        &native,
        &through_the_pre(&faithful_reading(&run, &native)),
    );
    assert!(verdict.passed(), "{:#?}", verdict.lines());
    assert_eq!(verdict.exit_code(), 0);
    assert!(verdict.cases.iter().all(|case| case.bit_identical));
    let lines = verdict.lines();
    for case in &native {
        assert!(lines.contains(&format!(
            "case ballistics_wasm_agreement_{} ... ok",
            case.case_id
        )));
    }
    assert!(lines.contains(&format!("bit-identical cases: {COUNT}/{COUNT}")));
    assert_eq!(
        lines.last().map(String::as_str),
        Some(format!("ballistics-wasm-agreement: PASS {COUNT}/{COUNT}").as_str())
    );
}

#[test]
fn an_output_scaled_by_one_part_in_a_thousand_fails() {
    let (run, native) = fixture();
    let mut reading = faithful_reading(&run, &native);
    for case in &mut reading.cases {
        let mut solution = serde_json::to_value(&case.solution).expect("serialises");
        scale_floats(&mut solution, 1.0 + 1e-3);
        case.solution = serde_json::from_value(solution).expect("still a solution");
        restamp(case);
    }
    let verdict = judge_reading(&run, &native, &through_the_pre(&reading));
    assert!(!verdict.passed());
    assert_eq!(verdict.exit_code(), 1);
    assert!(verdict.cases.iter().any(|case| !case.agrees()));
    assert!(verdict.cases.iter().all(|case| !case.bit_identical));
    let lines = verdict.lines();
    assert!(lines.iter().any(|line| line.ends_with(" ... FAILED")));
    assert!(
        lines
            .last()
            .expect("a tally")
            .starts_with("ballistics-wasm-agreement: FAIL ")
    );
}

#[test]
fn a_one_ulp_difference_agrees_but_is_not_bit_identical() {
    let (run, native) = fixture();
    let mut reading = faithful_reading(&run, &native);
    let case = &mut reading.cases[0];
    let lead = &mut case.solution.as_mut().expect("the case solves").guns[0];
    lead.azimuth_mils = f64::from_bits(lead.azimuth_mils.to_bits() + 1);
    restamp(case);
    let verdict = judge_reading(&run, &native, &through_the_pre(&reading));
    assert!(verdict.passed(), "{:#?}", verdict.lines());
    assert!(!verdict.cases[0].bit_identical);
    assert!(
        verdict
            .lines()
            .contains(&format!("bit-identical cases: {}/{COUNT}", COUNT - 1))
    );
}

#[test]
fn a_bench_that_solved_other_inputs_fails() {
    let (run, native) = fixture();
    let mut reading = faithful_reading(&run, &native);
    reading.cases[1].inputs.target.x += 0.5;
    restamp(&mut reading.cases[1]);
    let verdict = judge_reading(&run, &native, &through_the_pre(&reading));
    assert!(!verdict.passed());
    assert!(
        verdict.cases[1]
            .failures
            .iter()
            .any(|failure| failure.contains("other inputs"))
    );
}

#[test]
fn bit_patterns_that_do_not_restate_the_values_fail() {
    let (run, native) = fixture();
    let mut reading = faithful_reading(&run, &native);
    let pattern = reading.cases[0]
        .bit_patterns
        .get_mut("/solution/guns/0/azimuth_mils")
        .expect("the lead azimuth has a pattern");
    *pattern = "0000000000000000".to_string();
    let verdict = judge_reading(&run, &native, &through_the_pre(&reading));
    assert!(!verdict.cases[0].agrees());
    assert!(
        verdict.cases[0]
            .failures
            .iter()
            .any(|failure| failure.contains("do not restate"))
    );
}

#[test]
fn a_refusal_against_a_native_solution_fails() {
    let (run, native) = fixture();
    let mut reading = faithful_reading(&run, &native);
    reading.cases[2].solution = None;
    reading.cases[2].refusal = Some("the battery has no gun".to_string());
    restamp(&mut reading.cases[2]);
    let verdict = judge_reading(&run, &native, &through_the_pre(&reading));
    assert!(!verdict.cases[2].agrees());
    assert!(!verdict.passed());
}

#[test]
fn a_missing_case_or_another_run_fails_the_run() {
    let (run, native) = fixture();
    let mut short = faithful_reading(&run, &native);
    short.cases.pop();
    let verdict = judge_reading(&run, &native, &through_the_pre(&short));
    assert!(!verdict.passed());
    assert!(!verdict.run_failures.is_empty());

    let mut other_seed = faithful_reading(&run, &native);
    other_seed.seed += 1;
    let verdict = judge_reading(&run, &native, &through_the_pre(&other_seed));
    assert!(!verdict.passed());
    assert!(
        verdict
            .run_failures
            .iter()
            .any(|failure| failure.starts_with("seed"))
    );
}

#[test]
fn zero_cases_never_pass() {
    let catalog = committed_catalog();
    let run = requested(&catalog, 0);
    let native = native_cases(&catalog, SEED, 0);
    let verdict = judge_reading(&run, &native, &faithful_reading(&run, &native));
    assert!(!verdict.passed());
    assert_eq!(verdict.exit_code(), 1);
    assert_eq!(
        verdict.lines().last().map(String::as_str),
        Some("ballistics-wasm-agreement: FAIL 0/0")
    );
    assert!(!AgreementVerdict::failed("no goldens".to_string()).passed());
}

#[test]
fn a_reading_with_an_unknown_key_does_not_decode() {
    let (run, native) = fixture();
    let mut value = serde_json::to_value(faithful_reading(&run, &native)).expect("serialises");
    value["cases"][0]["extra"] = json!(1);
    assert!(decode_bench_reading(&value.to_string()).is_err());
    assert!(decode_bench_reading("").is_err());
}
