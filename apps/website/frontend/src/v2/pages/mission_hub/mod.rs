//! The mission hub: the scenario catalogue, the scenario dossier and the create dialog.
//!
//! **Role:** groups the pages that browse and describe missions, and the dialog that starts a new
//! one before handing the author to the editor.
//! **Position:** the `/missions` and `/missions/:id` routes; the create dialog has no route of
//! its own and opens over the library.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** the overview's dossier body is shared — the library's slide-over renders the
//! same read-only content — so it stays free of any authoring control.

pub mod create_dialog;
pub mod library;
pub mod overview;
