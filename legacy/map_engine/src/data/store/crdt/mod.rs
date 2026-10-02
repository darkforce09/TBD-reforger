//! Role: Module boundary for doc/crdt.
//! Position: `doc/crdt` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Ordered native Yrs ID arrays and legacy payload conversion.
pub mod id_arrays;

/// Row-aligned slot projections and string interning.
pub mod soa;

/// Local gesture grouping and bounded undo history.
pub mod undo_groups;
