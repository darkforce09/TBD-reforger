use super::*;
use crate::execution_metrics::measured::{self as metrics, MetricsState};
use crate::test_support::{Scratch, corpus_of, program, work, work_scoped};
use serde_json::json;
use std::any::TypeId;

fn write_est(root: &Path, name: &str, text: &str) -> String {
    let dir = estimates_dir(root);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(name), text).unwrap();
    format!("{}/{name}", ticket_engine::repository::ESTIMATES_DIR)
}

fn diff_loc_json(id: &str, loc: u64, factor: u64, shas: &[&str]) -> String {
    serde_json::to_string_pretty(&json!({
        "id": id,
        "source": "diff_loc",
        "factor": factor,
        "tokens_estimated": loc * factor,
        "generated_at": "2026-08-14T23:48:32Z",
        "loc_changed": loc,
        "derived_from_shas": shas,
    }))
    .unwrap()
}

fn cohort_json(id: &str, tokens: u64, class: Option<&str>, size: u64) -> String {
    let mut cohort = json!({});
    if let Some(c) = class {
        cohort["class"] = json!(c);
    }
    serde_json::to_string_pretty(&json!({
        "id": id,
        "source": "cohort_median",
        "factor": 150,
        "tokens_estimated": tokens,
        "generated_at": "2026-08-14T23:48:32Z",
        "cohort": cohort,
        "cohort_size": size,
    }))
    .unwrap()
}

fn loaded(state: EstimatesState) -> EstimatesModel {
    match state {
        EstimatesState::Loaded(m) => m,
        EstimatesState::NoEstimates => panic!("expected Loaded, got NoEstimates"),
    }
}

fn row<'a>(rows: &'a [EstimatedRow], key: &str) -> &'a EstimatedRow {
    rows.iter()
        .find(|r| r.key == key)
        .unwrap_or_else(|| panic!("no row for {key}"))
}

/// Hand fixture: website/feature diff_loc 400×150 = 60,000 over 2 shas;
/// repo/feature cohort 2,500 (n=3); website/bug diff_loc 10×150 =
/// 1,500 over 1 sha.
fn write_hand_estimates(root: &Path) {
    write_est(
        root,
        "T-1.json",
        &diff_loc_json("T-1", 400, 150, &["0123abc", "4567def"]),
    );
    write_est(
        root,
        "T-2.json",
        &cohort_json("T-2", 2_500, Some("feature"), 3),
    );
    write_est(
        root,
        "T-3.json",
        &diff_loc_json("T-3", 10, 150, &["89abcde"]),
    );
}

fn hand_corpus() -> Corpus {
    corpus_of(vec![
        work_scoped(
            "T-1",
            "domain = \"website\"\nlayer = \"frontend\"",
            "class = \"feature\"\n",
        ),
        work("T-2", "status = \"idea\"", "class = \"feature\"\n"),
        work_scoped(
            "T-3",
            "domain = \"website\"\nlayer = \"backend\"",
            "class = \"bug\"\n",
        ),
    ])
}

#[test]
fn absent_estimates_dir_is_the_explicit_no_estimates_state() {
    let s = Scratch::new("e-absent");
    fs::create_dir_all(s.path().join(ticket_engine::repository::TICKETS_DIR)).unwrap();
    let raw = load_raw(s.path());
    assert_eq!(raw, RawEstimates::default());
    assert_eq!(
        build_state(raw, &hand_corpus()),
        EstimatesState::NoEstimates
    );
    // The pinned render text names the directory and what produces a file in it.
    assert!(no_estimates_text().contains(".ai/tickets/estimates/"));
    assert!(no_estimates_text().contains("no estimates yet"));
    assert!(no_estimates_text().contains("run receipt"));
}

#[test]
fn empty_estimates_dir_is_no_estimates() {
    let s = Scratch::new("e-empty");
    fs::create_dir_all(estimates_dir(s.path())).unwrap();
    let raw = load_raw(s.path());
    assert!(raw.present && raw.records.is_empty() && raw.errors.is_empty());
    assert_eq!(
        build_state(raw, &hand_corpus()),
        EstimatesState::NoEstimates
    );
}

