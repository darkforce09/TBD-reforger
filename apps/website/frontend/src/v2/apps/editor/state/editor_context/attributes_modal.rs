//! Role: opening and closing the Attributes modal, and the selection change opening it implies.
//! Position: `editor/state/editor_context` in the frontend editor shell.
//! Signals & state: the installed context's open-target and tab-index signals, and the selection
//! whose renderer tint the open rebinds.
//! Invariants: opening over a multi-selection that already contains the target PRESERVES that
//! selection, so the modal edits every selected entity; opening over anything else collapses the
//! selection to the one target. Opening the Arsenal is the same act with the tab forced.

use super::*;

/// Open attrs modal using the supplied domain data. A multi-selection now OPENS the modal while preserving its selected targets.
pub(in crate::v2::apps::editor::state::editor_context) fn open_attrs_modal(
    id: String,
    arsenal_tab: bool,
) {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let keep_selection = {
            let sel = ctx.selection.borrow();
            sel.len() > 1 && sel.contains(&id)
        };
        if !keep_selection {
            *ctx.selection.borrow_mut() = vec![id.clone()];
            let ids = ctx.selection.borrow().clone();
            let mut eng = ctx.engine.borrow_mut();
            if let Some(e) = eng.as_mut() {
                e.set_selection(ids);
            }
        }
        if arsenal_tab {
            ctx.attrs_tab.set(3);
        }
        ctx.attrs_open.set(Some(id));
    });
    mission_history::refresh_selection();
}

/// Open Attributes for `id` (the dbl-click / outliner-activate contract). A multi-selection opens the modal in MULTI-EDIT mode over the whole selection — see [`open_attrs_modal`] for the inversion of the old suppress-on-multi rule. Leaves the Attributes tab index alone (default Identity until the user changes it).
pub fn open_attributes(id: String) {
    open_attrs_modal(id, false);
}

/// Open arsenal using the supplied domain data.
pub fn open_arsenal(id: String) {
    open_attrs_modal(id, true);
}

/// Close the modal (Esc / backdrop / close button).
pub fn close_attributes() {
    EDITOR_CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.attrs_open.set(None);
        }
    });
}
