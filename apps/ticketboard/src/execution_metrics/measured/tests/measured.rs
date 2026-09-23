use super::*;
use crate::test_support::Scratch;
use serde_json::json;

/// Write one receipt file under the metrics tree and return its repository-relative path,
/// which is the surface the error rows name.
fn write_file(root: &Path, id: &str, name: &str, text: &str) -> String {
    let dir = metrics_dir(root).join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(name), text).unwrap();
    format!("{}/{id}/{name}", ticket_engine::repository::METRICS_DIR)
}

fn receipt(id: &str, agent: &str, input: u64, started: &str, finished: Option<&str>) -> String {
    let mut v = json!({
        "id": id,
        "agent": agent,
        "started": started,
        "tokens_consumed": {
            "input": input, "output": 0, "cache_read": 0, "cache_write": 0,
            "total": input
        }
    });
    if let Some(fin) = finished {
        v["finished"] = json!(fin);
    }
    serde_json::to_string_pretty(&v).unwrap()
}

fn loaded(state: MetricsState) -> MetricsModel {
    match state {
        MetricsState::Loaded(m) => m,
        MetricsState::NoReceipts => panic!("expected Loaded, got NoReceipts"),
    }
}

fn row<'a>(rows: &'a [MeasuredRow], key: &str) -> &'a MeasuredRow {
    rows.iter()
        .find(|r| r.key == key)
        .unwrap_or_else(|| panic!("no row for {key}"))
}

/// The acceptance's hand computation, spelled out (proof-6 numbers):
/// three receipts — /agent-a 100 tokens (01:00→01:02 = 120 s),
/// /agent-a 50 tokens (01:10→01:11 = 60 s), /agent-b 20 tokens
/// (01:20:00→01:20:30 = 30 s).
fn write_hand_corpus(root: &Path) {
    write_file(
        root,
        "T-990",
        "a.json",
        &receipt(
            "T-990",
            "agent-a",
            100,
            "2026-08-14T01:00:00Z",
            Some("2026-08-14T01:02:00Z"),
        ),
    );
    write_file(
        root,
        "T-990",
        "b.json",
        &receipt(
            "T-990",
            "agent-a",
            50,
            "2026-08-14T01:10:00Z",
            Some("2026-08-14T01:11:00Z"),
        ),
    );
    write_file(
        root,
        "T-991",
        "c.json",
        &receipt(
            "T-991",
            "agent-b",
            20,
            "2026-08-14T01:20:00Z",
            Some("2026-08-14T01:20:30Z"),
        ),
    );
}

#[test]
fn absent_metrics_dir_is_the_explicit_no_receipts_state() {
    let s = Scratch::new("m-absent");
    fs::create_dir_all(s.path().join(ticket_engine::repository::TICKETS_DIR)).unwrap();
    assert_eq!(load_metrics(s.path()), MetricsState::NoReceipts);
    // The pinned render text names the directory and the producer — no zeros.
    assert!(no_receipts_text().contains(".ai/tickets/metrics/"));
    assert!(no_receipts_text().contains("no receipts yet"));
    assert!(no_receipts_text().contains("slice-run"));
}

#[test]
fn empty_metrics_dir_is_no_receipts_even_with_empty_id_subdirs() {
    let s = Scratch::new("m-empty");
    fs::create_dir_all(metrics_dir(s.path()).join("T-990")).unwrap();
    assert_eq!(load_metrics(s.path()), MetricsState::NoReceipts);
}

