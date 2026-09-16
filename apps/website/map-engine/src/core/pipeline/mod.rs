//! Role: Module boundary for core/pipeline.
//! Position: `core/pipeline` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Damage.
pub mod damage;

/// Draw order.
pub mod draw_order;

/// Stable vector and texture upload identifiers.
pub mod roles;
