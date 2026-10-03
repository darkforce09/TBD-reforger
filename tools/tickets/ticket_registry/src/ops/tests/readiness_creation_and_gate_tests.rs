use super::*;

/// `mark_ready`: spec argument lands, deps gate fires exactly like cmd_mark_ready
/// ("Blocked by …"), story backfills summary→title→id, acceptance backfills
/// `["See spec."]`, and the `plan` field lands on the default path. A
/// queued ticket with empty owns stays legal — queued was already live, so this
/// op did not MAKE it live (no retro-policing).
///
/// The story/acceptance backfills only fire on QUARANTINED tickets now —
/// a non-quarantined ticket with empty body fields refuses at the ready-tier gate
/// (`mark_ready_refuses_empty_ready_tier_fields`) — so T-1 here carries the
/// nonempty `migration_legacy` that exempts it.
#[test]
fn mark_ready_backfills_and_gates() {
    let root = scratch_root("mark-ready");
    let plans = root.join(ticket_model::repository::documentation::PLANS_DIR);
    fs::create_dir_all(&plans).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(plans.join("t-1_plan.md"), "# plan\n").unwrap();
    fs::write(plans.join("t-2_plan.md"), "# plan\n").unwrap();
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

    let err = mark_ready(&mut c, &TicketId::from("T-2"), None, None, CLOCK).expect_err("dep gate");
    assert_eq!(err, "Blocked by T-1 (status=queued)");

    let out = mark_ready(
        &mut c,
        &TicketId::from("T-1"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect("ready");
    assert_eq!(out.changed, vec![TicketId::from("T-1")]);
    match c.get(&TicketId::from("T-1")).unwrap() {
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
                Some("documentation/tickets/plans/t-1_plan.md"),
                "unset plan defaults to the id-derived path and is WRITTEN"
            );
            assert_eq!(w.main_goal.as_deref(), Some("T-1 summary"));
            assert_eq!(w.acceptance, vec!["See spec.".to_string()]);
        }
        Ticket::Program(_) => panic!("T-1 must stay work"),
    }
    // T-1 ready (not shipped/cancelled) still blocks T-2; a cancel unblocks.
    let err =
        mark_ready(&mut c, &TicketId::from("T-2"), None, None, CLOCK).expect_err("still blocked");
    assert_eq!(err, "Blocked by T-1 (status=ready)");
    set_status(&mut c, &TicketId::from("T-1"), "cancelled", CLOCK).expect("cancel");
    mark_ready(&mut c, &TicketId::from("T-2"), None, None, CLOCK)
        .expect("deps satisfied; T-404 absent is skipped");
}

