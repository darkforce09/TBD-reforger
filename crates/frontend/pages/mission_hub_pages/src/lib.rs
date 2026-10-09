//! The mission hub: the mission catalogue, the mission dossier and the create dialog.
//!
//! **Role:** groups the pages that browse and describe missions and the dialog that starts a new
//! mission before handing the author to the Mission Creator. The review record they show lives in
//! the `mission_review_record` crate; the read-only review workspace in the app's
//! `workspaces::editor::review_workspace`.
//! **Position:** a page crate above the foundation crates and `mission_review_record`; the app's
//! route table mounts its `/missions` and `/missions/:id` routes, and the create dialog has no
//! route of its own and opens over the library.
//! **Signals & state:** none at this level; each page owns its fetch and its signals.
//! **Invariants:** the overview's dossier body is shared — the library's slide-over renders the
//! same read-only content — so it stays free of any authoring control; the review record sits
//! beside it, never inside it. The route components and every panel that fetches are compiled
//! for `wasm32` only, because the endpoints they call exist only in the browser build.

pub mod create_dialog;
pub mod library;
pub mod overview;
pub mod prelude;
