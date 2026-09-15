//! Role: lib.
//! Position: `apps/website/mission-core/src` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Mission payload compilation and validation.
#[cfg(feature = "compiler")]
pub mod mission;

/// Plain-text ORBAT slot formatting.
pub mod slot_line;

/// Expose map engine core :: doc at this domain boundary.
#[cfg(all(test, feature = "doc"))]
pub use map_engine_core::doc;

#[cfg(test)]
#[path = "tests/feature_gate.rs"]
mod feature_gate;
