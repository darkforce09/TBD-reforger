//! The names a caller of the blueprint compiler imports in one line.
//!
//! **Role:** re-exports the crate's error type and the command entries.
//! **Position:** imported by the `cargo xtask map` adapters.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; no item is defined here.

pub use crate::error::{Error, Result};
pub use crate::{
    run, run_bvh_batch, run_bvh_emit, run_bvh_parity, run_instances_verify, run_pak_cat,
    run_rotation_pin, run_voxels_from_mesh, run_xob_inspect,
};