/// per-class: feature = 60,000 + 2,500 = 62,500 over 2 files (1 diff_loc /
/// 1 cohort_median); bug = 1,500 over 1 (1/0). per-domain: website =
/// 60,000 + 1,500 = 61,500 over 2 (2/0); repo = 2,500 over 1 (0/1). Grand:
/// 3 files, 64,000 tokens, 2 diff_loc / 1 cohort_median.
#[test]
fn hand_computed_sums_feature_62_500_bug_1_500_website_61_500_repo_2_500() {
    let s = Scratch::new("e-hand");
    write_hand_estimates(s.path());
    let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
    assert!(m.errors.is_empty(), "{:?}", m.errors);

    let feature = row(&m.per_class, "feature");
    assert_eq!(
        (
            feature.tickets,
            feature.tokens,
            feature.diff_loc,
            feature.cohort_median
        ),
        (2, EstimatedTokens(60_000 + 2_500), 1, 1)
    );
    let bug = row(&m.per_class, "bug");
    assert_eq!(
        (bug.tickets, bug.tokens, bug.diff_loc, bug.cohort_median),
        (1, EstimatedTokens(1_500), 1, 0)
    );

    let website = row(&m.per_domain, "website");
    assert_eq!(
        (website.tickets, website.tokens, website.diff_loc),
        (2, EstimatedTokens(60_000 + 1_500), 2)
    );
    let repo = row(&m.per_domain, "repo");
    assert_eq!(
        (repo.tickets, repo.tokens, repo.cohort_median),
        (1, EstimatedTokens(2_500), 1)
    );

    assert_eq!(m.grand.files, 3);
    assert_eq!(m.grand.tokens, EstimatedTokens(62_500 + 1_500));
    assert_eq!((m.grand.diff_loc, m.grand.cohort_median), (2, 1));
    assert_eq!((m.grand.classes, m.grand.domains), (2, 2));
    assert!(
        m.grand.strip.contains("64,000 tokens (estimated)"),
        "{}",
        m.grand.strip
    );
    assert!(
        m.grand.strip.contains("2 diff_loc / 1 cohort_median"),
        "{}",
        m.grand.strip
    );

    // Default sort: tokens desc — feature (62,500) and website (61,500) lead.
    assert_eq!(m.per_class[0].key, "feature");
    assert_eq!(m.per_domain[0].key, "website");
    // Precomputed display strings for the same hand numbers.
    assert_eq!(feature.tokens_str, "62,500");
    assert_eq!(website.tokens_str, "61,500");
    assert_eq!(
        (feature.tickets_str.as_str(), feature.diff_loc_str.as_str()),
        ("2", "1")
    );
}

/// Estimates whose ticket is missing, classless, or a program bucket under
/// explicit markers — stated, never guessed into a real class/domain.
#[test]
fn missing_ticket_classless_and_program_buckets_are_explicit() {
    let s = Scratch::new("e-buckets");
    write_est(
        s.path(),
        "T-7.json",
        &diff_loc_json("T-7", 2, 150, &["0123abc"]),
    );
    write_est(
        s.path(),
        "T-8.json",
        &diff_loc_json("T-8", 3, 150, &["0123abc"]),
    );
    write_est(
        s.path(),
        "T-9.json",
        &diff_loc_json("T-9", 4, 150, &["0123abc"]),
    );
    // absent from the corpus; classless work; a chore-classed
    // program (class is legal on programs; scope is not).
    let corpus = corpus_of(vec![
        work("T-8", "status = \"idea\"", ""),
        program("T-9", "status = \"idea\"\nclass = \"chore\"", &["T-9.1"]),
    ]);
    let m = loaded(build_state(load_raw(s.path()), &corpus));
    assert_eq!(
        row(&m.per_class, "(no ticket file)").tokens,
        EstimatedTokens(300)
    );
    assert_eq!(row(&m.per_class, "(no class)").tokens, EstimatedTokens(450));
    assert_eq!(row(&m.per_class, "chore").tokens, EstimatedTokens(600));
    assert_eq!(
        row(&m.per_domain, "(no ticket file)").tokens,
        EstimatedTokens(300)
    );
    assert_eq!(row(&m.per_domain, "repo").tokens, EstimatedTokens(450));
    assert_eq!(row(&m.per_domain, "(program)").tokens, EstimatedTokens(600));
}

