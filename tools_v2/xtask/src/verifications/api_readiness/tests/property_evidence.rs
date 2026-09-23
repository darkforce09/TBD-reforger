//! Property evidence binds observed executions to required IDs and explicit runner configuration.

use super::super::{
    fingerprint,
    register::{EvidenceClass, PropertyRequirement},
};
use super::*;
use serde_json::{Value, json};

fn configuration() -> PropertyTestConfiguration {
    PropertyTestConfiguration { rng_seed: 42 }
}

fn check() -> Check {
    Check {
        id: "quota_properties".into(),
        class: EvidenceClass::Property,
        command: Some(vec!["cargo".into(), "test".into()]),
        timeout_seconds: 60,
        minimum_cases: 1,
        success_marker: "test result: ok.".into(),
        case_pattern: r"(?m)^test quota_conservation \.\.\. ok$".into(),
        properties: vec![PropertyRequirement {
            id: "quota_conservation".into(),
            minimum_cases: 16,
        }],
    }
}

fn record(id: &str, requested_cases: u32, executed_cases: u32) -> PropertyRun {
    PropertyRun {
        version: 1,
        id: id.into(),
        requested_cases,
        executed_cases,
        seed: 42,
        algorithm: "ChaCha".into(),
        input_sha256: "a".repeat(64),
    }
}

fn evidence(records: Vec<PropertyRun>) -> (Receipt, String) {
    let mut output = format!("{}\n", configuration().marker());
    for record in &records {
        output.push_str(&format!(
            "property-run: {}\n",
            serde_json::to_string(record).unwrap()
        ));
    }
    output.push_str(
        "test quota_conservation ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n",
    );
    let property_runs = parse(&output).expect("fixture records have unique IDs and valid JSON");
    let receipt = Receipt {
        version: 1,
        check_id: "quota_properties".into(),
        class: EvidenceClass::Property,
        source_sha256: "source".into(),
        configuration_sha256: "configuration".into(),
        command: vec!["cargo".into(), "test".into()],
        tool_versions: vec!["proptest 1.11.0".into()],
        started_unix_seconds: 100,
        duration_milliseconds: 10,
        exit_code: 0,
        output_file: "quota_properties.log".into(),
        output_sha256: fingerprint::digest(output.as_bytes()),
        environment: configuration().receipt_environment(),
        observations: None,
        property_runs,
    };
    (receipt, output)
}

fn fixture() -> (Check, Receipt, String) {
    let (receipt, output) = evidence(vec![record("quota_conservation", 16, 16)]);
    (check(), receipt, output)
}

fn validates(check: &Check, receipt: &Receipt, output: &str) -> Result<()> {
    validate_with_configuration(check, receipt, output, configuration())
}

#[test]
fn complete_observed_counts_at_or_above_the_requirement_are_accepted() {
    for cases in [16, 32, u32::MAX] {
        let (receipt, output) = evidence(vec![record("quota_conservation", cases, cases)]);
        validates(&check(), &receipt, &output).unwrap();
    }
}

#[test]
fn zero_under_minimum_partial_and_excess_execution_counts_are_rejected() {
    for (requested, executed) in [(0, 0), (15, 15), (16, 0), (16, 15), (16, 17)] {
        let (receipt, output) = evidence(vec![record("quota_conservation", requested, executed)]);
        assert!(
            validates(&check(), &receipt, &output).is_err(),
            "accepted {requested}/{executed}"
        );
    }
}

#[test]
fn empty_required_properties_cannot_pass() {
    let (mut check, receipt, output) = fixture();
    check.properties.clear();
    assert!(validates(&check, &receipt, &output).is_err());
}

#[test]
fn successful_unrelated_property_cannot_replace_a_missing_required_id() {
    let (receipt, output) = evidence(vec![record("unrelated_property", 512, 512)]);
    assert!(validates(&check(), &receipt, &output).is_err());
    let (receipt, output) = evidence(vec![]);
    assert!(validates(&check(), &receipt, &output).is_err());
}

