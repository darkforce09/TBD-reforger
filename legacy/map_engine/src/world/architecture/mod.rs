//! Role: Module boundary for architecture.
//! Position: `world/architecture` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 2B: `los/` left for `spatial/los/interior/`, to sit beside the terrain and
//! world LOS layers it stacks with. What is left here is the model of a building, not a
//! question asked about one.

/// Blueprint.
pub mod blueprint;

/// Compound.
pub mod compound;

/// Section.
pub mod section;
