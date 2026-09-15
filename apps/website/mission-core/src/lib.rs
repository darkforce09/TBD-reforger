//! Role: lib.
//! Position: `apps/website/mission-core/src` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Mission payload compilation and validation.
#[cfg(feature = "compiler")]
pub mod mission;

/// Plain-text ORBAT slot formatting.
pub mod slot_line;

/// Mission CRDT state, projections, and document operations.
#[cfg(feature = "doc")]
pub mod doc;

#[cfg(test)]
#[path = "tests/feature_gate.rs"]
mod feature_gate;
