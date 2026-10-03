//! The debug HUD tail of a ledger.
//!
//! **Role:** [`Ledger::hud_suffix`]: the reserved bytes against the budget and the satellite
//! floor with its raise count, as the Mission Creator's debug HUD appends them.
//! **Position:** read by the map engine's live ledger, whose `hud_suffix` the frame pump shows.
//! **Signals & state:** none; reads the ledger.
//! **Invariants:** empty while nothing is held and no floor is chosen.

use super::model::{Ledger, MIB};

impl Ledger {
    /// The debug-HUD tail: reserved bytes against the budget, and the current satellite floor.
    #[must_use]
    pub fn hud_suffix(&self) -> String {
        if self.held_total() == 0 && self.sat_floor.is_none() {
            return String::new();
        }
        let sat = match (self.sat_floor, self.sat_raised) {
            (Some(f), 0) => format!(" · sat L{f}"),
            (Some(f), n) => format!(" · sat L{f} (+{n})"),
            (None, _) => String::new(),
        };
        format!(
            " · mem {}/{}MB{sat}",
            self.held_total() / MIB,
            self.budget / MIB
        )
    }
}
