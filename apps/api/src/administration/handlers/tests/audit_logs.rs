use super::*;

/// formula-leading cells must be prefixed so Excel/Sheets do not execute them.
///
/// RED: delete the `Some(b'=' | …)` arm (or make the helper return `Cow::Borrowed` always) —
/// `assert!(escaped.starts_with('\''))` fails and raw `=cmd` survives.
#[test]
fn escape_csv_formula_prefixes_equals_plus_minus_at() {
    for dangerous in [
        "=cmd|'/C calc'!A0",
        "=HYPERLINK(\"http://evil\")",
        "+1+1",
        "-1+1",
        "@SUM(A1)",
    ] {
        let escaped = escape_csv_formula(dangerous);
        assert!(
            escaped.starts_with('\''),
            "formula cell {dangerous:?} must be quote-prefixed, got {escaped:?}"
        );
        assert_eq!(&escaped[1..], dangerous);
    }
    // Safe cells pass through unchanged (including empty and leading digit/letter).
    assert_eq!(escape_csv_formula(""), "");
    assert_eq!(escape_csv_formula("alice"), "alice");
    assert_eq!(escape_csv_formula("mission.updated"), "mission.updated");
    assert_eq!(escape_csv_formula("9=ok"), "9=ok");
}

/// the export writer path (same `csv::Writer` + `escape_csv_formula` as
/// [`export_audit_logs_csv`]) must not emit a record whose decoded field still starts with a
/// formula character. A helper-only green with a raw write path is a false green.
///
/// RED: write `&l.message` (etc.) without `escape_csv_formula` — the decoded field is `=cmd…`
/// and this assert fires.
#[test]
fn export_writer_path_prefixes_formula_cells() {
    let mut w = csv::Writer::from_writer(Vec::new());
    let _ = w.write_record([
        "timestamp",
        "severity",
        "actor",
        "action",
        "message",
        "target_type",
        "target_id",
    ]);
    let formula_message = "=cmd|'/C calc'!A0";
    let formula_actor = "@SUM(A1)";
    let _ = w.write_record([
        escape_csv_formula("2026-07-27T00:00:00Z").as_ref(),
        escape_csv_formula("info").as_ref(),
        escape_csv_formula(formula_actor).as_ref(),
        escape_csv_formula("audit.test").as_ref(),
        escape_csv_formula(formula_message).as_ref(),
        escape_csv_formula("mission").as_ref(),
        escape_csv_formula("-uuid-lookalike").as_ref(),
    ]);
    let body = String::from_utf8(w.into_inner().unwrap_or_default()).expect("utf8 csv");

    // Byte-level: raw `=cmd` must not appear as a CSV field start (after comma or line start).
    assert!(
        !body.contains(",=cmd") && !body.lines().any(|l| l.starts_with('=')),
        "raw formula must not survive in CSV bytes:\n{body}"
    );
    assert!(
        body.contains("'=cmd|'/C calc'!A0") || body.contains("\"'=cmd|'/C calc'!A0\""),
        "escaped formula message must appear quote-prefixed:\n{body}"
    );

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(body.as_bytes());
    let rec = rdr
        .records()
        .next()
        .expect("one data row")
        .expect("csv row");
    for (i, field) in rec.iter().enumerate() {
        assert!(
            !matches!(field.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@')),
            "decoded field[{i}] still formula-leading: {field:?}\ncsv:\n{body}"
        );
    }
    assert_eq!(rec.get(2).unwrap(), "'@SUM(A1)");
    assert_eq!(rec.get(4).unwrap(), "'=cmd|'/C calc'!A0");
    assert_eq!(rec.get(6).unwrap(), "'-uuid-lookalike");
}

/// The SSE fields of one stream item, rendered through axum's writer, as `(field, value)` lines.
async fn sse_fields(item: AuditStreamItem) -> Vec<(String, String)> {
    let event = stream_event(&item).expect("stream items serialize");
    let response =
        Sse::new(futures::stream::iter([Ok::<Event, Infallible>(event)])).into_response();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("sse body");
    String::from_utf8(bytes.to_vec())
        .expect("utf8 sse")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (field, value) = line.split_once(": ").expect("field: value");
            (field.to_string(), value.to_string())
        })
        .collect()
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value.as_str())
}

