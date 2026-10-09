//! **Role:** unit tests of [`crate::water_export_images::json_field_reading`]: which values count
//! as given, the fallback order of [`first_given`], the number readers and the NaN coordinate.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/json_field_reading.rs`.
//! **Signals & state:** none; pure calls.
//! **Invariants:** each case names the value class it pins.

use serde_json::json;

use super::*;

#[test]
fn null_false_zero_and_empty_text_are_not_given() {
    for value in [json!(null), json!(false), json!(0), json!(0.0), json!("")] {
        assert!(!is_given(&value), "{value}");
    }
}

#[test]
fn empty_arrays_and_objects_and_negative_numbers_are_given() {
    for value in [
        json!([]),
        json!({}),
        json!(-1),
        json!(true),
        json!("x"),
        json!(0.5),
    ] {
        assert!(is_given(&value), "{value}");
    }
}

#[test]
fn first_given_skips_absent_values_but_stops_at_an_empty_array() {
    let document = json!({"lakes": null, "waterBodies": [], "other": [1]});
    assert_eq!(
        first_given(&document, &["lakes", "waterBodies", "other"]),
        Some(&json!([]))
    );
    assert_eq!(first_given(&document, &["missing", "lakes"]), None);
    assert_eq!(
        first_given(&json!([1, 2]), &["lakes"]),
        None,
        "an array has no fields"
    );
}

#[test]
fn given_number_falls_back_on_zero_and_non_numbers() {
    assert_eq!(given_number(Some(&json!(3))), Some(3.0));
    assert_eq!(given_number(Some(&json!(-2.5))), Some(-2.5));
    assert_eq!(given_number(Some(&json!(0))), None);
    assert_eq!(given_number(Some(&json!("3"))), None);
    assert_eq!(given_number(None), None);
}

#[test]
fn json_number_keeps_zero_and_negatives() {
    assert_eq!(json_number(Some(&json!(0))), Some(0.0));
    assert_eq!(json_number(Some(&json!(-4))), Some(-4.0));
    assert_eq!(json_number(Some(&json!(true))), None);
    assert_eq!(json_number(None), None);
}

#[test]
fn a_missing_or_non_number_coordinate_is_nan() {
    let point = json!([1.5, "y"]);
    assert_eq!(coordinate(Some(&point), 0), 1.5);
    assert!(coordinate(Some(&point), 1).is_nan());
    assert!(coordinate(Some(&point), 2).is_nan());
    assert!(coordinate(Some(&json!({"0": 1})), 0).is_nan());
    assert!(coordinate(None, 0).is_nan());
}
