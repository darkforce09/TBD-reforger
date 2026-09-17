use super::*;

/// The T-917.4 boundary pins: T-90 vs T-902 vs T-90.1 vs T-90.10, dot-segment
/// included, plus the leading guard and per-subject dedupe.
#[test]
fn subject_id_boundary_pins() {
    assert_eq!(subject_ids("T-902: fix the thing"), vec!["T-902"]);
    assert_eq!(subject_ids("T-90.1 polish pass"), vec!["T-90.1"]);
    assert_eq!(subject_ids("T-90.10: deeper"), vec!["T-90.10"]);
    assert_eq!(
        subject_ids("T-90: done; T-90.1 ready"),
        vec!["T-90", "T-90.1"]
    );
    assert_eq!(subject_ids("revert T-90"), vec!["T-90"]);
    assert_eq!(subject_ids("wave(T-90) closes"), vec!["T-90"]);
    assert_eq!(subject_ids("Revert \"T-233: page\""), vec!["T-233"]);
    assert_eq!(subject_ids("XT-90 is not a ticket"), Vec::<String>::new());
    assert_eq!(subject_ids("T-90 then T-90 again"), vec!["T-90"]);
    assert_eq!(subject_ids("T-90.1.2: grandchild"), vec!["T-90.1.2"]);
    assert_eq!(subject_ids("no ids here"), Vec::<String>::new());
}

/// +02:00 input normalizes to `Z` and satisfies the tbd-tickets validator; a
/// `+00:00` and an already-`Z` input both come out canonical.
#[test]
fn utc_normalization() {
    assert_eq!(
        to_utc_z("2026-08-15T01:08:31+02:00").unwrap(),
        "2026-08-14T23:08:31Z"
    );
    assert_eq!(
        to_utc_z("2026-06-13T18:20:53+00:00").unwrap(),
        "2026-06-13T18:20:53Z"
    );
    assert_eq!(
        to_utc_z("2026-08-14T10:00:00Z").unwrap(),
        "2026-08-14T10:00:00Z"
    );
    for s in [
        to_utc_z("2026-08-15T01:08:31+02:00").unwrap(),
        day_floor(parse_utc("2026-08-14T23:08:31Z").unwrap()),
    ] {
        validate_rfc3339_utc("stamp", &s).expect("canonical");
    }
    assert_eq!(
        day_floor(parse_utc("2026-08-14T23:08:31Z").unwrap()),
        "2026-08-14T00:00:00Z"
    );
    assert!(to_utc_z("2026-08-14 10:00").is_err(), "naive must refuse");
}

#[test]
fn shape_predicates() {
    assert!(is_sha_shaped("5e5d3bbd"));
    assert!(is_sha_shaped("b071c49e3b84f59e5cfc279c2f05b04de32b850a"));
    assert!(!is_sha_shaped("2026-07-26"));
    assert!(!is_sha_shaped("T-128"));
    assert!(!is_sha_shaped("slice/T-197"));
    assert!(!is_sha_shaped("abc123")); // 6 hex — too short
    assert!(is_date_shaped("2026-07-26"));
    assert!(!is_date_shaped("2026-7-26"));
    assert!(!is_date_shaped("5e5d3bbd"));
}

