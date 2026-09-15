//! Role: slot ids.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

/// Expose website mission core :: doc :: operations :: slot ids :: duplicate slot ids at this domain boundary.
pub use website_mission_core::doc::operations::slot_ids::duplicate_slot_ids;
