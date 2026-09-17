//! Outliner hierarchy visibility and comments tests for the outliner.

use super::*;

fn slot(id: &str, role: &str) -> SlotRow {
    SlotRow {
        id: id.to_string(),
        role: role.to_string(),
    }
}
fn layer(id: &str, name: &str, parent: Option<&str>, ents: &[&str]) -> LayerRow {
    layer_flags(id, name, parent, ents, false, false)
}

/// T-665 — a layer row with explicit `hidden`/`locked` flags.
fn layer_flags(
    id: &str,
    name: &str,
    parent: Option<&str>,
    ents: &[&str],
    hidden: bool,
    locked: bool,
) -> LayerRow {
    LayerRow {
        id: id.to_string(),
        name: name.to_string(),
        parent_id: parent.map(str::to_string),
        entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
        hidden,
        locked,
    }
}

/// The boot state: 8 seed slots, zero layers (`seed_random` files nothing). Every slot must be
/// reachable under Unfiled, id-sorted — this is what makes the dock non-empty at boot and the
/// gate's "click row 0 → s0" assertion exact.
#[test]
fn seed_boot_state_lists_all_slots_under_unfiled_id_sorted() {
    // Deliberately out of order: `materialize()` row order is arbitrary.
    let slots: Vec<SlotRow> = ["s3", "s0", "s7", "s1", "s5", "s2", "s6", "s4"]
        .iter()
        .map(|id| slot(id, "Rifleman"))
        .collect();

    let tree = build_outliner(&[], &slots);

    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].id, UNFILED_ID);
    assert_eq!(tree[0].kind, NodeKind::Unfiled);
    assert_eq!(tree[0].label, "Unfiled (8)");
    let ids: Vec<&str> = tree[0].children.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7"]);
    assert!(tree[0].children.iter().all(|n| n.label == "Rifleman"));
}

/// After the first place: the new slot is filed under the lazily-minted default layer and leaves
/// Unfiled, which keeps the remaining seeds.
#[test]
fn filed_slot_leaves_unfiled_and_appears_in_its_layer() {
    let slots = vec![slot("s0", "Rifleman"), slot("n0", "US Rifleman")];
    let layers = vec![layer("layer-1", "Layer 1", None, &["n0"])];

    let tree = build_outliner(&layers, &slots);

    assert_eq!(tree.len(), 2, "Unfiled then the real root layer");
    assert_eq!(tree[0].id, UNFILED_ID);
    assert_eq!(tree[0].label, "Unfiled (1)");
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(tree[0].children[0].id, "s0");

    assert_eq!(tree[1].id, "layer-1");
    assert_eq!(tree[1].kind, NodeKind::Folder);
    assert_eq!(tree[1].children.len(), 1);
    assert_eq!(tree[1].children[0].id, "n0");
    assert_eq!(tree[1].children[0].label, "US Rifleman");
}

/// No Unfiled root at all once every slot is filed — the React-parity shape.
#[test]
fn no_unfiled_root_when_everything_is_filed() {
    let slots = vec![slot("n0", "US Rifleman")];
    let layers = vec![layer("layer-1", "Layer 1", None, &["n0"])];
    let tree = build_outliner(&layers, &slots);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].id, "layer-1");
}

/// React's `[...childFolders, ...entityNodes]` order + `parentId` nesting.
#[test]
fn child_folders_precede_slots_and_nest_by_parent_id() {
    let slots = vec![slot("a", "Alpha"), slot("b", "Bravo")];
    let layers = vec![
        layer("root", "Root", None, &["a"]),
        layer("kid", "Kid", Some("root"), &["b"]),
    ];
    let tree = build_outliner(&layers, &slots);

    assert_eq!(tree.len(), 1, "only the root layer is top-level");
    let root = &tree[0];
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].id, "kid", "child folder first");
    assert_eq!(root.children[0].children[0].id, "b");
    assert_eq!(root.children[1].id, "a", "then this folder's slots");
}

/// Empty role → React's `'Unit'` fallback.
#[test]
fn empty_role_falls_back_to_unit() {
    let tree = build_outliner(&[], &[slot("s0", "")]);
    assert_eq!(tree[0].children[0].label, "Unit");
}

/// A slot id listed by a layer but absent from the doc is skipped, not rendered blank.
#[test]
fn dangling_entity_id_is_skipped() {
    let layers = vec![layer("layer-1", "Layer 1", None, &["ghost", "s0"])];
    let tree = build_outliner(&layers, &[slot("s0", "Rifleman")]);
    assert_eq!(tree[0].children.len(), 1);
    assert_eq!(tree[0].children[0].id, "s0");
}

