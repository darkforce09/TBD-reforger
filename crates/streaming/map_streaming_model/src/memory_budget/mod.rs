//! The memory budget model: the ledger of the world assets' bytes and the satellite floor walk.
//!
//! **Role:** the seven asset rows and the [`Ledger`] that judges, records and releases their
//! bytes against one ceiling (`model.rs`, `ledger.rs`), the satellite level costs and the walk
//! that raises the mip floor until the ledger accepts a level (`satellite_floor.rs`), the debug
//! HUD tail [`Ledger::hud_suffix`] (`hud_suffix.rs`) and the budget a boot's settings resolve to,
//! [`budget_bytes_from_settings`] (`budget_settings.rs`).
//! **Position:** pure model under the map engine's live ledger, which keeps one [`Ledger`] per
//! page, reads the page's settings and heap size, and publishes the snapshot.
//! **Signals & state:** none; a [`Ledger`] is a plain value its owner mutates.
//! **Invariants:** measured growth never enters the held bytes; a reservation records nothing
//! unless it fits; the floor walk never returns a level finer than the one it started from.

mod budget_settings;
mod hud_suffix;
mod ledger;
mod model;
mod satellite_floor;

pub use budget_settings::budget_bytes_from_settings;
pub use model::{Asset, DEFAULT_BUDGET_MB, Decision, Entry, Ledger, MIB};
pub use satellite_floor::{FloorWalk, LevelBytes, floor_for_budget, satellite_resident_bytes};

#[cfg(test)]
mod tests;
