use super::*;

/// A malformed injected clock refuses on entry — including in ops that never stamp
/// (uniform contract: caller wiring bugs surface on the first op).
#[test]
fn ops_refuse_malformed_clock() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    for bad in ["2026-08-14 12:00", "2026-08-14T12:00:00+05:00", ""] {
        let err = ship(&mut c, "T-1", bad).expect_err("bad clock must refuse");
        assert!(err.contains("now_utc"), "{err}");
        let err = advance_slice(&mut c, "T-1", bad).expect_err("bad clock must refuse");
        assert!(err.contains("now_utc"), "{err}");
    }
}

#[test]
fn set_status_refuses_empty_and_invalid() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    let before = c.clone();
    let err = set_status(&mut c, "T-1", "  ", CLOCK).expect_err("empty refuses");
    assert!(err.contains("non-empty"), "{err}");
    let err = set_status(&mut c, "T-1", "not-a-real-status", CLOCK).expect_err("enum gate");
    assert!(
        err.contains("invalid status `not-a-real-status`") && err.contains("cancelled"),
        "{err}"
    );
    assert_eq!(before, c, "refused ops must leave the corpus untouched");
}

/// T-916.1 acceptance 2 — →ready on a ticket without order (an idea) refuses up
/// front, naming the missing data, instead of the legacy mid-save wedge.
#[test]
fn set_status_ready_without_order_refuses() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Idea))]);
    let before = c.clone();
    let err = set_status(&mut c, "T-1", "ready", CLOCK).expect_err("must refuse");
    assert!(err.contains("order"), "must name the missing order: {err}");
    assert!(err.contains("wedges mid-save"), "{err}");
    assert_eq!(before, c);
}

#[test]
fn set_status_cancelled_stamps_completed_at() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    let out = set_status(&mut c, "T-1", "cancelled", CLOCK).expect("cancel");
    assert_eq!(out.changed, vec!["T-1".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(w.status, Status::Cancelled { order: Some(5) });
            assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
        }
        Ticket::Program(_) => panic!("T-1 must stay work"),
    }
}

/// Preserved asymmetry: `set-status shipped` neither stamps `completed_at` nor
/// clears `active` — `ship` owns both (cmds.rs T-913.1 comment).
#[test]
fn set_status_shipped_keeps_active_and_does_not_stamp() {
    let mut c = corpus(vec![
        program(
            "T-1",
            Status::Queued { order: 5 },
            &["T-1.1"],
            Some("T-1.1"),
        ),
        child_of("T-1", "T-1.1", Status::Idea),
    ]);
    set_status(&mut c, "T-1", "shipped", CLOCK).expect("shipped");
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => {
            assert_eq!(
                p.status,
                Status::Shipped {
                    shipped_at: None,
                    order: Some(5)
                }
            );
            assert_eq!(
                p.active.as_deref(),
                Some("T-1.1"),
                "set-status must not clear active"
            );
            assert_eq!(p.completed_at, None, "only ship/cancel stamp completed_at");
        }
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
}

#[test]
fn set_status_idea_with_order_refuses() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    let err = set_status(&mut c, "T-1", "idea", CLOCK).expect_err("must refuse");
    assert!(err.contains("idea must not carry order"), "{err}");
}

/// T-916.1 acceptance 4 — ship of a dotted child id succeeds at the op layer (the
/// legacy "Unknown ticket" hole), and the new invariant: a parent whose `active`
/// names the shipped child is cleared and counted as changed.
#[test]
fn ship_dotted_child_clears_matching_parent_active() {
    let mut c = corpus(vec![
        program(
            "T-1",
            Status::Queued { order: 5 },
            &["T-1.1", "T-1.2"],
            Some("T-1.1"),
        ),
        child_of("T-1", "T-1.1", Status::Queued { order: 7 }),
        child_of("T-1", "T-1.2", Status::Idea),
    ]);
    let out = ship(&mut c, "T-1.1", CLOCK).expect("ship child");
    assert_eq!(
        out.changed,
        vec!["T-1".to_string(), "T-1.1".to_string()],
        "parent counts as changed"
    );
    assert!(out.deleted.is_empty());
    match c.get("T-1.1").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(
                w.status,
                Status::Shipped {
                    shipped_at: None,
                    order: Some(7)
                },
                "order preserved; no SHA invented"
            );
            assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
        }
        Ticket::Program(_) => panic!("T-1.1 must stay work"),
    }
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => assert_eq!(p.active, None, "stale active cleared"),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
}

