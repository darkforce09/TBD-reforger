//! Role: the comments an author pins on the map — place, edit, duplicate, refile and delete.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: a comment is not a slot — it never reaches `materialize`, carries no SoA row, and
//! is therefore read here rather than through any slot reader. Every mutator runs exactly one
//! post-change tail. The folder a new comment is filed under is the HOST's answer and crosses as a
//! closure, because which folder is active is host state.

use crate::data::store::MissionDocCore;
use crate::data::store::operations::entity as entity_ops;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

use super::document_edit::commit_document_edit;

/// One comment with everything a surface renders it from.
pub use crate::data::store::operations::entity::CommentDetail;

/// Every comment the document carries.
#[must_use]
pub fn comment_list() -> Vec<CommentDetail> {
    with_doc(entity_ops::comment_details).unwrap_or_default()
}

/// How many comments the document carries — the dock's readout, without materialising a row.
#[must_use]
pub fn comment_count() -> usize {
    with_doc(MissionDocCore::comment_count).unwrap_or(0)
}

/// One comment by id, or `None` when the document no longer carries it.
#[must_use]
pub fn read_comment(id: &str) -> Option<CommentDetail> {
    comment_list().into_iter().find(|c| c.id == id)
}

/// Place a new comment at `(x, z)` world metres. Its title and tooltip start as placeholders an
/// author overwrites; they are non-empty so the new row is visible and clickable the instant it
/// appears, rather than a blank line nobody can hit.
pub fn place_comment(
    x: f64,
    z: f64,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let id = with_doc(|core| entity_ops::place_comment(core, x, z, ensure_layer)).flatten()?;
    after_local_edit();
    Some(id)
}

/// Retitle a comment.
pub fn rename_comment(id: String, title: String) -> bool {
    commit_document_edit(|core| core.set_comment_title(&id, &title))
}

/// Rewrite a comment's tooltip.
pub fn set_comment_tooltip(id: String, tooltip: String) -> bool {
    commit_document_edit(|core| core.set_comment_tooltip(&id, &tooltip))
}

/// Move a comment to `(x, z)` world metres.
pub fn move_comment(id: String, x: f64, z: f64) -> bool {
    commit_document_edit(|core| core.set_comment_position(&id, x, z))
}

/// Copy a comment, offset from its source. The offset exists so the copy is not perfectly stacked:
/// two comments at identical coordinates are one indistinguishable row in the tree and one
/// unclickable glyph on the map.
pub fn duplicate_comment(
    id: &str,
    offset: f64,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let new_id =
        with_doc(|core| entity_ops::duplicate_comment(core, id, offset, ensure_layer)).flatten()?;
    after_local_edit();
    Some(new_id)
}

/// Delete a comment.
pub fn delete_comment(id: String) -> bool {
    commit_document_edit(|core| core.remove_comment(&id))
}

/// File a comment under another folder.
pub fn refile_comment_to_layer(comment_id: &str, layer_id: &str) -> bool {
    commit_document_edit(|core| core.move_comment_to_layer(comment_id, layer_id))
}
