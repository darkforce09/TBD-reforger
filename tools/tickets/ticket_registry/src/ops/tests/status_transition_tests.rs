use super::*;

/// A malformed injected clock refuses on entry — including in ops that never stamp
/// (uniform contract: caller wiring bugs surface on the first op).
#[test]
fn ops_refuse_malformed_clock() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    for bad in ["2026-08-14 12:00", "2026-08-14T12:00:00+05:00", ""] {
        let err = ship(&mut c, &TicketId::from("T-1"), bad).expect_err("bad clock must refuse");
        assert!(err.contains("now_utc"), "{err}");
        let err =
            advance_slice(&mut c, &TicketId::from("T-1"), bad).expect_err("bad clock must refuse");
        assert!(err.contains("now_utc"), "{err}");
    }
}

#[test]
fn set_status_refuses_empty_and_invalid() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    let before = c.clone();
    let err = set_status(&mut c, &TicketId::from("T-1"), "  ", CLOCK).expect_err("empty refuses");
    assert!(err.contains("non-empty"), "{err}");
    let err = set_status(&mut c, &TicketId::from("T-1"), "not-a-real-status", CLOCK)
        .expect_err("enum gate");
    assert!(
        err.contains("invalid status `not-a-real-status`") && err.contains("cancelled"),
        "{err}"
    );
    assert_eq!(before, c, "refused ops must leave the corpus untouched");
}

/// Acceptance 2 — →ready on a ticket without order (an idea) refuses up
/// front, naming the missing data, instead of wedging mid-save.
#[test]
fn set_status_ready_without_order_refuses() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Idea))]);
    let before = c.clone();
    let err = set_status(&mut c, &TicketId::from("T-1"), "ready", CLOCK).expect_err("must refuse");
    assert!(err.contains("order"), "must name the missing order: {err}");
    assert!(err.contains("mid-save wedge is the alternative"), "{err}");
    assert_eq!(before, c);
}

#[test]
fn set_status_cancelled_stamps_completed_at() {
    let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
    let out = set_status(&mut c, &TicketId::from("T-1"), "cancelled", CLOCK).expect("cancel");
    assert_eq!(out.changed, vec![TicketId::from("T-1")]);
    match c.get(&TicketId::from("T-1")).unwrap() {
        Ticket::Work(w) => {
            assert_eq!(w.status, Status::Cancelled { order: Some(5) });
            assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
        }
        Ticket::Program(_) => panic!("T-1 must stay work"),
    }
}

/// Preserved asymmetry: `set-status shipped` neither stamps `completed_at` nor
/// clears `active` — `ship` owns both.
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
    set_status(&mut c, &TicketId::from("T-1"), "shipped", CLOCK).expect("shipped");
    match c.get(&TicketId::from("T-1")).unwrap() {
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
    let err = set_status(&mut c, &TicketId::from("T-1"), "idea", CLOCK).expect_err("must refuse");
    assert!(err.contains("idea must not carry order"), "{err}");
}

/// An order-less idea parent moved to `deferred`, `cancelled` or `shipped` takes the order
/// `reorder` mints when anchored after the highest-ordered ticket in the corpus (a child here),
/// so `ticket check` stays green; `cancelled` still stamps `completed_at`.
#[test]
fn set_status_non_live_mints_the_append_order_on_an_order_less_parent() {
    for target in ["deferred", "cancelled", "shipped"] {
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            program("T-2", Status::Queued { order: 20 }, &["T-2.1"], None),
            child_of("T-2", "T-2.1", Status::Queued { order: 70 }),
            Ticket::Work(work("T-3", Status::Idea)),
        ]);
        let mut via_reorder = c.clone();
        reorder(
            &mut via_reorder,
            &TicketId::from("T-3"),
            &TicketId::from("T-2.1"),
            CLOCK,
        )
        .expect("reorder after the highest");
        let out = set_status(&mut c, &TicketId::from("T-3"), target, CLOCK)
            .expect("non-live target mints");
        assert_eq!(out.changed, vec![TicketId::from("T-3")], "{target}");
        let t = c.get(&TicketId::from("T-3")).unwrap();
        assert_eq!(t.status().name().as_str(), target);
        assert_eq!(
            t.status().order(),
            Some(71),
            "{target}: highest order 70, plus one"
        );
        assert_eq!(
            t.status().order(),
            via_reorder
                .get(&TicketId::from("T-3"))
                .unwrap()
                .status()
                .order(),
            "{target}: the minted order is the one reorder gives"
        );
        let Ticket::Work(w) = t else {
            panic!("T-3 must stay work")
        };
        let stamp = (target == "cancelled").then_some(CLOCK);
        assert_eq!(w.completed_at.as_deref(), stamp, "{target}");
    }
}

