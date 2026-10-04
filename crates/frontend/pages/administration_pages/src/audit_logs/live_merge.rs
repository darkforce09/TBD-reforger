//! The audit board: history pages and live rows merged into one trail keyed by audit id.
//!
//! **Role:** holds every loaded audit line once, whichever way it arrived, in id-descending order,
//! with the keyset continuation of the history and the count of lines the live stream delivered.
//! **Position:** kept in one signal by the audit route. The stream callbacks insert live rows,
//! the history loads merge list pages, and the trail and the inspector read from it.
//! **Signals & state:** none of its own; a plain value the route keeps in a signal.
//! **Invariants:** one entry per audit id: a line delivered live and again by a history page, or
//! twice by either, shows once. Rows read newest id first, so a live line with an old id slots in
//! at its id rather than at the top. "Load more" continues below the smallest id a **history page**
//! delivered, never below a live line's id, which may sit far below the history window and would
//! skip the lines between. A restart empties the board and moves it to a new epoch; a history page
//! requested under an older epoch is dropped on arrival, so a reload never mixes with the history
//! it replaces.

#[cfg(any(target_arch = "wasm32", test))]
use super::page::parse_next_cursor;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::CursorList;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::administration::AuditLogEntry;
#[cfg(any(target_arch = "wasm32", test))]
use std::collections::{BTreeMap, BTreeSet};

/// The merged audit trail of one page visit.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AuditBoard {
    /// Every loaded line, keyed by its audit id.
    entries: BTreeMap<i64, AuditLogEntry>,
    /// The ids the live stream delivered before any history page did.
    live_ids: BTreeSet<i64>,
    /// The smallest id any history page delivered since the last restart.
    history_floor: Option<i64>,
    /// The last history page reported a further page.
    history_continues: bool,
    /// Bumped by every restart; history loads carry the epoch they were requested under.
    epoch: u64,
}

#[cfg(any(target_arch = "wasm32", test))]
impl AuditBoard {
    /// An empty board at epoch 0.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The epoch a history load must carry to be merged.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Empty the board for a history reload and return the new epoch the reload carries.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn restart(&mut self) -> u64 {
        let epoch = self.epoch + 1;
        *self = Self {
            epoch,
            ..Self::default()
        };
        epoch
    }

    /// Insert a line the live stream delivered. Returns whether it was new to the board; a line
    /// already on it is left as it is.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn insert_live(&mut self, entry: AuditLogEntry) -> bool {
        if self.entries.contains_key(&entry.id.get()) {
            return false;
        }
        self.live_ids.insert(entry.id.get());
        self.entries.insert(entry.id.get(), entry);
        true
    }

    /// Merge one history page requested under `epoch`. Returns `false`, changing nothing, when the
    /// board has restarted since the request went out.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn merge_history(&mut self, epoch: u64, page: CursorList<AuditLogEntry>) -> bool {
        if epoch != self.epoch {
            return false;
        }
        let continues = parse_next_cursor(&page.next_cursor).is_some() && !page.data.is_empty();
        for entry in page.data {
            self.history_floor = Some(
                self.history_floor
                    .map_or(entry.id.get(), |f| f.min(entry.id.get())),
            );
            self.entries.entry(entry.id.get()).or_insert(entry);
        }
        self.history_continues = continues;
        true
    }

    /// The `before` of the next history page, or `None` when the history is exhausted or has not
    /// delivered a page yet.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn continuation(&self) -> Option<i64> {
        self.history_floor.filter(|_| self.history_continues)
    }

    /// Every line, newest id first.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn rows(&self) -> Vec<AuditLogEntry> {
        self.entries.values().rev().cloned().collect()
    }

    /// The line with this id, if it is on the board.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn get(&self, id: i64) -> Option<&AuditLogEntry> {
        self.entries.get(&id)
    }

    /// Whether the board holds no line.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many lines on the board the live stream delivered first.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn live_count(&self) -> usize {
        self.live_ids.len()
    }
}

#[cfg(test)]
#[path = "tests/live_merge.rs"]
mod tests;
