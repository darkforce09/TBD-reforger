//! Role: Module boundary for world.
//! Position: `world` in the map engine.
//! Signals & state: the static, immutable facts about the terrain being drawn.
//! Invariants: nothing here is authored, undoable or persisted. It describes the ground.

/// The scene anchor and the synthetic instance scenes measured against it.
pub mod scene;
