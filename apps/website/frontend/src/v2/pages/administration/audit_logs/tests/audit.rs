//! Guards on the audit trail: the keyset paths, the cursor, the load-more merge, the list golden
//! and the stream teardown.

use super::*;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
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
fn first_page_path_has_no_before() {
    assert_eq!(audit_logs_path(None), "/admin/audit-logs");
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
    let ids: Vec<i64> = board.rows().iter().map(|l| l.id).collect();
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

/// Class-R perturbation: a page that *has* a continuation cursor must not be treated like a
/// terminal page. Ignoring `next_cursor` makes the continuation look like `None` and the path
/// collapses to the first page — this asserts the RED difference.
#[test]
fn ignoring_next_cursor_is_detectably_wrong() {
    let mut board = AuditBoard::new();
    let epoch = board.restart();
    board.merge_history(epoch, page_of(&[20], Some(20)));
    let forwarded = board.continuation();
    let discarded: Option<i64> = None; // the defect this guards against: the cursor never read
    assert_eq!(forwarded, Some(20));
    assert_ne!(
        audit_logs_path(forwarded),
        audit_logs_path(discarded),
        "discarding next_cursor must not produce the same request path as forwarding it"
    );
}

/// The committed list golden decodes into typed lines, newest id first, with the optional fields
/// absent where the contract leaves them out.
#[test]
fn the_list_golden_decodes_into_typed_lines() {
    let golden = include_str!("../../../../../../tests/fixtures/api/GET__admin__audit-logs.json");
    let page: CursorList<AuditLogEntry> =
        serde_json::from_str(golden).expect("the list golden decodes as typed audit lines");
    let mut board = AuditBoard::new();
    let epoch = board.restart();
    assert!(board.merge_history(epoch, page));
    let ids: Vec<i64> = board.rows().iter().map(|l| l.id).collect();
    assert_eq!(ids, (1..=10).rev().collect::<Vec<i64>>());
    let system = board.get(4).expect("line 4 is in the golden");
    assert_eq!(system.actor_id, None);
    assert!(system.actor_name.is_empty());
    assert_eq!(board.continuation(), None);
}

/// The stream fixture the DOM oracle serves opens with a `ready` that agrees with the list golden.
#[test]
fn the_stream_golden_opens_with_ready() {
    use crate::v2::core::api::audit_stream::{AuditStreamStep, AuditStreamTracker};
    use crate::v2::core::api::sse_frames::SseParser;
    let golden =
        include_str!("../../../../../../tests/fixtures/api/GET__admin__audit-logs__stream.sse.txt");
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

/// The helper's own unit tests do not pin the load control itself: replacing the board with the
/// new page instead of merging into it keeps them green while truncating everything already read.
/// This binds to the live load closure, with comments and literals blanked.
#[test]
fn on_load_more_merges_into_the_board() {
    let production = live_code(&crate::v2::core::test_support::pins::audit_source());
    // Scope the pin to the Load-more closure so a dead string elsewhere cannot false-green.
    let load_more = production
        .split("let on_load_more = move |_|")
        .nth(1)
        .and_then(|rest| rest.split("let master_header =").next())
        .expect("on_load_more closure must sit before master_header");
    let code: String = load_more.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        code.contains("b.continuation()") && code.contains("audit_logs_path(Some(before))"),
        "on_load_more must request the page below the board's smallest history id"
    );
    assert!(
        code.contains("board.try_update(|b| b.merge_history(epoch, page))"),
        "on_load_more must merge the page into the board under the epoch it was requested in"
    );
    assert!(
        !code.contains("restart()") && !code.contains("board.set("),
        "on_load_more must not empty or replace the board (perturbation: that truncates the \
         pages already read)"
    );
}

/// The stream must die with the page: the route opens it once, keeps the handle itself, and
/// aborts it on cleanup — on live code, not in prose.
#[test]
fn the_route_aborts_its_stream_on_cleanup() {
    let production = live_code(&crate::v2::core::test_support::pins::audit_source());
    let feed = only_body(&production, "fn connect_live_feed(");
    assert!(feed.contains("StoredValue::new_local(open_audit_stream(store, callbacks))"));
    assert!(feed.contains("on_cleanup(move ||"));
    assert!(feed.contains("handle.try_with_value(|live| live.abort())"));
    assert!(
        !production.contains("thread_local!"),
        "the stream handle belongs to the route, never to a global slot"
    );
}

/// The history waits for the stream: the route loads it from the stream's callbacks only.
#[test]
fn the_route_loads_history_only_from_the_stream_callbacks() {
    let production = live_code(&crate::v2::core::test_support::pins::audit_source());
    let inner = only_body(&production, "fn AuditLogsInner(");
    assert!(inner.contains("connect_live_feed(store, board, stream, history, rejected)"));
    assert!(!inner.contains("reload_history("));
    assert!(!inner.contains("LocalResource"));
    let feed = only_body(&production, "fn connect_live_feed(");
    assert_eq!(
        feed.matches("reload_history(store, board, history)")
            .count(),
        3
    );
}
