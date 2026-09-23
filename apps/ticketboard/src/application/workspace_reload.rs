use super::*;
impl WorkspaceState {
    /// Rebuild projections while resolving selections by identity against the new corpus.
    pub(super) fn reload(bundle: LoadBundle, previous: Option<&Self>) -> Result<Self, LoadError> {
        let corpus = bundle.corpus?;
        let (carried, selected_id, compare_id, legacy_expanded) = match previous {
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
                board.legacy_expanded,
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
        board.selected = selected_id.and_then(|id| board.board.id_to_index.get(&id).copied());
        board.compare = compare_id.and_then(|id| board.board.id_to_index.get(&id).copied());
        board.legacy_expanded = legacy_expanded && board.selected.is_some();
        Ok(board)
    }
}
#[cfg(test)]
#[path = "tests/workspace_reload.rs"]
mod tests;
