//! Role: Module boundary for doll/scene.
//! Position: `doll/scene` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Instances.
pub mod instances;

/// Mesh.
pub mod mesh;

/// Model.
pub mod model;