/// Ship preserves an existing `shipped_at` value and the order — it never invents
/// the SHA (that stays hand-edited; design §Explicit leftovers).
#[test]
fn ship_preserves_shipped_at_and_order() {
    let mut w = work("T-2", Status::Queued { order: 7 });
    w.shipped_at = Some("beefcafe".into());
    let mut c = corpus(vec![Ticket::Work(w)]);
    ship(&mut c, "T-2", CLOCK).expect("ship");
    match c.get("T-2").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(
                w.status,
                Status::Shipped {
                    shipped_at: Some("beefcafe".into()),
                    order: Some(7)
                }
            );
            assert_eq!(w.shipped_at.as_deref(), Some("beefcafe"));
            assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
        }
        Ticket::Program(_) => panic!("T-2 must stay work"),
    }
}

/// T-917.6 — ship REFUSES a created_at-less ticket pre-write (the birth stamp can
/// never arrive later honestly), naming the field and the fix; the corpus is
/// byte-untouched. Both kinds refuse.
#[test]
fn ship_refuses_created_at_less_pre_write() {
    let mut unstamped = work("T-3", Status::Queued { order: 4 });
    unstamped.created_at = None;
    let mut unstamped_prog = match program("T-4", Status::Queued { order: 5 }, &["T-4.1"], None) {
        Ticket::Program(p) => p,
        Ticket::Work(_) => unreachable!(),
    };
    unstamped_prog.created_at = None;
    let mut c = corpus(vec![
        Ticket::Work(unstamped),
        Ticket::Program(unstamped_prog),
        child_of("T-4", "T-4.1", Status::Idea),
    ]);
    let before = c.clone();
    for id in ["T-3", "T-4"] {
        let err = ship(&mut c, id, CLOCK).expect_err("created_at-less must refuse");
        assert!(
            err.contains(id) && err.contains("created_at") && err.contains("hand-stamp"),
            "must name ticket, field and fix: {err}"
        );
    }
    assert_eq!(before, c, "refused ship must leave the corpus untouched");
}

/// T-917.6 — stamp_sha writes the landing SHA through both arms, is a no-op on
/// the same sha, refuses a different sha / a non-shipped ticket / a garbage sha,
/// and drops a stale "shipped_at" estimated[] marker when it closes the field.
#[test]
fn stamp_sha_writes_noops_and_refuses() {
    let mut absent_marked = work(
        "T-1",
        Status::Shipped {
            shipped_at: None,
            order: Some(7),
        },
    );
    absent_marked.estimated = vec!["shipped_at".into()];
    absent_marked.estimate_note = Some("no subject commits; no SHA mined".into());
    let mut c = corpus(vec![
        Ticket::Work(absent_marked),
        Ticket::Work(work("T-2", Status::Queued { order: 9 })),
        program(
            "T-5",
            Status::Shipped {
                shipped_at: None,
                order: Some(11),
            },
            &["T-5.1"],
            None,
        ),
        child_of("T-5", "T-5.1", Status::Idea),
    ]);
    let before = c.clone();
    // Garbage shapes refuse (empty, too short, uppercase, branch-shaped).
    for bad in ["", "abc123", "ABCDEF12", "slice/T-197", "2026-07-26"] {
        let err = stamp_sha(&mut c, "T-1", bad, CLOCK).expect_err("garbage sha");
        assert!(
            err.contains("refusing stamp-sha T-1") && err.contains("lowercase hex"),
            "{err}"
        );
    }
    // Non-shipped refuses naming the status.
    let err = stamp_sha(&mut c, "T-2", "abcdef12", CLOCK).expect_err("not shipped");
    assert!(
        err.contains("refusing stamp-sha T-2") && err.contains("queued"),
        "{err}"
    );
    assert_eq!(before, c, "refusals must not mutate");
    // Work write: both arms + marker dropped (measured provenance now).
    let out = stamp_sha(&mut c, "T-1", "abcdef12", CLOCK).expect("stamp work");
    assert_eq!(out.changed, vec!["T-1".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(w.shipped_at.as_deref(), Some("abcdef12"));
            assert_eq!(
                w.status,
                Status::Shipped {
                    shipped_at: Some("abcdef12".into()),
                    order: Some(7)
                }
            );
            assert!(
                !w.estimated.iter().any(|e| e == "shipped_at"),
                "stale absent-marker must drop: {:?}",
                w.estimated
            );
        }
        Ticket::Program(_) => panic!("work"),
    }
    // Program write: the status arm (programs carry no standalone field).
    stamp_sha(&mut c, "T-5", "beadfeed", CLOCK).expect("stamp program");
    match c.get("T-5").unwrap() {
        Ticket::Program(p) => assert_eq!(
            p.status,
            Status::Shipped {
                shipped_at: Some("beadfeed".into()),
                order: Some(11)
            }
        ),
        Ticket::Work(_) => panic!("program"),
    }
    // Same sha again: no-op with an empty changed set.
    let out = stamp_sha(&mut c, "T-1", "abcdef12", CLOCK).expect("re-stamp same sha");
    assert!(out.changed.is_empty(), "no-op must report nothing changed");
    // Different sha: refuse — shipped_at is never overwritten.
    let err = stamp_sha(&mut c, "T-1", "00000001", CLOCK).expect_err("different sha");
    assert!(
        err.contains("never overwritten") && err.contains("abcdef12"),
        "{err}"
    );
}

