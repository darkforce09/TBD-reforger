//! `WorkspaceState::reload`: a new corpus under the same session.
//!
//! **Role:** rebuilds the board from a fresh `LoadBundle` while carrying the filters, the sorts and
//! the selected and compared tickets over by id.
//! **Position:** part of `crate::application_state`; called by the desktop application on every
//! watch reload.
//! **Signals & state:** replaces the `WorkspaceState` it is called on.
//! **Invariants:** raw corpus indices are resolved again by ticket id, never reused; the quarantine
//! expansion survives only while a ticket stays selected.

use super::*;
impl WorkspaceState {
    /// Rebuild projections while resolving selections by identity against the new corpus.
    pub fn reload(bundle: LoadBundle, previous: Option<&Self>) -> Result<Self, LoadError> {
        let corpus = bundle.corpus?;
        let (carried, selected_id, compare_id, quarantine_expanded) = match previous {
            Some(board) => (
                ReloadPreferences {
                    filters: board.filters.clone(),
                    metrics_sort: board.metrics_sort,
                    est_sort: board.est_sort,
                },
                board
                    .selected
                    .map(|index| board.corpus.tickets[index].ticket.id().to_owned()),
                board
                    .compare
                    .map(|index| board.corpus.tickets[index].ticket.id().to_owned()),
                board.quarantine_expanded,
            ),
            None => (ReloadPreferences::default(), None, None, false),
        };
        let mut board = Self::new(
            corpus,
            bundle.lock,
            bundle.metrics,
            bundle.estimates,
            bundle.vocab,
            carried,
        );
        board.selected =
            selected_id.and_then(|id| board.board.id_to_index.get(id.as_str()).copied());
        board.compare = compare_id.and_then(|id| board.board.id_to_index.get(id.as_str()).copied());
        board.quarantine_expanded = quarantine_expanded && board.selected.is_some();
        Ok(board)
    }
}
#[cfg(test)]
#[path = "tests/workspace_reload.rs"]
mod tests;
