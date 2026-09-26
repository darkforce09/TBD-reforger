use super::*;
use crate::ticket_registry::services::corpus_loading::{Corpus, Counts, LoadedTicket, is_child_id};
use std::path::PathBuf;
use ticket_engine::ScopeV2;

fn parse(toml: &str) -> Ticket {
    ticket_engine::parse_ticket_toml(toml).unwrap()
}

fn work(id: &str, status_lines: &str, extra: &str) -> Ticket {
    parse(&format!(
        r#"id = "{id}"
kind = "work"
title = "title of {id}"
{status_lines}
{extra}
[scope]
domain = "repo"
layer = "docs"
"#
    ))
}

fn corpus_of(tickets: Vec<Ticket>) -> Corpus {
    let tickets: Vec<LoadedTicket> = tickets
        .into_iter()
        .map(|ticket| {
            let path = PathBuf::from(format!("{}.toml", ticket.id()));
            LoadedTicket { ticket, path }
        })
        .collect();
    let parents = tickets
        .iter()
        .filter(|t| !is_child_id(t.ticket.id()))
        .count();
    let children = tickets.len() - parents;
    Corpus {
        counts: Counts {
            total: tickets.len(),
            parents,
            children,
        },
        tickets,
    }
}

fn column_ids(board: &BoardModel, status: StatusName) -> Vec<&str> {
    board.columns[column_of(status)]
        .cards
        .iter()
        .map(|c| c.id.as_str())
        .collect()
}

#[test]
fn status_order_matches_column_of() {
    for (i, status) in STATUS_ORDER.iter().enumerate() {
        assert_eq!(column_of(*status), i);
    }
    assert_eq!(STATUS_ORDER[0].as_str(), "idea");
    assert_eq!(STATUS_ORDER[7].as_str(), "cancelled");
}

#[test]
fn bucketing_headers_and_chips() {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        work("T-2", "status = \"queued\"\norder = 10", ""),
        work("T-3", "status = \"queued\"\norder = 20", ""),
        work("T-4", "status = \"shipped\"", ""),
    ]);
    let board = BoardModel::build(&corpus);
    assert_eq!(column_ids(&board, StatusName::Idea), vec!["T-1"]);
    assert_eq!(column_ids(&board, StatusName::Queued), vec!["T-2", "T-3"]);
    assert_eq!(column_ids(&board, StatusName::Shipped), vec!["T-4"]);
    assert!(column_ids(&board, StatusName::Ready).is_empty());
    let queued = &board.columns[column_of(StatusName::Queued)];
    assert_eq!(queued.header, "queued · 2");
    assert_eq!(queued.chip, "queued\n2");
}

#[test]
fn cards_sort_by_order_then_numeric_id() {
    let corpus = corpus_of(vec![
        work("T-2", "status = \"queued\"\norder = 30", ""),
        work("T-3", "status = \"queued\"\norder = 10", ""),
        work("T-1", "status = \"queued\"\norder = 10", ""),
    ]);
    let board = BoardModel::build(&corpus);
    // The documented schema and display invariants apply to every loaded ticket.
    assert_eq!(
        column_ids(&board, StatusName::Queued),
        vec!["T-1", "T-3", "T-2"]
    );
}

#[test]
fn ideas_sort_by_numeric_id_not_string() {
    let corpus = corpus_of(vec![
        work("T-100", "status = \"idea\"", ""),
        work("T-9", "status = \"idea\"", ""),
        work("T-10", "status = \"idea\"", ""),
    ]);
    let board = BoardModel::build(&corpus);
    assert_eq!(
        column_ids(&board, StatusName::Idea),
        vec!["T-9", "T-10", "T-100"]
    );
}

#[test]
fn child_ids_sort_numerically_within_ties() {
    let corpus = corpus_of(vec![
        work("T-915.10", "status = \"queued\"\norder = 5", ""),
        work("T-915.2", "status = \"queued\"\norder = 5", ""),
    ]);
    let board = BoardModel::build(&corpus);
    assert_eq!(
        column_ids(&board, StatusName::Queued),
        vec!["T-915.2", "T-915.10"]
    );
}

