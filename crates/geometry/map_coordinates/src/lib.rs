//! The map's coordinate facts and conversions.
//!
//! **Role:** the fixed world facts of the served terrains ([`terrain_frames`]: the Everon anchor
//! and bounds, the opening view, the Arland centre), the streaming chunk grid over a viewport
//! ([`chunk_math`]), JavaScript's `Math.round` ([`rounding::round`]) and the grid reference a map
//! prints and a user types ([`grid_reference`]).
//! **Position:** geometry tier 0, depending on `thiserror` only. The map engine's render path,
//! streaming scheduler, line of sight, terrain and cameras read it; `camera_math` rounds viewport
//! sizes with it; the Mission Creator's toolbelt and the mortar page format and parse grid
//! references through it.
//! **Signals & state:** none; constants and pure functions over world metres.
//! **Invariants:** world x runs east and y north, in metres from the terrain's south-west corner;
//! one grid-reference convention (`floor(m / cell) mod (100 km / cell)` per axis, easting first);
//! `round` rounds a half toward +∞, as JavaScript does.

pub mod chunk_math;
pub mod error;
pub mod grid_reference;
pub mod prelude;
pub mod rounding;
pub mod terrain_frames;

pub use error::{Error, Result};
