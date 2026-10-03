//! A terrain's vegetation: the forest and land-cover regions, the canopy mass the map outlines and
//! fills, the tree counts that switch it between instances and a heatmap, and the density bins the
//! heatmap samples.
//!
//! **Role:** reads land-cover regions from the JSON export or the binary archive and writes the
//! archive rows ([`regions`]); traces the canopy mass outline and fill from the density corners
//! ([`mass`]); counts the trees a view draws and packs the per-chunk density grid ([`canopy`]);
//! and stitches, packs and samples the island density bins ([`density`]).
//! **Position:** world objects category, tier 4, depending on `world_file_formats` (the forest
//! archive and the region id), `world_chunks` (the chunk rows the counts sum), `prefab_catalog`
//! (the tree and vegetation class codes), `map_draw_lanes` (the instance budget and class zoom
//! gates) and `map_coordinates` (the view box). The map engine's vegetation loader and GPU belts,
//! its draw-buffer packer and the developer tools' world export read it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** the map draws tree instances only while the exact count stays within the
//! instance budget and the heatmap otherwise; a region the archive cannot hold (a ring of fewer
//! than three vertices, no ring) is refused, never truncated; the density island is 1601 corners
//! square, 25 chunks of 64 cells per axis.

pub mod canopy;
pub mod density;
pub mod error;
pub mod mass;
pub mod prelude;
pub mod regions;

pub use error::{Error, Result};
