//! A mission's review, as the pages that show it share it: the history, the thread, the reply box,
//! an artifact's provenance, the submission control and the words they are all written in.
//!
//! **Role:** declares the review record the mission hub shows an author, the submission control
//! with its refusal panel, the history and thread views, the comment composer, the artifact
//! provenance and findings views, and the pure wording under them.
//! **Position:** used by the mission hub's overview and library dossier, by the approvals drawer,
//! and by the read-only review workspace.
//! **Signals & state:** none at this level; each component owns its own.
//! **Invariants:** one rendering of a review, a thread entry and a compile finding, so the author
//! and the reviewer read the same record in the same words.
#![allow(dead_code)]

pub(crate) mod artifact_provenance_view;
pub(crate) mod comment_composer;
pub(crate) mod review_history_view;
pub(crate) mod review_record;
pub(crate) mod review_wording;
pub(crate) mod submission_action;
pub(crate) mod submission_refusal;

#[cfg(test)]
#[path = "tests/review_wording.rs"]
mod review_wording_tests;

#[cfg(test)]
#[path = "tests/submission_refusal.rs"]
mod submission_refusal_tests;
