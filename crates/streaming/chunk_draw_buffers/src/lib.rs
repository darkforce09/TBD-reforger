//! The world draw buffers and the world residency that owns them.
//!
//! **Role:** the draw state composed from `chunk_scheduler`'s chunk residency
//! (`draw_buffers.rs`): the draw set, the tree, prop and badge glyphs ([`glyphs`], [`packer`]), the
//! pier, bridge-rail and fence strips ([`strips`]), the building fills and outlines
//! ([`footprint`]), the layer toggles ([`toggles`]), the getters the loaders upload
//! ([`revision`]) and the statistics ([`residency_statistics`]); and
//! [`world_residency::WorldResidency`], which owns the residency and these buffers and applies
//! every rebuild request the residency returns.
//! **Position:** streaming category, tier 5, over `chunk_scheduler`, the overlay, terrain, world
//! object and graphics CPU crates; held by the map engine's world and occluder loaders and by the
//! frontend's debug world line-of-sight bench.
//! **Signals & state:** the draw buffers' glyph lookup, toggles, draw set, packed buffers and memo
//! keys, private to this crate.
//! **Invariants:** the draw buffers read the chunk residency only through its public methods; no
//! public call returns with a rebuild request left unapplied.

pub mod chunk_residency_delegates;
mod draw_buffers;
mod error;
pub mod footprint;
pub mod glyphs;
pub mod packer;
pub mod prelude;
pub mod residency_statistics;
pub mod revision;
pub mod strips;
pub mod toggles;
pub mod world_residency;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
