use super::*;

/// Acceptance 1 — the whole live tree round-trips byte-identically:
/// load every on-disk `T-*.toml` (parents AND children), render each typed ticket,
/// and the bytes must equal the file exactly — modulo the two pinned hand-edit
/// exceptions above, which must still be value-equal. N is measured on disk at run
/// time, never hardcoded (the corpus grows weekly).
#[test]
fn corpus_roundtrip_real_tree_byte_identical() {
    let root = repo_root();
    let dir = root.join(crate::repository::TICKETS_DIR);
    assert!(dir.is_dir(), "live tree missing at {}", dir.display());
    let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read tickets dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        })
        .collect();
    files.sort();
    let n = files.len();
    assert!(
        n > 800,
        "corpus scan must see the live tree (saw {n} files)"
    );
    assert_eq!(
        corpus.tickets.len(),
        n,
        "typed corpus must hold every on-disk file"
    );
    let mut identical = 0usize;
    let mut pinned_seen: Vec<String> = Vec::new();
    for path in &files {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let disk = fs::read_to_string(path).expect("read ticket file");
        let ticket = corpus.get(&stem).expect("loaded ticket");
        let rendered = render_ticket_toml(ticket).expect("render");
        let pinned = HAND_EDITED_NOT_CANONICAL.contains(&stem.as_str());
        if rendered == disk {
            assert!(
                !pinned,
                "{stem} is byte-canonical now — shrink HAND_EDITED_NOT_CANONICAL in this commit"
            );
            identical += 1;
            continue;
        }
        if !pinned {
            let min = rendered.len().min(disk.len());
            let mut i = 0;
            while i < min && rendered.as_bytes()[i] == disk.as_bytes()[i] {
                i += 1;
            }
            let lo = i.saturating_sub(60);
            panic!(
                "{stem}: render differs from disk at byte {i}\nrendered: {:?}\ndisk: {:?}",
                &rendered[lo..(i + 80).min(rendered.len())],
                &disk[lo..(i + 80).min(disk.len())]
            );
        }
        // Pinned files must still be pure formatting deviations: the canonical
        // render must re-parse to the very same typed value.
        let back = crate::parse_ticket_toml(&rendered)
            .unwrap_or_else(|e| panic!("{stem}: canonical render does not parse: {e}"));
        assert_eq!(&back, ticket, "{stem}: pinned exception is not value-equal");
        pinned_seen.push(stem);
    }
    assert_eq!(
        identical + pinned_seen.len(),
        n,
        "every file is byte-identical or a pinned hand-edit exception"
    );
    println!("{identical}/{n} files byte-identical");
    if !pinned_seen.is_empty() {
        println!(
            "{} pinned hand-edit exception(s), value-equal, formatting only: {}",
            pinned_seen.len(),
            pinned_seen.join(", ")
        );
    }
}

// after the latest drain batch
/// Permanent reversibility + ratchet proof over the live tree. The verb's
/// own in-run assertion proved join("\n") == the pre-move summary bytes while it
/// still held the original in memory; what stays provable forever is: (a) the
/// carrier count equals the pin exactly, (b) every carrier is a WORK ticket (only
/// the work-only quarantine pass may mint the field), (c) every parked wall is
/// still an actual wall (>SUMMARY_WORD_CAP words — the move criterion), and (d)
/// the wall survives a render → re-parse cycle byte-identically, so the
/// newline-join stays stable through every future canonical rewrite.
#[test]
fn migration_legacy_ratchet_pin() {
    let root = repo_root();
    let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
    let mut carriers = 0usize;
    for (id, t) in &corpus.tickets {
        let wall_lines = match t {
            Ticket::Program(p) => {
                assert!(
                    p.migration_legacy.is_empty(),
                    "{id}: program carries migration_legacy — only the work-only quarantine pass may mint the field"
                );
                continue;
            }
            Ticket::Work(w) => &w.migration_legacy,
        };
        if wall_lines.is_empty() {
            continue;
        }
        carriers += 1;
        let wall = wall_lines.join("\n");
        assert!(
            wall.split_whitespace().count() > crate::SUMMARY_WORD_CAP,
            "{id}: parked migration_legacy joins to {} words — not a wall; the quarantine only moves >{} word summaries",
            wall.split_whitespace().count(),
            crate::SUMMARY_WORD_CAP
        );
        let rendered = render_ticket_toml(t).expect("render carrier");
        let back = parse_ticket_toml(&rendered)
            .unwrap_or_else(|e| panic!("{id}: carrier render does not re-parse: {e}"));
        let back_legacy = match &back {
            Ticket::Work(w) => w.migration_legacy.clone(),
            Ticket::Program(_) => panic!("{id}: carrier re-parsed as program"),
        };
        assert_eq!(
            back_legacy.join("\n"),
            wall,
            "{id}: migration_legacy newline-join is not render-stable"
        );
    }
    assert_eq!(
        carriers, MIGRATION_LEGACY_PIN,
        "migration_legacy carrier count drifted from the pin — a drain commit must \
         SHRINK the pin in the same commit; growth means an illegal hand-mint"
    );
}

