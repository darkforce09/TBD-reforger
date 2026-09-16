//! Role: pushing the open document into the signals the docks read — the folder tree, the ORBAT
//! tree, the selected-id list and the reactive document tick — plus the Connections panel's own
//! signals and the comment seed a brand-new mission starts with.
//! Position: `editor/state/editor_context` in the frontend editor shell.
//! Signals & state: the installed context's dock mirrors, and the Connections panel and connection
//! selection signals registered above.
//! Invariants: the document has no change subscription, so the mirrors are pushed from the shared
//! post-edit refresh at every mutation site — place, drag, undo, redo, click-select, the restore
//! swap — and a dock can therefore never show a slot set the document no longer holds. Reads of the
//! document are scoped so the borrow drops before anything reactive runs.

use super::*;

/// Rebuild the dock mirrors from the live doc + selection. Called from `mission_history::refresh_signals`, i.e. from **every** mutation site (place, drag-move, undo, redo, click-select, the IDB restore swap) — so the tree can never show a stale slot set.
pub fn refresh_docks() {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let (nodes, orbat) = {
            let d = ctx.doc.borrow();
            match d.as_ref() {
                Some(core) => {
                    let slots = slot_rows(core);
                    (
                        build_outliner_with_comments(
                            &layer_rows(core),
                            &slots,
                            &comment_rows(core),
                        ),
                        crate::v2::apps::editor::panels::outliner::build_orbat(
                            &faction_rows(core),
                            &squad_rows(core),
                            &slots,
                        ),
                    )
                }
                None => (Vec::new(), Vec::new()),
            }
        };
        ctx.outliner_nodes.set(nodes);
        ctx.orbat_nodes.set(orbat);
        mirror_selection(ctx);
        ctx.doc_tick
            .set(ctx.doc_tick.get_untracked().wrapping_add(1));
    });
}

/// Mirror selection using the supplied domain data.
pub(in crate::v2::apps::editor::state::editor_context) fn mirror_selection(ctx: &EditorContext) {
    reconcile_connection_selection(ctx);
    ctx.selected_ids.set(ctx.selection.borrow().clone());
}

/// Refresh selection mirrors using the supplied domain data.
pub fn refresh_selection_mirrors() {
    EDITOR_CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            mirror_selection(ctx);
            if let Some(open_id) = ctx.attrs_open.get_untracked() {
                if ctx.selection.borrow().iter().any(|s| *s == open_id) {
                    ctx.attrs_open.set(Some(open_id));
                } else {
                    ctx.attrs_open.set(None);
                }
            }
        }
    });
}

/// Called once from `mission_editor`'s boot, BEFORE the IndexedDB restore and the server hydrate — both of which replace the document wholesale, which is exactly right: a restored or downloaded mission is not a new mission and gets whatever comments it was saved with.
pub fn seed_new_mission_template(doc: &DocHandle) -> usize {
    let borrowed = doc.borrow();
    let Some(core) = borrowed.as_ref() else {
        return 0;
    };
    core.set_origin_init(true);
    let ids = core.seed_template_comments();
    core.set_origin_init(false);
    ids.len()
}

/// Set connections panel signal using the supplied domain data.
pub fn set_connections_panel_signal(sig: RwSignal<bool>) {
    CONNECTIONS_PANEL.with(|s| *s.borrow_mut() = Some(sig));
}

/// Open connections panel using the supplied domain data.
pub fn open_connections_panel() {
    CONNECTIONS_PANEL.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(true);
        }
    });
}

/// Close connections panel using the supplied domain data.
pub fn close_connections_panel() {
    CONNECTIONS_PANEL.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(false);
        }
    });
}

/// Set connection selection signal using the supplied domain data.
pub fn set_connection_selection_signal(sig: RwSignal<Option<String>>) {
    CONNECTION_SELECTION.with(|s| *s.borrow_mut() = Some(sig));
}

/// Reconcile connection selection using the supplied domain data.
pub(in crate::v2::apps::editor::state::editor_context) fn reconcile_connection_selection(
    ctx: &EditorContext,
) {
    CONNECTION_SELECTION.with(|s| {
        let Some(sig) = *s.borrow() else {
            return;
        };
        let Some(Some(id)) = sig.try_get_untracked() else {
            return;
        };

        let entity_selected = !ctx.selection.borrow().is_empty();
        let still_there = {
            let d = ctx.doc.borrow();
            d.as_ref()
                .is_some_and(|core| connection_id_in_doc(core, &id))
        };
        if entity_selected || !still_there {
            sig.set(None);
        }
    });
}

/// Nudge the reactive doc tick so the Zones panel re-reads mid-draw. Cheaper and safer than `after_local_edit`, which schedules a persist for a document that has not changed yet.
pub(crate) fn bump_doc_tick() {
    EDITOR_CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let n = ctx.doc_tick.get_untracked();
            ctx.doc_tick.set(n.wrapping_add(1));
        }
    });
}
