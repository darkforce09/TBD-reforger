use super::*;

/// Acceptance 4 — ship of a dotted child id succeeds at the op layer (the
/// the "Unknown ticket" hole), and the invariant: a parent whose `active`
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
    let out = ship(&mut c, &TicketId::from("T-1.1"), CLOCK).expect("ship child");
    assert_eq!(
        out.changed,
        vec![TicketId::from("T-1"), TicketId::from("T-1.1")],
        "parent counts as changed"
    );
    assert!(out.deleted.is_empty());
    match c.get(&TicketId::from("T-1.1")).unwrap() {
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
    match c.get(&TicketId::from("T-1")).unwrap() {
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
    ship(&mut c, &TicketId::from("T-2"), CLOCK).expect("ship");
    match c.get(&TicketId::from("T-2")).unwrap() {
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

/// Ship REFUSES a created_at-less ticket pre-write (the birth stamp can
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
        let err =
            ship(&mut c, &TicketId::from(id), CLOCK).expect_err("created_at-less must refuse");
        assert!(
            err.contains(id) && err.contains("created_at") && err.contains("hand-stamp"),
            "must name ticket, field and fix: {err}"
        );
    }
    assert_eq!(before, c, "refused ship must leave the corpus untouched");
}

/// stamp_sha writes the landing SHA through both arms, is a no-op on
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
        let err = stamp_sha(&mut c, &TicketId::from("T-1"), bad, CLOCK).expect_err("garbage sha");
        assert!(
            err.contains("refusing stamp-sha T-1") && err.contains("lowercase hex"),
            "{err}"
        );
    }
    // Non-shipped refuses naming the status.
    let err =
        stamp_sha(&mut c, &TicketId::from("T-2"), "abcdef12", CLOCK).expect_err("not shipped");
    assert!(
        err.contains("refusing stamp-sha T-2") && err.contains("queued"),
        "{err}"
    );
    assert_eq!(before, c, "refusals must not mutate");
    // Work write: both arms + marker dropped (measured provenance now).
    let out = stamp_sha(&mut c, &TicketId::from("T-1"), "abcdef12", CLOCK).expect("stamp work");
    assert_eq!(out.changed, vec![TicketId::from("T-1")]);
    match c.get(&TicketId::from("T-1")).unwrap() {
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
    stamp_sha(&mut c, &TicketId::from("T-5"), "beadfeed", CLOCK).expect("stamp program");
    match c.get(&TicketId::from("T-5")).unwrap() {
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
    let out =
        stamp_sha(&mut c, &TicketId::from("T-1"), "abcdef12", CLOCK).expect("re-stamp same sha");
    assert!(out.changed.is_empty(), "no-op must report nothing changed");
    // Different sha: refuse — shipped_at is never overwritten.
    let err =
        stamp_sha(&mut c, &TicketId::from("T-1"), "00000001", CLOCK).expect_err("different sha");
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
    let out = ship(&mut c, &TicketId::from("T-1.1"), CLOCK).expect("ship");
    assert_eq!(out.changed, vec![TicketId::from("T-1.1")]);
    match c.get(&TicketId::from("T-1")).unwrap() {
        Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.2")),
        Ticket::Work(_) => panic!("T-1 must stay program"),
    }
}

/// Acceptance — the ship ready-tier refusal (future ships): a ship from
/// queued with empty body fields refuses pre-write naming each — main_goal
/// included (the queued→shipped jump never passes the ready-class parse) — and
/// the quarantine exemption lets a wall-carrying ticket ship (the drain fills its
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
    let err =
        ship(&mut c, &TicketId::from("T-1"), CLOCK).expect_err("empty body must refuse the ship");
    assert!(
        err.contains(
            "empty: main_goal, context, requirement, current_state, approach, verify, acceptance"
        ),
        "must name each empty field, main_goal first: {err}"
    );
    assert!(err.contains("refusing ship T-1"), "{err}");
    assert_eq!(before, c, "refusal must not mutate");
    // Quarantined ships (exempt) …
    ship(&mut c, &TicketId::from("T-2"), CLOCK).expect("quarantined ticket ships");
    // … and a filled ticket ships.
    let filled = work("T-3", Status::Queued { order: 7 });
    let mut c = corpus(vec![Ticket::Work(filled)]);
    ship(&mut c, &TicketId::from("T-3"), CLOCK).expect("filled body ships");
}