/// A parent whose `active` names a DIFFERENT child is untouched by a child ship.
#[test]
fn ship_leaves_unrelated_parent_active() {
    let mut c = corpus(vec![
        program(
            "T-1",
            Status::Queued { order: 5 },
            &["T-1.1", "T-1.2"],
            Some("T-1.2"),
        ),
        child_of("T-1", "T-1.1", Status::Idea),
        child_of("T-1", "T-1.2", Status::Idea),
    ]);
    let out = ship(&mut c, "T-1.1", CLOCK).expect("ship");
    assert_eq!(out.changed, vec!["T-1.1".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.2")),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
}

/// `mark_ready`: spec argument lands, deps gate fires exactly like cmd_mark_ready
/// ("Blocked by …"), story backfills summary→title→id, acceptance backfills
/// `["See spec."]`, and (T-917.6) the `plan` field lands on the default path. A
/// queued ticket with empty owns stays legal — queued was already live, so this
/// op did not MAKE it live (no retro-policing).
///
/// T-920.1: the story/acceptance backfills only fire on QUARANTINED tickets now —
/// a non-quarantined ticket with empty body fields refuses at the ready-tier gate
/// (`mark_ready_refuses_empty_ready_tier_fields`) — so T-1 here carries the
/// nonempty `migration_legacy` that exempts it.
#[test]
fn mark_ready_backfills_and_gates() {
    let root = scratch_root("mark-ready");
    fs::create_dir_all(root.join(crate::repository::documentation::PLANS_DIR)).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(root.join("docs/plans/t-1_plan.md"), "# plan\n").unwrap();
    fs::write(root.join("docs/plans/t-2_plan.md"), "# plan\n").unwrap();
    let mut c = Corpus::new(&root);
    let mut t1 = work("T-1", Status::Queued { order: 10 });
    t1.owns = vec![];
    t1.main_goal = None;
    t1.acceptance = vec![];
    t1.context = vec![];
    t1.requirement = vec![];
    t1.current_state = vec![];
    t1.approach = vec![];
    t1.verify = vec![];
    t1.migration_legacy = vec!["parked wall".into()];
    c.tickets.insert("T-1".into(), Ticket::Work(t1));
    let mut t2 = work("T-2", Status::Queued { order: 11 });
    t2.depends_on = vec!["T-1".into(), "T-404".into()];
    t2.spec = Some("docs/spec.md".into());
    c.tickets.insert("T-2".into(), Ticket::Work(t2));

    let err = mark_ready(&mut c, "T-2", None, None, CLOCK).expect_err("dep gate");
    assert_eq!(err, "Blocked by T-1 (status=queued)");

    let out = mark_ready(&mut c, "T-1", Some("docs/spec.md"), None, CLOCK).expect("ready");
    assert_eq!(out.changed, vec!["T-1".to_string()]);
    match c.get("T-1").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(
                w.status,
                Status::Ready {
                    order: 10,
                    spec: "docs/spec.md".into(),
                    main_goal: "T-1 summary".into(),
                    acceptance: vec!["See spec.".into()],
                }
            );
            assert_eq!(w.spec.as_deref(), Some("docs/spec.md"));
            assert_eq!(
                w.plan.as_deref(),
                Some("docs/plans/t-1_plan.md"),
                "unset plan defaults to the id-derived path and is WRITTEN"
            );
            assert_eq!(w.main_goal.as_deref(), Some("T-1 summary"));
            assert_eq!(w.acceptance, vec!["See spec.".to_string()]);
        }
        Ticket::Program(_) => panic!("T-1 must stay work"),
    }
    // T-1 ready (not shipped/cancelled) still blocks T-2; a cancel unblocks.
    let err = mark_ready(&mut c, "T-2", None, None, CLOCK).expect_err("still blocked");
    assert_eq!(err, "Blocked by T-1 (status=ready)");
    set_status(&mut c, "T-1", "cancelled", CLOCK).expect("cancel");
    mark_ready(&mut c, "T-2", None, None, CLOCK).expect("deps satisfied; T-404 absent is skipped");
}