/// per-agent: agent-a = 100+50 = 150 tokens over 2 runs, 120+60 = 180 s;
/// agent-b = 20 tokens over 1 run, 30 s. per-ticket: = 150/2/180,
/// = 20/1/30. Grand: 3 runs, 150+20 = 170 tokens, 180+30 = 210 s.
#[test]
fn hand_computed_sums_agent_a_150tok_180s_agent_b_20tok_30s() {
    let s = Scratch::new("m-hand");
    write_hand_corpus(s.path());
    let m = loaded(load_metrics(s.path()));
    assert!(m.errors.is_empty(), "{:?}", m.errors);

    let a = row(&m.per_agent, "agent-a");
    assert_eq!((a.runs, a.tokens, a.elapsed), (2, 100 + 50, 120 + 60));
    assert_eq!((a.finished_runs, a.unfinished), (2, 0));
    assert_eq!(a.min_started, "2026-08-14T01:00:00Z");
    assert_eq!(a.max_finished.as_deref(), Some("2026-08-14T01:11:00Z"));
    let b = row(&m.per_agent, "agent-b");
    assert_eq!((b.runs, b.tokens, b.elapsed), (1, 20, 30));

    let t990 = row(&m.per_ticket, "T-990");
    assert_eq!(
        (t990.runs, t990.tokens, t990.elapsed),
        (2, 100 + 50, 120 + 60)
    );
    let t991 = row(&m.per_ticket, "T-991");
    assert_eq!((t991.runs, t991.tokens, t991.elapsed), (1, 20, 30));

    assert_eq!(m.grand.runs, 3);
    assert_eq!(m.grand.tokens, 150 + 20);
    assert_eq!(m.grand.elapsed, 180 + 30);
    assert_eq!((m.grand.tickets, m.grand.agents), (2, 2));
    assert_eq!(m.grand.unfinished, 0);
    assert!(m.grand.strip.contains("170 tokens"), "{}", m.grand.strip);
    assert!(
        m.grand.strip.contains("3m 30s over 3 finished"),
        "{}",
        m.grand.strip
    );
    assert!(
        m.grand.strip.contains("in flight / unfinished: 0"),
        "{}",
        m.grand.strip
    );

    // Default sort: tokens desc — the 150-token rows lead both tables.
    assert_eq!(m.per_agent[0].key, "agent-a");
    assert_eq!(m.per_ticket[0].key, "T-990");
    // Precomputed display strings for the same hand numbers.
    assert_eq!(t990.tokens_str, "150");
    assert_eq!(t990.elapsed_str, "3m 00s");
    assert_eq!(b.elapsed_str, "30s");
}

#[test]
fn bad_sum_receipt_is_a_named_error_row_excluded_from_sums() {
    let s = Scratch::new("m-badsum");
    write_hand_corpus(s.path());
    // total 99 != 1+2+3+4 = 10 — the checker's d.json case.
    let rel = write_file(
        s.path(),
        "T-990",
        "d.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:30:00Z",
               "tokens_consumed":{"input":1,"output":2,"cache_read":3,"cache_write":4,"total":99}}"#,
    );
    let m = loaded(load_metrics(s.path()));
    assert_eq!(m.errors.len(), 1);
    assert_eq!(m.errors[0].rel, rel, "the error row NAMES the file");
    assert!(
        m.errors[0].reason.contains("total (99)"),
        "{}",
        m.errors[0].reason
    );
    // The three valid receipts still aggregate to the hand numbers — the
    // broken file is excluded, not coerced and not fatal.
    assert_eq!(row(&m.per_agent, "agent-a").tokens, 150);
    assert_eq!(m.grand.tokens, 170);
    assert_eq!(m.grand.runs, 3);
}

#[test]
fn missing_tokens_consumed_is_a_named_error_row_never_zero() {
    let s = Scratch::new("m-notokens");
    let rel = write_file(
        s.path(),
        "T-990",
        "a.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z"}"#,
    );
    let m = loaded(load_metrics(s.path()));
    assert_eq!(m.errors.len(), 1);
    assert_eq!(m.errors[0].rel, rel);
    assert!(
        m.errors[0].reason.contains("tokens_consumed"),
        "{}",
        m.errors[0].reason
    );
    // Zero VALID runs: the tables are empty and the strip says so — the
    // dashboard never renders tokens=0 for the broken file.
    assert!(m.per_ticket.is_empty() && m.per_agent.is_empty());
    assert_eq!(m.grand.runs, 0);
    assert!(
        m.grand
            .strip
            .contains("no valid receipts — 1 malformed file(s)"),
        "{}",
        m.grand.strip
    );
}

#[test]
fn unfinished_run_counts_in_runs_and_unfinished_never_in_elapsed() {
    let s = Scratch::new("m-inflight");
    write_file(
        s.path(),
        "T-990",
        "a.json",
        &receipt(
            "T-990",
            "agent-a",
            100,
            "2026-08-14T01:00:00Z",
            Some("2026-08-14T01:02:00Z"),
        ),
    );
    // No finished stamp — in flight.
    write_file(
        s.path(),
        "T-990",
        "b.json",
        &receipt("T-990", "agent-a", 50, "2026-08-14T01:10:00Z", None),
    );
    let m = loaded(load_metrics(s.path()));
    let t = row(&m.per_ticket, "T-990");
    assert_eq!(t.runs, 2, "the unfinished run IS a run");
    assert_eq!(t.tokens, 150, "its tokens are real and counted");
    assert_eq!(t.elapsed, 120, "elapsed covers ONLY the finished run");
    assert_eq!((t.finished_runs, t.unfinished), (1, 1));
    assert_eq!(t.unfinished_str, "1");
    assert_eq!(t.max_finished.as_deref(), Some("2026-08-14T01:02:00Z"));
    assert!(
        m.grand.strip.contains("in flight / unfinished: 1"),
        "{}",
        m.grand.strip
    );

    // A key with ONLY in-flight runs shows a dash, not a fabricated "0s".
    write_file(
        s.path(),
        "T-991",
        "c.json",
        &receipt("T-991", "agent-b", 20, "2026-08-14T01:20:00Z", None),
    );
    let m = loaded(load_metrics(s.path()));
    let t991 = row(&m.per_ticket, "T-991");
    assert_eq!(t991.elapsed_str, "—");
    assert_eq!(t991.max_finished, None);
}

