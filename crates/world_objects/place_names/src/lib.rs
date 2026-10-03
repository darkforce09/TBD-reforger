//! A terrain's place names: the spot heights, town labels and road names the map writes on the
//! ground, which of them show at a zoom, and the glyphs they draw as.
//!
//! **Role:** finds and declutters spot heights on the elevation model ([`peaks`]); reads the town
//! and height label JSON and the labels archive's lanes ([`towns`]); places curated road names
//! along their segments and declutters them ([`route_placement`], with the polyline geometry of
//! [`route_geometry`] and the `road-names.json` model of [`route_labels`]); and packs the kept
//! labels into glyph instances and instance bytes ([`label_packing`]). Each kind converts to the
//! label layout's `LabelSpec` here.
//! **Position:** world objects category, tier 4, over `label_layout` (label specs, town declutter,
//! glyph packing), `terrain_elevation` (raster placement), `road_network` (segments, the class
//! codec), `world_file_formats` (the labels archive and its ids) and `render_primitives` (glyph
//! metrics). The map engine's label loader and the developer tools' label archive, height label
//! export and label checks read it.
//! **Signals & state:** none; plain data and pure functions.
//! **Invariants:** a kept label is at least its kind's zoom-scaled separation from every other
//! kept label of that kind, and each kind keeps at most its cap; the archive reads back exactly
//! the lanes it was baked from, at any buffer offset; a road name floor no road class can carry
//! is refused, never rounded.

pub mod error;
pub mod label_packing;
pub mod peaks;
pub mod prelude;
pub mod road_name_ids;
pub mod route_geometry;
pub mod route_labels;
pub mod route_placement;
pub mod towns;

pub use error::{Error, Result};
