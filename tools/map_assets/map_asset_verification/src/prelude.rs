//! The names a caller of the map asset gates imports in one line.
//!
//! **Role:** re-exports the crate's error type and the gate entries.
//! **Position:** imported by the CI task catalogue's map asset checks and the xtask map adapters.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; no item is defined here.

pub use crate::blas_manifest::verify_blas_manifest;
pub use crate::error::{Error, Result};
pub use crate::labels::{height_labels, locations, road_names, terrain_alignment, town_labels};
pub use crate::object_goldens::map_object_golden;
pub use crate::terrain_manifest::terrain_manifest;