/// `ready` is a named event whose id is the start cursor and whose data is the ready body.
///
/// RED: send the ready body as an unnamed event — a client waiting for `ready` never sees it.
#[tokio::test]
async fn audit_stream_ready_is_a_named_event_with_the_start_cursor() {
    use crate::administration::models::audit_stream::AuditStreamReady;
    let fields = sse_fields(AuditStreamItem::Ready(AuditStreamReady {
        resume_after: 7,
        retained_after: 3,
    }))
    .await;
    assert_eq!(field(&fields, "event"), Some("ready"));
    assert_eq!(field(&fields, "id"), Some("7"));
    let data: Value = serde_json::from_str(field(&fields, "data").expect("data")).expect("json");
    assert_eq!(data, json!({ "resume_after": 7, "retained_after": 3 }));
}

/// `reset` is a named event whose id is the tail and whose data names the reason.
///
/// RED: give the reset the old cursor as its id — a reconnect replays history it cannot have.
#[tokio::test]
async fn audit_stream_reset_is_a_named_event_with_the_tail() {
    use crate::administration::models::audit_stream::{AuditStreamReset, AuditStreamResetReason};
    let fields = sse_fields(AuditStreamItem::Reset(AuditStreamReset {
        reason: AuditStreamResetReason::HistoryUnavailable,
        resume_after: 90,
        retained_after: 55,
    }))
    .await;
    assert_eq!(field(&fields, "event"), Some("reset"));
    assert_eq!(field(&fields, "id"), Some("90"));
    let data: Value = serde_json::from_str(field(&fields, "data").expect("data")).expect("json");
    assert_eq!(
        data,
        json!({ "reason": "history_unavailable", "resume_after": 90, "retained_after": 55 })
    );
}

/// A row is an unnamed event whose id is the publication sequence and whose data is the list
/// route's row JSON, byte for byte.
///
/// RED: use the audit id as the SSE id — a reconnect resumes in the wrong order.
#[tokio::test]
async fn audit_stream_row_is_an_unnamed_event_with_the_list_row_json() {
    use crate::administration::services::audit_delivery::AuditDelivery;
    let row: AuditLog = serde_json::from_str(
        r#"{"id":5,"severity":"warn","actor_id":"1001","actor_name":"Ops","action":"user.ban",
            "message":"banned","target_type":"user","target_id":"2002",
            "metadata":{"reason":"late"},"created_at":"2026-09-26T10:00:00Z"}"#,
    )
    .expect("audit row");
    let list_json = serde_json::to_string(&row).expect("list row json");
    let fields = sse_fields(AuditStreamItem::Delivery(AuditDelivery {
        sequence: 12,
        row,
    }))
    .await;
    assert_eq!(field(&fields, "event"), None, "rows are unnamed events");
    assert_eq!(field(&fields, "id"), Some("12"));
    assert_eq!(field(&fields, "data"), Some(list_json.as_str()));
}

/// `Last-Event-ID` is absent or a non-negative integer; anything else answers 400.
///
/// RED: treat an unparsable header as absent — the client silently loses its replay position.
#[test]
fn audit_stream_last_event_id_is_absent_or_a_non_negative_integer() {
    let with = |value: &str| {
        let mut headers = HeaderMap::new();
        headers.insert("last-event-id", value.parse().expect("header value"));
        requested_cursor(&headers)
    };
    assert_eq!(requested_cursor(&HeaderMap::new()).expect("absent"), None);
    assert_eq!(with("0").expect("zero"), Some(0));
    assert_eq!(with("42").expect("number"), Some(42));
    for malformed in ["", "-1", "abc", "1.5", "99999999999999999999", " 4"] {
        let error = with(malformed).expect_err(malformed);
        assert_eq!(
            error.status,
            axum::http::StatusCode::BAD_REQUEST,
            "{malformed:?}"
        );
    }
}