/// Third shrink-only ratchet (the [`migration_legacy_ratchet_pin`]
/// pattern, red BOTH ways): work+program tickets whose title is debt by THE
/// instrument ([`crate::title_is_debt`] — `title == id` OR TOML-parsed title
/// `split_whitespace().count() > 10`) must equal [`crate::TITLE_DEBT_PIN`]
/// exactly.
///
/// - **Growth is impossible by rule**: the ops post-image gate refuses writing a
///   debt title on any CHANGED ticket, so a count above the pin means a hand-edit
///   minted one — fix the title, never the pin.
/// - **Every drain batch that repairs titles SHRINKS the pin in the same
///   commit** by the measured batch amount (t920 spec §the drain).
#[test]
fn title_debt_ratchet_pin() {
    let root = repo_root();
    let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
    let mut debt = 0usize;
    for (id, t) in &corpus.tickets {
        let title = match t {
            Ticket::Program(p) => &p.title,
            Ticket::Work(w) => &w.title,
        };
        if crate::title_is_debt(id, title) {
            debt += 1;
        }
    }
    assert_eq!(
        debt,
        crate::TITLE_DEBT_PIN,
        "title-debt count drifted from TITLE_DEBT_PIN — a repair commit must SHRINK \
         the pin in the same commit; growth means a gate bypass (instrument: \
         title == id or TOML-parsed title split_whitespace().count() > 10)"
    );
}

/// Fourth shrink-only ratchet, same pattern: queued/ready/running/review
/// WORK tickets with empty `main_goal` ([`crate::main_goal_is_debt`]) must equal
/// [`crate::MAIN_GOAL_DEBT_PIN`] exactly. Quarantined carriers COUNT (the wall
/// holds the content unprocessed; the drain fills main_goal and shrinks
/// this pin in the same commit); new offenders are impossible — the ops
/// post-image gate refuses a changed non-quarantined live work ticket without
/// main_goal, and a quarantine mint past the cutover is red in check.
#[test]
fn main_goal_debt_ratchet_pin() {
    let root = repo_root();
    let corpus = Corpus::load(&root).expect("fail-closed load of the live tree");
    let debt = corpus
        .tickets
        .values()
        .filter(|t| match t {
            Ticket::Program(_) => false,
            Ticket::Work(w) => crate::main_goal_is_debt(w),
        })
        .count();
    assert_eq!(
        debt,
        crate::MAIN_GOAL_DEBT_PIN,
        "main_goal-debt count drifted from MAIN_GOAL_DEBT_PIN — a fill commit must \
         SHRINK the pin in the same commit; growth means a gate bypass (instrument: \
         queued/ready/running/review work tickets with empty main_goal)"
    );
}

/// `derive_next_parent_id` mirrors `tickets_store::derive_next_id`: max PARENT
/// numeric + 1; children never affect it.
#[test]
fn derive_next_parent_id_ignores_children() {
    let mut c = Corpus::new("/nonexistent");
    c.tickets
        .insert("T-001".into(), work("T-001", Status::Idea));
    c.tickets
        .insert("T-910".into(), work("T-910", Status::Idea));
    c.tickets
        .insert("T-090.6".into(), work("T-090.6", Status::Idea));
    // A dotted id with a huge numeral must not leak into the parent tier.
    c.tickets
        .insert("T-910.999".into(), work("T-910.999", Status::Idea));
    assert_eq!(c.derive_next_parent_id(), 911);
    let mut planted = Corpus::new("/nonexistent");
    planted
        .tickets
        .insert("T-950".into(), work("T-950", Status::Idea));
    assert_eq!(planted.derive_next_parent_id(), 951);
    assert_eq!(Corpus::new("/nonexistent").derive_next_parent_id(), 1);
}

