//! Field-by-field differences between a golden and a live JSON answer.
//!
//! **Role:** names every JSON pointer at which two values differ, so a reproduction failure says
//! which field of which golden moved rather than only that the documents differ.
//!
//! **Position:** used by the reproduction and event-stream cases of `contract_parity_goldens`
//! (through `mod contract_parity_support;`), by the catalogue row comparison of the same binary,
//! and by `contract_parity_equipment_viewer`, which compiles this file alone through a `#[path]`
//! module; it depends on `serde_json` only.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** equality is `serde_json::Value` equality (object key order is irrelevant,
//! array order and number representation are not); no field is skipped or normalised, so an empty
//! result means the two values are equal.

use serde_json::Value;

/// Longest rendering of one value inside a report line.
const EXCERPT_CHARS: usize = 160;

/// Every pointer at which `live` differs from `golden`, one report line each.
pub(crate) fn differences(golden: &Value, live: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    walk("", golden, live, &mut lines);
    lines
}

fn walk(pointer: &str, golden: &Value, live: &Value, lines: &mut Vec<String>) {
    match (golden, live) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, expected_value) in expected {
                let child = child_pointer(pointer, key);
                match actual.get(key) {
                    Some(actual_value) => walk(&child, expected_value, actual_value, lines),
                    None => lines.push(format!(
                        "{child}: missing from the live answer (golden {})",
                        excerpt(expected_value)
                    )),
                }
            }
            for (key, actual_value) in actual {
                if !expected.contains_key(key) {
                    lines.push(format!(
                        "{}: only in the live answer ({})",
                        child_pointer(pointer, key),
                        excerpt(actual_value)
                    ));
                }
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                lines.push(format!(
                    "{}: golden has {} elements, live has {}",
                    shown(pointer),
                    expected.len(),
                    actual.len()
                ));
            }
            for (index, (expected_value, actual_value)) in expected.iter().zip(actual).enumerate() {
                walk(
                    &child_pointer(pointer, &index.to_string()),
                    expected_value,
                    actual_value,
                    lines,
                );
            }
        }
        _ if golden == live => {}
        _ => lines.push(format!(
            "{}: golden {} but live {}",
            shown(pointer),
            excerpt(golden),
            excerpt(live)
        )),
    }
}

/// `pointer` extended by one RFC 6901 reference token.
fn child_pointer(pointer: &str, token: &str) -> String {
    format!("{pointer}/{}", token.replace('~', "~0").replace('/', "~1"))
}

/// The document root reads as `/` in reports.
fn shown(pointer: &str) -> &str {
    if pointer.is_empty() { "/" } else { pointer }
}

/// Compact JSON, cut to [`EXCERPT_CHARS`] characters.
pub(crate) fn excerpt(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > EXCERPT_CHARS {
        let cut: String = text.chars().take(EXCERPT_CHARS).collect();
        format!("{cut}…")
    } else {
        text
    }
}
