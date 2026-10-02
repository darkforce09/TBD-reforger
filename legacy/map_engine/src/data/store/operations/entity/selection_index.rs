//! Role: the selected entities, read off the one document index the search also reads.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: derived from the document index rather than re-read per collection, so a filter
//! chip and a search row can never disagree about an entity's kind or faction. Document order is
//! preserved, not selection order: the result is a set readers count and filter, never a sequence.

use super::MissionDocCore;
use super::{DocEntity, document_entities};

/// The indexed entities for `selection`, in document order. Empty when nothing is selected.
pub fn selection_entities(core: &MissionDocCore, selection: &[String]) -> Vec<DocEntity> {
    if selection.is_empty() {
        return Vec::new();
    }
    document_entities(core)
        .into_iter()
        .filter(|entity| selection.iter().any(|id| id == &entity.id))
        .collect()
}

#[cfg(test)]
#[path = "tests/selection_index.rs"]
mod tests;
