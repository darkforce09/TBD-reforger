//! The outliner's drag set: which rows a drag carries, in tree order, with whole subtrees.

use super::{drag_set_for, node_descendant_ids};
use mission_creator_state::ids::OutlinerNodeId;
use mission_creator_state::outliner_model::{NodeKind, OutlinerNode};

fn node(id: &str, children: Vec<OutlinerNode>) -> OutlinerNode {
    OutlinerNode {
        id: OutlinerNodeId::new(id),
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
