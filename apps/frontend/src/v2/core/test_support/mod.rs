//! Helpers shared by the crate's own tests: source scrubbing, fixture loading, and source pins.
//!
//! **Role:** gives every test one place to reach production text and captured API fixtures from,
//! so that a file move changes one path here instead of dozens of relative `include_str!` calls.
//! **Position:** compiled only under `cfg(test)`; no shipped code depends on it.
//! **Signals & state:** none.
//! **Invariants:** a pin helper returns the concatenation of a file's shards, so a test asserting
//! on a split file sees exactly the text the single file used to hold.

pub mod class_r_scrub;
pub mod editor_operations;
pub mod fixtures;
pub mod pins;
