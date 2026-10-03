//! What a ticket-action control may see.
//!
//! **Role:** `MutationContext` (the repository root and `busy`) and `TicketActionContext` (the corpus
//! and the id-to-index map).
//! **Position:** part of `crate::ticket_actions::models`; built by the desktop application for every
//! menu and dialog.
//! **Signals & state:** none; borrowed views.
//! **Invariants:** a control never reaches application state; every dispatching control is disabled
//! while `busy`.

use super::*;
// ---- read-only context every mutation affordance needs ----

#[derive(Clone, Copy)]
/// What every mutation affordance reads: the repository root and whether a command runs.
pub struct MutationContext<'a> {
    /// The repository root, when one is adopted.
    pub repo_root: Option<&'a Path>,
    /// A verb subprocess is in flight — every dispatch affordance disables.
    pub busy: bool,
}

/// Ticket data required by command affordances; excludes all application and rendering state.
pub struct TicketActionContext<'a> {
    /// The loaded corpus.
    pub corpus: &'a Corpus,
    /// Ticket id to corpus index.
    pub id_to_index: &'a HashMap<String, usize>,
}