#[test]
fn unparsable_ids_sort_last() {
    let (segs, _) = id_sort_key("T-abc");
    assert_eq!(segs, vec![u64::MAX]);
    let (segs, _) = id_sort_key("T-915.1");
    assert_eq!(segs, vec![915, 1]);
}

#[test]
fn executor_chip_defaults_to_claude_code() {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        work("T-2", "status = \"idea\"", "executor = \"documentation\"\n"),
    ]);
    let board = BoardModel::build(&corpus);
    let idea = &board.columns[column_of(StatusName::Idea)];
    assert_eq!(idea.cards[0].executor, "claude-code");
    assert_eq!(idea.cards[1].executor, "documentation");
}

#[test]
fn order_label_precomputed() {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"queued\"\norder = 5961", ""),
        work("T-2", "status = \"idea\"", ""),
    ]);
    let board = BoardModel::build(&corpus);
    assert_eq!(
        board.columns[column_of(StatusName::Queued)].cards[0].order_label,
        "#5961"
    );
    assert_eq!(
        board.columns[column_of(StatusName::Idea)].cards[0].order_label,
        ""
    );
}

#[test]
fn id_to_index_resolves_every_ticket() {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", ""),
        work("T-2.1", "status = \"idea\"", ""),
    ]);
    let board = BoardModel::build(&corpus);
    assert_eq!(board.id_to_index.len(), 2);
    let i = board.id_to_index["T-2.1"];
    assert_eq!(corpus.tickets[i].ticket.id(), "T-2.1");
}

#[test]
fn truncate_chars_is_char_safe() {
    assert_eq!(truncate_chars("short", 10), "short");
    let long = "ünïcödé".repeat(20);
    let cut = truncate_chars(&long, 12);
    assert_eq!(cut.chars().count(), 12);
    assert!(cut.ends_with('…'));
}

fn work_scope(scope: &ScopeV2, estimated: &[String]) -> Breadcrumb {
    breadcrumb(scope, estimated)
}

fn scope_of(t: &Ticket) -> &ScopeV2 {
    match t {
        Ticket::Work(w) => &w.scope,
        Ticket::Program(_) => unreachable!("work builder"),
    }
}

/// breadcrumb string assembly over all four v2 levels — empty
/// component is SKIPPED (`domain › layer`, no marker: component-free layers
/// are complete); empty surface UNDER a component sets `no_surface` (the
/// detail-panel `(no surface)` marker; cards omit it).
#[test]
fn breadcrumb_assembly_variants() {
    // No component (⇒ no surface tier): two segments, no marker.
    let repo = work("T-1", "status = \"idea\"", "");
    let bc = work_scope(scope_of(&repo), &[]);
    assert_eq!(bc.label(), "repo › docs");
    assert_eq!(
        bc.segs.iter().map(|s| s.level).collect::<Vec<_>>(),
        vec![ScopeLevel::Domain, ScopeLevel::Layer]
    );
    assert!(!bc.no_surface);
    assert!(!bc.estimated);

    // Component with an EMPTY surface list: marker flag on.
    let backend = parse(
        r#"id = "T-2"
kind = "work"
title = "b"
status = "idea"

[scope]
domain = "website"
layer = "backend"
component = "http_api"
"#,
    );
    let bc = work_scope(scope_of(&backend), &[]);
    assert_eq!(bc.label(), "website › backend › http_api");
    assert!(bc.no_surface, "component present + no surface ⇒ marker");

    // Full four-level path: surfaces join into ONE segment.
    let editor = parse(
        r#"id = "T-3"
kind = "work"
title = "e"
status = "idea"

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
surface = ["dock_left", "map_canvas"]
"#,
    );
    let bc = work_scope(scope_of(&editor), &[]);
    assert_eq!(
        bc.label(),
        "website › frontend › mission_creator › dock_left+map_canvas"
    );
    assert_eq!(bc.segs.len(), 4);
    assert_eq!(bc.segs[3].level, ScopeLevel::Surface);
    assert!(!bc.no_surface);
}

