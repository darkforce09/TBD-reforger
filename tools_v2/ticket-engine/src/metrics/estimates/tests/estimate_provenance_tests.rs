use super::*;

/// The factor authority: the const, the doc, and the pinned marker line agree.
/// The doc also names the three LOC exclusions the miner enforces.
#[test]
fn factor_constant_is_pinned_in_the_doc() {
    let doc = fs::read_to_string(repo_root().join(TOKEN_ESTIMATE_FACTOR_DOC)).expect("factor doc");
    let marker = format!("TOKENS_PER_LOC = {TOKENS_PER_LOC}");
    assert!(
        doc.contains(&marker),
        "{TOKEN_ESTIMATE_FACTOR_DOC} must quote the constant verbatim: {marker:?}"
    );
    assert!(
        doc.contains("pending calibration"),
        "the factor is a declared constant pending calibration"
    );
    for needle in [".ai/", "docs/TICKET_", "Cargo.lock"] {
        assert!(
            doc.contains(needle),
            "{TOKEN_ESTIMATE_FACTOR_DOC} must document the {needle} LOC exclusion"
        );
    }
}

#[test]
fn excluded_paths_and_numstat_parse() {
    assert!(is_excluded_path(".ai/tickets/T-001.toml"));
    assert!(is_excluded_path(".ai/artifacts/run.log"));
    assert!(is_excluded_path("docs/TICKET_LEAD.md"));
    assert!(is_excluded_path("docs/TICKET_REGISTRY.md"));
    assert!(is_excluded_path("Cargo.lock"));
    assert!(is_excluded_path("apps/website/api_v2/Cargo.lock"));
    assert!(!is_excluded_path("docs/platform/token_estimate_factor.md"));
    assert!(!is_excluded_path("tools_v2/xtask/src/main.rs"));
    assert!(!is_excluded_path("docs/TICKETING.rs")); // suffix rule: .md only

    let sha_a = "a".repeat(40);
    let sha_b = "b".repeat(40);
    let sha_c = "c".repeat(40); // merge: no numstat lines at all
    let text = format!(
        "{sha_a}\n\n10\t5\txtask/src/main.rs\n3\t1\t.ai/tickets/T-001.toml\n1\t1\tdocs/TICKET_LEAD.md\n7\t0\tCargo.lock\n2\t2\tapps/website/Cargo.lock\n-\t-\tassets/logo.png\n{sha_b}\n\n0\t4\tdocs/x.md\n{sha_c}\n"
    );
    let map = parse_numstat(&text);
    assert_eq!(
        map.get(&sha_a).copied(),
        Some(15),
        "only tools_v2/xtask/src counts"
    );
    assert_eq!(map.get(&sha_b).copied(), Some(4));
    assert_eq!(map.get(&sha_c).copied(), Some(0), "merge counts zero");
}

#[test]
fn median_is_deterministic() {
    assert_eq!(median(vec![3]), 3);
    assert_eq!(median(vec![3, 1]), 2, "even = floor of middle mean");
    assert_eq!(median(vec![4, 1, 2]), 2);
    assert_eq!(median(vec![10, 1, 3, 2]), 2, "(2+3)/2 floors to 2");
    assert_eq!(median(vec![6000, 1500, 4500, 3000]), 3750);
}

