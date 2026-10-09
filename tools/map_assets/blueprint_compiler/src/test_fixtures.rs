//! The blueprint compiler's committed test inputs.
//!
//! **Role:** locates a file under the crate's `test_fixtures/blueprint/` folder (voxel dumps,
//! golden blueprints and sidecars, parity files, prefab `.et` trees).
//! **Position:** read by this crate's unit tests and, through the `test_fixtures` feature, by
//! the world line-of-sight tests of the map asset verification.
//! **Signals & state:** none.
//! **Invariants:** the path resolves from the checkout root, whatever directory the test runs in.

use std::path::PathBuf;

/// The committed blueprint fixture `name`, under the checkout this crate was compiled from.
///
/// # Panics
///
/// When no checkout root is found from the working directory.
#[must_use]
pub fn fixture(name: &str) -> PathBuf {
    ::repository_root::find_repository_root()
        .expect("repository root")
        .join("tools/map_assets/blueprint_compiler/test_fixtures/blueprint")
        .join(name)
}
