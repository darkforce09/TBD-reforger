use super::*;

#[test]
fn ready_prose_on_t090_family() {
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
fn t159_23_shipped_at_pin() {
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

#[test]
fn mapper_minted_t674_t675_children() {
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

/// T-090.6 keeps its engine scope through the v2 cutover (the old per-id override
/// table put it there; the v2 migrator maps `[scope.engine]` → domain engine).
#[test]
fn t090_6_is_engine_scope() {
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

/// T-917.2: the target synthesis over v2 scopes keeps the exact v1 outputs.
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

/// T-912.2 regression pin, RETARGETED by T-916.2: `ticket_to_value` mirrors
/// `children`/`active` into their legacy spellings, and `value_to_ticket` must accept its
/// own output — the alias-vs-mirror clash made `duplicate field \`children\`` out of every
/// loaded program and broke every registry mutator (`ticket ship T-905` → `save T-067`
/// refuse, measured at the T-912.1 tip). Since T-916.2 no MUTATOR reaches
/// `value_to_ticket` (see `mutators_never_reach_the_value_writer_pin`), but the mirrored
/// Value is still what brief/show/get/sync/queue.json consume — this pin keeps the alias
/// class dead on that read surface.
#[test]
fn value_to_ticket_accepts_ticket_to_value_output() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    // T-067 is the program the live failure named; round-trip the whole loaded registry so
    // any ticket whose value carries mirrored keys is covered, not just one.
    let reg = load_phase2_tree(&root).expect("load phase2 tree");
    for t in reg["tickets"].as_array().expect("tickets") {
        let id = t["id"].as_str().unwrap_or("?");
        let back = value_to_ticket(t).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        assert_eq!(back.id(), id);
    }
}

/// T-916.2 — the write path is the TYPED one. Two facts, pinned together:
///
/// 1. `registry::save_registry` REFUSES a phase-2 tree (in-memory probe against the live
///    root — nothing is written on the refusal path), so `save_tree` / `value_to_ticket`
///    are unreachable as writers even if a caller sneaks back;
/// 2. `cmds.rs` — the mutator surface — no longer names `save_registry` at all: every
///    verb writes through `crate::ops` + `Corpus::write_back`. Needle assembled at
///    runtime (the T-912.1 tripwire trick) so this test's own source cannot satisfy it.
#[test]
fn mutators_never_reach_the_value_writer_pin() {
    let root = repo_root();
    if !tree_is_phase2(&root) {
        return;
    }
    let reg = crate::registry::load_registry(&root).expect("load live registry");
    let err = crate::registry::save_registry(&root, &reg)
        .expect_err("save_registry must refuse a phase-2 tree");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("typed ops") && msg.contains("migration/test-only"),
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
        "cmds.rs names `{needle}` again — mutators must write through tbd_tickets ops \
         (T-916.2), never the Value round-trip"
    );
}
