//! Selection for editor outliner trees.

#[cfg(any(test, target_arch = "wasm32"))]
use super::*;

/// every node id BENEATH `id` in the built `OutlinerNode` tree (folders, slots
/// and comments alike), at any depth. `id` itself is not included.
///
/// This is what [`crate::ui::outliner::drag::plan_drop`] asks for: "would this drop put
/// a dragged row inside its own subtree?". It reads the RENDERED tree rather than `LayerRow`s
/// because that is what the drop handler already holds (`RowAuthoring::nodes`), and because a
/// multi-select drag can carry slot and comment rows, which the `parentId` chain in
/// [`layer_descendant_slots`] does not model as containers.
///
/// Unknown `id` → empty, which reads as "nothing is under it", so the planner allows the drop and
/// the core's own cycle guard remains the backstop. Depth-first, cycle-guarded by construction:
/// `OutlinerNode` is a tree by ownership, so no `seen` set is needed here.
#[must_use]
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn node_descendant_ids(nodes: &[OutlinerNode], id: &str) -> Vec<String> {
    fn collect(nodes: &[OutlinerNode], out: &mut Vec<String>) {
        for n in nodes {
            out.push(n.id.to_string());
            collect(&n.children, out);
        }
    }
    fn find<'a>(nodes: &'a [OutlinerNode], id: &str) -> Option<&'a OutlinerNode> {
        for n in nodes {
            if n.id == id {
                return Some(n);
            }
            if let Some(hit) = find(&n.children, id) {
                return Some(hit);
            }
        }
        None
    }
    let mut out = Vec::new();
    if let Some(node) = find(nodes, id) {
        collect(&node.children, &mut out);
    }
    out
}

/// build the [`crate::ui::outliner::drag::DragSet`] a row's
/// `pointerdown` arms: `anchor` plus, when the anchor is itself part of the current selection,
/// every OTHER selected row, in the tree's own top-to-bottom order.
///
/// The order is the render order rather than the selection's arrival order because the drop
/// applies the ids in sequence, and "the rows moved in the order you see them" is the only order an
/// operator can predict. When the anchor is NOT selected the drag is a single row — pressing an
/// unselected row and dragging it must not silently take the selection with it.
///
/// Folder, slot, and comment rows share this traversal so they arm the same ordered drag set.
#[must_use]
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn drag_set_for(
    anchor: &str,
    selection: &[String],
    nodes: &[OutlinerNode],
) -> crate::ui::outliner::drag::DragSet {
    let mut ids = vec![anchor.to_string()];
    if selection.iter().any(|s| s == anchor) {
        let sel_set: std::collections::HashSet<&str> =
            selection.iter().map(String::as_str).collect();
        let mut ordered = Vec::new();
        fn walk(
            ns: &[OutlinerNode],
            sel_set: &std::collections::HashSet<&str>,
            out: &mut Vec<String>,
        ) {
            for n in ns {
                if sel_set.contains(n.id.as_str()) {
                    out.push(n.id.to_string());
                }
                walk(&n.children, sel_set, out);
            }
        }
        walk(nodes, &sel_set, &mut ordered);
        if !ordered.is_empty() {
            ids = ordered;
        }
    }
    crate::ui::outliner::drag::DragSet {
        anchor: anchor.to_string(),
        ids,
    }
}

/// SEL-GROUP-ICON-001 — does this folder DIRECTLY contain any slots (vs only sub-folders)?
/// Drives the distinct folder glyph: a folder holding slots reads differently from a pure
/// grouping folder. "Directly" = its own `entityIds` is non-empty (a folder whose only content is
/// sub-folders that hold slots is still a grouping folder at this level).
#[cfg(test)]
#[must_use]
pub(crate) fn folder_holds_slots(layers: &[LayerRow], id: &str) -> bool {
    layers
        .iter()
        .find(|l| l.id == id)
        .is_some_and(|l| !l.entity_ids.is_empty())
}

/// SEL-GROUP-ICON-001 (render side) — collect the ids of every Folder node that DIRECTLY holds at
/// least one Slot child, walking the built `OutlinerNode` tree. The windowed renderer draws from a
/// flat `FlatRow` slice with no per-row "holds slots" bit and `FlatRow` lives in `outliner.rs`
/// (not owned here), so the distinction is precomputed from the tree the dock already has and
/// looked up by id in [`single_row`]. Same "direct children only" rule as [`folder_holds_slots`]:
/// a slot filed straight in this folder counts; one filed in a sub-folder does not.
#[must_use]
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) fn folders_holding_slots(nodes: &[OutlinerNode]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    fn walk(nodes: &[OutlinerNode], out: &mut std::collections::HashSet<String>) {
        for n in nodes {
            if n.kind == NodeKind::Folder && n.children.iter().any(|c| c.kind == NodeKind::Slot) {
                out.insert(n.id.to_string());
            }
            walk(&n.children, out);
        }
    }
    walk(nodes, &mut out);
    out
}
