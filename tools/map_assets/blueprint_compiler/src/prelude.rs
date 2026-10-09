//! The names a caller of the blueprint compiler imports in one line.
//!
//! **Role:** re-exports the crate's error type and the command entries.
//! **Position:** imported by the `cargo xtask map` adapters.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; no item is defined here.

pub use crate::error::{Error, Result};
pub use crate::{
    run, run_instance_verification, run_mesh_voxelization, run_occlusion_sidecar_batch,
    run_occlusion_sidecar_emission, run_occlusion_sidecar_parity, run_pak_file_print,
    run_rotation_validation, run_xob_inspection,
};