/// T-920.1 acceptance — the mark-ready ready-tier refusal: promotion of a
/// non-quarantined work ticket refuses pre-write NAMING EACH empty ready-tier
/// field; the corpus is untouched; filling the fields (or quarantining) restores
/// the promotion.
#[test]
fn mark_ready_refuses_empty_ready_tier_fields() {
    let root = scratch_root("mark-ready-tier");
    fs::create_dir_all(root.join(crate::repository::documentation::PLANS_DIR)).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(root.join("docs/plans/t-3_plan.md"), "# plan\n").unwrap();
    let mut c = Corpus::new(&root);
    let mut t3 = work("T-3", Status::Queued { order: 10 });
    t3.context = vec![];
    t3.requirement = vec![];
    t3.current_state = vec![];
    t3.approach = vec![];
    t3.verify = vec!["   ".into()]; // whitespace-only counts as empty
    t3.acceptance = vec![];
    c.tickets.insert("T-3".into(), Ticket::Work(t3));
    let before = c.clone();
    let err = mark_ready(&mut c, "T-3", Some("docs/spec.md"), None, CLOCK)
        .expect_err("empty tier fields must refuse");
    assert!(
        err.contains("empty: context, requirement, current_state, approach, verify, acceptance"),
        "must name each empty field in tier order: {err}"
    );
    assert!(err.contains("refusing mark-ready T-3"), "{err}");
    assert_eq!(before, c, "refusal must not mutate");
    // Partial fill: the refusal names exactly the still-empty fields.
    if let Some(Ticket::Work(w)) = c.tickets.get_mut("T-3") {
        w.context = vec!["why".into()];
        w.requirement = vec!["ask".into()];
        w.acceptance = vec!["outcome".into()];
    }
    let err = mark_ready(&mut c, "T-3", Some("docs/spec.md"), None, CLOCK)
        .expect_err("still three empty");
    assert!(
        err.contains("empty: current_state, approach, verify —"),
        "must name exactly the empty ones: {err}"
    );
    // Full fill promotes.
    if let Some(Ticket::Work(w)) = c.tickets.get_mut("T-3") {
        w.current_state = vec!["today".into()];
        w.approach = vec!["steps".into()];
        w.verify = vec!["cargo test".into()];
    }
    mark_ready(&mut c, "T-3", Some("docs/spec.md"), None, CLOCK)
        .expect("filled tier fields promote");
}