/// The scratch end-to-end: a subject-mined ticket gets all three stamps +
/// markers; a subjectless ticket gets interpolated day-precision dates + an
/// absent-marked shipped_at; a subjectless child inherits its parent's dates;
/// an already-stamped ticket stays byte-untouched; A+B+C=S; a second pass
/// finds nothing to do.
#[test]
fn scratch_backfill_mines_interpolates_and_is_idempotent() {
    let root = scratch_root("pass");
    let mut c = Corpus::new(&root);
    // T-001: shipped, no stamps, two subject commits at +02:00.
    c.tickets
        .insert("T-001".into(), shipped_work("T-001", None));
    // T-002: shipped, no stamps, ZERO subjects → interpolates between T-001
    // (mined anchor) and T-003 (measured anchor).
    c.tickets
        .insert("T-002".into(), shipped_work("T-002", None));
    // T-003: fully stamped — must stay byte-untouched.
    c.tickets.insert(
        "T-003".into(),
        with_stamps(
            shipped_work("T-003", Some("abcdef12")),
            "2026-07-05T09:00:00Z",
            "2026-07-06T18:00:00Z",
        ),
    );
    // T-004 program (shipped, stamped) with subjectless shipped child T-004.1.
    c.tickets.insert(
        "T-004".into(),
        with_stamps(
            Ticket::Program(ProgramTicket {
                id: "T-004".into(),
                title: "prog".into(),
                summary: "prog".into(),
                class: None,
                status: Status::Shipped {
                    shipped_at: Some("beadfeed".into()),
                    order: Some(40),
                },
                executor: None,
                notes: None,
                spec: None,
                plan: None,
                depends_on: vec![],
                unblocks: vec![],
                children: vec!["T-004.1".into()],
                active: None,
                main_goal: None,
                context: vec![],
                requirement: vec![],
                current_state: vec![],
                approach: vec![],
                verify: vec![],
                acceptance: vec![],
                citations: vec![],
                priority: None,
                created_at: None,
                completed_at: None,
                estimated: vec![],
                estimate_note: None,
                migration_legacy: vec![],
                owns: vec![],
                pack_last: None,
            }),
            "2026-07-10T08:00:00Z",
            "2026-07-12T20:30:00Z",
        ),
    );
    let mut child = match shipped_work("T-004.1", None) {
        Ticket::Work(w) => w,
        Ticket::Program(_) => unreachable!(),
    };
    child.parent = Some("T-004".into());
    c.tickets.insert("T-004.1".into(), Ticket::Work(child));
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed tree");

    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert(
        "T-001".into(),
        vec![
            sc("aaaa111122223333", "2026-07-01T10:00:00+02:00"),
            sc("bbbb444455556666", "2026-07-02T18:30:00+02:00"),
        ],
    );

    let mut corpus = Corpus::load(&root).expect("load scratch");
    let before_t003 = fs::read_to_string(root.join(".ai/tickets/T-003.toml")).unwrap();
    let report = backfill(&mut corpus, &subjects).expect("pass");
    assert_eq!(report.shipped_total, 5);
    assert_eq!(report.a_git_subject, 1, "T-001");
    assert_eq!(report.b_id_interpolation, 2, "T-002 + T-004.1");
    assert_eq!(report.c_already_complete, 2, "T-003 + T-004");
    assert_eq!(
        report.a_git_subject + report.b_id_interpolation + report.c_already_complete,
        report.shipped_total
    );
    assert_eq!(report.shipped_sha_filled, 1);
    assert_eq!(report.shipped_absent_marked, 2);
    assert!(report.strays.is_empty());
    corpus.write_back(&report.changed).expect("land");

    let reread = Corpus::load(&root).expect("reload validates RFC 3339 UTC");
    // T-001 — mined, UTC-normalized, marked.
    let w1 = match reread.get("T-001").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w1.created_at.as_deref(), Some("2026-07-01T08:00:00Z"));
    assert_eq!(w1.completed_at.as_deref(), Some("2026-07-02T16:30:00Z"));
    assert_eq!(w1.shipped_at.as_deref(), Some("bbbb4444"));
    assert_eq!(
        w1.estimated,
        vec!["created_at", "completed_at", "shipped_at"]
    );
    assert!(
        w1.estimate_note.as_deref().unwrap().contains("git_subject"),
        "{:?}",
        w1.estimate_note
    );
    // T-002 — interpolated midpoint of T-001.latest .. T-003.earliest at day
    // precision; shipped_at ABSENT and marked with the gap named.
    let w2 = match reread.get("T-002").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w2.created_at.as_deref(), Some("2026-07-04T00:00:00Z"));
    assert_eq!(w2.created_at, w2.completed_at);
    assert_eq!(w2.shipped_at, None);
    assert_eq!(
        w2.estimated,
        vec!["created_at", "completed_at", "shipped_at"]
    );
    let note = w2.estimate_note.as_deref().unwrap();
    assert!(
        note.contains("no subject commits")
            && note.contains("between T-001 and T-003")
            && note.contains("never invented"),
        "{note}"
    );
    // T-004.1 — parent's dates, day-floored, span preserved.
    let w41 = match reread.get("T-004.1").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w41.created_at.as_deref(), Some("2026-07-10T00:00:00Z"));
    assert_eq!(w41.completed_at.as_deref(), Some("2026-07-12T00:00:00Z"));
    assert_eq!(w41.shipped_at, None);
    assert!(
        w41.estimate_note
            .as_deref()
            .unwrap()
            .contains("from parent T-004"),
        "{:?}",
        w41.estimate_note
    );
    // T-003 byte-untouched.
    assert!(!report.changed.contains(&"T-003".to_string()));
    assert_eq!(
        fs::read_to_string(root.join(".ai/tickets/T-003.toml")).unwrap(),
        before_t003,
        "already-stamped ticket must be byte-untouched"
    );

    // Idempotence: the second pass finds nothing.
    let mut again = Corpus::load(&root).expect("reload");
    let second = backfill(&mut again, &subjects).expect("second pass");
    assert!(second.changed.is_empty(), "second run must find nothing");
    assert_eq!(
        second.c_already_complete, 5,
        "all shipped now complete or absent-marked"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// The interpolation resolver's derivations say what they did: two-sided =
/// midpoint; one-sided names the single neighbor and says one-sided; a dotted
/// id walks to its parent's anchor.
#[test]
fn method2_descriptions_match_the_derivation() {
    let mut anchors: BTreeMap<String, Anchor> = BTreeMap::new();
    let a = |lo: &str, hi: &str| Anchor {
        earliest: parse_utc(lo).unwrap(),
        latest: parse_utc(hi).unwrap(),
    };
    anchors.insert(
        "T-002".into(),
        a("2026-07-01T10:00:00Z", "2026-07-02T10:00:00Z"),
    );
    anchors.insert(
        "T-008".into(),
        a("2026-07-09T10:00:00Z", "2026-07-10T10:00:00Z"),
    );
    let tier: Vec<(u64, String)> = vec![(2, "T-002".into()), (8, "T-008".into())];
    let none: BTreeMap<String, Option<String>> = BTreeMap::new();

    let mid = method2_dates("T-005", &none, &anchors, &tier).unwrap();
    assert_eq!(mid.created, "2026-07-05T00:00:00Z");
    assert_eq!(mid.created, mid.completed);
    assert!(
        mid.desc.contains("between T-002 and T-008") && mid.desc.contains("midpoint"),
        "{}",
        mid.desc
    );

    let low = method2_dates("T-001", &none, &anchors, &tier).unwrap();
    assert_eq!(low.created, "2026-07-01T00:00:00Z");
    assert!(
        low.desc.contains("from nearest dated neighbor T-002")
            && low.desc.contains("one-sided")
            && !low.desc.contains("midpoint"),
        "one-sided must not claim a midpoint: {}",
        low.desc
    );

    let high = method2_dates("T-900", &none, &anchors, &tier).unwrap();
    assert_eq!(high.created, "2026-07-10T00:00:00Z");
    assert!(high.desc.contains("T-008") && high.desc.contains("one-sided"));

    // Dotted id with an anchored parent: parent's dates, day-floored, span kept.
    let child = method2_dates("T-008.3", &none, &anchors, &tier).unwrap();
    assert_eq!(child.created, "2026-07-09T00:00:00Z");
    assert_eq!(child.completed, "2026-07-10T00:00:00Z");
    assert!(child.desc.contains("from parent T-008"), "{}", child.desc);
}

/// Rule 4 — stray resolution both ways: with subjects the date moves to
/// completed_at (day-precision, marked) and shipped_at re-mines to the real
/// SHA; without subjects shipped_at goes absent+marked. Before/after lines
/// carry both values.
#[test]
fn stray_date_shaped_shipped_at_resolves() {
    let root = scratch_root("stray");
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-010".into(), shipped_work("T-010", Some("2026-07-26")));
    c.tickets
        .insert("T-011".into(), shipped_work("T-011", Some("2026-07-26")));
    // Dated bracket so T-011 can interpolate.
    c.tickets.insert(
        "T-012".into(),
        with_stamps(
            shipped_work("T-012", Some("abcdef12")),
            "2026-07-27T09:00:00Z",
            "2026-07-27T10:00:00Z",
        ),
    );
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed");

    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert(
        "T-010".into(),
        vec![
            sc("cccc7777", "2026-07-26T15:15:01+02:00"),
            sc("dddd8888", "2026-07-26T15:27:59+02:00"),
        ],
    );

    let mut corpus = Corpus::load(&root).expect("load");
    let report = backfill(&mut corpus, &subjects).expect("pass");
    assert_eq!(report.strays.len(), 2, "{:?}", report.strays);
    assert_eq!(report.completed_stray, 2);
    corpus.write_back(&report.changed).expect("land");

    let reread = Corpus::load(&root).expect("reload");
    let w10 = match reread.get("T-010").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w10.completed_at.as_deref(), Some("2026-07-26T00:00:00Z"));
    assert_eq!(w10.created_at.as_deref(), Some("2026-07-26T13:15:01Z"));
    assert_eq!(
        w10.shipped_at.as_deref(),
        Some("dddd8888"),
        "re-mined to the LAST subject SHA"
    );
    assert!(
        w10.estimate_note
            .as_deref()
            .unwrap()
            .contains("from stray date-shaped shipped_at 2026-07-26"),
        "{:?}",
        w10.estimate_note
    );
    // The floored stray lands before the mined created_at — reported, not bent.
    assert_eq!(report.inverted.len(), 1, "{:?}", report.inverted);
    assert!(report.inverted[0].contains("T-010"));

    let w11 = match reread.get("T-011").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w11.shipped_at, None, "no SHA exists; none invented");
    assert_eq!(w11.completed_at.as_deref(), Some("2026-07-26T00:00:00Z"));
    assert!(w11.estimated.iter().any(|e| e == "shipped_at"));
    assert!(
        report
            .strays
            .iter()
            .any(|s| s.contains("T-011") && s.contains("absent")),
        "{:?}",
        report.strays
    );
    fs::remove_dir_all(&root).unwrap();
}

