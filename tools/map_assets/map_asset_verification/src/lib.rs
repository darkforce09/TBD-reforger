//! The gates over a terrain's committed map assets and the map golden fixtures.
//!
//! **Role:** checks the terrain manifest, the prefab BLAS library, the height, location, town and
//! road labels, the elevation anchors and the map-object goldens with the world crates' own
//! readers and geometry, and probes line of sight through the world occluder over the committed
//! catalogue.
//! **Position:** a tools crate under `tools/map_assets`; the CI task catalogue's map asset checks
//! and the `cargo xtask map world-los` adapter call one gate each with the checkout root; reads
//! `assets/terrains/<terrain>/` and `contracts/`.
//! **Signals & state:** none at the crate root; each gate reads the checkout afresh.
//! **Invariants:** every gate returns its exit code (0 pass, 1 a finding, 2 an unknown terrain
//! for the manifest gate) and never exits the process; a gate writes nothing under `assets/` or
//! `contracts/`; a missing input a gate needs is a finding or an [`Error`], never a skip, except
//! where the gate prints its `SKIP` line.

pub mod blas_manifest;
mod error;
pub mod labels;
pub mod object_goldens;
pub mod prelude;
pub mod terrain_manifest;
pub mod world_line_of_sight;

pub use error::{Error, Result};
