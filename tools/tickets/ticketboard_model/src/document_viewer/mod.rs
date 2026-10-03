//! Repository documents read on a worker thread, fenced to the repository root.
//!
//! **Role:** resolves a repository-relative document path, reads it off the UI thread and keeps
//! the viewer's state machine ([`models::ViewerState`]).
//! **Position:** used by [`crate::ticket_browser`] (document links) and the application;
//! `apps/ticketboard`'s `document_viewer::ui` paints the state and emits
//! [`events::DocumentEvent`]s.
//! **Signals & state:** one worker thread per read, reporting over a channel; a late read is
//! dropped by the state machine.
//! **Invariants:** a path outside the repository root is refused, never read.

pub mod events;
pub mod models;
pub mod services;