/// A `parentId` cycle must terminate rather than hang the tab.
#[test]
fn parent_id_cycle_terminates() {
    let layers = vec![
        layer("a", "A", None, &[]),
        layer("b", "B", Some("a"), &[]),
        layer("c", "C", Some("b"), &[]),
    ];
    // Force a cycle: c is b's child, and b is also c's child.
    let mut cyclic = layers.clone();
    cyclic.push(layer("b2", "B2", Some("c"), &[]));
    let tree = build_outliner(&cyclic, &[]);
    assert_eq!(tree.len(), 1, "only `a` is rooted");
}

fn faction(id: &str, name: &str, squads: &[&str]) -> FactionRow {
    FactionRow {
        id: id.into(),
        key: name.into(), // tests use name as display; key mirrors for row shape
        name: name.into(),
        squad_ids: squads.iter().map(|s| (*s).to_string()).collect(),
    }
}
fn squad(id: &str, name: &str, faction: &str, slots: &[&str], leader: &str) -> SquadRow {
    SquadRow {
        id: id.into(),
        name: name.into(),
        faction_id: faction.into(),
        slot_ids: slots.iter().map(|s| (*s).to_string()).collect(),
        leader_slot_id: leader.into(),
        vehicle_ids: Vec::new(),
    }
}

/// No factions/squads (seed boot) → empty ORBAT tree.
#[test]
fn orbat_empty_before_any_squad() {
    assert!(build_orbat(&[], &[], &[slot("s0", "Rifleman")]).is_empty());
}

/// faction → squad → slot in doc (`squadIds`/`slotIds`) order; squad label carries its count.
#[test]
fn orbat_nests_faction_squad_slot_in_order() {
    let factions = vec![faction("f1", "US Army", &["sq1"])];
    let squads = vec![squad("sq1", "Alpha", "f1", &["s1", "s0"], "s1")];
    let slots = vec![slot("s0", "Rifleman"), slot("s1", "Squad Leader")];
    let tree = build_orbat(&factions, &squads, &slots);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].kind, NodeKind::Faction);
    assert_eq!(tree[0].label, "US Army");
    let sq = &tree[0].children[0];
    assert_eq!(sq.kind, NodeKind::Squad);
    assert_eq!(sq.label, "Alpha (2)");
    // slotIds order preserved (s1 before s0).
    let ids: Vec<&str> = sq.children.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["s1", "s0"]);
    assert!(sq.children.iter().all(|n| n.kind == NodeKind::Slot));
}

/// F3 — place-shaped rows (one side faction, two minted squads) both appear in the ORBAT tree.
#[test]
fn orbat_includes_two_squads_after_place_shaped_rows() {
    let factions = vec![faction(
        "faction-BLUFOR",
        "BLUFOR",
        &["squad-BLUFOR-1", "squad-BLUFOR-2"],
    )];
    let squads = vec![
        squad("squad-BLUFOR-1", "Squad 1", "faction-BLUFOR", &["a"], "a"),
        squad("squad-BLUFOR-2", "Squad 2", "faction-BLUFOR", &["b"], "b"),
    ];
    let slots = vec![slot("a", "Rifleman"), slot("b", "Rifleman")];
    let tree = build_orbat(&factions, &squads, &slots);
    assert_eq!(tree.len(), 1);
    let sq_ids: Vec<&str> = tree[0].children.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(sq_ids, ["squad-BLUFOR-1", "squad-BLUFOR-2"]);
}

/// F-L6 — SL badge flag from `leaderSlotId` only (not role / tag text).
#[test]
fn orbat_sl_badge_from_leader_slot_id() {
    let factions = vec![faction("f1", "BLUFOR", &["sq1"])];
    let squads = vec![squad("sq1", "Alpha", "f1", &["s0", "s1"], "s0")];
    // Role text looks like a tag — must not drive is_leader.
    let slots = vec![slot("s0", "Rifleman"), slot("s1", "SL")];
    let tree = build_orbat(&factions, &squads, &slots);
    let kids = &tree[0].children[0].children;
    assert!(kids.iter().find(|n| n.id == "s0").unwrap().is_leader);
    assert!(!kids.iter().find(|n| n.id == "s1").unwrap().is_leader);
    let flat = flatten(&tree);
    assert!(flat.iter().find(|r| r.id == "s0").unwrap().is_leader);
    assert!(!flat.iter().find(|r| r.id == "s1").unwrap().is_leader);
}

