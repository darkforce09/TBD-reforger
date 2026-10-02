//! Role: the searchable index of everything the hosted document places.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document comes from the host.
//! Invariants: one read of one index, so the search box and the filter chips can never disagree
//! about what an entity's kind or faction is. No document hosted means an empty index, never a
//! partial one.

use crate::data::store::operations::document_index;
use crate::data::store::operations::entity as entity_ops;
use crate::editing::host::{selection_ids, with_doc};

/// One searchable entity, as the search box's row needs it.
pub use crate::data::store::operations::document_index::DocEntity;

/// Every entity the document places, for the Outliner's search.
#[must_use]
pub fn document_entities() -> Vec<DocEntity> {
    with_doc(document_index::document_entities).unwrap_or_default()
}

/// The selected entities, as rows of the same index. Derived from the one index rather than
/// re-read per kind, so the selection filter's chips and the search's rows can never disagree about
/// what an entity's type or faction is. Selection ORDER is not preserved — the chips are counts and
/// id sets, and nothing downstream reads a selection as a sequence.
#[must_use]
pub fn selection_entities() -> Vec<DocEntity> {
    let sel = selection_ids();
    with_doc(|core| entity_ops::selection_entities(core, &sel)).unwrap_or_default()
}
