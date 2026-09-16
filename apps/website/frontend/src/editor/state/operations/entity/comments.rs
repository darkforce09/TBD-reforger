//! Role: comments.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Comment list using the supplied domain data.
#[must_use]
pub fn comment_list() -> Vec<CommentDetail> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        d.as_ref().map(comment_details).unwrap_or_default()
    })
}

/// Comment count using the supplied domain data.
#[must_use]
pub fn comment_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::comment_count))
            .unwrap_or(0)
    })
}

/// The default title/tooltip are placeholders an operator overwrites; they are non-empty so the new row is visible and clickable the instant it appears (the `SLOT_FALLBACK_LABEL` reasoning).
pub fn place_comment(x: f64, z: f64) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::place_comment(core, x, z, |core| {
            ensure_layer(ctx, core)
        })
    })?;
    mission_history::after_local_edit();
    Some(id)
}

/// Rename comment using the supplied domain data.
pub fn rename_comment(id: String, title: String) -> bool {
    edit_comment(|core| core.set_comment_title(&id, &title))
}

/// Set comment tooltip using the supplied domain data.
pub fn set_comment_tooltip(id: String, tooltip: String) -> bool {
    edit_comment(|core| core.set_comment_tooltip(&id, &tooltip))
}

/// Move comment using the supplied domain data.
pub fn move_comment(id: String, x: f64, z: f64) -> bool {
    edit_comment(|core| core.set_comment_position(&id, x, z))
}

/// The offset exists so the copy is not perfectly stacked on its source: two comments at identical coordinates are one indistinguishable row in the tree and one unclickable glyph on any future map render.
pub fn duplicate_comment(id: &str, offset: f64) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::duplicate_comment(
            core,
            id,
            offset,
            |core| ensure_layer(ctx, core),
        )
    })?;
    mission_history::after_local_edit();
    Some(new_id)
}

/// Delete comment using the supplied domain data.
pub fn delete_comment(id: String) -> bool {
    edit_comment(|core| core.remove_comment(&id))
}

/// Refile comment to layer using the supplied domain data.
pub fn refile_comment_to_layer(comment_id: &str, layer_id: &str) -> bool {
    edit_comment(|core| core.move_comment_to_layer(comment_id, layer_id))
}

/// Edit comment using the supplied domain data.
pub(in crate::editor::state::operations) fn edit_comment(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