/// Flatten is pre-order (parent before children) with correct depths, one row per node.
#[test]
fn flatten_is_preorder_with_depths() {
    // Unfiled (depth 0) → its 2 slots (depth 1); a root layer (0) → child folder (1) → slot (2).
    let slots = vec![slot("s0", "A"), slot("s1", "B"), slot("n0", "N")];
    let layers = vec![
        layer("root", "Root", None, &[]),
        layer("kid", "Kid", Some("root"), &["n0"]),
    ];
    let tree = build_outliner(&layers, &slots);
    let flat = flatten(&tree);
    // Unfiled, s0, s1, root, kid, n0 = 6 rows.
    assert_eq!(flat.len(), 6);
    assert_eq!(flat[0].kind, NodeKind::Unfiled);
    assert_eq!(flat[0].depth, 0);
    assert_eq!((flat[1].id.as_str(), flat[1].depth), ("s0", 1));
    assert_eq!((flat[3].id.as_str(), flat[3].depth), ("root", 0));
    assert_eq!((flat[4].id.as_str(), flat[4].depth), ("kid", 1));
    assert_eq!((flat[5].id.as_str(), flat[5].depth), ("n0", 2));
}

/// T-177 A1 — the YouTube-guide continuation vector. `ancestors[k]` = "column k continues below
/// this row"; roots are `[]`, a non-last parent leads a child's vector with `true`, and last
/// children trim to `false`. Self-contained per row so the windowed slice needs no sibling peek.
#[test]
fn flatten_visible_computes_ancestor_continuation() {
    fn node(id: &str, kind: NodeKind, children: Vec<OutlinerNode>) -> OutlinerNode {
        OutlinerNode {
            id: id.to_string(),
            label: id.to_string(),
            kind,
            children,
            is_leader: false,
            hidden: false,
            locked: false,
            hidden_effective: false,
            locked_effective: false,
            tooltip: String::new(),
        }
    }
    // Root(+sib Root2) → [ChildA(+sib ChildB) → GrandA, ChildB(last) → Leaf]; Root2(last) → Leaf2.
    let tree = vec![
        node(
            "Root",
            NodeKind::Folder,
            vec![
                node(
                    "ChildA",
                    NodeKind::Folder,
                    vec![node("GrandA", NodeKind::Slot, vec![])],
                ),
                node(
                    "ChildB",
                    NodeKind::Folder,
                    vec![node("Leaf", NodeKind::Slot, vec![])],
                ),
            ],
        ),
        node(
            "Root2",
            NodeKind::Folder,
            vec![node("Leaf2", NodeKind::Slot, vec![])],
        ),
    ];
    let flat = flatten(&tree);
    let by = |id: &str| flat.iter().find(|r| r.id == id).unwrap().ancestors.clone();
    let ids = |id: &str| flat.iter().find(|r| r.id == id).unwrap().guide_ids.clone();
    assert_eq!(by("Root"), Vec::<bool>::new(), "roots draw no guide column");
    assert_eq!(ids("Root"), Vec::<String>::new());
    assert_eq!(
        by("ChildA"),
        vec![true],
        "non-last child's own connector continues"
    );
    assert_eq!(ids("ChildA"), vec!["Root".to_string()]);
    assert_eq!(
        by("GrandA"),
        vec![true, false],
        "non-last parent spine (true) + last child (false)"
    );
    assert_eq!(
        ids("GrandA"),
        vec!["Root".to_string(), "ChildA".to_string()]
    );
    assert_eq!(by("ChildB"), vec![false], "last child trims its connector");
    assert_eq!(
        by("Leaf"),
        vec![false, false],
        "last-child parent spine blank + last child"
    );
    assert_eq!(by("Root2"), Vec::<bool>::new());
    assert_eq!(by("Leaf2"), vec![false], "only child trims");
    assert_eq!(ids("Leaf2"), vec!["Root2".to_string()]);
}

/// Dangling squad/slot ids are skipped, not rendered blank.
#[test]
fn orbat_skips_dangling_ids() {
    let factions = vec![faction("f1", "US Army", &["ghostSquad", "sq1"])];
    let squads = vec![squad("sq1", "Alpha", "f1", &["ghostSlot", "s0"], "s0")];
    let tree = build_orbat(&factions, &squads, &[slot("s0", "Rifleman")]);
    assert_eq!(tree[0].children.len(), 1, "ghost squad skipped");
    assert_eq!(tree[0].children[0].children.len(), 1, "ghost slot skipped");
    assert_eq!(tree[0].children[0].children[0].id, "s0");
}

