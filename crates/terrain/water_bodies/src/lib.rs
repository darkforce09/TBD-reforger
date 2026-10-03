//! What a terrain's map data says about water.
//!
//! **Role:** reads the bathymetry container as a pyramid of depth and mask levels and pairs it
//! with the world rectangle as a water mask, plans the level suffix a capped load fetches, reads
//! the inland water archive of lakes, rivers and ponds ([`vectors`]), and triangulates the sea
//! band into the sea fill mesh ([`mesh`]).
//! **Position:** terrain category, tier 4, depending on `terrain_relief` (the sea band),
//! `render_primitives` (the fill mesh and triangulation), `world_file_formats` (the `TBDB` header
//! and the vectors archive), `rkyv`, `bytemuck` and `thiserror`. The map engine's water loader
//! fetches the files and keeps the mask; its relief host draws the sea mesh; the developer tools'
//! inland water pipeline writes both files with [`vectors::downsample_index`].
//! **Signals & state:** none; plain data and pure functions over byte slices.
//! **Invariants:** every level agrees with level 0 under the emitter's own downsampling; a point
//! off the map, or at a level the file does not hold, is unknown, never dry; a level suffix
//! answers exactly as the whole file does; a malformed container, extent or archive is refused
//! rather than guessed.

pub mod error;
pub mod mesh;
pub mod prelude;
pub mod vectors;

pub use error::{Error, Result};
