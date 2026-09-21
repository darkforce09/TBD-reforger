use super::*;

/// T-916.1 acceptance 2 — add-child onto a work parent refuses without `promote`;
/// with `promote` the parent atomically becomes a program (scope dropped, every
/// other field preserved) carrying the freshly minted first child.
#[test]
fn add_child_onto_work_refuses_then_promotes() {
    let mut c = corpus(vec![Ticket::Work(work("T-5", Status::Queued { order: 9 }))]);
    let before = c.clone();
    let err = add_child(&mut c, "T-5", "First slice", "", false, CLOCK)
        .expect_err("work parent without promote must refuse");
    assert!(
        err.contains("kind work") && err.contains("promote"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");

    let (cid, out) = add_child(&mut c, "T-5", "First slice", "", true, CLOCK)
        .expect("promote rewrites work→program");
    assert_eq!(cid, "T-5.1");
    assert_eq!(out.changed, vec!["T-5".to_string(), "T-5.1".to_string()]);
    match c.get("T-5").unwrap() {
        Ticket::Program(p) => {
            assert_eq!(p.children, vec!["T-5.1".to_string()]);
            assert_eq!(p.active, None);
            assert_eq!(p.status, Status::Queued { order: 9 }, "status preserved");
            assert_eq!(
                p.executor.as_deref(),
                Some("claude-code"),
                "fields preserved"
            );
            assert_eq!(p.owns, vec!["T-5.surface".to_string()], "owns preserved");
            let rendered = render_ticket_toml(c.get("T-5").unwrap()).unwrap();
            assert!(rendered.contains("kind = \"program\""), "{rendered}");
            assert!(
                !rendered.contains("[scope"),
                "scope must be dropped:\n{rendered}"
            );
        }
        Ticket::Work(_) => panic!("T-5 must be a program now"),
    }
    match c.get("T-5.1").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(w.parent.as_deref(), Some("T-5"));
            assert_eq!(w.status, Status::Idea);
            assert_eq!(w.created_at.as_deref(), Some(CLOCK));
            assert_eq!(w.summary, "First slice", "summary falls back to title");
        }
        Ticket::Program(_) => panic!("minted child must be work"),
    }
}

/// Promotion refuses, by name, the two fields a program cannot carry.
#[test]
fn promote_refuses_parented_and_stray_shipped_at() {
    let mut parented = work("T-6.1", Status::Idea);
    parented.parent = Some("T-6".into());
    let mut c = corpus(vec![
        program("T-6", Status::Idea, &["T-6.1"], None),
        Ticket::Work(parented),
    ]);
    let err = add_child(&mut c, "T-6.1", "x", "", true, CLOCK).expect_err("parented");
    assert!(err.contains("parent") && err.contains("T-6"), "{err}");

    let mut stray = work("T-7", Status::Queued { order: 3 });
    stray.shipped_at = Some("abc123".into());
    let mut c = corpus(vec![Ticket::Work(stray)]);
    let err = add_child(&mut c, "T-7", "x", "", true, CLOCK).expect_err("stray shipped_at");
    assert!(err.contains("shipped_at"), "{err}");
}

#[test]
fn add_child_onto_program_appends_next_free_id() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1"], None),
        child_of("T-1", "T-1.1", Status::Idea),
    ]);
    let (cid, out) = add_child(&mut c, "T-1", "Second", "sum", false, CLOCK).expect("append");
    assert_eq!(cid, "T-1.2");
    assert_eq!(out.changed, vec!["T-1".to_string(), "T-1.2".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => {
            assert_eq!(p.children, vec!["T-1.1".to_string(), "T-1.2".to_string()]);
        }
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
}

/// T-916.1 acceptance 2 — duplicate `children[]` entries refuse at the post-image
/// gate (corpus-wide; the 4a2f3426 class), whatever op tries to write.
#[test]
fn duplicate_child_refuses_any_op() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1", "T-1.1"], None),
        child_of("T-1", "T-1.1", Status::Idea),
    ]);
    let before = c.clone();
    let err = set_status(&mut c, "T-1.1", "deferred", CLOCK).expect_err("dup children");
    assert!(
        err.contains("duplicate child") && err.contains("T-1.1"),
        "{err}"
    );
    assert_eq!(before, c);
}