#[test]
fn every_required_id_must_have_its_own_sufficient_execution() {
    let (mut check, receipt, output) = fixture();
    check.properties.push(PropertyRequirement {
        id: "opening_boundaries".into(),
        minimum_cases: 32,
    });
    assert!(validates(&check, &receipt, &output).is_err());
    let (receipt, output) = evidence(vec![
        record("quota_conservation", 16, 16),
        record("opening_boundaries", 31, 31),
    ]);
    assert!(validates(&check, &receipt, &output).is_err());
    let (receipt, output) = evidence(vec![
        record("quota_conservation", 16, 16),
        record("opening_boundaries", 32, 32),
    ]);
    validates(&check, &receipt, &output).unwrap();
}

#[test]
fn parsing_sorts_property_records_by_id_independently_of_completion_order() {
    let (receipt, output) = evidence(vec![
        record("zeta_property", 32, 32),
        record("quota_conservation", 16, 16),
        record("alpha_property", 32, 32),
    ]);
    let parsed = parse(&output).unwrap();
    let ids: Vec<_> = parsed.iter().map(|record| record.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["alpha_property", "quota_conservation", "zeta_property"]
    );
    assert_eq!(parsed, receipt.property_runs);
    validates(&check(), &receipt, &output).unwrap();
}

#[test]
fn duplicate_execution_ids_are_rejected_even_when_records_are_identical() {
    let first = record("quota_conservation", 16, 16);
    for second in [first.clone(), record("quota_conservation", 32, 32)] {
        let (receipt, mut output) = evidence(vec![first.clone()]);
        output.push_str(&format!(
            "property-run: {}\n",
            serde_json::to_string(&second).unwrap()
        ));
        assert!(parse(&output).is_err());
        assert!(validates(&check(), &receipt, &output).is_err());
    }
}

#[test]
fn repeated_partial_records_cannot_be_summed_into_a_complete_execution() {
    let partial = record("quota_conservation", 16, 8);
    let (receipt, mut output) = evidence(vec![partial.clone()]);
    output.push_str(&format!(
        "property-run: {}\n",
        serde_json::to_string(&partial).unwrap()
    ));
    assert!(parse(&output).is_err());
    assert!(validates(&check(), &receipt, &output).is_err());
}

#[test]
fn malformed_json_unknown_fields_and_missing_fields_are_rejected() {
    for raw in ["", "{", "null", "[]", "{}", "{\"id\": 42}", "{} trailing"] {
        assert!(
            parse(&format!("property-run: {raw}\n")).is_err(),
            "accepted {raw:?}"
        );
    }
    let valid = serde_json::to_value(record("quota_conservation", 16, 16)).unwrap();
    let mut extra = valid.clone();
    extra["success"] = json!(true);
    assert!(parse(&format!("property-run: {extra}\n")).is_err());
    for missing in [
        "version",
        "id",
        "requested_cases",
        "executed_cases",
        "seed",
        "algorithm",
        "input_sha256",
    ] {
        let mut incomplete = valid.clone();
        incomplete.as_object_mut().unwrap().remove(missing);
        assert!(
            parse(&format!("property-run: {incomplete}\n")).is_err(),
            "accepted missing {missing}"
        );
    }
}

#[test]
fn wrong_numeric_shapes_and_bounds_cannot_become_observed_counts() {
    for (field, value) in [
        ("requested_cases", json!(-1)),
        ("executed_cases", json!(u64::from(u32::MAX) + 1)),
        ("executed_cases", json!(1.0)),
        ("executed_cases", json!("16")),
        ("executed_cases", json!(false)),
        ("seed", Value::Null),
    ] {
        let mut value_record = serde_json::to_value(record("quota_conservation", 16, 16)).unwrap();
        value_record[field] = value;
        assert!(parse(&format!("property-run: {value_record}\n")).is_err());
    }
}

#[test]
fn changed_seed_algorithm_or_version_is_rejected_even_when_receipt_matches_log() {
    for (field, value) in [
        ("seed", json!(43)),
        ("algorithm", json!("XorShift")),
        ("algorithm", json!("chacha")),
        ("version", json!(0)),
        ("version", json!(2)),
    ] {
        let mut value_record = serde_json::to_value(record("quota_conservation", 16, 16)).unwrap();
        value_record[field] = value;
        let changed = serde_json::from_value(value_record).unwrap();
        let (receipt, output) = evidence(vec![changed]);
        assert!(
            validates(&check(), &receipt, &output).is_err(),
            "accepted changed {field}"
        );
    }
}

