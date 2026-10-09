//! The building-blueprint compiler behind line of sight through buildings and prefabs.
//!
//! **Role:** decodes Reforger `.xob` models and Workbench voxel dumps, interprets each building
//! into a blueprint (floors, walls, plates, roof), builds the per-prefab occlusion sidecars and
//! instance tables, folds them into the blueprint archive, and checks every step against
//! Workbench recordings of the engine.
//! **Position:** a tools crate under `tools/map_assets`; each `cargo xtask map` adapter calls one
//! entry with the checkout root and its raw arguments; writes under `assets/terrains/<terrain>/`.
//! **Signals & state:** none at the crate root; each command reads its inputs and writes its
//! outputs once.
//! **Invariants:** every entry returns its exit code (0 success, 1 a refusal or a mismatch) and
//! never exits the process; every emitted document passes its contract schema before it is
//! written.

mod architectural_analysis;
mod archive_emission;
mod blueprint_from_voxels;
pub mod blueprint_ingestion;
mod error;
mod mesh_decoding;
mod occlusion_sidecars;
pub mod parity_report;
pub mod prelude;
#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;
mod voxel_processing;

pub use blueprint_from_voxels::run;
pub use error::{Error, Result};
pub use mesh_decoding::archive_inspection::{run_pak_file_print, run_xob_inspection};
pub use occlusion_sidecars::batch_processing::run_occlusion_sidecar_batch;
pub use occlusion_sidecars::construction::{
    run_occlusion_sidecar_emission, run_occlusion_sidecar_parity,
};
pub use occlusion_sidecars::instance_verification::run_instance_verification;
pub use occlusion_sidecars::rotation_validation::run_rotation_validation;
pub use voxel_processing::mesh_voxelization::run_mesh_voxelization;
