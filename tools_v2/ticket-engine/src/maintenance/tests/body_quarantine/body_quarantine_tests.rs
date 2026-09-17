use super::*;

/// Walls move (multi-line split on newlines, single-line one-element), join
/// reproduces the original bytes ON DISK, summary becomes the title verbatim
/// (over-cap titles included, untruncated), short summaries and programs stay
/// untouched, and a second pass finds nothing.
#[test]
fn quarantine_moves_walls_reversibly_then_second_run_is_empty() {
    let root = scratch_root("pass");
    let multiline = format!(
        "{}\n{}\nquoted \"err\" and a \\ path",
        words(30, "a"),
        words(20, "b")
    );
    let long_title = words(45, "t");
    let program_toml = format!(
        "id = \"T-005\"\nkind = \"program\"\ntitle = \"prog\"\nsummary = \"{}\"\nstatus = \"idea\"\nchildren = [\"T-005.1\"]\n",
        words(60, "p")
    );
    let mut c = Corpus::new(&root);
    for t in [
        work("T-001", "multi-line wall", &multiline),
        work("T-002", "short stays", "well under the cap"),
        work("T-003", "single-line wall", &words(41, "s")),
        work("T-004", &long_title, &words(50, "w")),
    ] {
        c.tickets.insert(t.id().to_string(), t);
    }
    let seed: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&seed).expect("seed tree");
    fs::write(root.join(".ai/tickets/T-005.toml"), program_toml).expect("seed program");

    let mut corpus = Corpus::load(&root).expect("load scratch");
    let before_short = fs::read_to_string(root.join(".ai/tickets/T-002.toml")).unwrap();
    let report = quarantine_walls(&mut corpus).expect("pass");
    assert_eq!(report.moved, vec!["T-001", "T-003", "T-004"]);
    assert_eq!(
        report.over_cap_titles,
        vec!["T-004"],
        "45-word title is over-cap"
    );
    assert_eq!(
        report.program_walls,
        vec!["T-005"],
        "program wall reported, not moved"
    );
    corpus.write_back(&report.moved).expect("land");

    let reread = Corpus::load(&root).expect("reload");
    for (id, original, want_lines) in [
        ("T-001", multiline.as_str(), 3),
        ("T-003", &words(41, "s") as &str, 1),
    ] {
        let w = match reread.get(id) {
            Some(Ticket::Work(w)) => w,
            _ => panic!("{id} must be work"),
        };
        assert_eq!(
            w.migration_legacy.len(),
            want_lines,
            "{id} split on newlines only"
        );
        assert_eq!(
            w.migration_legacy.join("\n"),
            original,
            "{id} join == original bytes"
        );
        assert_eq!(w.summary, w.title, "{id} summary is the title verbatim");
    }
    let w4 = match reread.get("T-004") {
        Some(Ticket::Work(w)) => w,
        _ => panic!("work"),
    };
    assert_eq!(
        w4.summary, long_title,
        "over-cap title kept verbatim, untruncated"
    );
    assert_eq!(
        fs::read_to_string(root.join(".ai/tickets/T-002.toml")).unwrap(),
        before_short,
        "under-cap ticket is byte-untouched"
    );
    match reread.get("T-005") {
        Some(Ticket::Program(p)) => {
            assert!(p.migration_legacy.is_empty(), "programs are never moved")
        }
        _ => panic!("program"),
    }

    // Second pass: idempotent by emptiness.
    let mut again = Corpus::load(&root).expect("reload");
    let second = quarantine_walls(&mut again).expect("second pass");
    assert!(
        second.moved.is_empty(),
        "second run must find nothing to move"
    );
    assert_eq!(second.program_walls, vec!["T-005"], "program note persists");
    fs::remove_dir_all(&root).unwrap();
}

/// The moved files still render canonically: a full render → re-parse of a parked
/// ticket is value-stable (what keeps the corpus roundtrip gate green).
#[test]
fn parked_ticket_renders_canonically() {
    let root = scratch_root("canonical");
    let mut c = Corpus::new(&root);
    let wall = format!(
        "{}\nsecond \"line\" with escapes \\ and | pipes",
        words(41, "x")
    );
    c.tickets.insert("T-001".into(), work("T-001", "t", &wall));
    c.write_back(&["T-001".into()]).expect("seed");
    let mut corpus = Corpus::load(&root).expect("load");
    let report = quarantine_walls(&mut corpus).expect("pass");
    assert_eq!(report.moved, vec!["T-001"]);
    corpus.write_back(&report.moved).expect("land");
    let disk = fs::read_to_string(root.join(".ai/tickets/T-001.toml")).unwrap();
    let reparsed = crate::parse_ticket_toml(&disk).expect("landed file parses");
    assert_eq!(
        render_ticket_toml(&reparsed).unwrap(),
        disk,
        "landed bytes are the canonical render"
    );
    fs::remove_dir_all(&root).unwrap();
}
