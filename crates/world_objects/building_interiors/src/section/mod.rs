//! The section cuts of a building's occlusion mesh and the height fields of its surfaces.
//!
//! **Role:** declares the plan cutter, height fields and level drawings ([`cutter`]) and the
//! y-interval index and sparse height raster they read and write ([`index`]).
//! **Position:** under the crate root; the Mission Creator's building viewer draws the cuts.
//! **Signals & state:** none.
//! **Invariants:** an indexed cut equals the brute-force cut over every triangle.

pub mod cutter;
pub mod index;
