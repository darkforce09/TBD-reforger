//! Helpers the frontend crates' tests share: repository files and captured fixtures.
//!
//! **Role:** gives every frontend test one way to reach repository files, the captured API
//! fixtures and the API route tables from the repository root, whatever the depth of the calling
//! crate.
//! **Position:** a dev-only crate: every frontend crate names it under `[dev-dependencies]`, so no
//! shipped build links it.
//! **Signals & state:** one process-wide cache of repository file texts ([`repository_root`]).
//! **Invariants:** repository files are found from the caller's `CARGO_MANIFEST_DIR` by walking
//! up to the repository root, never by a fixed number of parent-folder steps.

pub mod fixtures;
pub mod prelude;
pub mod repository_root;
