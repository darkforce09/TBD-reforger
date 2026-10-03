//! The Arsenal paper doll's scene: the soldier, its clickable equipment regions and the picking
//! over them.
//!
//! **Role:** names the 14 equipment regions and builds the schematic soldier from scaled unit
//! meshes ([`soldier_parts`], [`part_meshes`]), colours each region by its loadout state, and
//! answers which region lies under a pixel and where a region's callout belongs on screen
//! ([`region_picking`]).
//! **Position:** paper doll category, tier 2, over `camera_math`; `paper_doll_renderer` draws
//! the parts and calls the picks with the yaw and size it draws with, and the Arsenal of the
//! Mission Creator reaches the scene only through that renderer.
//! **Signals & state:** none; pure data and functions.
//! **Invariants:** a region's index in [`soldier_parts::REGION_KEYS`] is its number everywhere
//! (part region, state byte, pick result); the picks use the same orbit camera matrix the renderer
//! draws with, so the region a pick returns is the one drawn under the pixel.

pub mod part_meshes;
pub mod prelude;
pub mod region_picking;
pub mod soldier_parts;

#[cfg(test)]
#[path = "tests/soldier_model_tests.rs"]
mod tests;