#[test]
fn malformed_estimate_is_a_named_error_row_excluded_from_sums() {
    let s = Scratch::new("e-badsum");
    write_hand_estimates(s.path());
    // tokens_estimated 999 != 4 x 150 = 600 — the arithmetic mirror.
    let rel = write_est(
        s.path(),
        "T-4.json",
        r#"{"id":"T-4","source":"diff_loc","factor":150,"tokens_estimated":999,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":4,
               "derived_from_shas":["0123abc"]}"#,
    );
    let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
    assert_eq!(m.errors.len(), 1);
    assert_eq!(m.errors[0].rel, rel, "the error row NAMES the file");
    assert!(
        m.errors[0]
            .reason
            .contains("tokens_estimated (999) != loc_changed (4) x factor (150) = 600"),
        "{}",
        m.errors[0].reason
    );
    // The three valid files still aggregate to the hand numbers — the
    // broken file is excluded, not coerced and not fatal.
    assert_eq!(m.grand.files, 3);
    assert_eq!(m.grand.tokens, EstimatedTokens(64_000));
    assert!(
        !m.by_id.contains_key("T-4"),
        "no detail row off a broken file"
    );
}

#[test]
fn zero_valid_files_says_so_never_a_zeros_panel() {
    let s = Scratch::new("e-allbad");
    write_est(s.path(), "T-1.json", "not json at all");
    let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
    assert!(m.per_class.is_empty() && m.per_domain.is_empty());
    assert_eq!(m.grand.files, 0);
    assert!(
        m.grand
            .strip
            .contains("no valid estimate files — 1 malformed file(s)"),
        "{}",
        m.grand.strip
    );
}

#[test]
fn checker_mirror_rules_each_produce_a_named_error_row() {
    let s = Scratch::new("e-mirror");
    let not_json = write_est(s.path(), "a.json", "not json at all");
    // deny_unknown_fields mirrors additionalProperties: false.
    let unknown = write_est(
        s.path(),
        "T-1.json",
        r#"{"id":"T-1","source":"diff_loc","factor":150,"tokens_estimated":150,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":1,
               "derived_from_shas":["0123abc"],"elapsed_sec":9}"#,
    );
    // id does not match the filename stem.
    let mismatch = write_est(
        s.path(),
        "T-2.json",
        &diff_loc_json("T-3", 1, 150, &["0123abc"]),
    );
    let bad_sha = write_est(
        s.path(),
        "T-4.json",
        &diff_loc_json("T-4", 1, 150, &["XYZ"]),
    );
    // Fractional seconds fail the schema's second-precision pattern.
    let bad_shape = write_est(
        s.path(),
        "T-5.json",
        &diff_loc_json("T-5", 1, 150, &["0123abc"])
            .replace("2026-08-14T23:48:32Z", "2026-08-14T23:48:32.5Z"),
    );
    // Shape-valid digits, semantically impossible instant.
    let bad_instant = write_est(
        s.path(),
        "T-6.json",
        &diff_loc_json("T-6", 1, 150, &["0123abc"])
            .replace("2026-08-14T23:48:32Z", "2026-13-99T25:61:00Z"),
    );
    let bad_source = write_est(
        s.path(),
        "T-7.json",
        r#"{"id":"T-7","source":"guesswork","factor":150,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z"}"#,
    );
    // diff_loc carrying cohort fields (the per-source exclusion).
    let mixed = write_est(
        s.path(),
        "T-8.json",
        r#"{"id":"T-8","source":"diff_loc","factor":150,"tokens_estimated":150,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":1,
               "derived_from_shas":["0123abc"],"cohort_size":3}"#,
    );
    // cohort_median without its cohort key.
    let keyless = write_est(
        s.path(),
        "T-9.json",
        r#"{"id":"T-9","source":"cohort_median","factor":150,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z","cohort_size":3}"#,
    );
    let bad_factor = write_est(
        s.path(),
        "T-10.json",
        r#"{"id":"T-10","source":"cohort_median","factor":0,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z","cohort":{},"cohort_size":3}"#,
    );
    let bad_id = write_est(
        s.path(),
        "T-bogus.json",
        &diff_loc_json("T-bogus", 1, 150, &["0123abc"]),
    );
    // Files live flat — a subdirectory is an error row.
    fs::create_dir_all(estimates_dir(s.path()).join("T-11")).unwrap();

    let raw = load_raw(s.path());
    assert!(raw.records.is_empty(), "nothing valid may aggregate");
    let reason_of = |rel: &str| {
        &raw.errors
            .iter()
            .find(|e| e.rel == rel)
            .unwrap_or_else(|| panic!("no error row for {rel}: {:?}", raw.errors))
            .reason
    };
    assert!(reason_of(&not_json).contains("expected"));
    assert!(reason_of(&unknown).contains("elapsed_sec"));
    assert!(reason_of(&mismatch).contains("does not match its filename stem T-2"));
    assert!(reason_of(&bad_sha).contains("not 7-40 lowercase hex"));
    assert!(reason_of(&bad_shape).contains("schema pattern"));
    assert!(reason_of(&bad_instant).contains("RFC 3339"));
    assert!(reason_of(&bad_source).contains("unknown source \"guesswork\""));
    assert!(reason_of(&mixed).contains("carries no cohort fields"));
    assert!(reason_of(&keyless).contains("requires the cohort key"));
    assert!(reason_of(&bad_factor).contains("factor must be >= 1"));
    assert!(reason_of(&bad_id).contains("schema pattern"));
    let subdir = format!("{}/T-11", ticket_engine::repository::ESTIMATES_DIR);
    assert!(reason_of(&subdir).contains("unexpected subdirectory"));
    assert_eq!(raw.errors.len(), 12, "{:?}", raw.errors);
}

