//! **Role:** the symbology's browser half: the symbol atlas upload and the slot instance GPU
//! bridges.
//! **Position:** `overlay/symbology` in the map engine; the CPU symbology (`label_layout`,
//! `unit_symbology`, `overlay_instances`) is imported from the map overlay crates.
//! **Signals & state:** the atlas texture and the slot lanes' GPU state, owned by their modules.
//! **Invariants:** preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// The symbol atlas upload.
pub mod atlas;

/// The slot instance bridges and world icon lanes.
// `graphics_engine` is optional from `streaming` up, so the belts that name a graphics layout
// type are gated with it.
#[cfg(feature = "streaming")]
pub mod instances;