/// G1 — dialog class is near-fullscreen (`w-[min(` / `max-w-6xl`), not `max-w-xl`-only.
#[test]
fn orbat_manager_dialog_class_near_fullscreen() {
    assert!(
        ORBAT_MANAGER_DIALOG_CLASS.contains("w-[min(")
            || ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-6xl")
            || ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-4xl"),
        "{ORBAT_MANAGER_DIALOG_CLASS}"
    );
    assert!(
        !ORBAT_MANAGER_DIALOG_CLASS.contains("max-w-xl"),
        "max-w-xl must not be the width constraint"
    );
}

/// G7 — empty factions ⇒ empty filtered tree (no Stitch sample SoT).
#[test]
fn orbat_manager_empty_doc_empty_tree() {
    let filtered = filter_orbat_squads_by_side_key(&[], &[], &[], "BLUFOR");
    assert!(filtered.is_empty());
    assert!(!ORBAT_MANAGER_EMPTY.contains("L85A3"));
    assert!(!ORBAT_MANAGER_EMPTY.contains("US 1980s"));
}

/// G8 — OPFOR tab filters by FactionRow.key, not name substring.
#[test]
fn orbat_side_tab_filters_by_faction_key() {
    let factions = vec![
        FactionRow {
            id: "faction-OPFOR".into(),
            key: "OPFOR".into(),
            name: "Enemy Force BLUFOR-looking".into(), // name must not match BLUFOR tab
            squad_ids: vec!["sq-op".into()],
        },
        FactionRow {
            id: "faction-BLUFOR".into(),
            key: "BLUFOR".into(),
            name: "OPFOR string in name".into(), // name must not match OPFOR tab
            squad_ids: vec!["sq-blu".into()],
        },
    ];
    let squads = vec![
        squad("sq-op", "Op Squad", "faction-OPFOR", &["a"], "a"),
        squad("sq-blu", "Blu Squad", "faction-BLUFOR", &["b"], "b"),
    ];
    let slots = vec![slot("a", "Rifleman"), slot("b", "Rifleman")];
    let opfor = filter_orbat_squads_by_side_key(&factions, &squads, &slots, "OPFOR");
    assert_eq!(opfor.len(), 1);
    assert_eq!(opfor[0].id, "sq-op");
    let blufor = filter_orbat_squads_by_side_key(&factions, &squads, &slots, "BLUFOR");
    assert_eq!(blufor.len(), 1);
    assert_eq!(blufor[0].id, "sq-blu");
}

/// T-172 B6 — collapse hides the subtree, keeps the container row, and `has_children` is
/// true only for containers with kids; depths of surviving rows are unchanged.
#[test]
fn flatten_visible_collapse_hides_subtree() {
    let layers = vec![
        layer("l1", "Alpha", None, &["s0"]),
        layer("l2", "Bravo", Some("l1"), &["s1"]),
    ];
    let slots = vec![slot("s0", "SL"), slot("s1", "AR")];
    let tree = build_outliner(&layers, &slots);
    let all = flatten(&tree);
    // l1 > [s0, l2 > s1] — full walk has 4 rows (order: folders/slots per build rules).
    assert_eq!(all.len(), 4);
    let l1_row = all.iter().find(|r| r.id == "l1").unwrap();
    assert!(l1_row.has_children);
    assert!(!all.iter().find(|r| r.id == "s1").unwrap().has_children);

    let mut collapsed = std::collections::HashSet::new();
    collapsed.insert("l1".to_string());
    let vis = flatten_visible(&tree, &collapsed);
    assert_eq!(vis.len(), 1, "collapsed root leaves only its own row");
    assert_eq!(vis[0].id, "l1");
    assert_eq!(vis[0].depth, 0);

    // Collapsing the nested folder keeps l1's direct children visible.
    let mut collapsed = std::collections::HashSet::new();
    collapsed.insert("l2".to_string());
    let vis = flatten_visible(&tree, &collapsed);
    assert!(vis.iter().any(|r| r.id == "l2"));
    assert!(!vis.iter().any(|r| r.id == "s1"));
}

/* ───────────────────────────── T-665 — layer flags in the tree ───────────────────────────── */

