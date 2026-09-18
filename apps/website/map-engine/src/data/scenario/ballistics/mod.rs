//! Role: Module boundary for data/scenario/ballistics.
//! Position: `data/scenario/ballistics` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Mortar charge tables and high-angle firing solutions.
pub mod mortar_fire_solution;

/// Expose mortar fire solution :: { fire solution , solve error , solve fire mission } at this domain boundary.
pub use mortar_fire_solution::{FireSolution, SolveError, solve_fire_mission};
