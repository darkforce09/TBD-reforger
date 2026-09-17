use super::*;

#[test]
fn recorded_cursor_dialect_parses_and_total_is_the_sum() {
    let t = parse_tokens_from_cli_json(&fixture("slice_run_cursor_agent.json")).unwrap();
    assert_eq!(t.input, 12000);
    assert_eq!(t.output, 3400);
    assert_eq!(t.cache_read, 8000);
    assert_eq!(t.cache_write, 200);
    assert_eq!(t.total, 12000 + 3400 + 8000 + 200);
    assert_eq!(t.reasoning, Some(0));
    t.validate().unwrap();
}

#[test]
fn recorded_claude_dialect_parses_and_total_is_the_sum() {
    let t = parse_tokens_from_cli_json(&fixture("slice_run_claude_print.json")).unwrap();
    assert_eq!(t.input, 10);
    assert_eq!(t.output, 20);
    assert_eq!(t.cache_read, 5);
    assert_eq!(t.cache_write, 1);
    assert_eq!(t.total, 36);
    assert_eq!(t.reasoning, None);
}

#[test]
fn missing_usage_fails_closed_never_zero() {
    let err = parse_tokens_from_cli_json(&fixture("slice_run_no_usage.json")).unwrap_err();
    assert!(err.to_string().contains("no usage object"), "{err:#}");
}

#[test]
fn reasoning_is_a_sibling_never_summed_into_total() {
    let cli = json!({"usage": {
        "inputTokens": 10, "outputTokens": 5,
        "cacheReadTokens": 0, "cacheWriteTokens": 0,
        "reasoningTokens": 7, "totalTokens": 22
    }});
    let t = parse_tokens_from_cli_json(&cli).unwrap();
    assert_eq!(t.total, 15, "reasoning must not be in total");
    assert_eq!(t.reasoning, Some(7));
}

#[test]
fn drifted_reported_total_is_loud() {
    let cli = json!({"usage": {
        "inputTokens": 10, "outputTokens": 5,
        "cacheReadTokens": 0, "cacheWriteTokens": 0,
        "totalTokens": 999
    }});
    let err = parse_tokens_from_cli_json(&cli).unwrap_err();
    assert!(err.to_string().contains("drifted"), "{err:#}");
}

#[test]
fn total_sum_invariant_is_enforced() {
    let t = TokensConsumed {
        input: 1,
        output: 2,
        cache_read: 3,
        cache_write: 4,
        total: 11,
        reasoning: None,
    };
    assert!(t.validate().is_err());
}