/// Spec rule: `reasoning` is a sibling observation, never in `total`.
#[test]
fn reasoning_is_excluded_from_the_total_check() {
    let s = Scratch::new("m-reasoning");
    // total = 10+5+0+0 = 15 with reasoning 7 present → VALID.
    write_file(
        s.path(),
        "T-990",
        "a.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z",
               "finished":"2026-08-14T01:00:30Z",
               "tokens_consumed":{"input":10,"output":5,"cache_read":0,"cache_write":0,
                                  "total":15,"reasoning":7}}"#,
    );
    // total 22 = 15 + reasoning → RED: reasoning must not be summed in.
    let rel = write_file(
        s.path(),
        "T-990",
        "b.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:10:00Z",
               "tokens_consumed":{"input":10,"output":5,"cache_read":0,"cache_write":0,
                                  "total":22,"reasoning":7}}"#,
    );
    let m = loaded(load_metrics(s.path()));
    assert_eq!(row(&m.per_ticket, "T-990").tokens, 15);
    assert_eq!(m.errors.len(), 1);
    assert_eq!(m.errors[0].rel, rel);
    assert!(
        m.errors[0].reason.contains("total (22)"),
        "{}",
        m.errors[0].reason
    );
}

/// `deny_unknown_fields` mirrors the schema's `additionalProperties: false`.
#[test]
fn unknown_field_is_an_error_row_mirroring_the_schema() {
    let s = Scratch::new("m-unknown");
    let rel = write_file(
        s.path(),
        "T-990",
        "a.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z","elapsed_sec":42,
               "tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
    );
    let m = loaded(load_metrics(s.path()));
    assert_eq!(m.errors.len(), 1);
    assert_eq!(m.errors[0].rel, rel);
    assert!(
        m.errors[0].reason.contains("elapsed_sec"),
        "{}",
        m.errors[0].reason
    );
}

#[test]
fn checker_mirror_rules_each_produce_a_named_error_row() {
    let s = Scratch::new("m-mirror");
    // Passes the schema's length floor but is not a real instant — the
    // SEMANTIC timestamp rule fires (the checker's a.json case).
    let bad_started = write_file(
        s.path(),
        "T-990",
        "a.json",
        &receipt("T-990", "agent-a", 1, "2026-13-99T25:61:00Z", None),
    );
    let not_json = write_file(s.path(), "T-990", "b.json", "not json at all");
    // id does not match the directory it lives in.
    let mismatch = write_file(
        s.path(),
        "T-990",
        "c.json",
        &receipt("T-991", "agent-a", 1, "2026-08-14T01:00:00Z", None),
    );
    let backwards = write_file(
        s.path(),
        "T-990",
        "d.json",
        &receipt(
            "T-990",
            "agent-a",
            1,
            "2026-08-14T02:00:00Z",
            Some("2026-08-14T01:00:00Z"),
        ),
    );
    let bad_sha = write_file(
        s.path(),
        "T-990",
        "e.json",
        r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z","git_sha":"XYZ",
               "tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
    );
    let bad_id = write_file(
        s.path(),
        "T-990",
        "f.json",
        &receipt("T-bogus", "agent-a", 1, "2026-08-14T01:00:00Z", None),
    );
    let m = loaded(load_metrics(s.path()));
    assert!(m.per_ticket.is_empty(), "nothing valid may aggregate");
    let reason_of = |rel: &str| {
        &m.errors
            .iter()
            .find(|e| e.rel == rel)
            .unwrap_or_else(|| panic!("no error row for {rel}: {:?}", m.errors))
            .reason
    };
    assert!(reason_of(&bad_started).contains("RFC 3339"));
    assert!(reason_of(&not_json).contains("expected"));
    assert!(reason_of(&mismatch).contains("does not match its directory T-990"));
    assert!(reason_of(&backwards).contains("before started"));
    assert!(reason_of(&bad_sha).contains("git_sha"));
    assert!(reason_of(&bad_id).contains("schema pattern"));
    assert_eq!(m.errors.len(), 6, "{:?}", m.errors);
}

