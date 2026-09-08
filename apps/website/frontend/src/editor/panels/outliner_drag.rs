//! The Outliner's pointer-drag latch and its drop PLANNER.
//!
//! T-946.86 (.83) — ORDER MATTERS IN THIS FILE. Every production item is above the single
//! `#[cfg(test)]` module at the bottom, because `class_r_scrub::live_source` cuts from the FIRST
//! `#[cfg(test)]` to EOF: when the test module sat in the MIDDLE (as it did when this file
//! shipped in wave 255), `LayerDrag`, `PENDING_DRAG` and every `begin_*` below it were invisible
//! to source-scrubbing pins, which could then only ever examine the planner. Do not move the test
//! module back up, and do not add production items after it.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragSet {
    pub anchor: String,
    pub ids: Vec<String>,
}

/// Pure drop planning function that rejects dropping a parent/container into any of its dragged children.
/// Returns the list of ids to drop, or None if the drop is invalid.
pub fn plan_drop(
    drag: &DragSet,
    dest: &str,
    folder_descendants: impl Fn(&str) -> Vec<String>,
) -> Option<Vec<String>> {
    for id in &drag.ids {
        if id == dest {
            return None;
        }
        let descendants = folder_descendants(id);
        if descendants.contains(&dest.to_string()) {
            return None;
        }
    }
    Some(drag.ids.clone())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerDrag {
    Folder(DragSet),
    Slot(DragSet),
    Comment(DragSet),
}

thread_local! {
    pub static PENDING_DRAG: std::cell::RefCell<Option<LayerDrag>> = const { std::cell::RefCell::new(None) };
}

pub fn begin_layer_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(drag_set)));
}

pub fn begin_layer_slot_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

pub fn begin_layer_comment_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(drag_set)));
}