/// T-916.1 acceptance 2/3 — remove of a program refuses without `force`; with
/// `force` it cascade-deletes the full descendant closure (children[] edges AND
/// work parent back-edges, nested programs included).
#[test]
fn remove_program_refuses_then_force_cascades() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
        child_of("T-1", "T-1.1", Status::Idea),
        program("T-1.2", Status::Idea, &["T-1.2.1"], None),
        child_of("T-1.2", "T-1.2.1", Status::Idea),
    ]);
    let before = c.clone();
    let err = remove(&mut c, "T-1", false, CLOCK).expect_err("program refuses");
    assert!(err.contains("force") && err.contains("cascade"), "{err}");
    assert_eq!(before, c);

    let out = remove(&mut c, "T-1", true, CLOCK).expect("force cascades");
    assert_eq!(
        out.deleted,
        vec![
            "T-1".to_string(),
            "T-1.1".to_string(),
            "T-1.2".to_string(),
            "T-1.2.1".to_string()
        ]
    );
    assert!(out.changed.is_empty());
    assert!(c.tickets.is_empty(), "closure removes every descendant");
}

/// T-916.1 acceptance 2 — removing the last child of a program refuses, naming the
/// fix (programs require children).
#[test]
fn remove_last_child_of_program_refuses() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1"], None),
        child_of("T-1", "T-1.1", Status::Idea),
    ]);
    let before = c.clone();
    let err = remove(&mut c, "T-1.1", false, CLOCK).expect_err("last child");
    assert!(
        err.contains("no children") && err.contains("T-1"),
        "must name program and fix: {err}"
    );
    assert_eq!(before, c);
}

#[test]
fn remove_work_scrubs_parent_children() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
        child_of("T-1", "T-1.1", Status::Idea),
        child_of("T-1", "T-1.2", Status::Idea),
    ]);
    let out = remove(&mut c, "T-1.1", false, CLOCK).expect("remove child");
    assert_eq!(out.deleted, vec!["T-1.1".to_string()]);
    assert_eq!(out.changed, vec!["T-1".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => assert_eq!(p.children, vec!["T-1.2".to_string()]),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
    assert!(c.get("T-1.1").is_none());
}

/// A work ticket double-listed by a second program (the live T-067.1 shape) cannot
/// be removed while that listing dangles — the corpus-wide referential check
/// refuses, naming the listing program.
#[test]
fn remove_double_listed_child_refuses() {
    let mut c = corpus(vec![
        program("T-8", Status::Idea, &["T-8.1", "T-8.2"], None),
        child_of("T-8", "T-8.1", Status::Idea),
        child_of("T-8", "T-8.2", Status::Idea),
        program("T-9", Status::Idea, &["T-8.1"], None),
    ]);
    let before = c.clone();
    let err = remove(&mut c, "T-8.1", false, CLOCK).expect_err("dangling listing");
    assert!(err.contains("T-9") && err.contains("T-8.1"), "{err}");
    assert_eq!(before, c);
}

/// T-916.1 acceptance 2 — a reorder that would land on an occupied live order
/// refuses instead of writing red state (the sanctioned wedge fix).
#[test]
fn reorder_collision_refuses() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(work("T-2", Status::Queued { order: 11 })),
        Ticket::Work(work("T-3", Status::Queued { order: 20 })),
    ]);
    let before = c.clone();
    let err = reorder(&mut c, "T-3", "T-1", CLOCK).expect_err("11 is taken");
    assert!(err.contains("duplicate live order 11"), "{err}");
    assert!(err.contains("T-2") && err.contains("T-3"), "{err}");
    assert_eq!(before, c, "the colliding write never lands");
    // A free slot works and flips nothing else.
    let out = reorder(&mut c, "T-3", "T-2", CLOCK).expect("12 is free");
    assert_eq!(out.changed, vec!["T-3".to_string()]);
    match c.get("T-3").unwrap() {
        Ticket::Work(w) => assert_eq!(w.status, Status::Queued { order: 12 }),
        Ticket::Program(_) => panic!("T-3 must stay work"),
    }
}

/// Reorder flips idea→queued (cmd_reorder), which makes the ticket live — so an
/// owns-empty work idea refuses (the op made it live), while an owned one flips.
#[test]
fn reorder_flips_idea_and_requires_owns() {
    let mut bare = work("T-2", Status::Idea);
    bare.owns = vec![];
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(bare),
        Ticket::Work(work("T-3", Status::Idea)),
    ]);
    let err = reorder(&mut c, "T-2", "T-1", CLOCK).expect_err("owns-empty made live");
    assert!(err.contains("owns required"), "{err}");
    let out = reorder(&mut c, "T-3", "T-1", CLOCK).expect("owned idea flips");
    assert_eq!(out.changed, vec!["T-3".to_string()]);
    match c.get("T-3").unwrap() {
        Ticket::Work(w) => assert_eq!(w.status, Status::Queued { order: 11 }),
        Ticket::Program(_) => panic!("T-3 must stay work"),
    }
}

