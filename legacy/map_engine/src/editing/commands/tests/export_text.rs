//! Role: the export bytes, the refusal wording, the gesture latch and the metadata projection.
//! Position: `editing/commands/tests` in the map engine.
//! Signals & state: explicit inputs built in the test body.
//! Invariants: the compiled download is proven byte-identical to the wire text, and a value round-trip is proven NOT to be — the difference is the whole point of the function.

use super::*;

use crate::data::scenario::validate::{Finding, Primitive, Severity};

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

/// Measured wire key order from `GET /compiled` (ticket T-417 / ModMissionDocument field order).
const WIRE_ORDER_COMPACT: &[u8] = br#"{"schemaVersion":"1.1","meta":{"id":"m","title":"t","author":"a","terrain":"everon","playerRange":[1,1]},"environment":{"timeOfDay":"0800","weatherPreset":"clear"},"factions":[],"orbat":{},"slots":[{"id":"s1"}],"radioPlan":{"nets":[]},"zones":[],"flow":{"briefingDurationSec":0},"winConditions":{"mode":"none"}}"#;

#[test]
fn class_r_compiled_export_is_byte_identical_to_wire() {
    let out = compiled_export_text(WIRE_ORDER_COMPACT).expect("utf-8");
    assert_eq!(
        out.as_bytes(),
        WIRE_ORDER_COMPACT,
        "Export Compiled must ship compact wire bytes — not a Value pretty-print"
    );
    // Top-level key order pin (the measured /compiled order from T-417).
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
    // Even with serde_json `preserve_order` (unified from map-engine-core / T-220), a
    // Value→pretty round-trip changes bytes (whitespace). The old comment claiming
    // "whitespace-only" while inviting a live `/compiled` compare set a trap; shipping
    // compact makes the download byte-identical to the flatten/`/compiled` body.
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

#[test]
fn class_r_auth_failure_is_not_no_saved_row() {
    let auth = row_meta_missing_message(false);
    let no_row = row_meta_missing_message(true);
    assert!(
        auth.contains("Sign in") || auth.contains("session"),
        "unauthenticated must name auth, got: {auth}"
    );
    assert!(
        !auth.contains("save a version first"),
        "auth failure must not suggest save-first: {auth}"
    );
    assert!(
        no_row.contains("save a version first"),
        "authenticated-but-no-row keeps the save-first remedy: {no_row}"
    );
}

/// A finding shaped like the ones a compile emits.
fn finding(rule_id: &'static str, severity: Severity, subject_id: Option<&str>) -> Finding {
    Finding {
        rule_id,
        severity,
        primitive: Primitive::PerObjectInvariant,
        message: "the compile dropped a value".to_string(),
        subject: "/editor/slots/0/rank".to_string(),
        subject_id: subject_id.map(ToString::to_string),
    }
}

/// A clean compile gets the message it always had — never a celebratory "0 issues".
#[test]
fn a_clean_compile_produces_no_diagnostics_summary() {
    assert_eq!(compile_diagnostics_summary(&[]), None);
}

/// The toast shrinks to a verdict + a pointer: counts by severity, worst first, only the
/// non-zero rungs, correctly pluralised, and it names where the list actually lives.
#[test]
fn the_diagnostics_summary_counts_by_severity_and_points_at_the_panel() {
    let findings = [
        finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
        finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
        finding("COMPILE-DROP-SLOT-TAG", Severity::Info, Some("s1")),
    ];
    let s = compile_diagnostics_summary(&findings).expect("some findings");
    assert!(s.contains("1 warning"), "{s}");
    assert!(s.contains("2 notes"), "{s}");
    assert!(!s.contains("error"), "no zero-count rung may appear: {s}");
    assert!(
        s.contains("validation panel"),
        "the toast must point at the render surface rather than try to be it: {s}"
    );
}

/// **The feed.** The compile's findings reach the T-655 panel through the panel's OWN row type,
/// so a compile finding renders — and click-to-selects on its `subject_id` — exactly like a
/// validation finding. No second panel, no parallel vocabulary.

#[test]
fn t799_same_gesture_stamp_is_a_duplicate() {
    // Two activations of ONE physical click carry the SAME timeStamp (the DOM synthesises the
    // second): the first is let through, the second must be dropped. This is the F-28/F-34
    // "Export Compiled fires twice" bug, decided over the two stamps the intercept would see.
    let stamp = 1234.5_f64;
    assert!(
        !export_gesture_is_duplicate(0.0, stamp),
        "the first activation of a gesture is never a duplicate"
    );
    assert!(
        export_gesture_is_duplicate(stamp, stamp),
        "a second activation with the SAME stamp is the double-fire — drop it"
    );
}

#[test]
fn t799_a_real_second_click_still_fires() {
    // A genuine second intent (a later, distinct stamp) MUST run — the guard is a gesture latch,
    // not a blind debounce that would eat "export again".
    let first = 1000.0_f64;
    let second = 1000.001_f64; // even a sub-ms-later real click has its own stamp
    assert!(
        !export_gesture_is_duplicate(first, second),
        "a distinct later stamp is a new gesture, not a duplicate"
    );
}

#[test]
fn t799_zero_stamp_never_latches() {
    // `Event.timeStamp` is 0.0 when there is no live event — a chord-driven or programmatic
    // export, and the native path. Those do not double-activate through the DOM, and two of them
    // must both run, so 0.0 is never a duplicate (even against a stored 0.0).
    assert!(
        !export_gesture_is_duplicate(0.0, 0.0),
        "a 0.0 stamp is 'no event' and must always pass — never coalesce two of them"
    );
    assert!(
        !export_gesture_is_duplicate(500.0, 0.0),
        "a 0.0 stamp passes regardless of the last real stamp"
    );
}

// ── T-799 (b) — one metadata source: the row for maxPlayers/gameMode, live doc for title ────

/// A `compile_export`-shaped envelope with the hard-coded defaults the review caught.
fn export_envelope_with_defaults() -> serde_json::Value {
    serde_json::json!({
        "exportFormatVersion": 1,
        "missionId": "m",
        "title": "UXREVIEW-Aqqqqqq",
        "terrain": "everon",
        "gameMode": "",
        "weather": "clear",
        "timeOfDay": "06:00",
        "maxPlayers": 0,
        "version": "0.2.0",
        "briefing": "",
        "armory": [],
        "payload": {},
        "exportedAt": "1970-01-01T00:00:00.000Z",
    })
}

#[test]
fn t799_json_export_sources_maxplayers_and_gamemode_from_the_row() {
    // The bug: JSON export carried maxPlayers:0 / gameMode:'' while Compiled carried playerRange
    // [1,64] from the row. After the projection, JSON reads BOTH from the row — the same place
    // the create dialog wrote and Export Compiled reads max_players.
    let out =
        apply_row_metadata_to_export(export_envelope_with_defaults(), Some(64), Some("pve_coop"));
    assert_eq!(out["maxPlayers"], serde_json::json!(64), "maxPlayers ← row");
    assert_eq!(
        out["gameMode"],
        serde_json::json!("pve_coop"),
        "gameMode ← row"
    );
    // The other two legs of the acceptance triple/title are untouched by the projection: version
    // stays the caller's (latest saved semver), title stays the live doc title compile_export read.
    assert_eq!(
        out["version"],
        serde_json::json!("0.2.0"),
        "version untouched"
    );
    assert_eq!(
        out["title"],
        serde_json::json!("UXREVIEW-Aqqqqqq"),
        "title untouched by the metadata projection"
    );
}

#[test]
fn t799_missing_row_leaves_export_defaults_and_still_a_valid_object() {
    // The envelope is the re-importable superset, not the mod document, so a missing row is not a
    // refusal here (it is for Export Compiled). With no row values the defaults stand and the
    // download still goes out.
    let out = apply_row_metadata_to_export(export_envelope_with_defaults(), None, None);
    assert_eq!(
        out["maxPlayers"],
        serde_json::json!(0),
        "no row ⇒ keep compile_export's default"
    );
    assert_eq!(
        out["gameMode"],
        serde_json::json!(""),
        "no row ⇒ keep default"
    );
    assert!(out.is_object(), "still a well-formed envelope");
}

#[test]
fn t799_compiled_title_override_reads_the_live_doc_title() {
    // The compiled export used ROW_META.title (the stale library title); the override reads the
    // LIVE doc title out of small_maps_json — the SAME field compile_export reads for the JSON
    // export — so both exports name the mission identically after a retitle.
    let small = r#"{"meta":{"id":"m","title":"UXREVIEW-Aqqqqqq","terrain":"everon"}}"#;
    assert_eq!(
        live_doc_title(small).as_deref(),
        Some("UXREVIEW-Aqqqqqq"),
        "the live doc title is what both exports must carry"
    );
}

#[test]
fn t799_blank_or_whitespace_live_title_falls_back_to_the_row() {
    // A whitespace-only doc title is not a title (core's meta_title_nonblank rule); the override
    // must return None so the compiled export keeps the row title rather than blanking the name.
    assert_eq!(live_doc_title(r#"{"meta":{"title":"   "}}"#), None);
    assert_eq!(live_doc_title(r#"{"meta":{"title":""}}"#), None);
    assert_eq!(live_doc_title(r#"{"meta":{}}"#), None);
    assert_eq!(live_doc_title("not json"), None);
    // A real title is trimmed, matching the wire emit.
    assert_eq!(
        live_doc_title(r#"{"meta":{"title":"  Trimmed  "}}"#).as_deref(),
        Some("Trimmed")
    );
}
