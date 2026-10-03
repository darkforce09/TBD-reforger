//! Line of sight over the bare ground of a terrain's elevation model.
//!
//! **Role:** samples the ground elevation profile along a sight line ([`elevation_profile`]) and
//! computes the viewshed around an observer, the raster of cells its eye sees, hides or cannot
//! judge, in one call ([`viewshed`]) or a ray at a time under a time budget ([`viewshed_job`]).
//! **Position:** line of sight category, tier 3, over `terrain_elevation` (the manifest, the
//! coverage test and the samplers). The interior line of sight reuses its visibility classes and
//! cap refusal; the map engine's line-of-sight tool and visibility scheduler, the GPU upload of the
//! viewshed lane and the mortar map picker read it.
//! **Signals & state:** none; plain data and pure functions, except a viewshed job's own raster
//! and cursor.
//! **Invariants:** no height is ever invented for a point off coverage; a sliced job's raster
//! equals the one-call raster; a request over the cell cap is refused with the cap and the
//! measured count.

pub mod elevation_profile;
pub mod error;
pub mod prelude;
mod radial_march;
pub mod viewshed;
pub mod viewshed_job;

#[cfg(test)]
#[path = "tests/viewshed_fixtures.rs"]
mod viewshed_fixtures;

pub use error::{Error, Result};