/// Next child id is max direct numeric extension + 1 over BOTH corpus keys and the
/// parent's `children[]`; grandchildren never count; default is `.1`.
#[test]
fn next_child_id_direct_extensions_only() {
    let mut c = Corpus::new("/nonexistent");
    c.tickets
        .insert("T-916.1".into(), work("T-916.1", Status::Idea));
    c.tickets
        .insert("T-916.2".into(), work("T-916.2", Status::Idea));
    // Grandchild — a deeper extension must not bump the parent's own tier.
    c.tickets
        .insert("T-916.2.9".into(), work("T-916.2.9", Status::Idea));
    assert_eq!(c.next_child_id("T-916"), "T-916.3");
    assert_eq!(c.next_child_id("T-916.2"), "T-916.2.10");
    assert_eq!(c.next_child_id("T-999"), "T-999.1");
}

/// Fail-closed load: one broken file refuses the whole corpus, naming the file.
#[test]
fn load_refuses_naming_broken_file() {
    let root = scratch_dir("broken-load");
    let good = render_ticket_toml(&work("T-001", Status::Idea)).unwrap();
    fs::write(
        root.join(crate::repository::TICKETS_DIR).join("T-001.toml"),
        good,
    )
    .unwrap();
    fs::write(
        root.join(crate::repository::TICKETS_DIR).join("T-002.toml"),
        "id = \"T-002\"\nkind = \"nope\"\n",
    )
    .unwrap();
    let err = Corpus::load(&root).expect_err("broken file must refuse the load");
    assert!(err.contains("T-002.toml"), "must name the file: {err}");
}

/// Fail-closed load: id/filename mismatch is corruption, not data.
#[test]
fn load_refuses_id_filename_mismatch() {
    let root = scratch_dir("stem-mismatch");
    let text = render_ticket_toml(&work("T-001", Status::Idea)).unwrap();
    fs::write(
        root.join(crate::repository::TICKETS_DIR).join("T-777.toml"),
        text,
    )
    .unwrap();
    let err = Corpus::load(&root).expect_err("stem mismatch must refuse the load");
    assert!(err.contains("T-777") && err.contains("T-001"), "{err}");
}

/// Legality resolves at load: a work ticket whose scope pair is not in
/// the vocabulary refuses naming ticket + pair; a MISSING vocabulary refuses
/// naming the path (fail-closed — never a silent legality skip).
#[test]
fn load_refuses_vocab_illegal_scope_and_missing_vocab() {
    let root = scratch_dir("vocab-legality");
    let mut w = match work("T-001", Status::Idea) {
        Ticket::Work(w) => w,
        Ticket::Program(_) => unreachable!(),
    };
    w.scope.layer = "ghost_layer".into();
    fs::write(
        root.join(crate::repository::TICKETS_DIR).join("T-001.toml"),
        render_ticket_toml(&Ticket::Work(w)).unwrap(),
    )
    .unwrap();
    let err = Corpus::load(&root).expect_err("illegal pair must refuse");
    assert!(
        err.contains("T-001") && err.contains("repo.ghost_layer"),
        "must name ticket + offending pair: {err}"
    );

    fs::write(
        root.join(crate::repository::TICKETS_DIR).join("T-001.toml"),
        render_ticket_toml(&work("T-001", Status::Idea)).unwrap(),
    )
    .unwrap();
    Corpus::load(&root).expect("legal scope loads");

    fs::remove_file(root.join(crate::repository::SCOPE_VOCAB)).unwrap();
    let err = Corpus::load(&root).expect_err("missing vocab must refuse");
    assert!(err.contains("scope-vocab.toml"), "{err}");
}

/// Surgical write: temp+rename lands the rendered bytes and leaves no temp file;
/// an unknown id refuses before any byte lands.
#[test]
fn write_back_is_surgical_and_clean() {
    let root = scratch_dir("write-back");
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-001".into(), work("T-001", Status::Queued { order: 10 }));
    c.write_back(&["T-001".into()]).expect("write");
    let on_disk =
        fs::read_to_string(root.join(crate::repository::TICKETS_DIR).join("T-001.toml")).unwrap();
    assert_eq!(
        on_disk,
        render_ticket_toml(c.get("T-001").unwrap()).unwrap()
    );
    let leftovers: Vec<_> = fs::read_dir(root.join(crate::repository::TICKETS_DIR))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains(".tmp"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files left behind: {leftovers:?}"
    );
    let err = c
        .write_back(&["T-404".into()])
        .expect_err("unknown id refuses");
    assert!(err.contains("T-404"), "{err}");
}

/// `delete_files` refuses ids still present in the corpus.
#[test]
fn delete_files_refuses_live_ids() {
    let root = scratch_dir("delete-live");
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-001".into(), work("T-001", Status::Idea));
    let err = c
        .delete_files(&["T-001".into()])
        .expect_err("live id must refuse");
    assert!(err.contains("still in the corpus"), "{err}");
}
