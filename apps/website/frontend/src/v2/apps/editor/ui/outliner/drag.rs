//! The Outliner's pointer-drag latch and drop planner.
//!
//! Production items precede test declarations so source-inspection tests see the full module.

/// What one Outliner drag carries: the row the pointer went down on, and every id the drop will
/// move, in render order.
///
/// `ids` always contains `anchor`. Pressing a row that is part of the selection drags the whole
/// selection; pressing one that is not drags that row alone, so an unselected row can never take
/// the selection with it.
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

/// A drag that is armed and waiting for a drop, tagged with the kind of row that armed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerDrag {
    Folder(DragSet),
    Slot(DragSet),
    Comment(DragSet),
}

thread_local! {
    /// The one armed Outliner drag, or `None` when no drag is in flight.
    ///
    /// Thread-local rather than a signal because it is latched and consumed inside pointer
    /// handlers, never rendered: arming it must not schedule a reactive pass over the tree.
    pub static PENDING_DRAG: std::cell::RefCell<Option<LayerDrag>> = const { std::cell::RefCell::new(None) };
}

/// Arm `drag_set` as a folder drag, replacing whatever was armed before.
pub fn begin_layer_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(drag_set)));
}

/// Arm `drag_set` as a slot drag, replacing whatever was armed before.
pub fn begin_layer_slot_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

/// Arm `drag_set` as a comment drag, replacing whatever was armed before.
pub fn begin_layer_comment_drag(drag_set: DragSet) {
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(drag_set)));
}

/// Arm `drag_set` as a refile — dragging rows out of one folder and into another.
///
/// A refile moves the same rows a slot drag does and is latched as one, so the drop path has a
/// single kind to consume rather than two that behave identically.
pub fn begin_refile(drag_set: DragSet) {
    // We can also store pending refile here
    PENDING_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(drag_set)));
}

/// Disarm every latch a row may have armed, so no later click can find a stale drag.
///
/// An Outliner row arms this module's latch alongside the engine's own layer-drag and refile
/// latches, and a cancellation that cleared only one of them would leave a drop armed that the
/// operator has already abandoned.
pub fn cancel_layer_drag() {
    PENDING_DRAG.with(|p| *p.borrow_mut() = None);
    // Rows arm the legacy latch alongside the set (including ORBAT's separate refile latch).
    // Every completion/cancellation must consume all of them before a later click can see one.
    #[cfg(target_arch = "wasm32")]
    {
        use website_map_engine::editing::hosted_commands as engine_ops;
        engine_ops::cancel_layer_drag();
        engine_ops::cancel_refile();
    }
}

/// Consumes the pending [`DragSet`] onto `dest_folder_id`, moving every id in one undo group.
/// Returns whether a set was armed, including when a cyclic drop is refused.
///
/// ## Why it takes the descendants as a closure
///
/// [`plan_drop`] is pure and stays pure: the caller supplies "what is under this folder" from the
/// row tree it already renders. That keeps the parent-into-own-child refusal testable natively
/// without a document, and keeps this module free of the `OutlinerNode` type.
///
/// ## Undo
///
/// The whole drop is one `with_batch` group, so one undo restores every row.
///
/// A refused plan still consumes the drag and returns `true`, preventing the single-id path
/// from moving the anchor into its own subtree.
#[cfg(target_arch = "wasm32")]
pub fn complete_multi_drop_onto_folder(
    dest_folder_id: &str,
    folder_descendants: impl Fn(&str) -> Vec<String>,
) -> bool {
    use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures;
    use website_map_engine::editing::hosted_commands as engine_ops;

    let Some(drag) = PENDING_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    // The SAME pointerdown also armed the engine's single-id `layer_drag` latch. Drop it here or
    // it strands and is consumed by some later, unrelated drop — a move the operator never made.
    cancel_layer_drag();

    let set = match &drag {
        LayerDrag::Folder(s) | LayerDrag::Slot(s) | LayerDrag::Comment(s) => s,
    };
    let Some(ids) = plan_drop(set, dest_folder_id, folder_descendants) else {
        return true; // armed and refused — consumed, nothing moved
    };
    if ids.is_empty() {
        return true;
    }
    undo_grouped_gestures::with_batch("outliner-multi-drop", || {
        for id in &ids {
            match &drag {
                // A folder REPARENTS under the destination (the core cycle-guards a self/subtree
                // drop as well, so `plan_drop`'s refusal is the affordance, not the only guard).
                LayerDrag::Folder(_) => {
                    if id != dest_folder_id {
                        engine_ops::reparent_layer(id, Some(dest_folder_id.to_string()));
                    }
                }
                // A slot / comment REFILES into it — same latch, different mutator.
                LayerDrag::Slot(_) => {
                    engine_ops::refile_slot_to_layer(id, dest_folder_id);
                }
                LayerDrag::Comment(_) => {
                    engine_ops::refile_comment_to_layer(id, dest_folder_id);
                }
            };
        }
    });
    true
}

/// Consumes the pending set onto `dest_squad_id` in the ORBAT tree.
/// Returns whether a set was armed and groups all slot moves into one undo operation.
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
    use crate::v2::apps::editor::bridge::host_state::undo_grouped_gestures;
    use website_map_engine::editing::hosted_commands as engine_ops;

    let Some(drag) = PENDING_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    // Drop the single-id latch the same pointerdown armed, so it cannot strand into a later drop.
    cancel_layer_drag();

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
    undo_grouped_gestures::with_batch("orbat-multi-refile", || {
        for id in &ids {
            engine_ops::refile_slot(id.clone(), dest_squad_id.to_string());
        }
    });
    true
}

#[cfg(test)]
#[path = "tests/drag/drag_set_drop_planning.rs"]
mod drag_set_drop_planning;