#[test]
fn missing_malformed_or_noncanonical_input_digests_are_rejected() {
    for digest in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "g".repeat(64),
        "A".repeat(64),
    ] {
        let mut changed = record("quota_conservation", 16, 16);
        changed.input_sha256 = digest;
        let (receipt, output) = evidence(vec![changed]);
        assert!(validates(&check(), &receipt, &output).is_err());
    }
}

#[test]
fn missing_or_duplicate_receipt_environment_markers_are_rejected() {
    for index in 0..2 {
        let (check, mut receipt, output) = fixture();
        receipt.environment.remove(index);
        assert!(validates(&check, &receipt, &output).is_err());
        let (check, mut receipt, output) = fixture();
        receipt.environment.push(receipt.environment[index].clone());
        assert!(validates(&check, &receipt, &output).is_err());
    }
    let (check, mut receipt, output) = fixture();
    receipt.environment[0] = "PROPTEST_RNG_SEED=43".into();
    assert!(validates(&check, &receipt, &output).is_err());
}

#[test]
fn missing_wrong_or_embedded_log_configuration_marker_is_rejected() {
    for replacement in [
        "",
        "property-test-configuration: rng_seed=43; cases=suite-defined",
        "property-test-configuration: rng_seed=42; cases=0",
        "prefix property-test-configuration: rng_seed=42; cases=suite-defined",
    ] {
        let (check, receipt, output) = fixture();
        let changed = output.replace(&configuration().marker(), replacement);
        assert!(
            validates(&check, &receipt, &changed).is_err(),
            "accepted marker {replacement:?}"
        );
    }
}

#[test]
fn duplicate_or_conflicting_log_configuration_markers_are_rejected() {
    for extra in [
        configuration().marker(),
        "property-test-configuration: rng_seed=43; cases=suite-defined".into(),
        "property-test-configuration: rng_seed=42; cases=0".into(),
    ] {
        let (check, receipt, output) = fixture();
        let changed = format!("{output}{extra}\n");
        assert!(
            validates(&check, &receipt, &changed).is_err(),
            "accepted extra marker {extra:?}"
        );
    }
}

#[test]
fn extra_conflicting_receipt_configuration_values_are_rejected() {
    for extra in [
        "PROPTEST_RNG_SEED=43",
        "PROPTEST_CASES=0",
        "PROPTEST_CASES=16",
    ] {
        let (check, mut receipt, output) = fixture();
        receipt.environment.push(extra.into());
        assert!(
            validates(&check, &receipt, &output).is_err(),
            "accepted conflict {extra}"
        );
    }
}

#[test]
fn receipt_changes_cannot_override_measured_log_records() {
    for (field, value) in [
        ("input_sha256", json!("b".repeat(64))),
        ("seed", json!(43)),
        ("requested_cases", json!(32)),
        ("executed_cases", json!(32)),
    ] {
        let (check, mut receipt, output) = fixture();
        let mut changed = serde_json::to_value(&receipt.property_runs[0]).unwrap();
        changed[field] = value;
        receipt.property_runs[0] = serde_json::from_value(changed).unwrap();
        assert!(
            validates(&check, &receipt, &output).is_err(),
            "accepted receipt-only {field}"
        );
    }
    let (check, mut receipt, output) = fixture();
    receipt.property_runs.clear();
    assert!(validates(&check, &receipt, &output).is_err());
    let (check, mut receipt, output) = fixture();
    receipt.property_runs.push(receipt.property_runs[0].clone());
    assert!(validates(&check, &receipt, &output).is_err());
}

#[test]
fn changed_log_digest_is_rejected_when_receipt_retains_the_original_record() {
    let (check, receipt, output) = fixture();
    let changed = output.replace(&"a".repeat(64), &"b".repeat(64));
    assert_ne!(changed, output);
    assert!(validates(&check, &receipt, &changed).is_err());
}
