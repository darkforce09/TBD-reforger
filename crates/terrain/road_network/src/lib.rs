//! A terrain's road network: the segments, how the map styles and strokes them, the thin strips it
//! draws along footprints, and the airfield around the runways.
//!
//! **Role:** reads road segments from the JSON export or the binary archive ([`network`]) through
//! the closed road class codec ([`road_class`]); styles each class and gates it by zoom, and
//! expands polylines into triangle strips ([`styling`]); packs the visible roads into casing and
//! centreline buffers ([`mesh`]); runs fence, pier and bridge-rail strips along a footprint's long
//! axis ([`cartographic_strip`]); and boxes the runways and fills the flat apron inside the box
//! ([`airfield`]).
//! **Position:** terrain category, tier 3, depending on `world_file_formats` (the road archive and
//! the segment id), `prefab_catalog` (footprint corners), `terrain_elevation` (the vector grid the
//! apron reads), `render_primitives` (the apron's mesh) and `map_coordinates` (the box type). The
//! map engine's loaders, strip and glyph packers and road labels, the Mission Creator's debug
//! benches and the developer tools' world export read it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** a class's wire code is its table index plus one and an archive class code the
//! table cannot name is refused; a road's width is the median of its point pairs' widths, falling
//! back to the class width outside 0.3–40 m; the class signature changes exactly when a class
//! crosses its zoom gate.

pub mod airfield;
pub mod cartographic_strip;
pub mod error;
pub mod mesh;
pub mod network;
pub mod prelude;
pub mod road_class;
pub mod styling;

pub use error::{Error, Result};
