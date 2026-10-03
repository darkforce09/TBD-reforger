//! The relief the 2D map draws from a terrain's elevation model.
//!
//! **Role:** shades the metres cache into the hillshade image ([`hillshade`]), marches the vector
//! grid into contour rings and picks each summit's ring ([`contours`]), and fills the cells at or
//! below four heights around the waterline into the sea band ([`sea_band`]).
//! **Position:** terrain category, tier 3, depending on `terrain_elevation` (the vector grid) and
//! `map_coordinates` (rounding). The map engine's relief host uploads the results as lanes and
//! textures; `water_bodies` triangulates the sea band; the map engine's mesh composer draws the
//! contour rings.
//! **Signals & state:** none; pure functions over the grid and the metres cache.
//! **Invariants:** flat ground shades one uniform grey; each peak gets exactly one summit ring,
//! its highest closed one, and an open chain never counts; every contour and sea band ring is
//! closed by its own rule (a closed contour never repeats its first point, a sea band ring always
//! does); land above every sea band level gets no fill.

pub mod contours;
pub mod hillshade;
pub mod prelude;
pub mod sea_band;
