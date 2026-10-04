//! The folder a new entity is filed under when the outliner has one focused, and the gestures
//! that move that focus.
//!
//! **Role:** sets and clears the focused folder, resolves the folder a place files into (minting
//! the default folder under the local origin when none is live), and creates and deletes folders
//! with the focus following them.
//! **Position:** part of the bridge's host state, beside the editor context whose
//! `active_layer` signal it reads and writes. The outliner rows, the left dock, the context menu,
//! the ORBAT manager, the comment editor and the placement paths call it; it writes the document
//! only through the hosted commands.
//! **Signals & state:** the editor context's `active_layer` signal; nothing of its own.
//! **Invariants:** a focus pointing at a folder the document no longer holds is cleared as it is
//! resolved, and deleting the focused folder clears the focus.

use crate::bridge::host_state::editor_context::{EDITOR_CONTEXT, EditorContext};
use leptos::prelude::{GetUntracked, Set};
use mission_creator_state::outliner_model::{DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME};
use mission_document::MissionDocCore;
use mission_document::ids::LayerId;
use mission_editing_commands::hosted_commands as engine_ops;

/// Focus a folder, or clear the focus. A focused folder is the drop target for the next place.
pub fn set_active_layer(id: Option<LayerId>) {
    EDITOR_CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.active_layer.set(id.map(LayerId::into_inner));
        }
    });
}

/// Resolve the folder a new entity is filed under against one context: the focused folder when
/// one is set and still live, otherwise the default folder, minted under the LOCAL origin so
/// the mint is part of the same undoable act as the place it serves. A focus pointing at a
/// folder the document no longer holds is cleared as it is resolved.
fn ensure_layer(ctx: &EditorContext, core: &MissionDocCore) -> String {
    let ensured = mission_operations::entity::ensure_layer(
        core,
        ctx.active_layer.get_untracked(),
        DEFAULT_LAYER_ID,
        DEFAULT_LAYER_NAME,
    );
    if ensured.active_layer_was_stale {
        ctx.active_layer.set(None);
    }
    ensured.layer_id.into_inner()
}

/// Resolve the folder a new entity is filed under without a context in hand. This is the form
/// the engine's hosted commands take, which is why the folder id crosses the wall as an answer
/// rather than the engine reaching for the tree's focus itself.
pub fn ensure_active_layer(core: &MissionDocCore) -> String {
    EDITOR_CONTEXT
        .with(|c| c.borrow().as_ref().map(|ctx| ensure_layer(ctx, core)))
        .unwrap_or_else(|| DEFAULT_LAYER_ID.to_string())
}

/// Create a folder as a child of the focused folder (a root when none is focused), auto-named,
/// with its inline rename armed, and focus it. Returns the new folder's id.
pub fn create_layer() -> Option<String> {
    let active = EDITOR_CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.active_layer.get_untracked())
    });
    let created = engine_ops::create_layer(active)?;
    set_active_layer(Some(LayerId::from(created.as_str())));
    Some(created)
}

/// Delete a folder and its whole subtree, dropping the focus when it was the focused folder —
/// a focus on a folder that no longer exists would file the next place into nothing.
pub fn delete_layer(id: &mission_document::ids::LayerId) -> bool {
    let did = engine_ops::delete_layer(id.clone());
    if did {
        let focused = EDITOR_CONTEXT.with(|c| {
            c.borrow()
                .as_ref()
                .and_then(|ctx| ctx.active_layer.get_untracked())
        });
        if focused.as_deref() == Some(id.as_str()) {
            set_active_layer(None);
        }
    }
    did
}
