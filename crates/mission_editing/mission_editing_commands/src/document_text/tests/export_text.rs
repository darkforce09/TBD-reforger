//! **Role:** the export bytes of the compiled download.
//! **Position:** the `tests` module of `document_text::export_text` in `mission_editing_commands`.
//! **Signals & state:** explicit inputs built in the test body.
//! **Invariants:** the compiled download is proven byte-identical to the wire text, and a value
//! round-trip is proven NOT to be — the difference is the whole point of the function.

use super::*;

/// First-level object key order from a JSON object string (no full parse → no Map reorder).
fn raw_top_level_keys(json: &str) -> Vec<&str> {
    let mut keys = Vec::new();
    let bytes = json.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    assert_eq!(bytes.get(i), Some(&b'{'));
    i += 1;
    let mut depth = 1u32;
    let mut in_string = false;
    let mut escape = false;
    let mut key_start: Option<usize> = None;
    let mut expect_key = true;
    while i < bytes.len() {
        let b = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_string = false;
                if let (true, Some(s)) = (expect_key && depth == 1, key_start) {
                    keys.push(&json[s..i]);
                    expect_key = false;
                }
                key_start = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'"' => {
                in_string = true;
                if expect_key && depth == 1 {
                    key_start = Some(i + 1);
                }
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            b',' if depth == 1 => expect_key = true,
            b':' if depth == 1 => expect_key = false,
            _ => {}
        }
        i += 1;
    }
    keys
}

/// Measured wire key order from `GET /compiled` (the ModMissionDocument field order).
const WIRE_ORDER_COMPACT: &[u8] = br#"{"schemaVersion":"1.1","meta":{"id":"m","title":"t","author":"a","terrain":"everon","playerRange":[1,1]},"environment":{"timeOfDay":"0800","weatherPreset":"clear"},"factions":[],"orbat":{},"slots":[{"id":"s1"}],"radioPlan":{"nets":[]},"zones":[],"flow":{"briefingDurationSec":0},"winConditions":{"mode":"none"}}"#;

#[test]
fn class_r_compiled_export_is_byte_identical_to_wire() {
    let out = compiled_export_text(WIRE_ORDER_COMPACT).expect("utf-8");
    assert_eq!(
        out.as_bytes(),
        WIRE_ORDER_COMPACT,
        "Export Compiled must ship compact wire bytes — not a Value pretty-print"
    );
    // Top-level key order pin (the measured /compiled order).
    // Read order from the raw text — `serde_json::Value` would BTreeMap-sort without preserve_order.
    assert_eq!(
        raw_top_level_keys(&out),
        [
            "schemaVersion",
            "meta",
            "environment",
            "factions",
            "orbat",
            "slots",
            "radioPlan",
            "zones",
            "flow",
            "winConditions",
        ]
    );
}

#[test]
fn class_r_value_pretty_print_is_not_byte_identical() {
    // Even with serde_json `preserve_order` (which the workspace enables), a
    // Value→pretty round-trip changes bytes (whitespace), so a "whitespace-only" pretty download
    // would fail a live `/compiled` compare; shipping compact makes the download byte-identical
    // to the flatten/`/compiled` body.
    let value: serde_json::Value = serde_json::from_slice(WIRE_ORDER_COMPACT).unwrap();
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert_ne!(
        pretty.as_bytes(),
        WIRE_ORDER_COMPACT,
        "pretty-print must differ from compact wire bytes"
    );
    let shipped = compiled_export_text(WIRE_ORDER_COMPACT).unwrap();
    assert_eq!(shipped.as_bytes(), WIRE_ORDER_COMPACT);
    // Key order must still match wire (preserve_order keeps it; compact never reorders).
    assert_eq!(raw_top_level_keys(&shipped), raw_top_level_keys(&pretty));
    assert_eq!(
        raw_top_level_keys(&shipped)[0],
        "schemaVersion",
        "wire order starts with schemaVersion, not alpha environment"
    );
}