/// T-920.1 acceptance — the ship ready-tier refusal (future ships): a ship from
/// queued with empty body fields refuses pre-write naming each — main_goal
/// included (the queued→shipped jump never passes the ready-class parse) — and
/// the quarantine exemption lets a wall-carrying ticket ship (T-919 fills its
/// fields when the drain reaches it).
#[test]
fn ship_refuses_empty_ready_tier_fields() {
    let mut bare = work("T-1", Status::Queued { order: 5 });
    bare.main_goal = None;
    bare.context = vec![];
    bare.requirement = vec![];
    bare.current_state = vec![];
    bare.approach = vec![];
    bare.verify = vec![];
    bare.acceptance = vec![];
    let mut quarantined = work("T-2", Status::Queued { order: 6 });
    quarantined.main_goal = None;
    quarantined.context = vec![];
    quarantined.requirement = vec![];
    quarantined.current_state = vec![];
    quarantined.approach = vec![];
    quarantined.verify = vec![];
    quarantined.acceptance = vec![];
    quarantined.migration_legacy = vec!["the original wall".into()];
    let mut c = corpus(vec![Ticket::Work(bare), Ticket::Work(quarantined)]);
    let before = c.clone();
    let err = ship(&mut c, "T-1", CLOCK).expect_err("empty body must refuse the ship");
    assert!(
        err.contains(
            "empty: main_goal, context, requirement, current_state, approach, verify, acceptance"
        ),
        "must name each empty field, main_goal first: {err}"
    );
    assert!(err.contains("refusing ship T-1"), "{err}");
    assert_eq!(before, c, "refusal must not mutate");
    // Quarantined ships (exempt) …
    ship(&mut c, "T-2", CLOCK).expect("quarantined ticket ships");
    // … and a filled ticket ships.
    let filled = work("T-3", Status::Queued { order: 7 });
    let mut c = corpus(vec![Ticket::Work(filled)]);
    ship(&mut c, "T-3", CLOCK).expect("filled body ships");
}

