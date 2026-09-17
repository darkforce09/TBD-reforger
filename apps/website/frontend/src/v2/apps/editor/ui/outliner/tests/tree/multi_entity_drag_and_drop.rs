//! Multi entity drag and drop tests for the outliner.

/// T-946.86 (.83) — the drop CONSUMES the multi-selection instead of moving only its anchor.
use super::{drag_set_for, node_descendant_ids};
use crate::v2::apps::editor::ui::outliner::outliner::{NodeKind, OutlinerNode};
use crate::v2::core::test_support::class_r_scrub::live_code;

fn live() -> String {
    live_code(include_str!("../../tree.rs"))
}

fn node(id: &str, children: Vec<OutlinerNode>) -> OutlinerNode {
    OutlinerNode {
        id: id.to_string(),
        label: id.to_string(),
        kind: NodeKind::Folder,
        children,
        is_leader: false,
        hidden: false,
        locked: false,
        hidden_effective: false,
        locked_effective: false,
        tooltip: String::new(),
    }
}

/// **The drop reads the SET.** Wave 255 armed the whole `DragSet` on pointerdown and then
/// completed through the engine's single-id `layer_drag` latch, so a five-row drag moved one
/// row. PERTURB: drop the `complete_multi_drop_onto_folder` call and this goes RED.
#[test]
fn the_folder_drop_consumes_the_pending_drag_set() {
    let src = live();
    assert!(
        src.contains("complete_multi_drop_onto_folder("),
        "T-946.86 (.83): the folder-row drop must consume the multi-select DragSet — \
         completing through the single-id latch alone moves only the anchor"
    );
    assert!(
        src.contains("node_descendant_ids("),
        "T-946.86 (.83): the drop must supply the subtree answer `plan_drop` asks for, or the \
         planner cannot refuse a folder dropped into its own child"
    );
}

/// The single-id completion survives as the FALLBACK, and is reached only when the multi path
/// declines. Two unconditional completions would double-apply the anchor's move.
#[test]
fn the_single_id_completion_is_the_fallback_not_a_second_commit() {
    let src = live();
    let at_multi = src
        .find("complete_multi_drop_onto_folder(")
        .expect("checked by the pin above");
    let after = &src[at_multi..];
    let at_legacy = after.find("complete_layer_drop_onto_folder(").expect(
        "T-946.86 (.83): the single-id completion must survive for drags that arm \
                only that latch",
    );
    assert!(
        after[..at_legacy].contains("if !claimed"),
        "T-946.86 (.83): the legacy completion must be GATED on the multi drop declining — \
         running both would apply the anchor's move twice"
    );
}

/// **All three drag arms build a SET, and both drops consume one.** The `drag`
/// versions of `begin_layer_slot_drag`, `begin_layer_comment_drag` and `begin_refile` shipped
/// in wave 255 shadowed by the engine's single-id `layer_drag` namesakes and were never called;
/// the slot lane is where multi-drag is actually REACHABLE, because slot ids are what the
/// canvas selection mirror publishes (folder ids never enter it).
#[test]
fn every_drag_arm_builds_a_set_and_every_drop_consumes_one() {
    let src = live();
    for arm in [
        "drag::begin_layer_drag(drag)",
        "drag::begin_layer_slot_drag(drag)",
        "drag::begin_layer_comment_drag(drag)",
        "drag::begin_refile(drag)",
    ] {
        assert!(
            src.contains(arm),
            "T-946.86 (.83): `{arm}` must be the armed form — the single-id namesake alone \
             moves the anchor and throws the rest of the selection away"
        );
    }
    assert!(
        src.contains("complete_multi_refile_onto_squad(&dest)"),
        "T-946.86 (.83): the ORBAT squad drop must consume the set too — arming a set that \
         nothing consumes is the exact defect this repairs"
    );
    assert_eq!(
        src.matches("drag_set_for(").count(),
        5,
        "T-946.86 (.83): the definition plus exactly FOUR arms — the folder row, the slot \
         row's two branches (ORBAT refile and layer refile) and the comment row. One shared \
         builder: a second walk is how the arms drift into disagreeing about what a drag \
         contains, which is how the anchor-only drop survived review in the first place"
    );
}

/// The set is the SELECTION only when the pressed row is part of it. Pressing an UNSELECTED
/// row and dragging must move that row alone — silently dragging a selection the operator did
/// not grab is a worse outcome than moving one row too few.
#[test]
fn an_unselected_anchor_drags_alone() {
    let tree = vec![node("a", vec![]), node("b", vec![]), node("c", vec![])];
    let sel = vec!["b".to_string(), "c".to_string()];
    let solo = drag_set_for("a", &sel, &tree);
    assert_eq!(
        solo.ids,
        vec!["a".to_string()],
        "unselected anchor drags alone"
    );
    assert_eq!(solo.anchor, "a");

    let group = drag_set_for("b", &sel, &tree);
    assert_eq!(
        group.ids,
        vec!["b".to_string(), "c".to_string()],
        "a selected anchor drags the whole selection"
    );
    assert_eq!(group.anchor, "b", "the anchor is still the row pressed");
}

/// The set is ordered by the TREE, not by the selection's arrival order: the drop applies the
/// ids in sequence, and render order is the only order an operator can predict.
#[test]
fn the_set_is_ordered_by_the_tree_not_the_selection() {
    let tree = vec![node("a", vec![node("b", vec![node("c", vec![])])])];
    // Selection deliberately in reverse render order.
    let sel = vec!["c".to_string(), "b".to_string(), "a".to_string()];
    assert_eq!(
        drag_set_for("c", &sel, &tree).ids,
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        "top-to-bottom, depth-first — the order the rows appear on screen"
    );
}

/// `node_descendant_ids` answers "what is under this row", at any depth, excluding the row
/// itself. This is the input `plan_drop` refuses a parent-into-own-child drop with.
#[test]
fn descendants_are_the_whole_subtree_and_never_the_node_itself() {
    let tree = vec![
        node(
            "a",
            vec![node("b", vec![node("c", vec![])]), node("d", vec![])],
        ),
        node("e", vec![]),
    ];
    let mut under_a = node_descendant_ids(&tree, "a");
    under_a.sort();
    assert_eq!(
        under_a,
        vec!["b", "c", "d"],
        "every depth, excluding `a` itself"
    );
    assert!(
        node_descendant_ids(&tree, "e").is_empty(),
        "a leaf has no descendants"
    );
    assert!(
        node_descendant_ids(&tree, "nope").is_empty(),
        "an unknown id answers empty — the core's cycle guard is the backstop"
    );
    assert_eq!(
        node_descendant_ids(&tree, "b"),
        vec!["c"],
        "the walk finds nested nodes, not only roots"
    );
}