/// Acceptance — the mark-ready ready-tier refusal: promotion of a
/// non-quarantined work ticket refuses pre-write NAMING EACH empty ready-tier
/// field; the corpus is untouched; filling the fields (or quarantining) restores
/// the promotion.
#[test]
fn mark_ready_refuses_empty_ready_tier_fields() {
    let root = scratch_root("mark-ready-tier");
    let plans = root.join(ticket_model::repository::documentation::PLANS_DIR);
    fs::create_dir_all(&plans).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(plans.join("t-3_plan.md"), "# plan\n").unwrap();
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
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-3"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect_err("empty tier fields must refuse");
    assert!(
        err.contains("empty: context, requirement, current_state, approach, verify, acceptance"),
        "must name each empty field in tier order: {err}"
    );
    assert!(err.contains("refusing mark-ready T-3"), "{err}");
    assert_eq!(before, c, "refusal must not mutate");
    // Partial fill: the refusal names exactly the still-empty fields.
    if let Some(Ticket::Work(w)) = c.tickets.get_mut(&TicketId::from("T-3")) {
        w.context = vec!["why".into()];
        w.requirement = vec!["ask".into()];
        w.acceptance = vec!["outcome".into()];
    }
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-3"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect_err("still three empty");
    assert!(
        err.contains("empty: current_state, approach, verify —"),
        "must name exactly the empty ones: {err}"
    );
    // Full fill promotes.
    if let Some(Ticket::Work(w)) = c.tickets.get_mut(&TicketId::from("T-3")) {
        w.current_state = vec!["today".into()];
        w.approach = vec!["steps".into()];
        w.verify = vec!["cargo test".into()];
    }
    mark_ready(
        &mut c,
        &TicketId::from("T-3"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect("filled tier fields promote");
}

/// Acceptance — the post-image title gate: an op that would write a
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
    let err = set_status(&mut c, &TicketId::from("T-1"), "deferred", CLOCK)
        .expect_err("id-title refuses");
    assert!(
        err.contains("post-image T-1") && err.contains("equals the ticket id"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");

    // 11-word title names the count; 10 words passes.
    let mut wordy = work("T-2", Status::Queued { order: 6 });
    wordy.title = "one two three four five six seven eight nine ten eleven".into();
    let mut c = corpus(vec![Ticket::Work(wordy)]);
    let err = set_status(&mut c, &TicketId::from("T-2"), "deferred", CLOCK)
        .expect_err("11 words refuses");
    assert!(
        err.contains("title is 11 words (cap 10)"),
        "must name count and cap: {err}"
    );
    if let Some(Ticket::Work(w)) = c.tickets.get_mut(&TicketId::from("T-2")) {
        w.title = "one two three four five six seven eight nine ten".into();
    }
    set_status(&mut c, &TicketId::from("T-2"), "deferred", CLOCK).expect("10 words is legal");

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
    let err =
        advance_slice(&mut c, &TicketId::from("T-4"), CLOCK).expect_err("empty program title");
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
    set_status(&mut c, &TicketId::from("T-6"), "deferred", CLOCK)
        .expect("unchanged debt title must not block other ops");
}

/// Acceptance — the post-image queued-tier main_goal gate: an op leaving
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
    let err = reorder(
        &mut c,
        &TicketId::from("T-2"),
        &TicketId::from("T-1"),
        CLOCK,
    )
    .expect_err("made live without main_goal");
    assert!(
        err.contains("post-image T-2") && err.contains("without main_goal"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Quarantined: exempt (content exists, unprocessed).
    reorder(
        &mut c,
        &TicketId::from("T-3"),
        &TicketId::from("T-1"),
        CLOCK,
    )
    .expect("quarantined exemption");
    // Leaving the live set: the changed ticket is deferred in the post image.
    set_status(&mut c, &TicketId::from("T-4"), "deferred", CLOCK)
        .expect("leaving live needs no main_goal");
}

/// Plan ready-gate: mark-ready without the plan file refuses naming the
/// path (corpus untouched); an explicit PLAN argument overrides the default and
/// must exist too; an existing `plan` field is honored over the default.
#[test]
fn mark_ready_plan_gate_refuses_and_resolves() {
    let root = scratch_root("mark-ready-plan");
    let plans = root.join(ticket_model::repository::documentation::PLANS_DIR);
    fs::create_dir_all(&plans).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-9.1".into(),
        Ticket::Work(work("T-9.1", Status::Queued { order: 10 })),
    );
    let before = c.clone();
    // No plan file anywhere → refuse naming the DEFAULT path (dots → underscores).
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-9.1"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect_err("no plan");
    assert!(
        err.starts_with("Plan file not found: ")
            && err.contains("documentation/tickets/plans/t-9_1_plan.md"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Explicit PLAN argument that is missing → refuse naming THAT path.
    let custom = "documentation/tickets/plans/custom.md";
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-9.1"),
        Some("docs/spec.md"),
        Some(custom),
        CLOCK,
    )
    .expect_err("explicit plan missing");
    assert!(err.contains(custom), "{err}");
    // Present explicit plan lands and is written to the field.
    fs::write(plans.join("custom.md"), "# plan\n").unwrap();
    mark_ready(
        &mut c,
        &TicketId::from("T-9.1"),
        Some("docs/spec.md"),
        Some(custom),
        CLOCK,
    )
    .expect("explicit plan present");
    match c.get(&TicketId::from("T-9.1")).unwrap() {
        Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some(custom)),
        Ticket::Program(_) => panic!("work"),
    }
    // An already-set plan field is honored when no argument is passed.
    set_status(&mut c, &TicketId::from("T-9.1"), "queued", CLOCK).expect("back to queued");
    mark_ready(&mut c, &TicketId::from("T-9.1"), None, None, CLOCK).expect("field plan honored");
    match c.get(&TicketId::from("T-9.1")).unwrap() {
        Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some(custom)),
        Ticket::Program(_) => panic!("work"),
    }
    assert_eq!(
        default_plan_path(&TicketId::from("T-917.6")),
        "documentation/tickets/plans/t-917_6_plan.md"
    );
    assert_eq!(
        default_plan_path(&TicketId::from("T-090.4")),
        "documentation/tickets/plans/t-090_4_plan.md"
    );
}

#[test]
fn mark_ready_refuses_missing_spec_and_missing_file() {
    let root = scratch_root("mark-ready-missing");
    let mut c = Corpus::new(&root);
    c.tickets.insert(
        "T-1".into(),
        Ticket::Work(work("T-1", Status::Queued { order: 1 })),
    );
    let err = mark_ready(&mut c, &TicketId::from("T-1"), None, None, CLOCK).expect_err("no spec");
    assert_eq!(err, "Ticket T-1 needs a spec path");
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-1"),
        Some("docs/nope.md"),
        None,
        CLOCK,
    )
    .expect_err("file missing");
    assert!(err.starts_with("Spec file not found: "), "{err}");
    assert!(err.contains("docs/nope.md"), "{err}");
}

#[test]
fn mark_ready_without_order_refuses() {
    let root = scratch_root("mark-ready-order");
    let plans = root.join(ticket_model::repository::documentation::PLANS_DIR);
    fs::create_dir_all(&plans).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(plans.join("t-1_plan.md"), "# plan\n").unwrap();
    let mut c = Corpus::new(&root);
    c.tickets
        .insert("T-1".into(), Ticket::Work(work("T-1", Status::Idea)));
    let err = mark_ready(
        &mut c,
        &TicketId::from("T-1"),
        Some("docs/spec.md"),
        None,
        CLOCK,
    )
    .expect_err("no order");
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
    assert_eq!(out.changed, vec![TicketId::from("T-911")]);
    match c.get(&TicketId::from("T-911")).unwrap() {
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

/// The surface rule mirrors the owns rule: an op that makes a
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
    let err = reorder(
        &mut c,
        &TicketId::from("T-2"),
        &TicketId::from("T-1"),
        CLOCK,
    )
    .expect_err("surface-less made live");
    assert!(
        err.contains("surface required") && err.contains("mission_creator"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
    // Component-free scope (the mint default) stays mintable → live.
    reorder(
        &mut c,
        &TicketId::from("T-3"),
        &TicketId::from("T-1"),
        CLOCK,
    )
    .expect("component-free exempt");
    // The migrator's honest escape passes…
    reorder(
        &mut c,
        &TicketId::from("T-4"),
        &TicketId::from("T-3"),
        CLOCK,
    )
    .expect("scope ∈ estimated passes");
    // …and so does a real surface.
    reorder(
        &mut c,
        &TicketId::from("T-5"),
        &TicketId::from("T-4"),
        CLOCK,
    )
    .expect("surfaced ticket passes");
}
