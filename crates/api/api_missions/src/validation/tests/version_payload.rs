use super::*;
use axum::http::StatusCode;

/// The extractor accepts trimmed non-blank, rejects blank / whitespace / absent.
#[test]
fn payload_title_for_row_mirror_nonblank_trim() {
    assert_eq!(
        payload_title_for_row_mirror(r#"{"title":"  Authored Op  "}"#).as_deref(),
        Some("Authored Op")
    );
    assert_eq!(payload_title_for_row_mirror(r#"{"title":"   "}"#), None);
    assert_eq!(payload_title_for_row_mirror(r#"{"title":""}"#), None);
    assert_eq!(payload_title_for_row_mirror(r#"{"editor":{}}"#), None);
    assert_eq!(payload_title_for_row_mirror("not-json"), None);
}

/// Vacuous vs draft-skeleton accept set. `{}` is the measured hole (schema-valid, becomes
/// `current_version_id`, no API rollback). Explicit `editor.slots` (even `[]`) is the draft shape
/// integration tests and WIP saves use, and must stay accepted.
///
/// RED: make `version_payload_is_vacuous` always return `false` — the empty cases below fail.
#[test]
fn version_payload_vacuous_rejects_empty_keeps_editor_skeleton() {
    for empty in [
        "{}",
        r#"{"editor":{}}"#,
        r#"{"title":"x"}"#,
        r#"{"schemaVersion":1}"#,
        r#"{"map":{"terrain":"everon"}}"#,
        r#"{"environment":{}}"#,
        r#"{"orbat":[]}"#,
        "null",
        "[]",
    ] {
        assert!(
            version_payload_is_vacuous(empty),
            "expected vacuous: {empty}"
        );
    }
    for ok in [
        r#"{"editor":{"slots":[]}}"#,
        r#"{"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#,
        r#"{"schemaVersion":1,"editor":{"slots":[{"id":"s1"}]}}"#,
        r#"{"orbat":[{"faction":"BLUFOR","callsign":"A","squad":"Alpha","slots":[]}]}"#,
        r#"{"objectives":[{"id":"o1"}]}"#,
        r#"{"vehicles":[{"id":"v1"}]}"#,
        r#"{"markers":[{"id":"m1"}]}"#,
        r#"{"entities":[{"id":"e1"}]}"#,
        r#"{"editor":{"factions":[{"id":"f1","key":"BLUFOR","name":"US","squadIds":[]}]}}"#,
    ] {
        assert!(
            !version_payload_is_vacuous(ok),
            "expected non-vacuous: {ok}"
        );
    }
    // Malformed JSON: not our message — the schema pass owns it.
    assert!(!version_payload_is_vacuous("not-json"));
}

/// The helper surfaces a 400 with the refuse message .
#[test]
fn reject_vacuous_version_payload_is_bad_request() {
    let err = reject_vacuous_version_payload("{}").expect_err("empty must 400");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(
        err.message.contains("empty payload"),
        "error must name empty payload; got {:?}",
        err.message
    );
    assert!(reject_vacuous_version_payload(r#"{"editor":{"slots":[]}}"#).is_ok());
}