/// The glyph predicate per stamp + the verbatim-note tooltip: a measured
/// ticket (nothing in estimated[]) renders NO glyph and NO tooltip on any
/// stamp; a marked stamp carries the glyph with the note VERBATIM.
#[test]
fn stamp_cell_glyph_predicate_and_verbatim_note_tip() {
    let est = |names: &[&str]| names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    // Measured ticket: value present, estimated[] empty → Measured, glyphless.
    for name in ["created_at", "completed_at", "shipped_at"] {
        assert_eq!(
            stamp_cell(name, Some("2026-08-14T01:00:00Z"), &[], None),
            StampCell::Measured("2026-08-14T01:00:00Z".to_owned()),
            "{name} must carry no estimate state on a measured ticket"
        );
    }
    // Marked stamp with a note: glyph + the note text VERBATIM.
    let note = "created_at/completed_at/shipped_at git_subject-mined from 2 commit subject(s)";
    assert_eq!(
        stamp_cell(
            "created_at",
            Some("2026-06-18T23:27:00Z"),
            &est(&["created_at", "tokens"]),
            Some(note)
        ),
        StampCell::Estimated {
            value: "2026-06-18T23:27:00Z".to_owned(),
            tip: note.to_owned(),
        }
    );
    // A stamp NOT in estimated[] stays Measured even when others are marked.
    assert_eq!(
        stamp_cell(
            "completed_at",
            Some("2026-06-18T23:27:50Z"),
            &est(&["created_at"]),
            Some(note)
        ),
        StampCell::Measured("2026-06-18T23:27:50Z".to_owned())
    );
    // Marked, note absent: the explicit fallback — never an invented method.
    assert_eq!(
        stamp_cell("shipped_at", Some("abc1234"), &est(&["shipped_at"]), None),
        StampCell::Estimated {
            value: "abc1234".to_owned(),
            tip: NOTE_ABSENT_TIP.to_owned(),
        }
    );
    // Absent + unmarked: the plain em-dash state.
    assert_eq!(stamp_cell("shipped_at", None, &[], None), StampCell::Absent);
    assert!(stamp_estimated("scope", &est(&["scope"])));
    assert!(!stamp_estimated("tokens", &est(&["scope"])));
}