/// Committed-schema red/green: one green per source shape (full key, widened
/// key, all-key), one red per rule.
#[test]
fn estimates_schema_red_green() {
    let text = fs::read_to_string(repo_root().join(ESTIMATES_SCHEMA)).expect("schema");
    let schema: Value = serde_json::from_str(&text).expect("schema parses");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let diff = |extra: fn(&mut Value)| {
        let mut v = json!({
            "id": "T-001", "source": "diff_loc", "factor": 150,
            "tokens_estimated": 1500, "generated_at": "2026-08-15T00:00:00Z",
            "loc_changed": 10, "derived_from_shas": ["aaaa111122223333"]
        });
        extra(&mut v);
        v
    };
    let cohort = |extra: fn(&mut Value)| {
        let mut v = json!({
            "id": "T-004", "source": "cohort_median", "factor": 150,
            "tokens_estimated": 3000, "generated_at": "2026-08-15T00:00:00Z",
            "cohort": {"class": "chore", "domain": "repo", "layer": "docs"},
            "cohort_size": 3
        });
        extra(&mut v);
        v
    };
    // Greens: canonical diff_loc; cohort with the full, widened and all keys.
    for (name, v) in [
        ("diff_loc", diff(|_| {})),
        ("cohort full key", cohort(|_| {})),
        (
            "cohort widened",
            cohort(|v| v["cohort"] = json!({"class": "chore"})),
        ),
        ("cohort all", cohort(|v| v["cohort"] = json!({}))),
    ] {
        assert!(validator.validate(&v).is_ok(), "{name} must be green");
    }
    // Reds, each naming its rule.
    let reds: Vec<(&str, Value)> = vec![
        ("bad id", diff(|v| v["id"] = json!("X-001"))),
        ("unknown property", diff(|v| v["vibes"] = json!(1))),
        (
            "missing shas",
            diff(|v| {
                v.as_object_mut().unwrap().remove("derived_from_shas");
            }),
        ),
        ("empty shas", diff(|v| v["derived_from_shas"] = json!([]))),
        (
            "uppercase sha",
            diff(|v| v["derived_from_shas"] = json!(["AAAA111122223333"])),
        ),
        ("cohort on diff_loc", diff(|v| v["cohort"] = json!({}))),
        (
            "negative tokens",
            diff(|v| v["tokens_estimated"] = json!(-1)),
        ),
        ("factor zero", diff(|v| v["factor"] = json!(0))),
        (
            "offset generated_at",
            diff(|v| v["generated_at"] = json!("2026-08-15T00:00:00+02:00")),
        ),
        (
            "loc on cohort_median",
            cohort(|v| v["loc_changed"] = json!(5)),
        ),
        ("cohort_size zero", cohort(|v| v["cohort_size"] = json!(0))),
        (
            "missing cohort",
            cohort(|v| {
                v.as_object_mut().unwrap().remove("cohort");
            }),
        ),
        ("bogus source", diff(|v| v["source"] = json!("guess"))),
    ];
    for (name, v) in reds {
        assert!(validator.validate(&v).is_err(), "{name} must be red");
    }
}

