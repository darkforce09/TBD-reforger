//! Source pin for the match upsert's UUID parsing: helper-only tests stay green if the call
//! sites regress to the soft parser, so the call sites themselves are asserted on stripped
//! source.

use crate::match_telemetry::handlers::ingest_parsing::tests::{
    collapse_ws, production_half, strip_rust_comments,
};

const UPSERT_SRC: &str = include_str!("../match_upsert.rs");

/// `upsert_match` must call the strict helper for both fields.
#[test]
fn upsert_match_uses_strict_uuid_for_event_and_mission() {
    let production = production_half(UPSERT_SRC);
    let start = production
        .find("async fn upsert_match")
        .expect("upsert_match must exist");
    let after = &production[start..];
    let end = after[1..]
        .find("\nasync fn ")
        .or_else(|| after[1..].find("\npub(super) async fn "))
        .map(|i| i + 1)
        .unwrap_or(after.len());
    let body = strip_rust_comments(&after[..end]);
    let collapsed = collapse_ws(&body);
    assert!(
        collapsed.contains(r#"parse_uuid_opt_strict("event_id", &m.event_id)?"#),
        "upsert_match must reject unparseable event_id via parse_uuid_opt_strict"
    );
    assert!(
        collapsed.contains(r#"parse_uuid_opt_strict("mission_id", &m.mission_id)?"#),
        "upsert_match must reject unparseable mission_id via parse_uuid_opt_strict"
    );
    // Soft helper must remain the current_match_id path (ingest_server_status), not
    // silently replaced at the match upsert sites.
    assert!(
        !collapsed.contains("parse_uuid_opt(&m.event_id)"),
        "event_id must not use soft parse_uuid_opt"
    );
    assert!(
        !collapsed.contains("parse_uuid_opt(&m.mission_id)"),
        "mission_id must not use soft parse_uuid_opt"
    );
}