/// Don't retro-police: the live tree already carries parent↔child live-order
/// collisions the parents-only `validate_registry` never reds (order 900 across
/// the T-090 family, measured 2026-08-14). Ops that do not introduce a NEW
/// collision must keep working on such a corpus.
#[test]
fn preexisting_collision_is_not_retro_policed() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 900 })),
        Ticket::Work(work("T-2", Status::Queued { order: 900 })),
        Ticket::Work(work("T-3", Status::Queued { order: 10 })),
    ]);
    let out = set_status(&mut c, "T-3", "deferred", CLOCK)
        .expect("op away from the collision must not be blocked by preexisting red");
    assert_eq!(out.changed, vec!["T-3".to_string()]);
    // But JOINING the preexisting collision is still a new pair — refuse.
    let mut c2 = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 899 })),
        Ticket::Work(work("T-2", Status::Queued { order: 900 })),
        Ticket::Work(work("T-4", Status::Queued { order: 900 })),
        Ticket::Work(work("T-3", Status::Queued { order: 10 })),
    ]);
    let err = reorder(&mut c2, "T-3", "T-1", CLOCK).expect_err("joining 900 refuses");
    assert!(err.contains("duplicate live order 900"), "{err}");
}

/// Both anchor failure modes print the exact legacy string (cmd_reorder prints
/// "Unknown anchor ticket" for missing AND for order-less anchors).
#[test]
fn reorder_unknown_anchor_message() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(work("T-2", Status::Idea)),
    ]);
    let err = reorder(&mut c, "T-1", "T-404", CLOCK).expect_err("missing anchor");
    assert_eq!(err, "Unknown anchor ticket: T-404");
    let err = reorder(&mut c, "T-1", "T-2", CLOCK).expect_err("order-less anchor");
    assert_eq!(err, "Unknown anchor ticket: T-2");
}

/// cmd_advance_slice walk over typed children: first child when no active, next
/// after the current one, refuse past the end, refuse active-not-in-children —
/// legacy refusal strings verbatim.
#[test]
fn advance_slice_walks_and_refuses() {
    let mut c = corpus(vec![
        program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
        child_of("T-1", "T-1.1", Status::Idea),
        child_of("T-1", "T-1.2", Status::Idea),
        Ticket::Work(work("T-2", Status::Idea)),
    ]);
    advance_slice(&mut c, "T-1", CLOCK).expect("first child");
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.1")),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
    advance_slice(&mut c, "T-1", CLOCK).expect("next child");
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.2")),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
    let err = advance_slice(&mut c, "T-1", CLOCK).expect_err("past end");
    assert_eq!(err, "T-1: no slice after T-1.2");
    let err = advance_slice(&mut c, "T-2", CLOCK).expect_err("work has no children");
    assert_eq!(err, "T-2 has no slices[]");
    if let Some(Ticket::Program(p)) = c.tickets.get_mut("T-1") {
        p.active = Some("T-9.9".into());
    }
    let err = advance_slice(&mut c, "T-1", CLOCK).expect_err("bogus active");
    assert_eq!(err, "active_slice T-9.9 not in slices[]");
}

/// T-916.1 acceptance 3 — the 4a2f3426 alias-class regression pin. The bug: the
/// Value path mirrored `children`→`slices` and `active`→`active_slice`, and a
/// value carrying BOTH spellings blew up serde's alias handling ("duplicate field
/// `children`"), breaking every mutator. Through the typed path the mirrored-keys
/// condition is unrepresentable: legacy spellings on disk still PARSE (serde
/// aliases), but no rendered output of any op ever contains a `slices =` or
/// `active_slice =` line — there is nothing to clash.
#[test]
fn mirrored_keys_unrepresentable_4a2f3426_pin() {
    // Legacy spellings parse via alias into the canonical typed fields…
    let legacy = r#"
id = "T-1"
kind = "program"
title = "t"
summary = "s"
status = "idea"
slices = ["T-1.1", "T-1.2"]
active_slice = "T-1.1"
"#;
    let t = parse_ticket_toml(legacy).expect("aliases parse");
    let rendered = render_ticket_toml(&t).expect("render");
    assert!(rendered.contains("children = ["), "{rendered}");
    assert!(rendered.contains("active = \"T-1.1\""), "{rendered}");
    // …and after real ops, NO ticket in the corpus renders a mirrored key.
    let mut c = corpus(vec![
        t,
        child_of("T-1", "T-1.1", Status::Idea),
        child_of("T-1", "T-1.2", Status::Idea),
    ]);
    advance_slice(&mut c, "T-1", CLOCK).expect("advance");
    ship(&mut c, "T-1.1", CLOCK).expect("ship child");
    for (id, ticket) in &c.tickets {
        let out = render_ticket_toml(ticket).expect("render");
        for line in out.lines() {
            assert!(
                !line.starts_with("slices = ") && !line.starts_with("active_slice = "),
                "{id} rendered a mirrored legacy key:\n{out}"
            );
        }
    }
}

