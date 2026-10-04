//! A mission's review, as the pages that show it share it: the history, the thread, the reply box,
//! an artifact's provenance, the submission control and the words they are all written in.
//!
//! **Role:** declares the review record the mission hub shows an author, the submission control
//! with its refusal panel, the history and thread views, the comment composer, the artifact
//! provenance and findings views, and the pure wording under them.
//! **Position:** a feature crate above `frontend_session` (the signed-in store the browser calls
//! read), `frontend_transport` (the review endpoints and the refusal a submission answers with),
//! `frontend_api_dtos` (the review, comment, artifact and finding wire types) and `frontend_ui`
//! (badges, icons, UTC instants, toasts); used by the app's mission hub overview and library
//! dossier, its approvals queue and drawer, its deployment wording, and the read-only review
//! workspace.
//! **Signals & state:** none at this level; each component owns its own.
//! **Invariants:** one rendering of a review, a thread entry and a compile finding, so the author
//! and the reviewer read the same record in the same words. The wording, the refusal reading and
//! the history and provenance views compile on every target and are unit-tested natively; the
//! components that call the review endpoints — `comment_composer`, `submission_action` and the
//! review record's component — are `#[cfg(target_arch = "wasm32")]`, because those calls exist
//! only in the browser.

pub mod artifact_provenance_view;
// Posts a comment through the browser-only review endpoint.
#[cfg(target_arch = "wasm32")]
pub mod comment_composer;
pub mod error;
pub mod prelude;
pub mod review_history_view;
pub mod review_record;
pub mod review_wording;
// Submits a mission through the browser-only review endpoint.
#[cfg(target_arch = "wasm32")]
pub mod submission_action;
pub mod submission_refusal;

pub use error::{Error, Result};

#[cfg(test)]
#[path = "tests/review_wording.rs"]
mod review_wording_tests;

#[cfg(test)]
#[path = "tests/submission_refusal.rs"]
mod submission_refusal_tests;