/// A stray file directly under metrics/ is still validated (dir-mismatch
/// red), mirroring the checker's walk-everything rule.
#[test]
fn stray_file_at_the_metrics_root_is_an_error_row() {
    let s = Scratch::new("m-stray");
    let dir = metrics_dir(s.path());
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("stray.json"),
        receipt("T-990", "agent-a", 1, "2026-08-14T01:00:00Z", None),
    )
    .unwrap();
    let m = loaded(load_metrics(s.path()));
    assert_eq!(m.errors.len(), 1);
    assert!(
        m.errors[0]
            .reason
            .contains("does not match its directory metrics"),
        "{}",
        m.errors[0].reason
    );
}

/// min/max compare parsed INSTANTS: lexicographically "…00.500Z" sorts
/// before "…00Z" ('.' < 'Z'), but 00.500 is chronologically LATER.
#[test]
fn min_max_use_parsed_instants_not_string_order() {
    let s = Scratch::new("m-instants");
    write_file(
        s.path(),
        "T-990",
        "a.json",
        &receipt(
            "T-990",
            "agent-a",
            1,
            "2026-08-14T01:00:00.500Z",
            Some("2026-08-14T01:02:00.500Z"),
        ),
    );
    write_file(
        s.path(),
        "T-990",
        "b.json",
        &receipt(
            "T-990",
            "agent-a",
            1,
            "2026-08-14T01:00:00Z",
            Some("2026-08-14T01:02:00Z"),
        ),
    );
    let m = loaded(load_metrics(s.path()));
    let t = row(&m.per_ticket, "T-990");
    assert_eq!(t.min_started, "2026-08-14T01:00:00Z");
    assert_eq!(t.max_finished.as_deref(), Some("2026-08-14T01:02:00.500Z"));
    assert_eq!(t.elapsed, 120 + 120);
}

#[test]
fn sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak() {
    let mk = |key: &str, runs: u64, tokens: u64, elapsed: u64| MeasuredRow {
        key: key.to_owned(),
        runs,
        tokens,
        elapsed,
        finished_runs: runs,
        unfinished: 0,
        min_started: String::new(),
        max_finished: None,
        runs_str: runs.to_string(),
        tokens_str: format_tokens(tokens),
        elapsed_str: format_elapsed(elapsed),
        unfinished_str: "0".to_owned(),
    };
    let mut rows = vec![mk("b", 1, 20, 30), mk("a", 2, 150, 180), mk("c", 2, 20, 5)];
    sort_rows(&mut rows, Sort::default()); // tokens desc, tie by key asc
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["a", "b", "c"], "150 first; 20-token tie b before c");

    let sort = Sort::default().toggled(SortKey::Tokens); // same key → flip asc
    assert_eq!(
        sort,
        Sort {
            key: SortKey::Tokens,
            desc: false
        }
    );
    sort_rows(&mut rows, sort);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["b", "c", "a"]);

    let sort = sort.toggled(SortKey::Runs); // new key → desc
    assert_eq!(
        sort,
        Sort {
            key: SortKey::Runs,
            desc: true
        }
    );
    sort_rows(&mut rows, sort);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["a", "c", "b"], "2-run tie a before c");

    sort_rows(
        &mut rows,
        Sort {
            key: SortKey::Elapsed,
            desc: true,
        },
    );
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["a", "b", "c"], "180 > 30 > 5");
}

#[test]
fn format_tokens_groups_thousands() {
    assert_eq!(format_tokens(0), "0");
    assert_eq!(format_tokens(999), "999");
    assert_eq!(format_tokens(1000), "1,000");
    assert_eq!(format_tokens(170), "170");
    assert_eq!(format_tokens(1_234_567), "1,234,567");
}

#[test]
fn format_elapsed_h_m_s() {
    assert_eq!(format_elapsed(0), "0s");
    assert_eq!(format_elapsed(59), "59s");
    assert_eq!(format_elapsed(60), "1m 00s");
    assert_eq!(format_elapsed(180), "3m 00s");
    assert_eq!(format_elapsed(210), "3m 30s");
    assert_eq!(format_elapsed(3723), "1h 02m 03s");
}

#[test]
fn ticket_id_and_sha_pattern_mirrors() {
    assert!(valid_ticket_id("T-990"));
    assert!(valid_ticket_id("T-915.5"));
    assert!(valid_ticket_id("T-90.6.2"));
    for bad in ["T-", "T-990.", "T-.5", "990", "T-9a", "t-990", ""] {
        assert!(!valid_ticket_id(bad), "{bad:?}");
    }
    assert!(valid_git_sha("0123456"));
    assert!(valid_git_sha("0123456789abcdef0123456789abcdef01234567"));
    for bad in ["012345", "XYZABCD", "0123456789ABCDEF0", ""] {
        assert!(!valid_git_sha(bad), "{bad:?}");
    }
}
