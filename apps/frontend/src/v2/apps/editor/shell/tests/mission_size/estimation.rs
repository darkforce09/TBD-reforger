//! Mission Size tests tests.

use super::*;

#[test]
fn empty_slots_is_none() {
    assert_eq!(estimate_compiled_bytes("{}"), None);
    assert_eq!(estimate_compiled_bytes("not json"), None);
}

#[test]
fn small_set_is_avg_times_n_plus_envelope() {
    // Two identical slots → avg = len(one), estimate = 2·len + envelope.
    let one = r#"{"x":1.0,"y":2.0,"role":"Rifleman"}"#;
    let slots = format!(r#"{{"s0":{one},"s1":{one}}}"#);
    let est = estimate_compiled_bytes(&slots).unwrap();
    let one_len = serde_json::from_str::<serde_json::Value>(one)
        .unwrap()
        .to_string()
        .len();
    assert_eq!(est, one_len * 2 + SIZE_ENVELOPE_BYTES);
}

#[test]
fn format_bytes_units() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(2_048), "2 KB");
    assert_eq!(format_bytes(141_574_630), "141.6 MB");
    assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
}
