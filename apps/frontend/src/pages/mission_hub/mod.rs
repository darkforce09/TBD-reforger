//! The mission hub: the mission catalogue, the mission dossier and the create dialog.
//!
//! **Role:** groups the pages that browse and describe missions and the dialog that starts a new
//! mission before handing the author to the Mission Creator. The review record they show lives in
//! `features::mission_review_record`; the read-only review workspace in
//! `workspaces::editor::review_workspace`.
//! **Position:** the `/missions` and `/missions/:id` routes; the create dialog has no route of its
//! own and opens over the library.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** the overview's dossier body is shared — the library's slide-over renders the
//! same read-only content — so it stays free of any authoring control; the review record sits
//! beside it, never inside it.

pub mod create_dialog;
pub mod library;
pub mod overview;
