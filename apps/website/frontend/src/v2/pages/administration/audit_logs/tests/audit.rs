//! Guards on the audit trail: the keyset paths, the cursor, and the load-more append.

use super::*;
use serde_json::json;

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
    let mut lines = vec![json!({"id": 30, "action": "a"})];
    let page = CursorList {
        data: vec![
            json!({"id": 20, "action": "b"}),
            json!({"id": 10, "action": "c"}),
        ],
        next_cursor: Some(json!(10)),
    };
    let cursor = merge_audit_page(&mut lines, page);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1]["id"], 20);
    assert_eq!(cursor, Some(10));
    // The load control must call this path: discarding the cursor yields the first page's own
    // address, and the trail silently truncates.
    assert_eq!(audit_logs_path(cursor), "/admin/audit-logs?before=10");
    assert_ne!(audit_logs_path(cursor), audit_logs_path(None));
}

#[test]
fn empty_page_with_null_cursor_stops() {
    let mut lines: Vec<Value> = Vec::new();
    let page = CursorList {
        data: vec![],
        next_cursor: None,
    };
    let cursor = merge_audit_page(&mut lines, page);
    assert!(lines.is_empty());
    assert_eq!(cursor, None);
    assert_eq!(audit_logs_path(cursor), "/admin/audit-logs");
}

/// Class-R perturbation: a page that *has* a continuation cursor must not be treated like a
/// terminal page. Ignoring `next_cursor` (the old FE) makes `parse_next_cursor` look like
/// `None` and the path collapses to the first page — this asserts the RED difference.
#[test]
fn ignoring_next_cursor_is_detectably_wrong() {
    let page = CursorList {
        data: vec![json!({"id": 20})],
        next_cursor: Some(json!(20)),
    };
    let forwarded = parse_next_cursor(&page.next_cursor);
    let discarded: Option<i64> = None; // the earlier defect: the cursor was never read
    assert_eq!(forwarded, Some(20));
    assert_ne!(
        audit_logs_path(forwarded),
        audit_logs_path(discarded),
        "discarding next_cursor must not produce the same request path as forwarding it"
    );
}

/// Strip `//` and `/* */` so a match cannot be satisfied by a commented-out call.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '*' && matches!(chars.peek(), Some('/')) {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The helper's own unit tests do not pin the load control itself: replacing the trail with the
/// new page instead of appending to it keeps them green while truncating everything already read.
/// This binds to the live load closure, with comments stripped.
#[test]
fn on_load_more_appends_via_merge_audit_page() {
    let production = crate::v2::core::test_support::pins::audit_source();
    let production: &str = &production;
    // Scope the pin to the Load-more closure so a dead string elsewhere cannot false-green.
    let load_more = production
        .split("let on_load_more = move |_|")
        .nth(1)
        .and_then(|rest| rest.split("let master_header =").next())
        .expect("on_load_more closure must sit before master_header");
    let code = collapse_ws(&strip_rust_comments(load_more));

    assert!(
        code.contains("audit_logs_path(Some(before))"),
        "on_load_more must request the continuation page via audit_logs_path(Some(before))"
    );
    assert!(
        code.contains("let cursor = merge_audit_page(&mut rows, page)"),
        "on_load_more Ok-arm must append via merge_audit_page (not replace the trail)"
    );
    assert!(
        code.contains("lines.set(rows)"),
        "on_load_more must write the merged rows back into the lines signal"
    );
    assert!(
        !code.contains("lines.set(page.data)"),
        "on_load_more must not replace the trail with page.data alone \
         (perturbation: lines.set(page.data) truncates prior pages)"
    );
}
