//! Role: the authored zones — the play areas and objective areas a mission declares.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: every attribute write is one post-change tail, so an edit is one Ctrl+Z. The
//! `zone.type` vocabulary is the HOST's closed schema list and crosses as a closure, because a
//! value outside that enum saves once and then fails every compile after. An EMPTY label and no
//! label are different authored states the schema allows on purpose: `Some("")` stores an empty
//! label, which the mod reads as "use the pretty title fallback", and `None` removes the key.

use crate::data::store::MissionDocCore;
use crate::data::store::operations::entity as entity_ops;
use crate::data::store::operations::zones::DrawTarget;
use crate::editing::history::after_local_edit;
use crate::editing::host::with_doc;

use super::document_edit::commit_document_edit;

/// One authored zone, as the dock's row needs it.
pub use crate::data::store::operations::entity::ZoneRow;

/// Every authored zone, in document order — the dock's list.
#[must_use]
pub fn zone_rows() -> Vec<ZoneRow> {
    with_doc(entity_ops::zone_rows)
        .flatten()
        .unwrap_or_default()
}

/// How many zones the document declares — backs "does this mission define a play area?" without
/// materialising a row.
#[must_use]
pub fn zone_count() -> usize {
    with_doc(MissionDocCore::zone_count).unwrap_or(0)
}

/// Set a zone's schema `type`. `kind_is_authorable` is the host's closed enum; a value outside it
/// is refused rather than stored.
pub fn set_zone_kind(id: &str, kind: &str, kind_is_authorable: impl Fn(&str) -> bool) -> bool {
    if !kind_is_authorable(kind) {
        return false;
    }
    commit_document_edit(|core| core.set_zone_type(id, kind))
}

/// Set or remove a zone's label.
pub fn set_zone_label(id: &str, label: Option<String>) -> bool {
    commit_document_edit(|core| core.set_zone_label(id, label.as_deref()))
}

/// Set or clear a zone's faction — a faction key slug. `None` makes the zone faction-neutral.
pub fn set_zone_faction(id: &str, faction: Option<String>) -> bool {
    commit_document_edit(|core| core.set_zone_faction(id, faction.as_deref()))
}

/// Set or clear ONE `rules` key, read-modify-write over the opaque rules object. The read happens
/// before the write opens, so the write commits the whole object the read produced rather than a
/// patch the document would have to merge.
pub fn set_zone_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let Some(next) = with_doc(|core| entity_ops::set_zone_rule(core, id, key, value)).flatten()
    else {
        return false;
    };
    commit_document_edit(|core| core.set_zone_rules(id, Some(&next)))
}

/// Delete a zone.
pub fn delete_zone(id: &str) -> bool {
    commit_document_edit(|core| core.remove_zone(id))
}

/// Mint one authored row in `collection` and let `write` fill it, then take the post-change tail.
/// Returns the new row's id, or `None` when no document is hosted.
///
/// The geometry crosses in already decided: a ring or a centre-and-radius is the product of a draw
/// gesture the host ran, and the id is the only thing the document has to contribute.
pub fn add_authored_row(
    collection: DrawTarget,
    write: impl FnOnce(&MissionDocCore, &str),
) -> Option<String> {
    let id = with_doc(|core| entity_ops::write_row_returning_id(core, collection, write)).flatten();
    if id.is_some() {
        after_local_edit();
    }
    id
}