/// A layer's own `hidden` flag reaches its Folder row AND dims its own slots (own == effective),
/// while a sibling layer with no flag stays fully visible. Fired once: flag present vs absent.
#[test]
fn own_hidden_flag_marks_folder_and_its_slots() {
    let layers = vec![
        layer_flags("h", "Hidden", None, &["s0"], true, false),
        layer("v", "Visible", None, &["s1"]),
    ];
    let slots = vec![slot("s0", "SL"), slot("s1", "AR")];
    let rows = flatten(&build_outliner(&layers, &slots));

    let hf = rows.iter().find(|r| r.id == "h").unwrap();
    assert!(
        hf.hidden && hf.hidden_effective,
        "own+effective on the folder"
    );
    let s0 = rows.iter().find(|r| r.id == "s0").unwrap();
    assert!(s0.hidden_effective, "slot under the hidden layer is dimmed");
    assert!(!s0.hidden, "a slot has no OWN flag");

    let vf = rows.iter().find(|r| r.id == "v").unwrap();
    assert!(!vf.hidden && !vf.hidden_effective, "sibling stays visible");
    let s1 = rows.iter().find(|r| r.id == "s1").unwrap();
    assert!(!s1.hidden_effective, "sibling's slot stays visible");
}

/// INHERITANCE, resolved at build time: a hidden/locked PARENT dims + lock-marks a child folder
/// and the child's slots, while the child rows carry NO own flag (never copied down). Un-flagging
/// is just the absence — this asserts the propagation the same way the store resolves it at read.
#[test]
fn child_folder_and_slots_inherit_parent_hidden_and_locked() {
    let layers = vec![
        layer_flags("p", "Parent", None, &[], true, true),
        layer("c", "Child", Some("p"), &["s0"]),
    ];
    let rows = flatten(&build_outliner(&layers, &vec![slot("s0", "Rifleman")]));

    let cf = rows.iter().find(|r| r.id == "c").unwrap();
    assert!(!cf.hidden && !cf.locked, "child folder has no OWN flags");
    assert!(
        cf.hidden_effective && cf.locked_effective,
        "child folder inherits parent hidden+locked"
    );
    let s0 = rows.iter().find(|r| r.id == "s0").unwrap();
    assert!(
        s0.hidden_effective && s0.locked_effective,
        "slot two levels down inherits both"
    );
}

/* ───────────────── T-651 — editor comments / annotations (PLACE-COMMENT-001) ───────────────── */

fn comment(id: &str, title: &str, tooltip: &str) -> CommentRow {
    CommentRow {
        id: id.to_string(),
        title: title.to_string(),
        tooltip: tooltip.to_string(),
    }
}

/// A comment files into a folder through the SAME `entityIds` array a slot does, and it sits at
/// its authored position in that sequence rather than in a segregated block — so an operator who
/// dropped a note between two units sees it between them.
#[test]
fn a_filed_comment_sits_in_entity_ids_order_beside_slots() {
    let layers = vec![layer("L", "Layer", None, &["s1", "cmt-1", "s2"])];
    let slots = vec![slot("s1", "SL"), slot("s2", "Rifleman")];
    let comments = vec![comment("cmt-1", "Assembly area", "form up here")];
    let tree = build_outliner_with_comments(&layers, &slots, &comments);

    assert_eq!(tree.len(), 1, "no Unfiled root — everything is filed");
    let kids = &tree[0].children;
    assert_eq!(
        kids.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        vec!["s1", "cmt-1", "s2"],
        "the comment keeps its authored slot in the sequence"
    );
    assert_eq!(kids[1].kind, NodeKind::Comment);
    assert_eq!(kids[1].label, "Assembly area", "the title is the row label");
    assert_eq!(kids[1].tooltip, "form up here");
    assert!(kids[1].children.is_empty(), "a comment is a leaf");
}

/// A comment listed in no folder lands in the Unfiled pseudo-root, whose count covers BOTH kinds
/// (a header reading "Unfiled (1)" over two rows is the bug this pins).
#[test]
fn unfiled_comments_join_the_pseudo_root_and_are_counted() {
    let tree = build_outliner_with_comments(
        &[],
        &[slot("s1", "SL")],
        &[comment("cmt-2", "B", ""), comment("cmt-1", "A", "")],
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].kind, NodeKind::Unfiled);
    assert_eq!(
        tree[0].label, "Unfiled (3)",
        "one slot + two comments — the header counts BOTH kinds"
    );
    assert_eq!(
        tree[0]
            .children
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        vec!["s1", "cmt-1", "cmt-2"],
        "slots first, then comments sorted by id (Unfiled has no authored order)"
    );
}