/// A ticket that already carries an order keeps it through every non-live target; the mint
/// never moves an existing order, even when a higher one exists.
#[test]
fn set_status_non_live_keeps_an_existing_order() {
    for target in ["deferred", "cancelled", "shipped"] {
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 5 })),
            Ticket::Work(work("T-2", Status::Deferred { order: Some(90) })),
        ]);
        set_status(&mut c, &TicketId::from("T-1"), target, CLOCK).expect("from queued");
        assert_eq!(
            c.get(&TicketId::from("T-1")).unwrap().status().order(),
            Some(5),
            "{target}"
        );
        set_status(&mut c, &TicketId::from("T-2"), target, CLOCK).expect("from deferred");
        assert_eq!(
            c.get(&TicketId::from("T-2")).unwrap().status().order(),
            Some(90),
            "{target}"
        );
    }
}

/// `queued` without an order still refuses and names `ticket reorder`: there the order is
/// dispatch priority, chosen deliberately and never minted.
#[test]
fn set_status_queued_without_order_refuses_with_the_reorder_hint() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(work("T-2", Status::Idea)),
    ]);
    let before = c.clone();
    let err = set_status(&mut c, &TicketId::from("T-2"), "queued", CLOCK)
        .expect_err("queued needs a chosen order");
    assert!(
        err.contains("queued requires order")
            && err.contains("`ticket reorder T-2 <anchor>` mints one"),
        "{err}"
    );
    assert_eq!(before, c, "refusal must not mutate");
}

/// An order-less child stays order-less through every non-live target, `ship` included: the
/// check's order rule walks parent tickets only.
#[test]
fn non_live_transitions_leave_an_order_less_child_order_less() {
    let family = || {
        corpus(vec![
            program("T-1", Status::Queued { order: 5 }, &["T-1.1"], None),
            child_of("T-1", "T-1.1", Status::Idea),
        ])
    };
    for target in ["deferred", "cancelled", "shipped"] {
        let mut c = family();
        set_status(&mut c, &TicketId::from("T-1.1"), target, CLOCK).expect("child non-live target");
        assert_eq!(
            c.get(&TicketId::from("T-1.1")).unwrap().status().order(),
            None,
            "{target}"
        );
    }
    let mut c = family();
    ship(&mut c, &TicketId::from("T-1.1"), CLOCK).expect("child ship");
    assert_eq!(
        c.get(&TicketId::from("T-1.1")).unwrap().status().order(),
        None
    );
}

/// `ship` of an order-less idea parent takes the same append order as `set-status shipped`.
#[test]
fn ship_mints_the_append_order_on_an_order_less_parent() {
    let mut c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: 10 })),
        Ticket::Work(work("T-2", Status::Idea)),
    ]);
    ship(&mut c, &TicketId::from("T-2"), CLOCK).expect("ship mints");
    assert_eq!(
        c.get(&TicketId::from("T-2")).unwrap().status(),
        &Status::Shipped {
            shipped_at: None,
            order: Some(11)
        }
    );
}

/// The append order floors its anchor at 0: a corpus without a positive order mints 1, never
/// the 0 that `ticket check` reads as absent.
#[test]
fn append_order_never_mints_zero() {
    assert_eq!(append_order(&BTreeMap::new()), 1);
    let c = corpus(vec![
        Ticket::Work(work("T-1", Status::Queued { order: -3 })),
        Ticket::Work(work("T-2", Status::Idea)),
    ]);
    assert_eq!(append_order(&c.tickets), 1);
}