/// T-920.1 acceptance — the post-image title gate: an op that would write a
/// changed ticket whose title is empty, equals its id, or exceeds 10 words
/// refuses pre-write (corpus untouched); both kinds. An 11-word title names the
/// count; exactly 10 words passes; UNCHANGED debt-titled tickets never
/// retro-police an op that does not touch them.
#[test]
fn post_image_title_gate_refuses_changed_debt_titles() {
    // id-as-title on a changed work ticket.
    let mut id_titled = work("T-1", Status::Queued { order: 5 });
    id_titled.title = "T-1".into();
    let mut c = corpus(vec![Ticket::Work(id_titled)]);
    let before = c.clone();
    let err = set_status(&mut c, "T-1", "deferred", CLOCK).expect_err("id-title refuses");
    assert!(
        err.contains("post-image T-1") && err.contains("equals the ticket id"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");

    // 11-word title names the count; 10 words passes.
    let mut wordy = work("T-2", Status::Queued { order: 6 });
    wordy.title = "one two three four five six seven eight nine ten eleven".into();
    let mut c = corpus(vec![Ticket::Work(wordy)]);
    let err = set_status(&mut c, "T-2", "deferred", CLOCK).expect_err("11 words refuses");
    assert!(
        err.contains("title is 11 words (cap 10)"),
        "must name count and cap: {err}"
    );
    if let Some(Ticket::Work(w)) = c.tickets.get_mut("T-2") {
        w.title = "one two three four five six seven eight nine ten".into();
    }
    set_status(&mut c, "T-2", "deferred", CLOCK).expect("10 words is legal");

    // Empty title refuses; a program is gated too.
    let mut empty_prog = match program("T-4", Status::Idea, &["T-4.1"], None) {
        Ticket::Program(p) => p,
        Ticket::Work(_) => unreachable!(),
    };
    empty_prog.title = "  ".into();
    let mut c = corpus(vec![
        Ticket::Program(empty_prog),
        child_of("T-4", "T-4.1", Status::Idea),
    ]);
    let err = advance_slice(&mut c, "T-4", CLOCK).expect_err("empty program title");
    assert!(
        err.contains("post-image T-4") && err.contains("title is empty"),
        "{err}"
    );

    // Don't retro-police: an op away from a debt-titled ticket still commits.
    let mut debt = work("T-5", Status::Queued { order: 8 });
    debt.title = "T-5".into();
    let mut c = corpus(vec![
        Ticket::Work(debt),
        Ticket::Work(work("T-6", Status::Queued { order: 9 })),
    ]);
    set_status(&mut c, "T-6", "deferred", CLOCK)
        .expect("unchanged debt title must not block other ops");
}

/// T-920.1 acceptance — the post-image queued-tier main_goal gate: an op leaving
/// a changed non-quarantined work ticket live without main_goal refuses; the
/// quarantine exemption passes; leaving the live set with main_goal empty is
/// legal (the rule binds on the POST status).
#[test]
fn post_image_main_goal_gate_on_changed_live_work() {
    let mut bare = work("T-2", Status::Idea);
    bare.main_goal = None;
    let mut quarantined = work("T-3", Status::Idea);
    quarantined.main_goal = None;
    quarantined.migration_legacy = vec!["wall".into()];
    let mut leaving = work("T-4", Status::Queued { order: 20 });
    leaving.main_goal = None;
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(bare),
        Ticket::Work(quarantined),
        Ticket::Work(leaving),
    ]);
    let before = c.clone();
    let err = reorder(&mut c, "T-2", "T-1", CLOCK).expect_err("made live without main_goal");
    assert!(
        err.contains("post-image T-2") && err.contains("without main_goal"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Quarantined: exempt (content exists, unprocessed).
    reorder(&mut c, "T-3", "T-1", CLOCK).expect("quarantined exemption");
    // Leaving the live set: the changed ticket is deferred in the post image.
    set_status(&mut c, "T-4", "deferred", CLOCK).expect("leaving live needs no main_goal");
}

/// T-917.6 plan ready-gate: mark-ready without the plan file refuses naming the
/// path (corpus untouched); an explicit PLAN argument overrides the default and
/// must exist too; an existing `plan` field is honored over the default.
#[test]
fn mark_ready_plan_gate_refuses_and_resolves() {
    let root = scratch_root("mark-ready-plan");
    fs::create_dir_all(root.join(crate::repository::documentation::PLANS_DIR)).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-9.1".into(),
        Ticket::Work(work("T-9.1", Status::Queued { order: 10 })),
    );
    let before = c.clone();
    // No plan file anywhere → refuse naming the DEFAULT path (dots → underscores).
    let err = mark_ready(&mut c, "T-9.1", Some("docs/spec.md"), None, CLOCK).expect_err("no plan");
    assert!(
        err.starts_with("Plan file not found: ") && err.contains("docs/plans/t-9_1_plan.md"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Explicit PLAN argument that is missing → refuse naming THAT path.
    let err = mark_ready(
        &mut c,
        "T-9.1",
        Some("docs/spec.md"),
        Some("docs/plans/custom.md"),
        CLOCK,
    )
    .expect_err("explicit plan missing");
    assert!(err.contains("docs/plans/custom.md"), "{err}");
    // Present explicit plan lands and is written to the field.
    fs::write(root.join("docs/plans/custom.md"), "# plan\n").unwrap();
    mark_ready(
        &mut c,
        "T-9.1",
        Some("docs/spec.md"),
        Some("docs/plans/custom.md"),
        CLOCK,
    )
    .expect("explicit plan present");
    match c.get("T-9.1").unwrap() {
        Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some("docs/plans/custom.md")),
        Ticket::Program(_) => panic!("work"),
    }
    // An already-set plan field is honored when no argument is passed.
    set_status(&mut c, "T-9.1", "queued", CLOCK).expect("back to queued");
    mark_ready(&mut c, "T-9.1", None, None, CLOCK).expect("field plan honored");
    match c.get("T-9.1").unwrap() {
        Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some("docs/plans/custom.md")),
        Ticket::Program(_) => panic!("work"),
    }
    assert_eq!(default_plan_path("T-917.6"), "docs/plans/t-917_6_plan.md");
    assert_eq!(default_plan_path("T-090.4"), "docs/plans/t-090_4_plan.md");
}

#[test]
fn mark_ready_refuses_missing_spec_and_missing_file() {
    let root = scratch_root("mark-ready-missing");
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-1".into(),
        Ticket::Work(work("T-1", Status::Queued { order: 1 })),
    );
    let err = mark_ready(&mut c, "T-1", None, None, CLOCK).expect_err("no spec");
    assert_eq!(err, "Ticket T-1 needs a spec path");
    let err =
        mark_ready(&mut c, "T-1", Some("docs/nope.md"), None, CLOCK).expect_err("file missing");
    assert!(err.starts_with("Spec file not found: "), "{err}");
    assert!(err.contains("docs/nope.md"), "{err}");
}