/// The scratch end-to-end: diff_loc for subject-bearing tickets, cohort_median
/// with the documented widening for the rest, the zero-LOC fall-through, the
/// receipt skip, exact file bytes (sorted keys + trailing newline), markers via
/// write_back, a green self-check, and a second run that finds nothing.
#[test]
fn scratch_generator_cohorts_fallthrough_and_idempotence() {
    let root = scratch_root("pass");
    let mut c = Corpus::new(&root);
    for (id, loc_class) in [
        ("T-001", ("chore", Domain::Repo, "docs")),
        ("T-002", ("chore", Domain::Repo, "docs")),
        ("T-003", ("chore", Domain::Repo, "docs")),
        ("T-004", ("chore", Domain::Repo, "docs")), // zero subjects → L0 cohort
        ("T-005", ("feature", Domain::Website, "frontend")),
        ("T-006", ("feature", Domain::Website, "backend")), // widens to all
        ("T-007", ("chore", Domain::Repo, "docs")),         // zero-LOC fall-through
        ("T-009", ("chore", Domain::Repo, "docs")),         // has a receipt → skipped
    ] {
        let (class, domain, layer) = loc_class;
        c.tickets
            .insert(id.into(), shipped_work(id, class, domain, layer));
    }
    // Class-less program (child listed — programs require children) → all-key.
    let prog = match shipped_program("T-008") {
        Ticket::Program(mut p) => {
            p.children = vec!["T-008.1".into()];
            Ticket::Program(p)
        }
        Ticket::Work(_) => unreachable!(),
    };
    c.tickets.insert("T-008".into(), prog);
    let child = match shipped_work("T-008.1", "chore", Domain::Repo, "docs") {
        Ticket::Work(mut w) => {
            w.parent = Some("T-008".into());
            Ticket::Work(w)
        }
        Ticket::Program(_) => unreachable!(),
    };
    c.tickets.insert("T-008.1".into(), child); // zero subjects → L0 cohort
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed tree");
    crate::metrics::write_run_file(
        &root,
        &crate::metrics::RunRecord {
            id: "T-009".into(),
            agent: "agent-a".into(),
            started: "2026-08-14T01:00:00Z".into(),
            finished: Some("2026-08-14T01:02:00Z".into()),
            outcome: Some("ran".into()),
            git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
            tokens_consumed: crate::metrics::TokensConsumed {
                input: 100,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                total: 100,
                reasoning: None,
            },
        },
    )
    .expect("receipt for T-009");

    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert("T-001".into(), vec![sc("aaaa111122223333")]);
    subjects.insert("T-002".into(), vec![sc("bbbb111122223333")]);
    subjects.insert(
        "T-003".into(),
        vec![sc("cccc111122223333"), sc("cccc444455556666")],
    );
    subjects.insert("T-005".into(), vec![sc("dddd111122223333")]);
    subjects.insert("T-007".into(), vec![sc("eeee111122223333")]); // excluded-only diff
    let sha_loc: BTreeMap<String, u64> = [
        ("aaaa111122223333", 10),
        ("bbbb111122223333", 20),
        ("cccc111122223333", 12),
        ("cccc444455556666", 18), // T-003 sums to 30
        ("dddd111122223333", 40),
        ("eeee111122223333", 0),
    ]
    .into_iter()
    .map(|(s, n)| (s.to_string(), n))
    .collect();

    let report = run_estimates(&root, &subjects, &sha_loc, NOW).expect("pass");
    assert_eq!(report.shipped_total, 10);
    assert_eq!(report.with_receipt, 1, "T-009");
    assert_eq!(report.already_estimated, 0);
    assert_eq!(report.e_diff_loc, 4, "T-001 T-002 T-003 T-005");
    assert_eq!(report.c_cohort_median, 5, "T-004 T-006 T-007 T-008 T-008.1");
    assert_eq!(report.c_fell_through_zero_loc, 1, "T-007");
    assert_eq!(report.e_diff_loc + report.c_cohort_median, 9);

    // Exact bytes: sorted keys, 2-space pretty, trailing newline.
    let t1 = fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-001.json"))).unwrap();
    assert_eq!(
        t1,
        "{\n  \"derived_from_shas\": [\n    \"aaaa111122223333\"\n  ],\n  \"factor\": 150,\n  \"generated_at\": \"2026-08-15T00:00:00Z\",\n  \"id\": \"T-001\",\n  \"loc_changed\": 10,\n  \"source\": \"diff_loc\",\n  \"tokens_estimated\": 1500\n}\n"
    );
    // The L0 cohort (chore, repo, docs) has exactly the 3 members
    // 1500/3000/4500 → median 3000, full key recorded.
    let t4: EstimateRecord = serde_json::from_str(
        &fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-004.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(t4.tokens_estimated, 3000);
    assert_eq!(t4.cohort_size, Some(3));
    assert_eq!(
        t4.cohort,
        Some(CohortKey {
            class: Some("chore".into()),
            domain: Some("repo".into()),
            layer: Some("docs".into()),
        })
    );
    // The widening cohort: (feature, website, backend) empty → (feature, website) 1 →
    // (feature) 1 → all 4 members {1500,3000,4500,6000} → 3750, key {}.
    let t6: EstimateRecord = serde_json::from_str(
        &fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-006.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(t6.tokens_estimated, 3750);
    assert_eq!(t6.cohort_size, Some(4));
    assert_eq!(
        t6.cohort,
        Some(CohortKey {
            class: None,
            domain: None,
            layer: None
        }),
        "the WIDENED key actually used is the all-key"
    );
    // The fall-through ticket: cohort_median in its (chore, repo, docs) cohort.
    let t7: EstimateRecord = serde_json::from_str(
        &fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-007.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(t7.source, "cohort_median");
    assert_eq!(t7.tokens_estimated, 3000);
    // The class-less program: straight to the all-key.
    let t8: EstimateRecord = serde_json::from_str(
        &fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-008.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        t8.cohort,
        Some(CohortKey {
            class: None,
            domain: None,
            layer: None
        })
    );
    // The zero-subject child WITH class+scope: its own L0 cohort.
    let t81: EstimateRecord = serde_json::from_str(
        &fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-008.1.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(t81.tokens_estimated, 3000);
    assert_eq!(t81.cohort_size, Some(3));
    // The ticket with a receipt: NO estimate file, NO marker.
    assert!(!root.join(format!("{ESTIMATES_DIR}/T-009.json")).exists());

    let reread = Corpus::load(&root).expect("reload");
    for id in ["T-001", "T-004", "T-006", "T-007", "T-008", "T-008.1"] {
        assert!(
            estimated_of(reread.get(id).unwrap())
                .iter()
                .any(|e| e == "tokens"),
            "{id} must carry the tokens marker"
        );
    }
    assert!(
        !estimated_of(reread.get("T-009").unwrap())
            .iter()
            .any(|e| e == "tokens"),
        "a receipted ticket gets no estimate marker"
    );
    assert!(check_as_errors(&root).is_empty(), "tree must be green");

    // Idempotence: the second pass finds nothing and changes nothing.
    let before = fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-004.json"))).unwrap();
    let second = run_estimates(&root, &subjects, &sha_loc, "2026-08-16T00:00:00Z").expect("second");
    assert!(second.records.is_empty(), "second run must find nothing");
    assert_eq!(second.already_estimated, 9);
    assert_eq!(second.with_receipt, 1);
    assert_eq!(
        fs::read_to_string(root.join(format!("{ESTIMATES_DIR}/T-004.json"))).unwrap(),
        before,
        "existing estimates are never rewritten"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// THE collision objection, proved not fixed: an estimate PLANTED inside
/// `metrics/<id>/` (a) satisfies `has_receipt` — it would impersonate a measured
/// receipt — and (b) reds the EXISTING metrics walkers (deny_unknown_fields +
/// schema). Placement outside metrics/ is the fix; nothing here relaxes the
/// metrics check.
#[test]
fn planted_estimate_inside_metrics_reds_the_metrics_walker() {
    let root = scratch_root("collision");
    let dir = root.join(crate::repository::METRICS_DIR).join("T-001");
    fs::create_dir_all(&dir).unwrap();
    let est = EstimateRecord {
        cohort: None,
        cohort_size: None,
        derived_from_shas: Some(vec!["aaaa111122223333".into()]),
        factor: TOKENS_PER_LOC,
        generated_at: NOW.into(),
        id: "T-001".into(),
        loc_changed: Some(10),
        source: "diff_loc".into(),
        tokens_estimated: 1500,
    };
    fs::write(dir.join("T-001.json"), render_estimate(&est).unwrap()).unwrap();
    assert!(
        crate::metrics::has_receipt(&root, "T-001"),
        "the impersonation: ANY file under metrics/<id>/ satisfies has_receipt"
    );
    let errors = crate::metrics::check_as_errors(&root);
    assert!(
        !errors.is_empty(),
        "the metrics walker must red a planted estimate"
    );
    assert!(
        errors.iter().any(|e| e.contains("T-001.json")),
        "must name the planted file: {errors:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Mutual exclusion + marker coherence, red in every direction and green when
/// coherent.
#[test]
fn mutual_exclusion_and_marker_coherence() {
    let root = scratch_root("coherence");
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-001".into(),
        shipped_work("T-001", "chore", Domain::Repo, "docs"),
    );
    c.write_back(&["T-001".into()]).expect("seed");
    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert("T-001".into(), vec![sc("aaaa111122223333")]);
    let sha_loc: BTreeMap<String, u64> =
        [("aaaa111122223333".to_string(), 10)].into_iter().collect();
    run_estimates(&root, &subjects, &sha_loc, NOW).expect("generate");
    assert!(check_as_errors(&root).is_empty(), "coherent tree is green");

    // A receipt lands for the same id → red naming BOTH trees.
    let rdir = root.join(crate::repository::METRICS_DIR).join("T-001");
    fs::create_dir_all(&rdir).unwrap();
    fs::write(rdir.join("r.json"), "{}").unwrap();
    let errs = check_as_errors(&root);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("estimates/T-001.json")
            && errs[0].contains("metrics")
            && errs[0].contains("mutually exclusive"),
        "{}",
        errs[0]
    );
    fs::remove_dir_all(root.join(crate::repository::METRICS_DIR)).unwrap();
    assert!(check_as_errors(&root).is_empty());

    // Marker without file → red naming ticket + the missing path.
    let est_path = root.join(format!("{ESTIMATES_DIR}/T-001.json"));
    let est_bytes = fs::read_to_string(&est_path).unwrap();
    fs::remove_file(&est_path).unwrap();
    let errs = check_as_errors(&root);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001")
            && errs[0].contains("estimated[] lists tokens")
            && errs[0].contains("does not exist"),
        "{}",
        errs[0]
    );
    fs::write(&est_path, &est_bytes).unwrap();
    assert!(check_as_errors(&root).is_empty());

    // File without marker → red pointing at estimated[].
    let mut corpus = Corpus::load(&root).expect("load");
    estimated_mut(corpus.tickets.get_mut("T-001").unwrap()).retain(|e| e != "tokens");
    corpus.write_back(&["T-001".into()]).expect("strip marker");
    let errs = check_as_errors(&root);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("estimates/T-001.json") && errs[0].contains("estimated[]"),
        "{}",
        errs[0]
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Business rules over hand-planted files: factor drift, broken arithmetic,
/// stem mismatch, non-shipped ticket, missing ticket, and a generated_at that
/// passes the schema's digit pattern but fails the SEMANTIC RFC 3339 rule.
#[test]
fn business_rules_red() {
    let root = scratch_root("business");
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-001".into(),
        shipped_work("T-001", "chore", Domain::Repo, "docs"),
    );
    let queued = {
        let mut t = match shipped_work("T-003", "chore", Domain::Repo, "docs") {
            Ticket::Work(w) => w,
            Ticket::Program(_) => unreachable!(),
        };
        t.status = Status::Queued { order: 10 };
        t.shipped_at = None;
        t.owns = vec!["docs/README.md".into()];
        Ticket::Work(t)
    };
    c.tickets.insert("T-003".into(), queued);
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed");
    let dir = estimates_root(&root);
    fs::create_dir_all(&dir).unwrap();
    let diff_json = |id: &str, factor: u64, tokens: u64, generated: &str| {
        format!(
            "{{\n  \"derived_from_shas\": [\n    \"aaaa111122223333\"\n  ],\n  \"factor\": {factor},\n  \"generated_at\": \"{generated}\",\n  \"id\": \"{id}\",\n  \"loc_changed\": 10,\n  \"source\": \"diff_loc\",\n  \"tokens_estimated\": {tokens}\n}}\n"
        )
    };

    // Factor drift.
    fs::write(dir.join("T-001.json"), diff_json("T-001", 149, 1490, NOW)).unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter().any(|e| e.contains("factor 149")
            && e.contains("150")
            && e.contains(TOKEN_ESTIMATE_FACTOR_DOC)),
        "{errs:?}"
    );
    // Broken arithmetic.
    fs::write(dir.join("T-001.json"), diff_json("T-001", 150, 1501, NOW)).unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter()
            .any(|e| e.contains("tokens_estimated (1501)") && e.contains("= 1500")),
        "{errs:?}"
    );
    // Pattern-passing but semantically impossible generated_at.
    fs::write(
        dir.join("T-001.json"),
        diff_json("T-001", 150, 1500, "2026-13-99T25:61:00Z"),
    )
    .unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter().any(|e| e.contains("RFC 3339")),
        "semantic timestamp rule must fire past the pattern floor: {errs:?}"
    );
    // Stem mismatch (a file whose name is not the id inside it).
    fs::remove_file(dir.join("T-001.json")).unwrap();
    fs::write(dir.join("T-002.json"), diff_json("T-001", 150, 1500, NOW)).unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter()
            .any(|e| e.contains("id T-001") && e.contains("stem T-002")),
        "{errs:?}"
    );
    fs::remove_file(dir.join("T-002.json")).unwrap();
    // Estimate for a non-shipped ticket.
    fs::write(dir.join("T-003.json"), diff_json("T-003", 150, 1500, NOW)).unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter()
            .any(|e| e.contains("T-003") && e.contains("queued, not shipped")),
        "{errs:?}"
    );
    fs::remove_file(dir.join("T-003.json")).unwrap();
    // Estimate for a ticket that does not exist.
    fs::write(dir.join("T-404.json"), diff_json("T-404", 150, 1500, NOW)).unwrap();
    let errs = check_as_errors(&root);
    assert!(
        errs.iter().any(|e| e.contains("no ticket T-404 on disk")),
        "{errs:?}"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// The negative acceptance: `summarize_by_agent` over a tree carrying BOTH
/// receipts and estimates equals the receipts-only hand computation — the
/// estimate numbers appear nowhere in the sums. Both walkers stay green.
#[test]
fn summarize_by_agent_on_mixed_tree_equals_receipts_only() {
    let root = scratch_root("mixed-sum");
    let mut c = Corpus::new(&root);
    for id in ["T-001", "T-002", "T-003"] {
        c.tickets
            .insert(id.into(), shipped_work(id, "chore", Domain::Repo, "docs"));
    }
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed");
    let rec = |id: &str, agent: &str, input: u64, started: &str, finished: &str| {
        crate::metrics::write_run_file(
            &root,
            &crate::metrics::RunRecord {
                id: id.into(),
                agent: agent.into(),
                started: started.into(),
                finished: Some(finished.into()),
                outcome: Some("ran".into()),
                git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
                tokens_consumed: crate::metrics::TokensConsumed {
                    input,
                    output: 0,
                    cache_read: 0,
                    cache_write: 0,
                    total: input,
                    reasoning: None,
                },
            },
        )
        .expect("receipt");
    };
    rec(
        "T-001",
        "agent-a",
        100,
        "2026-08-14T01:00:00Z",
        "2026-08-14T01:02:00Z",
    );
    rec(
        "T-002",
        "agent-b",
        20,
        "2026-08-14T01:20:00Z",
        "2026-08-14T01:20:30Z",
    );
    // The receiptless ticket → a huge cohortless diff_loc estimate instead.
    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert("T-003".into(), vec![sc("aaaa111122223333")]);
    let sha_loc: BTreeMap<String, u64> = [("aaaa111122223333".to_string(), 6667)]
        .into_iter()
        .collect();
    run_estimates(&root, &subjects, &sha_loc, NOW).expect("generate");
    let estimated_tokens = 6667 * TOKENS_PER_LOC; // 1_000_050

    // The receipts-only hand computation, pasted alongside:
    //   agent-a: runs=1 elapsed=120s tokens=100
    //   agent-b: runs=1 elapsed=30s  tokens=20
    let mut hand: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    hand.insert("agent-a".into(), (1, 120, 100));
    hand.insert("agent-b".into(), (1, 30, 20));
    println!("receipts-only hand computation: {hand:?}");
    let sums = crate::metrics::summarize_by_agent(&root).expect("sum over receipts");
    println!("summarize_by_agent over the MIXED tree: {sums:?}");
    assert_eq!(sums, hand, "estimates must not leak into the receipt sums");
    assert!(
        !sums.values().any(|(_, _, t)| *t >= estimated_tokens),
        "no summed total may carry the estimated magnitude"
    );
    assert!(
        crate::metrics::check_as_errors(&root).is_empty(),
        "metrics walker green on the mixed tree"
    );
    assert!(
        check_as_errors(&root).is_empty(),
        "estimates walker green on the mixed tree"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Live-repo smoke: the batched numstat pass reads real history, and the
/// exclusion rule holds against a known commit — the oldest subject
/// commit (64c054a6…) touched `.ai/tickets/scope-vocab.toml` (excluded) AND
/// xtask sources (included), so its included LOC is strictly positive.
#[test]
fn collect_numstat_live_repo_smoke() {
    let root = repo_root();
    let map = collect_numstat(&root).expect("numstat over live history");
    assert!(map.len() > 1000, "live history has thousands of commits");
    let subjects = mine_subjects(&root).expect("mine live subjects");
    let t9171 = subjects.get("T-917.1").expect("T-917.1 has subjects");
    let oldest = &t9171.first().expect("nonempty").sha;
    let loc = map.get(oldest).copied().unwrap_or(0);
    assert!(
        loc > 0,
        "T-917.1's vocab commit {oldest} has included (non-bookkeeping) LOC"
    );
}
