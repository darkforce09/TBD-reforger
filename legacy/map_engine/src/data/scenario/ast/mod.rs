//! Role: Module boundary for mission/ast.
//! Position: `mission/ast` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Internal authored payload schema used by the compiler.
pub(crate) mod authoring;
/// Compiled entity, vehicle, and slot wire structures.
pub mod entities;
/// Faction and ORBAT hierarchy projections.
pub mod factions;
/// Compiled scenario, environment, and briefing structures.
pub mod scenario;
