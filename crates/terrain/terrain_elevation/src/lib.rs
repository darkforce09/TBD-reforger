//! A terrain's elevation model: where the height raster sits on the world, how its samples become
//! metres, and the grids the map reads heights from.
//!
//! **Role:** places the raster on the world ([`manifest`]), decodes the 16-bit PNG ([`png`]) and
//! the raw `TBDE` grid, whole or streamed ([`raw`]), converts and bilinearly samples heights
//! ([`sampling`]), box-averages the metres cache into the vector grid the relief and the line of
//! sight march over ([`grid`]), and keeps the native-spacing raster precise heights come from
//! ([`full_resolution`]).
//! **Position:** terrain category, tier 2, depending on `world_file_formats` (the `TBDE` header),
//! `map_coordinates` (rounding), `png`, `bytemuck` and `thiserror`. The map engine's terrain boot
//! decodes and publishes the model; `terrain_relief`, the line of sight, the airfield apron, the
//! spot heights, the Mission Creator's height readout and the developer tools read it.
//! **Signals & state:** none but the shared [`full_resolution::FullResolutionDemHandle`] cell the
//! terrain boot fills once; everything else is plain data and pure functions.
//! **Invariants:** a sample converts to metres linearly over the raster's declared height range;
//! a lookup off the raster is `None`, never a clamped edge value; a raw grid decodes to the same
//! samples in any chunk size and from a misaligned buffer, and a header whose dimensions the file
//! cannot hold is refused before any allocation.

pub mod error;
pub mod full_resolution;
pub mod grid;
pub mod manifest;
pub mod png;
pub mod prelude;
pub mod raw;
pub mod sampling;

pub use error::{Error, Result};
