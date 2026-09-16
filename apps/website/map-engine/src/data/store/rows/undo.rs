//! Role: undo.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;

impl MissionDocCore {
    /// Undo depth using the supplied domain data.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.apply_undo_cap();
        self.undo_mgr
            .undo_stack()
            .len()
            .saturating_sub(self.undo_cap_hidden.get())
    }
}

impl MissionDocCore {
    /// Apply undo cap using the supplied domain data.
    pub(super) fn apply_undo_cap(&self) {
        let n = self.undo_mgr.undo_stack().len();
        let hidden = crate::data::store::crdt::undo_groups::hidden_prefix_after(
            n,
            self.undo_cap_hidden.get(),
        );
        self.undo_cap_hidden.set(hidden);
    }
}

impl MissionDocCore {
    /// Open an explicit undo group. Nested calls are counted; the clock stays frozen until the matching [`Self::end_group`].
    pub fn begin_group(&self) {
        self.undo_groups.begin_group();
    }
}

impl MissionDocCore {
    /// Close an explicit undo group. The next LOCAL transaction is a new stack item.
    pub fn end_group(&mut self) {
        if self.undo_groups.end_group() {
            self.undo_mgr.reset();
        }
    }
}

impl MissionDocCore {
    /// Undo the most recent tracked group; `true` if anything was undone.
    pub fn undo(&mut self) -> bool {
        if self.undo_depth() == 0 {
            return false;
        }
        self.undo_mgr.undo_blocking()
    }
}

impl MissionDocCore {
    /// Redo the most recently undone transaction; `true` if anything was redone.
    pub fn redo(&mut self) -> bool {
        self.undo_mgr.redo_blocking()
    }
}

impl MissionDocCore {
    /// Can undo using the supplied domain data.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.undo_depth() > 0
    }
}

impl MissionDocCore {
    /// Can redo using the supplied domain data.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.undo_mgr.can_redo()
    }
}