/// The estimated-scope marker predicate: exactly the `"scope"` entry, nothing
/// else, flips the breadcrumb glyph.
#[test]
fn breadcrumb_estimated_scope_marker() {
    assert!(scope_estimated(&["scope".to_owned()]));
    assert!(scope_estimated(&["tokens".to_owned(), "scope".to_owned()]));
    assert!(!scope_estimated(&["tokens".to_owned()]));
    assert!(!scope_estimated(&[]));

    let inferred = work(
        "T-1",
        "status = \"idea\"",
        "estimated = [\"scope\"]\nestimate_note = \"owns-inferred\"\n",
    );
    assert!(work_scope(scope_of(&inferred), estimated_of(&inferred)).estimated);
    let carried = work("T-2", "status = \"idea\"", "estimated = [\"tokens\"]\n");
    assert!(!work_scope(scope_of(&carried), estimated_of(&carried)).estimated);
}

fn estimated_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Work(w) => &w.estimated,
        Ticket::Program(p) => &p.estimated,
    }
}

/// class enum stays in lockstep with the registry's closed set, and
/// the chip accents are total + pairwise distinct (the deny-wildcard matches
/// in `as_str`/`accent_rgb` make a 6th enum variant a compile error).
#[test]
fn class_parity_and_total_distinct_accents() {
    assert_eq!(
        Class::ALL.map(Class::as_str).to_vec(),
        ticket_engine::CLASS_VALUES.to_vec(),
        "Class::ALL must mirror ticket_engine::CLASS_VALUES exactly"
    );
    for class in Class::ALL {
        assert_eq!(Class::parse(class.as_str()), Some(class));
    }
    assert_eq!(Class::parse("epic"), None);
    assert_eq!(Class::parse("Bug"), None, "raw lowercase values only");
    let mut accents: Vec<_> = Class::ALL.iter().map(|c| c.accent_rgb()).collect();
    accents.sort_unstable();
    accents.dedup();
    assert_eq!(accents.len(), Class::ALL.len(), "chip accents must differ");
}

/// Cards precompute breadcrumb + class chip; programs carry neither.
#[test]
fn cards_precompute_breadcrumb_and_class() {
    let corpus = corpus_of(vec![
        work("T-1", "status = \"idea\"", "class = \"bug\"\n"),
        parse(
            r#"id = "T-9"
kind = "program"
title = "prog"
summary = "s"
status = "idea"
children = ["T-9.1"]
"#,
        ),
    ]);
    let board = BoardModel::build(&corpus);
    let idea = &board.columns[column_of(StatusName::Idea)];
    let work_card = idea.cards.iter().find(|c| c.id == "T-1").unwrap();
    assert_eq!(work_card.class, Some(Class::Bug));
    assert_eq!(
        work_card.breadcrumb.as_ref().unwrap().label(),
        "repo › docs"
    );
    let program_card = idea.cards.iter().find(|c| c.id == "T-9").unwrap();
    assert_eq!(program_card.class, None, "no class ⇒ no chip");
    assert!(program_card.breadcrumb.is_none(), "programs have no scope");
}

