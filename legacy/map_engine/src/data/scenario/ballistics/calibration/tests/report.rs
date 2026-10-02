//! Tests of the calibration report's wire projection: the upload report's keys, the count of
//! forward-angle samples left unjudged, and the typed failure kind kept off the wire.

use super::*;

fn report_with_one_failure() -> CalibrationReport {
    let mut report = CalibrationReport {
        cases: 414,
        interpolated_forward_samples: 15_427,
        ..CalibrationReport::default()
    };
    report.fail(
        FailureKind::NativeRowTimeOfFlight,
        "native/m821-he/1/3".to_owned(),
        "time of flight 9.4 s against 9.6 s".to_owned(),
    );
    report
}

#[test]
fn report_serialises_the_forward_samples_not_judged() {
    let value = serde_json::to_value(report_with_one_failure()).expect("serialises");
    let object = value.as_object().expect("an object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["cases", "failures", "forward_samples_not_judged"]);
    assert_eq!(value["cases"], serde_json::json!(414));
    assert_eq!(
        value["forward_samples_not_judged"],
        serde_json::json!(15_427)
    );
    assert_eq!(
        value["failures"],
        serde_json::json!([{
            "case_id": "native/m821-he/1/3",
            "reason": "time of flight 9.4 s against 9.6 s",
        }])
    );
}

#[test]
fn absorbing_a_report_adds_its_unjudged_forward_samples() {
    let mut total = report_with_one_failure();
    total.absorb(CalibrationReport {
        cases: 6,
        interpolated_forward_samples: 3,
        ..CalibrationReport::default()
    });
    assert_eq!(
        (total.cases, total.interpolated_forward_samples),
        (420, 15_430)
    );
    assert_eq!(total.failures.len(), 1);
    assert!(!total.accepted());
    let value = serde_json::to_value(&total).expect("serialises");
    assert_eq!(
        value["forward_samples_not_judged"],
        serde_json::json!(15_430)
    );
}

#[test]
fn an_empty_report_accepts_and_serialises_a_zero_count() {
    let report = CalibrationReport::default();
    assert!(report.accepted());
    let value = serde_json::to_value(&report).expect("serialises");
    assert_eq!(value["forward_samples_not_judged"], serde_json::json!(0));
}