/// Whether `ticket check`'s order rule reds `t`, evaluated the way the check does: on the
/// ticket's registry row (the typed projection), for parent rows only, where a status other
/// than idea needs a truthy order.
fn check_reds_for_lack_of_order(t: &Ticket) -> bool {
    let row = crate::registry::typed_projection::ticket_to_value(t);
    t.id().is_parent()
        && crate::registry::opt_str(&row, "status") != Some("idea")
        && !crate::registry::order_truthy(&row)
}

/// The invariant: no `set-status` call leaves `ticket check` red for lack of an order. Every
/// target from every start shape, parent and child, either refuses with the corpus untouched or
/// writes a post-image whose rows all pass the check's order rule.
#[test]
fn no_set_status_leaves_the_check_red_for_lack_of_an_order() {
    let starts = [
        ("T-2", Status::Idea),
        ("T-2", Status::Queued { order: 30 }),
        ("T-2", Status::Deferred { order: Some(40) }),
        ("T-2", Status::Cancelled { order: Some(50) }),
        (
            "T-2",
            Status::Shipped {
                shipped_at: None,
                order: Some(60),
            },
        ),
        ("T-1.1", Status::Idea),
        ("T-1.1", Status::Cancelled { order: None }),
    ];
    let mut minted = 0;
    for (subject, start) in &starts {
        for target in VALID_STATUS_NAMES {
            let status_of = |id: &str| {
                if id == *subject {
                    start.clone()
                } else {
                    Status::Idea
                }
            };
            let mut child = work("T-1.1", status_of("T-1.1"));
            child.parent = Some("T-1".into());
            child.spec = Some("docs/spec.md".into());
            let mut parent = work("T-2", status_of("T-2"));
            parent.spec = Some("docs/spec.md".into());
            let mut c = corpus(vec![
                program("T-1", Status::Queued { order: 10 }, &["T-1.1"], None),
                Ticket::Work(child),
                Ticket::Work(parent),
            ]);
            let before = c.clone();
            let case = format!("{subject} {start:?} -> {target}");
            if set_status(&mut c, &TicketId::from(*subject), target, CLOCK).is_err() {
                assert_eq!(before, c, "{case}: a refusal must not mutate");
                continue;
            }
            let after = c.get(&TicketId::from(*subject)).unwrap().status();
            assert_eq!(after.name().as_str(), *target, "{case}");
            if start.order().is_none() && after.order().is_some() {
                minted += 1;
            }
            for t in c.tickets.values() {
                assert!(
                    !check_reds_for_lack_of_order(t),
                    "{case}: {} lacks the order ticket check requires",
                    t.id()
                );
            }
        }
    }
    assert_eq!(
        minted, 3,
        "the idea parent mints into shipped, deferred and cancelled"
    );
}

/// The post-image gate enforces the check's order rule on changed parents: an order-less or
/// zero order on a non-idea parent refuses, while a changed child and an unchanged order-less
/// parent pass.
#[test]
fn post_image_gate_refuses_an_order_less_non_idea_parent() {
    let pre = corpus(vec![
        program("T-1", Status::Queued { order: 5 }, &["T-1.1"], None),
        child_of("T-1", "T-1.1", Status::Idea),
        Ticket::Work(work("T-2", Status::Deferred { order: None })),
    ])
    .tickets;
    let none_made_live = BTreeSet::new();
    for status in [
        Status::Deferred { order: None },
        Status::Cancelled { order: Some(0) },
    ] {
        let mut post = pre.clone();
        set_ticket_status(post.get_mut(&TicketId::from("T-1")).unwrap(), status);
        let changed = BTreeSet::from([TicketId::from("T-1")]);
        let err = validate_post_image(&pre, &post, &changed, &none_made_live)
            .expect_err("order-less non-idea parent");
        assert!(
            err.contains("post-image T-1: order required for status"),
            "{err}"
        );
    }
    let mut post = pre.clone();
    set_ticket_status(
        post.get_mut(&TicketId::from("T-1.1")).unwrap(),
        Status::Deferred { order: None },
    );
    let changed = BTreeSet::from([TicketId::from("T-1.1")]);
    validate_post_image(&pre, &post, &changed, &none_made_live)
        .expect("a changed child and the unchanged order-less T-2 pass");
}