/// cards precompute the main_goal hover tooltip — present and
/// nonblank only; a goal-less or blank-goal ticket carries `None` (no
/// empty bubble), and both ticket kinds surface their goal.
#[test]
fn cards_precompute_main_goal_tooltip() {
    let corpus = corpus_of(vec![
        work(
            "T-1",
            "status = \"idea\"",
            "main_goal = \"the operator reads the goal on hover\"\n",
        ),
        work("T-2", "status = \"idea\"", ""),
        work("T-3", "status = \"idea\"", "main_goal = \"   \"\n"),
        parse(
            r#"id = "T-9"
kind = "program"
title = "prog"
summary = "s"
status = "idea"
children = ["T-9.1"]
main_goal = "program goals surface too"
"#,
        ),
    ]);
    let board = BoardModel::build(&corpus);
    let idea = &board.columns[column_of(StatusName::Idea)];
    let by_id = |id: &str| idea.cards.iter().find(|c| c.id == id).unwrap();
    assert_eq!(
        by_id("T-1").tooltip.as_deref(),
        Some("the operator reads the goal on hover")
    );
    assert_eq!(by_id("T-2").tooltip, None, "absent goal ⇒ no tooltip");
    assert_eq!(by_id("T-3").tooltip, None, "blank goal ⇒ no tooltip");
    assert_eq!(
        by_id("T-9").tooltip.as_deref(),
        Some("program goals surface too")
    );
}

#[test]
fn status_label_forms() {
    let idea = work("T-1", "status = \"idea\"", "");
    assert_eq!(status_label(idea.status()), "idea");
    let queued = work("T-2", "status = \"queued\"\norder = 5", "");
    assert_eq!(status_label(queued.status()), "queued · #5");
    let shipped = work("T-3", "status = \"shipped\"", "");
    assert_eq!(status_label(shipped.status()), "shipped");
}

#[test]
fn view_maps_both_kinds() {
    let program = parse(
        r#"id = "T-9"
kind = "program"
title = "prog"
summary = "sum"
status = "queued"
order = 40
children = ["T-9.1", "T-9.2"]
active = "T-9.1"
"#,
    );
    let v = view(&program);
    assert_eq!(v.kind, "program");
    assert_eq!(v.children, &["T-9.1".to_string(), "T-9.2".to_string()][..]);
    assert_eq!(v.active, Some("T-9.1"));
    assert_eq!(v.parent, None);
    assert!(v.scope.is_none());
    assert_eq!(v.class, None);
    assert_eq!(v.plan, None);
    assert!(v.estimated.is_empty());

    let workt = work(
        "T-9.1",
        "status = \"shipped\"\nshipped_at = \"abc123\"",
        "parent = \"T-9\"\nclass = \"chore\"\nestimated = [\"scope\"]\n\
             plan = \"docs/plans/t-9_1_plan.md\"\n\
             context = [\"why now\"]\nrequirement = [\"the ask\"]\n\
             current_state = [\"what exists\"]\napproach = [\"step 1\"]\n\
             verify = [\"cargo test\"]\ncitations = [\"docs/x.md\"]\n\
             migration_legacy = [\"wall line\"]\n",
    );
    let v = view(&workt);
    assert_eq!(v.kind, "work");
    assert_eq!(v.parent, Some("T-9"));
    // the plan path reaches the view (the viewer's primary click).
    assert_eq!(v.plan, Some("docs/plans/t-9_1_plan.md"));
    assert_eq!(v.shipped_at, Some("abc123"));
    assert!(v.children.is_empty());
    // the body lists + quarantine reach the view verbatim.
    assert_eq!(v.context, &["why now".to_string()][..]);
    assert_eq!(v.requirement, &["the ask".to_string()][..]);
    assert_eq!(v.current_state, &["what exists".to_string()][..]);
    assert_eq!(v.approach, &["step 1".to_string()][..]);
    assert_eq!(v.verify, &["cargo test".to_string()][..]);
    assert_eq!(v.citations, &["docs/x.md".to_string()][..]);
    assert_eq!(v.migration_legacy, &["wall line".to_string()][..]);
    let bc = breadcrumb(v.scope.unwrap(), v.estimated);
    assert_eq!(bc.label(), "repo › docs");
    assert!(bc.estimated);
    assert_eq!(v.class, Some("chore"));
    assert_eq!(kind_str(&workt), "work");
}
