//! The mission hub: the scenario catalogue, the scenario dossier, the review record, the read-only
//! review workspace and the create dialog.
//!
//! **Role:** groups the pages that browse and describe missions, the review record and submission
//! control they share with the approvals queue, the route that opens a reviewed artifact's version
//! in the editor read-only, and the dialog that starts a new mission before handing the author to
//! the editor.
//! **Position:** the `/missions`, `/missions/:id` and `/missions/:id/artifacts/:artifact_id/workspace`
//! routes; the create dialog has no route of its own and opens over the library.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** the overview's dossier body is shared — the library's slide-over renders the
//! same read-only content — so it stays free of any authoring control; the review record sits
//! beside it, never inside it.

pub mod create_dialog;
pub mod library;
pub mod mission_review;
pub mod overview;
pub mod review_workspace;
