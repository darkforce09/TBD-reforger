//! Role: Module boundary for data.
//! Position: `data` in the map engine.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.
//!
//! T-0xx Phase 2A folded `website-mission-core` in here. Two halves, one rule:
//! `scenario` is the authored mission — AST, compiler, validation, extensions, slot lines — and
//! `store` is the `yrs` layer that edits it. The old crate called the second half "the mission
//! doc"; the term is retired, because it never named a concept. It is the CRDT store, so it is
//! `store`.

/// The authored mission: AST, compiler, validation, extensions, and slot lines.
#[cfg(feature = "scenario")]
pub mod scenario;

/// The CRDT document store: ordered arrays, row projections, operations, and selection.
#[cfg(feature = "store")]
pub mod store;
