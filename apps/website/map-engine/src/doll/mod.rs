//! Role: Module boundary for doll.
//! Position: `doll` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Interaction.
pub mod interaction;

/// Renderer.
pub mod renderer;

/// Scene.
pub mod scene;