/// The absent-but-marked shipped_at model: `"— (estimated absent)"` + the
/// note tooltip (the live shape: no subject commits, a SHA is
/// never invented).
#[test]
fn absent_but_marked_shipped_at_renders_estimated_absent() {
    let note = "no subject commits; shipped_at left absent — a SHA is never invented";
    assert_eq!(
        stamp_cell(
            "shipped_at",
            None,
            &["shipped_at".to_owned(), "tokens".to_owned()],
            Some(note)
        ),
        StampCell::AbsentEstimated {
            tip: note.to_owned()
        }
    );
    assert_eq!(ABSENT_ESTIMATED_MARKER, "— (estimated absent)");
}

/// One glyph language: the stamp/tokens glyph is the scope glyph.
#[test]
fn estimate_glyph_matches_the_scope_glyph() {
    assert_eq!(ESTIMATE_GLYPH, board::SCOPE_ESTIMATED_GLYPH);
    assert_eq!(ESTIMATE_GLYPH, "~");
}

/// The tokens detail row, both sources: value, source, factor and inputs
/// all from the estimate file; the tooltip names source, inputs and factor
/// (the acceptance line).
#[test]
fn tokens_detail_row_model_both_sources() {
    let s = Scratch::new("e-detail");
    write_hand_estimates(s.path());
    let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));

    let diff = &m.by_id["T-1"];
    assert_eq!(diff.value_str, "60,000");
    assert_eq!(diff.source, "diff_loc");
    assert_eq!(diff.factor, 150);
    assert_eq!(diff.inputs_str, "400 LOC over 2 sha(s)");
    assert_eq!(
        diff.row_str,
        "60,000 · diff_loc ×150 · 400 LOC over 2 sha(s)"
    );
    for needle in [
        "source diff_loc",
        "factor ×150",
        "400 LOC over 2 sha(s)",
        "generated 2026-08-14T23:48:32Z",
    ] {
        assert!(diff.tip.contains(needle), "{} missing {needle}", diff.tip);
    }

    let cohort = &m.by_id["T-2"];
    assert_eq!(cohort.value_str, "2,500");
    assert_eq!(cohort.source, "cohort_median");
    assert_eq!(cohort.inputs_str, "cohort class=feature · n=3");
    assert!(
        cohort.tip.contains("source cohort_median"),
        "{}",
        cohort.tip
    );
    assert!(cohort.tip.contains("factor ×150"), "{}", cohort.tip);

    // The widened-to-{} cohort key renders its documented meaning.
    assert_eq!(
        cohort_key_str(&CohortKey {
            class: None,
            domain: None,
            layer: None
        }),
        "{} (all diff_loc tickets)"
    );
    assert_eq!(
        cohort_key_str(&CohortKey {
            class: Some("feature".into()),
            domain: Some("website".into()),
            layer: Some("frontend".into())
        }),
        "class=feature · domain=website · layer=frontend"
    );
}

/// The tokens-row states: marked + file → Estimated; marked + no valid
/// file → the explicit MissingFile hole naming the expected path; unmarked
/// → NO row at all, even when a file exists.
#[test]
fn tokens_cell_states() {
    let s = Scratch::new("e-cell");
    write_hand_estimates(s.path());
    let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
    let marked = vec!["tokens".to_owned()];

    match tokens_cell("T-1", &marked, Some(&m)) {
        Some(TokensCell::Estimated(d)) => assert_eq!(d.value_str, "60,000"),
        other => panic!("expected Estimated, got {other:?}"),
    }
    match tokens_cell("T-99", &marked, Some(&m)) {
        Some(TokensCell::MissingFile { tip }) => {
            assert!(tip.contains(".ai/tickets/estimates/T-99.json"), "{tip}");
        }
        other => panic!("expected MissingFile, got {other:?}"),
    }
    // NoEstimates state (model None): still the explicit hole.
    assert!(matches!(
        tokens_cell("T-1", &marked, None),
        Some(TokensCell::MissingFile { .. })
    ));
    // Unmarked: no row — an estimate file alone must not conjure one (the
    // marker ⇔ file coherence is check's rule; the UI renders only marked).
    assert_eq!(tokens_cell("T-1", &[], Some(&m)), None);
}

