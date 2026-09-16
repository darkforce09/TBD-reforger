//! Role: the saved-composition library over the hosted document — capture a selection, and edit or
//! drop a saved row.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document, the selected ids and the id minter come from
//! the host.
//! Invariants: every mutator runs exactly one post-change tail, so an inline rename is one undo
//! step. A capture over an empty selection writes nothing and mints no id. Dropping a row is the
//! document's half only — an armed place that named the dropped row is the host's own state, and
//! the host clears it on a `true` return.

use crate::data::store::MissionDocCore;
use crate::data::store::operations::compositions;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, with_doc, with_host};

/// One saved composition as the palette's row needs it.
pub use crate::data::store::operations::compositions::CompositionRow;

/// Capture the current selection as a new saved composition. `None` when nothing is selected, when
/// the selection captured no placeable entry, or before a document exists.
#[must_use]
pub fn save_composition(title: String, category: String, author: String) -> Option<String> {
    let sel = selection_ids();
    if sel.is_empty() {
        return None;
    }
    let new_id = with_host(|host| {
        let doc = host.doc.borrow();
        let core = doc.as_ref()?;
        compositions::save_composition(core, title, category, author, sel, &host.next_id)
    })
    .flatten();
    if new_id.is_some() {
        after_local_edit();
    }
    new_id
}

/// Every saved composition, sorted by category then title — the palette's row order.
#[must_use]
pub fn composition_rows() -> Vec<CompositionRow> {
    with_doc(compositions::composition_rows).unwrap_or_default()
}

/// How many compositions are saved — the palette's tab badge.
#[must_use]
pub fn composition_count() -> usize {
    with_doc(MissionDocCore::composition_count).unwrap_or(0)
}

/// Rename a saved composition.
pub fn rename_composition(id: String, title: String) -> bool {
    edit_composition(|core| core.set_composition_title(&id, &title))
}

/// Move a saved composition into another category.
pub fn recategorize_composition(id: String, category: String) -> bool {
    edit_composition(|core| core.set_composition_category(&id, &category))
}

/// Re-attribute a saved composition.
pub fn set_composition_author(id: String, author: String) -> bool {
    edit_composition(|core| core.set_composition_author(&id, &author))
}

/// Drop a saved composition from the library. `true` once the document has taken the removal,
/// which is the host's cue to drop an armed place that named this row.
pub fn delete_composition(id: &str) -> bool {
    edit_composition(|core| core.remove_composition(id))
}

/// Shared edit tail for the composition mutators: run `f` against the document, then the tail, so
/// the edit is one undo step. `false` when there is no document.
fn edit_composition(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = with_doc(f).is_some();
    if did {
        after_local_edit();
    }
    did
}
