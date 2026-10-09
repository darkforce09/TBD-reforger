//! Guards on the audit trail: the keyset path, the cursor, the load-more merge and the list and
//! stream goldens.

use super::*;
use serde_json::json;

/// One typed audit line with the given id.
fn line(id: i64) -> AuditLogEntry {
    serde_json::from_value(json!({
        "id": id,
        "severity": "info",
        "action": "user.warn",
        "message": format!("line {id}"),
        "created_at": "2026-07-01T00:00:00Z"
    }))
    .expect("a valid audit line")
}

/// A history page of the given ids and cursor.
fn page_of(ids: &[i64], next_cursor: Option<i64>) -> CursorList<AuditLogEntry> {
    CursorList {
        data: ids.iter().map(|id| line(*id)).collect(),
        next_cursor: next_cursor.map(|c| json!(c)),
    }
}

#[test]
fn continuation_path_forwards_cursor_as_before() {
    assert_eq!(audit_logs_path(Some(42)), "/admin/audit-logs?before=42");
}

#[test]
fn parse_next_cursor_reads_json_number() {
    assert_eq!(parse_next_cursor(&Some(json!(99))), Some(99));
    assert_eq!(parse_next_cursor(&None), None);
    assert_eq!(parse_next_cursor(&Some(Value::Null)), None);
    assert_eq!(parse_next_cursor(&Some(json!("99"))), None);
}

#[test]
fn merge_appends_and_returns_cursor() {
    let mut board = AuditBoard::new();
    let epoch = board.restart();
    assert!(board.merge_history(epoch, page_of(&[30], Some(30))));
    assert!(board.merge_history(epoch, page_of(&[20, 10], Some(10))));
    let ids: Vec<i64> = board.rows().iter().map(|l| l.id.get()).collect();
    assert_eq!(ids, vec![30, 20, 10]);
    let cursor = board.continuation();
    assert_eq!(cursor, Some(10));
    // The load control must call this path: discarding the cursor yields the first page's own
    // address, and the trail silently truncates.
    assert_eq!(audit_logs_path(cursor), "/admin/audit-logs?before=10");
    assert_ne!(audit_logs_path(cursor), audit_logs_path(None));
}

#[test]
fn empty_page_with_null_cursor_stops() {
    let mut board = AuditBoard::new();
    let epoch = board.restart();
    assert!(board.merge_history(epoch, page_of(&[], None)));
    assert!(board.is_empty());
    assert_eq!(board.continuation(), None);
    assert_eq!(audit_logs_path(board.continuation()), "/admin/audit-logs");
}

/// The committed list golden decodes into typed lines, newest id first, with the optional fields
/// absent where the contract leaves them out.
#[test]
fn the_list_golden_decodes_into_typed_lines() {
    let golden = frontend_test_support::fixtures::golden!("GET__admin__audit-logs.json");
    let page: CursorList<AuditLogEntry> =
        serde_json::from_str(golden).expect("the list golden decodes as typed audit lines");
    let mut board = AuditBoard::new();
    let epoch = board.restart();
    assert!(board.merge_history(epoch, page));
    let ids: Vec<i64> = board.rows().iter().map(|l| l.id.get()).collect();
    assert_eq!(ids, (1..=15).rev().collect::<Vec<i64>>());
    let system = board.get(4).expect("line 4 is in the golden");
    assert_eq!(system.actor_id, None);
    assert!(system.actor_name.is_empty());
    assert_eq!(board.continuation(), None);
}

/// The stream fixture the DOM oracle serves opens with a `ready` that agrees with the list golden.
#[test]
fn the_stream_golden_opens_with_ready() {
    use frontend_transport::audit_stream::{AuditStreamStep, AuditStreamTracker};
    use frontend_transport::sse_frames::SseParser;
    let golden = frontend_test_support::fixtures::golden!("GET__admin__audit-logs__stream.sse.txt");
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let steps: Vec<AuditStreamStep> = SseParser::new()
        .feed(golden.as_bytes())
        .into_iter()
        .map(|item| tracker.observe(item))
        .collect();
    assert!(matches!(
        steps[..],
        [AuditStreamStep::Ready {
            history_required: true,
            ..
        }]
    ));
}
