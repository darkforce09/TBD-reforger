use super::*;

/// A ready-class ticket must carry all three prose fields, so the typed projection cannot hand a
/// brief an empty spec, goal or acceptance list.
#[test]
fn ready_class_tickets_carry_spec_main_goal_and_acceptance() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    for id in ["T-090", "T-090.4", "T-090.6", "T-090.7", "T-090.9"] {
        let t = parse_file(&root, id);
        let (spec, story, acc) = match t.status() {
            Status::Ready {
                spec,
                main_goal,
                acceptance,
                ..
            }
            | Status::Running {
                spec,
                main_goal,
                acceptance,
                ..
            }
            | Status::Review {
                spec,
                main_goal,
                acceptance,
                ..
            } => (spec, main_goal, acceptance),
            other @ Status::Idea
            | other @ Status::Queued { .. }
            | other @ Status::Shipped { .. }
            | other @ Status::Deferred { .. }
            | other @ Status::Cancelled { .. } => {
                panic!("{id} status {:?} is not ready-class", other.name())
            }
        };
        assert!(!spec.trim().is_empty(), "{id} spec");
        assert!(!story.trim().is_empty(), "{id} main_goal");
        assert!(acc.iter().any(|s| !s.trim().is_empty()), "{id} acceptance");
    }
}

#[test]
fn shipped_ticket_keeps_its_shipped_at_commit() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    let t = parse_file(&root, "T-159.23");
    match t.status() {
        Status::Shipped { shipped_at, .. } => {
            assert_eq!(shipped_at.as_deref(), Some("69dc5da5"));
        }
        other @ Status::Idea
        | other @ Status::Queued { .. }
        | other @ Status::Ready { .. }
        | other @ Status::Running { .. }
        | other @ Status::Review { .. }
        | other @ Status::Deferred { .. }
        | other @ Status::Cancelled { .. } => panic!("T-159.23 status {:?}", other.name()),
    }
    match t {
        Ticket::Work(w) => assert_eq!(w.shipped_at.as_deref(), Some("69dc5da5")),
        Ticket::Program(_) => panic!("T-159.23 must be Work"),
    }
}

/// Parent and child agree in both directions: a child projects as work with its parent set, and
/// the parent projects as a program that lists that child.
#[test]
fn program_children_parse_as_work_and_their_parents_list_them() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    for id in ["T-674.1", "T-674.2", "T-675.1", "T-675.2"] {
        let t = parse_file(&root, id);
        assert!(matches!(t, Ticket::Work(_)), "{id} must be Work");
        let parent = match &t {
            Ticket::Work(w) => w.parent.clone(),
            Ticket::Program(_) => None,
        };
        let want = if id.starts_with("T-674") {
            "T-674"
        } else {
            "T-675"
        };
        assert_eq!(parent.as_deref(), Some(want), "{id} parent");
    }
    for pid in ["T-674", "T-675"] {
        match parse_file(&root, pid) {
            Ticket::Program(p) => {
                assert!(p.children.iter().any(|c| c == &format!("{pid}.1")));
                assert!(p.children.iter().any(|c| c == &format!("{pid}.2")));
            }
            Ticket::Work(_) => panic!("{pid} must be Program"),
        }
    }
}

/// A ticket whose file carries a `[scope.engine]` table projects into [`Domain::Engine`].
#[test]
fn engine_scope_table_projects_to_the_engine_domain() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    match parse_file(&root, "T-090.6") {
        Ticket::Work(w) => {
            assert_eq!(
                w.scope.domain,
                Domain::Engine,
                "T-090.6 scope {:?}",
                w.scope
            );
        }
        Ticket::Program(_) => panic!("T-090.6 must be Work"),
    }
}

/// Target synthesis is a total function of the scope domain: one domain, one target list.
#[test]
fn targets_from_scope_v2_outputs() {
    let scope = |domain| ScopeV2 {
        domain,
        layer: "x".into(),
        component: None,
        surface: vec![],
    };
    assert_eq!(targets_from_scope(&scope(Domain::Website)), vec!["website"]);
    assert_eq!(targets_from_scope(&scope(Domain::Mod)), vec!["mod"]);
    assert_eq!(targets_from_scope(&scope(Domain::Schema)), vec!["shared"]);
    assert_eq!(targets_from_scope(&scope(Domain::Engine)), vec!["root"]);
    assert_eq!(targets_from_scope(&scope(Domain::Repo)), vec!["root"]);
}

/// `ticket_to_value` mirrors `children` and `active` into their second spellings, so
/// `value_to_ticket` must accept its own output: a serde alias that clashes with a mirrored key
/// raises `duplicate field` out of every loaded program and breaks the whole read surface.
/// No mutator reaches `value_to_ticket` (see `mutators_never_reach_the_value_writer_pin`), but
/// brief, show, get, sync and the queue view all consume the mirrored `Value`.
#[test]
fn value_to_ticket_accepts_ticket_to_value_output() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    // Round-trip the whole loaded registry, so every ticket whose value carries mirrored keys is
    // covered rather than one sampled program.
    let reg = load_phase2_tree(&root).expect("load phase2 tree");
    for t in reg["tickets"].as_array().expect("tickets") {
        let id = t["id"].as_str().unwrap_or("?");
        let back = value_to_ticket(t).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        assert_eq!(back.id(), id);
    }
}

/// The write path is the typed one. Two facts, pinned together:
///
/// 1. `registry::save_registry` REFUSES a typed tree — an in-memory probe against the live root,
///    which writes nothing on the refusal path — so `save_tree` and `value_to_ticket` are
///    unreachable as writers even if a caller finds its way back to them;
/// 2. no module under `cli/`, the mutator surface, names `save_registry` at all: every verb
///    writes through `crate::ops` and `Corpus::write_back`. The needle is assembled at runtime
///    so this test's own source cannot satisfy the search it performs.
#[test]
fn mutators_never_reach_the_value_writer_pin() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    let reg = crate::registry::load_registry(&root).expect("load live registry");
    let err = crate::registry::save_registry(&root, &reg)
        .expect_err("save_registry must refuse a typed tree");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("typed ops") && msg.contains("one file per changed ticket"),
        "refusal must name the typed path: {msg}"
    );

    let cli = crate::repository::find_repo_root()
        .unwrap()
        .join("tools_v2/ticket-engine/src/cli");
    let cmds_src = walkdir::WalkDir::new(cli)
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .filter(|entry| !entry.path().components().any(|c| c.as_os_str() == "tests"))
        .map(|entry| std::fs::read_to_string(entry.path()).unwrap())
        .collect::<String>();
    let needle = format!("save_{}", "registry");
    assert!(
        !cmds_src.contains(&needle),
        "a module under cli/ names `{needle}` again — mutators must write through the typed \
         ops surface, never the Value round-trip"
    );
}