/// Injected-clock determinism: the same op sequence with the same stamp produces
/// byte-identical renders — nothing inside ops ever reads the wall clock.
#[test]
fn injected_clock_determinism() {
    let build = || {
        corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            Ticket::Work(work("T-2", Status::Idea)),
        ])
    };
    let run = |c: &mut Corpus| {
        let (tid, _) = add(c, "Minted", "", CLOCK).expect("add");
        assert_eq!(tid, "T-003", "cmd_add zero-pads: T-{{:03}}");
        add_child(c, "T-2", "Slice one", "", true, CLOCK).expect("promote");
        reorder(c, "T-003", "T-1", CLOCK).expect_err("minted idea has empty owns");
        set_status(c, "T-1", "cancelled", CLOCK).expect("cancel");
        // T-920.1: a minted child ships only with its body filled (the ship
        // ready-tier gate) — fill it deterministically, then ship.
        if let Some(Ticket::Work(w)) = c.tickets.get_mut("T-2.1") {
            w.main_goal = Some("slice goal".into());
            w.context = vec!["ctx".into()];
            w.requirement = vec!["ask".into()];
            w.current_state = vec!["today".into()];
            w.approach = vec!["steps".into()];
            w.verify = vec!["cargo test".into()];
            w.acceptance = vec!["done".into()];
        }
        ship(c, "T-2.1", CLOCK).expect("ship child");
        let mut all = String::new();
        for ticket in c.tickets.values() {
            all.push_str(&render_ticket_toml(ticket).expect("render"));
            all.push('\n');
        }
        all
    };
    let (mut a, mut b) = (build(), build());
    let (ra, rb) = (run(&mut a), run(&mut b));
    assert_eq!(ra, rb, "same clock, same bytes");
    assert!(ra.contains(CLOCK), "stamps came from the injected clock");
}

/// End-to-end on disk: op outcome feeds write_back + delete_files, and the tree
/// reflects exactly the changed/deleted sets — nothing else.
#[test]
fn remove_cascade_end_to_end_on_disk() {
    let root = scratch_root("remove-e2e");
    let mut c = Corpus::new(&root);
    for t in [
        program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
        child_of("T-1", "T-1.1", Status::Idea),
        child_of("T-1", "T-1.2", Status::Idea),
        Ticket::Work(work("T-2", Status::Idea)),
    ] {
        c.tickets.insert(t.id().to_string(), t);
    }
    let all: Vec<String> = c.tickets.keys().cloned().collect();
    c.write_back(&all).expect("seed tree");
    let out = remove(&mut c, "T-1", true, CLOCK).expect("cascade");
    c.write_back(&out.changed).expect("write changed");
    c.delete_files(&out.deleted).expect("delete files");
    let left: Vec<String> = fs::read_dir(root.join(crate::repository::TICKETS_DIR))
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        left,
        vec!["T-2.toml".to_string()],
        "only the survivor remains"
    );
}

/// T-917.3: an op may not mint a NEW summary wall — `add` with a >40-word summary
/// refuses pre-write (corpus byte-untouched, the ops refusal-test pattern), naming
/// the count and the cap.
#[test]
fn add_refuses_wall_summary_pre_write() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Idea))]);
    let before = c.clone();
    let wall = "wall ".repeat(crate::SUMMARY_WORD_CAP + 1);
    let err = add(&mut c, "Short title", wall.trim(), CLOCK).expect_err("wall must refuse");
    assert!(
        err.contains("41 words") && err.contains("cap 40"),
        "must name count and cap: {err}"
    );
    assert_eq!(c, before, "refused op must leave the corpus untouched");
}

/// T-917.3: nonempty `migration_legacy` exempts exactly the summary cap — an op
/// touching a quarantined ticket (summary := title, possibly >40 words) commits.
#[test]
fn quarantined_ticket_is_exempt_from_summary_cap() {
    let mut w = work("T-1", Status::Queued { order: 5 });
    w.summary = "word ".repeat(50).trim().to_string();
    w.migration_legacy = vec!["the original wall".into()];
    let mut c = corpus(vec![Ticket::Work(w)]);
    set_status(&mut c, "T-1", "deferred", CLOCK).expect("quarantined ticket must stay mutable");

    // The same summary WITHOUT the quarantine marker refuses.
    let mut unmarked = work("T-2", Status::Queued { order: 6 });
    unmarked.summary = "word ".repeat(50).trim().to_string();
    let mut c = corpus(vec![Ticket::Work(unmarked)]);
    let err = set_status(&mut c, "T-2", "deferred", CLOCK)
        .expect_err("unquarantined wall in a changed ticket must refuse");
    assert!(err.contains("50 words") && err.contains("cap 40"), "{err}");
}
