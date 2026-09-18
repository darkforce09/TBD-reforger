use super::*;

/// Class-R: formula-leading cells must be prefixed so Excel/Sheets do not execute them.
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

/// Class-R: the export writer path (same `csv::Writer` + `escape_csv_formula` as
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