#[test]
fn mark_ready_without_order_refuses() {
    let root = scratch_root("mark-ready-order");
    fs::create_dir_all(root.join(crate::repository::documentation::PLANS_DIR)).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(root.join("docs/plans/t-1_plan.md"), "# plan\n").unwrap();
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-1".into(), Ticket::Work(work("T-1", Status::Idea)));
    let err = mark_ready(&mut c, "T-1", Some("docs/spec.md"), None, CLOCK).expect_err("no order");
    assert!(err.contains("order"), "{err}");
}

/// `add` mints max-parent+1 (children never affect it), stamps `created_at` from
/// the injected clock, defaults scope to repo/docs, and falls back summary→title.
#[test]
fn add_mints_next_parent_id_and_stamps() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-001", Status::Idea)),
        Ticket::Work(work("T-910", Status::Idea)),
        child_of("T-910", "T-910.7", Status::Idea),
    ]);
    let (tid, out) = add(&mut c, "New thing", "", CLOCK).expect("add");
    assert_eq!(tid, "T-911");
    assert_eq!(out.changed, vec!["T-911".to_string()]);
    match c.get("T-911").unwrap() {
        Ticket::Work(w) => {
            assert_eq!(w.status, Status::Idea);
            assert_eq!(w.summary, "New thing", "summary falls back to title");
            assert_eq!(w.created_at.as_deref(), Some(CLOCK));
            assert_eq!(
                w.scope,
                ScopeV2 {
                    domain: Domain::Repo,
                    layer: "docs".into(),
                    component: None,
                    surface: vec![],
                },
                "T-917.2 mint default: flat repo/docs, component-free"
            );
            assert_eq!(
                w.class.as_deref(),
                Some("feature"),
                "minted class comes from the classify_work triage"
            );
        }
        Ticket::Program(_) => panic!("minted ticket must be work"),
    }
}

/// T-917.2 — the surface rule mirrors the owns rule: an op that makes a
/// component-bearing, surface-less work ticket live refuses naming the fix;
/// a surface or the migrator's `"scope"` estimated-marker passes; component-free
/// scope is exempt (no vocabulary surfaces exist to require).
#[test]
fn made_live_component_without_surface_refuses() {
    let mut bare = work("T-2", Status::Idea);
    bare.scope = ScopeV2 {
        domain: Domain::Website,
        layer: "frontend".into(),
        component: Some("mission_creator".into()),
        surface: vec![],
    };
    let mut marked = work("T-4", Status::Idea);
    marked.scope = bare.scope.clone();
    marked.estimated = vec!["scope".into()];
    let mut surfaced = work("T-5", Status::Idea);
    surfaced.scope = ScopeV2 {
        surface: vec!["attr_panel".into()],
        ..bare.scope.clone()
    };
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(bare),
        Ticket::Work(work("T-3", Status::Idea)),
        Ticket::Work(marked),
        Ticket::Work(surfaced),
    ]);
    let before = c.clone();
    let err = reorder(&mut c, "T-2", "T-1", CLOCK).expect_err("surface-less made live");
    assert!(
        err.contains("surface required") && err.contains("mission_creator"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Component-free scope (the mint default) stays mintable → live.
    reorder(&mut c, "T-3", "T-1", CLOCK).expect("component-free exempt");
    // The migrator's honest escape passes…
    reorder(&mut c, "T-4", "T-3", CLOCK).expect("scope ∈ estimated passes");
    // …and so does a real surface.
    reorder(&mut c, "T-5", "T-4", CLOCK).expect("surfaced ticket passes");
}
