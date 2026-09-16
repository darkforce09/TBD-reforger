//! Role: Module boundary for editing.
//! Position: `editing` in the map engine.
//! Signals & state: the live authored document, its undo drive, and headless tool state machines.
//! Invariants: no browser, no UI framework, and no pointer types cross into this module — hosts
//! inject transports and confirmations as closures, and every state machine is driven by explicit
//! world coordinates.

/// Join spatial queries to document identifiers under a frozen camera.
pub mod picking;

/// Headless interactive map tools: their state machines, geometry, and verdicts.
pub mod tools;