#[test]
fn two_runs_in_one_second_yield_two_files() {
    let tmp = scratch("collide");
    let r = rec(
        "T-990",
        "agent-a",
        100,
        "2026-08-14T01:14:00Z",
        "2026-08-14T01:14:01Z",
    );
    let a = write_run_file(&tmp, &r).unwrap();
    let b = write_run_file(&tmp, &r).unwrap();
    assert_ne!(a, b, "same second + same sha must still be two files");
    assert!(a.is_file() && b.is_file());
    assert!(
        b.to_string_lossy().ends_with("-1.json"),
        "collision gets an increment suffix: {}",
        b.display()
    );
    // The newest is the collision file, not the lexicographically-last base file.
    let (latest, _) = latest_run_file(&tmp, "T-990").unwrap();
    assert_eq!(latest, b);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn malformed_started_missing_id_missing_tokens_and_bad_sum_are_red() {
    let tmp = scratch("check");
    let dir = metrics_root(&tmp).join("T-990");
    fs::create_dir_all(&dir).unwrap();
    // 20 chars (passes the schema's minLength) but not a real RFC 3339 instant —
    // proves the SEMANTIC timestamp rule fires, not just the schema's length floor.
    fs::write(
        dir.join("a.json"),
        r#"{"id":"T-990","agent":"a","started":"2026-13-99T25:61:00Z","tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("b.json"),
        r#"{"agent":"a","started":"2026-08-14T01:00:00Z","tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("c.json"),
        r#"{"id":"T-990","agent":"a","started":"2026-08-14T01:00:00Z"}"#,
    )
    .unwrap();
    fs::write(
        dir.join("d.json"),
        r#"{"id":"T-990","agent":"a","started":"2026-08-14T01:00:00Z","tokens_consumed":{"input":1,"output":2,"cache_read":3,"cache_write":4,"total":99}}"#,
    )
    .unwrap();
    let errors = check_as_errors(&tmp);
    let text = errors.join("\n");
    assert!(
        text.contains("a.json") && text.contains("RFC 3339"),
        "{text}"
    );
    assert!(text.contains("b.json") && text.contains("id"), "{text}");
    assert!(
        text.contains("c.json") && text.contains("tokens_consumed"),
        "{text}"
    );
    assert!(text.contains("d.json") && text.contains("total"), "{text}");
    // metrics summing must refuse the same tree, never print zeros for it.
    assert!(summarize_by_agent(&tmp).is_err());
    // Remove the plants → green.
    let _ = fs::remove_dir_all(metrics_root(&tmp));
    assert!(check_as_errors(&tmp).is_empty());
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn by_agent_sums_elapsed_and_totals_over_real_files() {
    let tmp = scratch("sum");
    write_run_file(
        &tmp,
        &rec(
            "T-990",
            "agent-a",
            100,
            "2026-08-14T01:00:00Z",
            "2026-08-14T01:02:00Z",
        ),
    )
    .unwrap();
    write_run_file(
        &tmp,
        &rec(
            "T-990",
            "agent-a",
            50,
            "2026-08-14T01:10:00Z",
            "2026-08-14T01:11:00Z",
        ),
    )
    .unwrap();
    write_run_file(
        &tmp,
        &rec(
            "T-991",
            "agent-b",
            20,
            "2026-08-14T01:20:00Z",
            "2026-08-14T01:20:30Z",
        ),
    )
    .unwrap();
    let sums = summarize_by_agent(&tmp).unwrap();
    assert_eq!(sums["agent-a"], (2, 180, 150));
    assert_eq!(sums["agent-b"], (1, 30, 20));
    let _ = fs::remove_dir_all(&tmp);
}

/// Proof-(d) vehicle — the parallel-lands surrogate. A real scratch git repo: two
/// committed ticket TOMLs, two committed receipts; stamp BOTH tickets' receipts and
/// show `git status --porcelain` touches ONLY `.ai/tickets/metrics/` files.
#[test]
fn land_stamp_two_tickets_touches_only_metrics_never_ticket_tomls() {
    let tmp = scratch("stamp-git");
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "user.email=t913@test",
                "-c",
                "user.name=t913",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&tmp)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    git(&["init", "-q"]);
    fs::write(tmp.join(".ai/tickets/T-990.toml"), "id = \"T-990\"\n").unwrap();
    fs::write(tmp.join(".ai/tickets/T-991.toml"), "id = \"T-991\"\n").unwrap();
    write_run_file(
        &tmp,
        &rec(
            "T-990",
            "agent-a",
            100,
            "2026-08-14T01:00:00Z",
            "2026-08-14T01:02:00Z",
        ),
    )
    .unwrap();
    write_run_file(
        &tmp,
        &rec(
            "T-991",
            "agent-b",
            20,
            "2026-08-14T01:20:00Z",
            "2026-08-14T01:20:30Z",
        ),
    )
    .unwrap();
    git(&["add", "--", ".ai/tickets"]);
    git(&["commit", "-q", "-m", "seed"]);

    let sha = "0123456789abcdef0123456789abcdef01234567";
    stamp_land_at(&tmp, "T-990", sha, "2026-08-14T02:00:00Z").unwrap();
    stamp_land_at(&tmp, "T-991", sha, "2026-08-14T02:00:00Z").unwrap();

    let status = git(&["status", "--porcelain"]);
    println!("git status --porcelain after stamping T-990 and T-991:\n{status}");
    assert!(
        !status.contains(".toml"),
        "stamping must NEVER touch a ticket TOML:\n{status}"
    );
    let touched: Vec<&str> = status.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(touched.len(), 2, "exactly the two receipts:\n{status}");
    for line in &touched {
        assert!(line.contains(".ai/tickets/metrics/"), "{line}");
    }
    for id in ["T-990", "T-991"] {
        let (_, rec) = latest_run_file(&tmp, id).unwrap();
        assert_eq!(rec.outcome.as_deref(), Some("landed"));
        assert_eq!(rec.git_sha.as_deref(), Some(sha));
        assert_eq!(rec.finished.as_deref(), Some("2026-08-14T02:00:00Z"));
    }
    let _ = fs::remove_dir_all(&tmp);
}

/// Proof-(g) vehicle — the strict land refusal and the `--bookkeeping` escape hatch.
#[test]
fn factory_land_without_receipt_refuses_and_bookkeeping_proceeds() {
    let tmp = scratch("land-gate");
    write_run_file(
        &tmp,
        &rec(
            "T-991",
            "agent-b",
            20,
            "2026-08-14T01:20:00Z",
            "2026-08-14T01:20:30Z",
        ),
    )
    .unwrap();
    let ids = vec!["T-990".to_string(), "T-991".to_string()];
    let refusal = land_receipt_refusal(&tmp, &ids, false)
        .expect("strict land with a missing receipt must refuse");
    println!("strict factory land refusal:\n{refusal}");
    assert!(
        refusal.contains("T-990") && !refusal.contains("T-991 "),
        "{refusal}"
    );
    assert!(refusal.contains("--bookkeeping"), "{refusal}");
    assert!(
        land_receipt_refusal(&tmp, &ids, true).is_none(),
        "--bookkeeping must waive the receipt requirement"
    );
    println!("with --bookkeeping: land proceeds (no refusal)");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn stamp_refuses_when_no_receipt_exists() {
    let tmp = scratch("stamp-none");
    let err = stamp_land_at(&tmp, "T-990", "deadbeefdead", "2026-08-14T02:00:00Z").unwrap_err();
    assert!(
        format!("{err:#}").contains("no slice-run receipt"),
        "{err:#}"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn finished_before_started_is_red() {
    let mut r = rec(
        "T-990",
        "agent-a",
        1,
        "2026-08-14T02:00:00Z",
        "2026-08-14T01:00:00Z",
    );
    assert!(validate_record(&r).is_err());
    r.finished = None;
    validate_record(&r).unwrap();
    assert_eq!(elapsed_sec(&r).unwrap(), None);
}
