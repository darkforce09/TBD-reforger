//! Role: Module boundary for editing.
//! Position: `editing` in the map engine.
//! Signals & state: the live authored document, its undo drive, and headless tool state machines.
//! Invariants: no browser, no UI framework, and no pointer types cross into this module — hosts
//! inject transports and confirmations as closures, and every state machine is driven by explicit
//! world coordinates.

/// Group a multi-transaction edit into one undo step.
pub mod batch;

/// The undo drive over the hosted document's own stack.
pub mod history;

/// The live authored document and the borrow chain every editing command reaches it by.
pub mod host;

/// The decidable half of the editor's commands: export bytes, report wording, selection digests.
pub mod commands;

/// The editor's commands that run against the installed host: they open the hosted document,
/// commit, and run the post-change tail.
pub mod hosted_commands;

/// The editor-authored overlay lanes: what the map draws from the document, and what a click on
/// one of them resolves to.
pub mod lanes;

/// Join spatial queries to document identifiers under a frozen camera.
pub mod picking;

/// Where a subject id would go if it were clicked, and whether that click would reach anything.
pub mod routing;

/// The ids a live document holds that a selection may name, and the render-only views of them.
pub mod selection_universe;

/// The decidable half of local draft persistence: record keys, blob verdicts, merge policy.
pub mod persist;

/// Headless interactive map tools: their state machines, geometry, and verdicts.
pub mod tools;
