//! Helpers the frontend crates' tests share: source scrubbing, repository files, captured fixtures,
//! source shards and the `view!` attribute guard.
//!
//! **Role:** gives every frontend test one way to reach repository files and captured API fixtures
//! (from the repository root, whatever the depth of the calling crate), one way to join the shards
//! a source pin reads, and the scanners several areas' guards share.
//! **Position:** a dev-only crate: every frontend crate names it under `[dev-dependencies]`, so no
//! shipped build links it. Each area keeps its own source pins in its `tests/source_pins.rs`.
//! **Signals & state:** one process-wide cache of repository file texts ([`repository_root`]).
//! **Invariants:** repository files are found from the caller's `CARGO_MANIFEST_DIR` by walking
//! up to the repository root, never by a fixed number of parent-folder steps; a joined source
//! keeps each shard once, so a test asserting on a split file sees exactly the text the single file
//! held.

pub mod class_r_scrub;
pub mod fixtures;
pub mod prelude;
pub mod repository_root;
pub mod source_shards;
pub mod view_attribute_guard;