pub fn begin_refile(drag_set: DragSet) {
    // We can also store pending refile here
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

pub fn cancel_layer_drag() {
    PENDING_DRAG.with(|p| *p.borrow_mut() = None);
}

/// T-946.86 (.83) — **CONSUME** the pending [`DragSet`] onto `dest_folder_id`, moving EVERY id in
/// one undo group. Returns whether a set was armed (i.e. whether this call owns the drop).
///
/// ## The defect this closes
///
/// The folder row armed TWO latches on one `pointerdown`: this module's [`PENDING_DRAG`], which
/// holds the whole multi-selection, and `state/operations`' single-id `PENDING_LAYER_DRAG`. The
/// `pointerup` then completed through `complete_layer_drop_onto_folder`, which reads the SINGLE-id
/// store — so a five-row drag moved one row, the anchor, and the other four silently stayed put.
/// [`plan_drop`] — the planner written for exactly this — had zero production callers, and
/// `PENDING_DRAG` was read only for `.is_some()` (the drag ghost) and cleared, never consumed.
///
/// ## Why it takes the descendants as a closure
///
/// [`plan_drop`] is pure and stays pure: the caller supplies "what is under this folder" from the
/// row tree it already renders. That keeps the parent-into-own-child refusal testable natively
/// without a document, and keeps this module free of the `OutlinerNode` type.
///
/// ## Undo
///
/// The whole drop is one `with_batch` group ⇒ ONE Ctrl+Z restores all N rows. Applying N separate
/// mutators without the group is what would make an operator press Ctrl+Z five times, and the
/// fourth press would look like it did nothing.
///
/// A refused plan (dropping a folder into its own subtree) still returns `true`: the drag WAS
/// armed and is now consumed. Returning `false` there would hand the drop to the legacy single-id
/// path, which does not know the set and would move the anchor — the exact defect, restored.
#[cfg(target_arch = "wasm32")]
pub fn complete_multi_drop_onto_folder(
    dest_folder_id: &str,
    folder_descendants: impl Fn(&str) -> Vec<String>,
) -> bool {
    use crate::editor::state::operations as ops;

    let Some(drag) = PENDING_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    // The SAME pointerdown also armed the single-id latch in `state/operations`. Drop it here or
    // it strands and is consumed by some later, unrelated drop — a move the operator never made.
    ops::cancel_layer_drag();

    let set = match &drag {
        LayerDrag::Folder(s) | LayerDrag::Slot(s) | LayerDrag::Comment(s) => s,
    };
    let Some(ids) = plan_drop(set, dest_folder_id, folder_descendants) else {
        return true; // armed and refused — consumed, nothing moved
    };
    if ids.is_empty() {
        return true;
    }
    ops::with_batch("outliner-multi-drop", || {
        for id in &ids {
            match &drag {
                // A folder REPARENTS under the destination (the core cycle-guards a self/subtree
                // drop as well, so `plan_drop`'s refusal is the affordance, not the only guard).
                LayerDrag::Folder(_) => {
                    if id != dest_folder_id {
                        ops::reparent_layer(id, Some(dest_folder_id.to_string()));
                    }
                }
                // A slot / comment REFILES into it — same latch, different mutator (T-651).
                LayerDrag::Slot(_) => {
                    ops::refile_slot_to_layer(id, dest_folder_id);
                }
                LayerDrag::Comment(_) => {
                    ops::refile_comment_to_layer(id, dest_folder_id);
                }
            };
        }
    });
    true
}

/// T-946.86 (.83) — **CONSUME** the pending set onto `dest_squad_id` (the ORBAT tree's squad-row
/// drop). Peer of [`complete_multi_drop_onto_folder`]; returns whether a set was armed.
///
/// The ORBAT lane had the same shape of defect as the layer lane and needed the same repair: the
/// slot row armed the single-id latch, so dragging a five-slot selection onto a squad refiled one.
/// [`begin_refile`] here — the `DragSet` version — was among the functions this file shipped with
/// no caller at all, shadowed by the `state/operations` single-id namesake.
///
/// `refile_slot` is the ORBAT mutator (slot → SQUAD), NOT `refile_slot_to_layer` (slot → folder):
/// they are different destinations and the squad row is the wrong drop for a layer move. Every id
/// rides one `with_batch` group, so a five-slot refile is one Ctrl+Z.
///
/// No `plan_drop` here, deliberately: that planner refuses a container dropped into its own
/// subtree, and a squad is not an ancestor of the slots it holds — there is no cycle to guard. The
/// only self-drop case (a slot already in the destination squad) is the core's own no-op.
#[cfg(target_arch = "wasm32")]
pub fn complete_multi_refile_onto_squad(dest_squad_id: &str) -> bool {
    use crate::editor::state::operations as ops;

    let Some(drag) = PENDING_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    // Drop the single-id latch the same pointerdown armed, so it cannot strand into a later drop.
    ops::cancel_layer_drag();

    let set = match &drag {
        LayerDrag::Folder(s) | LayerDrag::Slot(s) | LayerDrag::Comment(s) => s,
    };
    // A FOLDER or a COMMENT has no squad membership to change — the ORBAT tree does not render
    // them, so this can only be reached by a slot drag; anything else is consumed and ignored
    // rather than handed to a mutator that would not know what to do with it.
    if !matches!(drag, LayerDrag::Slot(_)) || set.ids.is_empty() {
        return true;
    }
    let ids = set.ids.clone();
    ops::with_batch("orbat-multi-refile", || {
        for id in &ids {
            ops::refile_slot(id.clone(), dest_squad_id.to_string());
        }
    });
    true
}

// ── TESTS LAST. See the module note: `live_source` cuts from the first `#[cfg(test)]` to EOF, so
//    anything below this line is invisible to every source-scrubbing pin in the crate.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_drop() {
        let drag = DragSet {
            anchor: "a".to_string(),
            ids: vec!["a".to_string(), "b".to_string()],
        };
        let descendants = |id: &str| -> Vec<String> {
            if id == "a" {
                vec!["c".to_string()]
            } else {
                vec![]
            }
        };
        // Drop into valid
        assert_eq!(
            plan_drop(&drag, "valid", descendants),
            Some(vec!["a".to_string(), "b".to_string()])
        );

        // Drop into self
        assert_eq!(plan_drop(&drag, "a", descendants), None);

        // Drop into child
        assert_eq!(plan_drop(&drag, "c", descendants), None);
    }

    /// T-946.86 (.83) — the WHOLE set survives the plan, not just the anchor.
    ///
    /// This is the property the defect violated: the planner always returned every id, and the
    /// drop path then threw all but `anchor` away by reading a different latch. Pinning
    /// "len == ids.len()" here means a future edit that quietly narrows the plan to the anchor
    /// fails rather than reproducing wave 255 silently.
    #[test]
    fn plan_drop_returns_every_dragged_id_not_just_the_anchor() {
        let ids: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let drag = DragSet {
            anchor: "a".to_string(),
            ids: ids.clone(),
        };
        let planned = plan_drop(&drag, "dest", |_| Vec::new()).expect("a clean drop is allowed");
        assert_eq!(
            planned, ids,
            "every dragged row must reach the drop, in order — moving only `anchor` is the \
             T-946.83 defect"
        );
        assert!(
            planned.len() > 1,
            "PERTURB: a plan that yields one id is the anchor-only drop this pin exists to catch"
        );
    }

    /// The refusal is per-SET, not per-anchor: a non-anchor member holding the destination in its
    /// subtree sinks the whole drop. Otherwise a five-row drag could reparent a folder under its
    /// own child as long as the ANCHOR was innocent.
    #[test]
    fn a_non_anchor_member_can_refuse_the_whole_drop() {
        let drag = DragSet {
            anchor: "a".to_string(),
            ids: vec!["a".to_string(), "parent".to_string()],
        };
        let descendants = |id: &str| -> Vec<String> {
            if id == "parent" {
                vec!["dest".to_string()]
            } else {
                vec![]
            }
        };
        assert_eq!(
            plan_drop(&drag, "dest", descendants),
            None,
            "a member whose subtree contains the destination must refuse the drop even when the \
             anchor is clean"
        );
    }
}