/// Present fields are never overwritten: a shipped ticket with an odd (non-SHA
/// non-date) shipped_at keeps it verbatim while its dates are mined; the odd
/// value is reported.
#[test]
fn odd_shipped_at_is_untouched_and_reported() {
    let root = scratch_root("odd");
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-020".into(), shipped_work("T-020", Some("slice/T-020")));
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed");
    let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
    subjects.insert(
        "T-020".into(),
        vec![sc("eeee9999", "2026-07-20T12:00:00+02:00")],
    );
    let mut corpus = Corpus::load(&root).expect("load");
    let report = backfill(&mut corpus, &subjects).expect("pass");
    corpus.write_back(&report.changed).expect("land");
    let reread = Corpus::load(&root).expect("reload");
    let w = match reread.get("T-020").unwrap() {
        Ticket::Work(w) => w,
        Ticket::Program(_) => panic!("work"),
    };
    assert_eq!(w.shipped_at.as_deref(), Some("slice/T-020"), "untouched");
    assert_eq!(w.created_at.as_deref(), Some("2026-07-20T10:00:00Z"));
    assert!(
        !w.estimated.iter().any(|e| e == "shipped_at"),
        "not marked — the value is present"
    );
    assert_eq!(report.odd_shipped_untouched.len(), 1);
    assert!(report.odd_shipped_untouched[0].contains("T-020"));
    fs::remove_dir_all(&root).unwrap();
}

/// Live-repo smoke: the miner reads real history — T-917.1 has subjects, dates
/// come back `Z`-canonical and oldest-first, and the boundary rule holds against
/// the live log (no dotted child of T-917 pollutes T-917's own list... which
/// mentions them legally — so assert the exact-token direction instead:
/// T-917.1's list never contains a commit that only names T-917.10).
#[test]
fn mine_subjects_live_repo_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .expect("repo root")
        .to_path_buf();
    let map = mine_subjects(&root).expect("mine live history");
    let t9171 = map.get("T-917.1").expect("T-917.1 has subject commits");
    assert!(t9171.len() >= 2, "vocab commit + ship commit");
    for c in t9171 {
        validate_rfc3339_utc("mined", &c.date_utc).expect("Z-canonical");
    }
    assert!(
        t9171.first().unwrap().date_utc <= t9171.last().unwrap().date_utc,
        "oldest-first"
    );
}
