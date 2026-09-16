//! Role: the loadouts placed slots carry — read one, seed one from the character's defaults, buffer
//! the selection's, and commit a planned set of writes.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document and the selected ids come from the host. The
//! copy buffer and the character-default map are the document operations' own process state.
//! Invariants: only a write the document acknowledged takes the post-change tail, so a pick against
//! an id the mission no longer holds dirties nothing and mints no undo step. A planned set commits
//! as one act and takes exactly one tail for the whole set. Planning which bytes land on which slot
//! is the HOST's — this module commits a plan, it never makes one.

use std::collections::HashMap;

use crate::data::store::operations::cargo;
use crate::data::store::operations::entity::selected_slot_ids;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, with_doc};

/// One buffered loadout: the bytes, and the slot they were copied from.
pub use crate::data::store::operations::cargo::BufferedLoadout;

/// One planned write: the target slot, the source it came from, and the bytes to store.
pub use crate::data::store::operations::cargo::LoadoutWrite;

/// The rows a character's engine defaults seed into an empty cargo list.
pub use crate::data::store::operations::cargo_rules::CargoRow;

/// Read a slot's embedded loadout document. `None` when the slot carries none.
#[must_use]
pub fn read_loadout(id: &str) -> Option<String> {
    with_doc(|core| cargo::read_loadout(core, id)).flatten()
}

/// Install the character → default-cargo map the Arsenal seeds from.
pub fn set_cargo_defaults(map: HashMap<String, Vec<CargoRow>>) {
    cargo::set_cargo_defaults(map);
}

/// Seed a slot that has never carried a loadout from its character's defaults. Returns the seeded
/// document so a panel can render it without a second read; `None` means nothing was seeded, and
/// nothing seeded runs no tail.
pub fn seed_slot_cargo(id: &str) -> Option<String> {
    let seeded = with_doc(|core| cargo::seed_slot_cargo_from_defaults(core, id)).flatten();
    if seeded.is_some() {
        after_local_edit();
    }
    seeded
}

/// The selected ids that are actually slots — the targets a bulk loadout gesture may write.
#[must_use]
pub fn selection_slot_targets() -> Vec<String> {
    let sel = selection_ids();
    with_doc(|core| selected_slot_ids(core, &sel)).unwrap_or_default()
}

/// Buffer the loadout of EVERY selected slot, not just one. Returns how many were buffered.
pub fn copy_loadouts_from_selection() -> usize {
    let sel = selection_ids();
    with_doc(|core| cargo::buffer_loadouts_from_selection(core, sel)).unwrap_or(0)
}

/// What the copy buffer holds right now — a panel's label and its receipt.
#[must_use]
pub fn loadout_buffer() -> Vec<BufferedLoadout> {
    cargo::loadout_buffer()
}

/// How many loadouts are buffered — the affordance an Apply button enables on.
#[must_use]
pub fn loadout_buffer_len() -> usize {
    cargo::loadout_buffer_len()
}

/// The seed that decides which buffered loadout each target receives. Drawn once per Apply so the
/// draw is reproducible across the whole gesture.
#[must_use]
pub fn next_apply_seed() -> u64 {
    cargo::next_apply_seed()
}

/// Commit a planned set of loadout writes and take ONE tail for the whole set. Returns how many the
/// document acknowledged; zero runs no tail.
pub fn commit_loadout_writes(writes: &[LoadoutWrite]) -> usize {
    if writes.is_empty() {
        return 0;
    }
    let commits = with_doc(|core| cargo::commit_loadout_writes(core, writes)).unwrap_or(0);
    if commits > 0 {
        after_local_edit();
    }
    commits
}