#[test]
fn sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak() {
    let mk = |key: &str, tickets: u64, tokens: u64, diff: u64, cohort: u64| EstimatedRow {
        key: key.to_owned(),
        tickets,
        tokens: EstimatedTokens(tokens),
        diff_loc: diff,
        cohort_median: cohort,
        tickets_str: tickets.to_string(),
        tokens_str: format_tokens(tokens),
        diff_loc_str: diff.to_string(),
        cohort_str: cohort.to_string(),
    };
    let mut rows = vec![
        mk("b", 1, 20, 1, 0),
        mk("a", 2, 150, 1, 1),
        mk("c", 2, 20, 0, 2),
    ];
    sort_rows(&mut rows, EstimatedSort::default()); // tokens desc, tie by key asc
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["a", "b", "c"], "150 first; 20-token tie b before c");

    let sort = EstimatedSort::default().toggled(EstimatedSortKey::Tokens); // same key → flip
    assert!(!sort.desc);
    sort_rows(&mut rows, sort);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["b", "c", "a"]);

    let sort = sort.toggled(EstimatedSortKey::Tickets); // new key → desc
    assert_eq!(
        sort,
        EstimatedSort {
            key: EstimatedSortKey::Tickets,
            desc: true
        }
    );
    sort_rows(&mut rows, sort);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["a", "c", "b"], "2-ticket tie a before c");

    sort_rows(
        &mut rows,
        EstimatedSort {
            key: EstimatedSortKey::CohortMedian,
            desc: true,
        },
    );
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["c", "a", "b"], "2 > 1 > 0 cohort_median");
}

/// Manual smoke against the LIVE repo estimates
/// (`cargo test -p ticketboard -- --ignored`): proves the mirror accepts
/// every real estimate file, so the estimated panel cannot light up with
/// false error rows on first launch. Ignored by default — the normal test
/// run stays hermetic (scratch dirs only).
#[test]
#[ignore = "reads the live repo estimates; run explicitly with -- --ignored"]
fn live_estimates_load_without_error_rows() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(root) =
        crate::ticket_registry::services::discovery::walk_up_for_tickets(&manifest_dir)
    else {
        panic!(
            "no {}/ above {}",
            ticket_engine::repository::TICKETS_DIR,
            manifest_dir.display()
        );
    };
    let raw = load_raw(&root);
    if !raw.present {
        println!("live estimates: directory absent — nothing to smoke");
        return;
    }
    assert!(
        raw.errors.is_empty(),
        "live estimate files refused: {:?}",
        raw.errors
    );
    println!(
        "live estimates: {} valid file(s), 0 error rows",
        raw.records.len()
    );
}

