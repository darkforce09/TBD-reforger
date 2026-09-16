//! Role: Module boundary for doll/renderer/engine.
//! Position: `doll/renderer/engine` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::doll::renderer::lifecycle_1::DollEngine`.
pub use crate::doll::renderer::lifecycle_1::DollEngine;
