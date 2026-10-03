//! The wiki's writes: the save and restore bodies, the `PUT` that sends them, and the refusal
//! the author is shown.
//!
//! **Role:** declares the request builders, the refusal classifier, the submission and the
//! problem view, and re-exports what the article pane and the revision view call.
//! **Position:** between the wiki's panes and `PUT /api/v1/wiki/{slug}`.
//! **Signals & state:** none at this level; the submission writes the page's and the article's
//! signals.
//! **Invariants:** both writes go through one submission, so a draft save and a restore are
//! refused, reported and reloaded the same way.

#[cfg(target_arch = "wasm32")]
mod save_problem_view;
mod save_refusal;
mod save_requests;
#[cfg(target_arch = "wasm32")]
mod save_submission;

#[cfg(target_arch = "wasm32")]
pub(super) use save_problem_view::save_problem_view;
#[cfg(target_arch = "wasm32")]
pub(super) use save_refusal::{SaveFailure, SaveOrigin};
#[cfg(target_arch = "wasm32")]
pub(super) use save_requests::{draft_save_request, restore_request};
#[cfg(target_arch = "wasm32")]
pub(super) use save_submission::submit_save;