/// THE LAW, function-level. Fixtures where the SAME id carries a measured
/// receipt (170 tokens grand: 100+50, 20) AND estimate files
/// (62,500 grand: 60,000 diff_loc, 2,500 cohort). Combined
/// figures — grand 170+62,500 = 62,670, per-ticket 150+60,000 = 60,150 and
/// 20+2,500 = 2,520 — must appear in NO rendered surface of either model:
/// every display string is precomputed at load, so sweeping the models
/// sweeps everything the paint path can show.
///
/// Structural pin: the measured and estimated aggregation types are
/// disjoint (no shared row/grand type), and the estimated sum is the
/// [`EstimatedTokens`] newtype with no arithmetic against the measured
/// `u64` — this test must unwrap `.0` on purpose just to COMPUTE the
/// forbidden figures.
#[test]
fn the_law_no_code_path_combines_measured_and_estimated() {
    // Disjoint aggregation types: nothing can hand one table's row to the
    // other's renderer or summer.
    assert_ne!(
        TypeId::of::<metrics::MeasuredRow>(),
        TypeId::of::<EstimatedRow>()
    );
    assert_ne!(
        TypeId::of::<metrics::Grand>(),
        TypeId::of::<EstimatedTotals>()
    );
    assert_ne!(
        TypeId::of::<metrics::MetricsModel>(),
        TypeId::of::<EstimatesModel>()
    );

    let s = Scratch::new("e-law");
    // Measured receipts: 100 + 50, 20 (the metrics hand corpus).
    let receipt = |id: &str, tokens: u64, name: &str| {
        let dir = metrics::metrics_dir(s.path()).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(name),
            format!(
                r#"{{"id":"{id}","agent":"agent-a","started":"2026-08-14T01:00:00Z",
                       "finished":"2026-08-14T01:02:00Z",
                       "tokens_consumed":{{"input":{tokens},"output":0,"cache_read":0,
                       "cache_write":0,"total":{tokens}}}}}"#
            ),
        )
        .unwrap();
    };
    receipt("T-990", 100, "a.json");
    receipt("T-990", 50, "b.json");
    receipt("T-991", 20, "c.json");
    // Estimates for the SAME ids (on disk check forbids receipt+estimate
    // coexistence — the UI must stay separated even over dirty state).
    write_est(
        s.path(),
        "T-990.json",
        &diff_loc_json("T-990", 400, 150, &["0123abc"]),
    );
    write_est(
        s.path(),
        "T-991.json",
        &cohort_json("T-991", 2_500, Some("feature"), 3),
    );
    let corpus = corpus_of(vec![
        work("T-990", "status = \"idea\"", "class = \"feature\"\n"),
        work("T-991", "status = \"idea\"", "class = \"feature\"\n"),
    ]);

    let measured = match metrics::load_metrics(s.path()) {
        MetricsState::Loaded(m) => m,
        MetricsState::NoReceipts => panic!("receipts written"),
    };
    let estimated = loaded(build_state(load_raw(s.path()), &corpus));

    // The pure figures land in their OWN model...
    assert_eq!(measured.grand.tokens, 170u64);
    assert_eq!(estimated.grand.tokens, EstimatedTokens(62_500));
    // ...and computing a combined figure REQUIRES the deliberate unwrap:
    let combined_grand = format_tokens(measured.grand.tokens + estimated.grand.tokens.0);
    let combined_t990 = format_tokens(150 + 60_000u64);
    let combined_t991 = format_tokens(20 + 2_500u64);
    assert_eq!(combined_grand, "62,670");
    assert_eq!(combined_t990, "60,150");
    assert_eq!(combined_t991, "2,520");

    // Sweep EVERY rendered surface of both models.
    let mut surfaces: Vec<String> = Vec::new();
    surfaces.push(measured.grand.strip.clone());
    for r in measured.per_ticket.iter().chain(measured.per_agent.iter()) {
        surfaces.extend([
            r.key.clone(),
            r.runs_str.clone(),
            r.tokens_str.clone(),
            r.elapsed_str.clone(),
            r.unfinished_str.clone(),
            r.min_started.clone(),
            r.max_finished.clone().unwrap_or_default(),
        ]);
    }
    surfaces.push(estimated.grand.strip.clone());
    for r in estimated
        .per_class
        .iter()
        .chain(estimated.per_domain.iter())
    {
        surfaces.extend([
            r.key.clone(),
            r.tickets_str.clone(),
            r.tokens_str.clone(),
            r.diff_loc_str.clone(),
            r.cohort_str.clone(),
        ]);
    }
    for d in estimated.by_id.values() {
        surfaces.extend([
            d.value_str.clone(),
            d.source.clone(),
            d.inputs_str.clone(),
            d.row_str.clone(),
            d.tip.clone(),
        ]);
    }
    for combined in [&combined_grand, &combined_t990, &combined_t991] {
        assert!(
            surfaces.iter().all(|s| !s.contains(combined.as_str())),
            "a surface renders the forbidden measured+estimated figure {combined}: {:?}",
            surfaces.iter().find(|s| s.contains(combined.as_str()))
        );
    }
    // Each pure figure appears exactly where it belongs.
    assert!(measured.grand.strip.contains("170 tokens"));
    assert!(estimated.grand.strip.contains("62,500 tokens (estimated)"));
    assert!(!measured.grand.strip.contains("62,500"));
    assert!(!estimated.grand.strip.contains("170"));
}
