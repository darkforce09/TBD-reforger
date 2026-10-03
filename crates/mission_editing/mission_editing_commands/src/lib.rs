//! The Mission Creator's editing commands over the live mission document.
//!
//! **Role:** holds the commands that run against the installed editing host
//! ([`hosted_commands`]: each names what to change, opens the hosted document, commits one
//! transaction and runs the post-change tail) and the pure document texts ([`document_text`]: the
//! bytes an export writes, the wording of the compile, merge and save reports, and the selection
//! digests a clipboard receives).
//! **Position:** mission editing category, tier 7, over `mission_editing_session`,
//! `mission_operations`, `mission_document`, `mission_model`, `mission_validation`,
//! `formation_geometry`, `map_coordinates` and `orbat_slot_ids`. The Mission Creator calls it
//! through the map engine's `editing` module.
//! **Signals & state:** two thread-locals of interaction state that never reach the document (the
//! copied clipboard rows and the armed connection); everything else lives in the session's host or
//! is pure over its arguments.
//! **Invariants:** no browser, UI framework or GPU type crosses into this crate; every hosted
//! command opens one document borrow and drops it before the post-change tail runs; a command that
//! changed nothing runs no tail; every id a command takes is a newtype id.

pub mod document_text;
pub mod error;
pub mod hosted_commands;
pub mod prelude;

pub use error::{Error, Result};