/// An Unfiled root appears for comments ALONE — a mission whose only annotation is unfiled must
/// still show it, not silently swallow it because there are no unfiled slots.
#[test]
fn a_lone_unfiled_comment_still_gets_the_pseudo_root() {
    let layers = vec![layer("L", "Layer", None, &["s1"])];
    let tree = build_outliner_with_comments(
        &layers,
        &[slot("s1", "SL")],
        &[comment("cmt-1", "note", "")],
    );
    assert_eq!(tree[0].kind, NodeKind::Unfiled);
    assert_eq!(tree[0].label, "Unfiled (1)");
    assert_eq!(tree[0].children[0].kind, NodeKind::Comment);
}

/// An untitled comment still renders a clickable row (the `SLOT_FALLBACK_LABEL` rule) — a blank
/// title must not produce a zero-width row you cannot select in order to fix it.
#[test]
fn an_untitled_comment_falls_back_to_a_label() {
    let tree = build_outliner_with_comments(&[], &[], &[comment("cmt-1", "", "body")]);
    assert_eq!(tree[0].children[0].label, COMMENT_FALLBACK_LABEL);
}

/// A comment does NOT inherit its folder's hidden/locked adornments: it is not in the render SoA
/// (so "hidden" has nothing to hide) and its position is not transform-locked (see
/// `MissionDocCore::set_comment_position`). Dimming it would advertise a refusal that does not
/// exist — while the sibling SLOT in the same folder does inherit both, which is the contrast
/// that makes this a decision rather than an omission.
#[test]
fn a_comment_does_not_inherit_hidden_or_locked_but_its_sibling_slot_does() {
    let layers = vec![layer_flags(
        "L",
        "Layer",
        None,
        &["s1", "cmt-1"],
        true,
        true,
    )];
    let tree = build_outliner_with_comments(
        &layers,
        &[slot("s1", "SL")],
        &[comment("cmt-1", "note", "")],
    );
    let kids = &tree[0].children;
    assert!(
        kids[0].hidden_effective && kids[0].locked_effective,
        "slot inherits"
    );
    assert!(
        !kids[1].hidden_effective && !kids[1].locked_effective,
        "comment does not: {:?}",
        kids[1]
    );
}

/// `build_outliner` (the comment-free entry point) is exactly `build_outliner_with_comments`
/// with an empty slice — so no caller that predates comments can drift from the one that has
/// them.
#[test]
fn build_outliner_is_the_empty_comment_case() {
    let layers = vec![layer("L", "Layer", None, &["s1"])];
    let slots = vec![slot("s1", "SL")];
    assert_eq!(
        build_outliner(&layers, &slots),
        build_outliner_with_comments(&layers, &slots, &[])
    );
}

/// The tooltip survives the flatten into windowed rows — the windowed renderer draws from
/// `FlatRow`, so a body that stopped at `OutlinerNode` would vanish on any tree past the
/// virtualization threshold and nowhere else (the nastiest possible way to lose it).
#[test]
fn flatten_carries_the_comment_tooltip_into_the_windowed_row() {
    let layers = vec![layer("L", "Layer", None, &["cmt-1"])];
    let tree = build_outliner_with_comments(&layers, &[], &[comment("cmt-1", "T", "long body")]);
    let rows = flatten(&tree);
    let row = rows
        .iter()
        .find(|r| r.kind == NodeKind::Comment)
        .expect("a comment row");
    assert_eq!(row.tooltip, "long body");
    assert_eq!(row.label, "T");
    assert_eq!(row.depth, 1);
    // Every other row carries an empty tooltip — the field is comment-only.
    assert!(rows
        .iter()
        .filter(|r| r.kind != NodeKind::Comment)
        .all(|r| r.tooltip.is_empty()));
}

/// A dangling id in `entityIds` (the comment was deleted, the folder not yet patched) is skipped
/// exactly as a dangling slot id is — a stale reference must never panic or render a ghost row.
#[test]
fn a_dangling_comment_id_is_skipped_not_rendered() {
    let layers = vec![layer("L", "Layer", None, &["cmt-gone", "s1"])];
    let tree = build_outliner_with_comments(&layers, &[slot("s1", "SL")], &[]);
    assert_eq!(
        tree[0]
            .children
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        vec!["s1"]
    );
}
