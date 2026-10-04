//! The review-record items most pages name, for `use mission_review_record::prelude::*;`.
//!
//! **Role:** re-exports the review record component, the submission control, the history and
//! provenance views and the wording the approvals pages and the review workspace read.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module; the browser-only components
//! are re-exported only where they exist, on `wasm32`.

pub use crate::artifact_provenance_view::{artifact_provenance, diagnostics_list};
#[cfg(target_arch = "wasm32")]
pub use crate::comment_composer::ReviewCommentComposer;
pub use crate::error::{Error, Result};
pub use crate::review_history_view::{review_list, review_thread};
#[cfg(target_arch = "wasm32")]
pub use crate::review_record::MissionReviewRecord;
pub use crate::review_wording::{
    review_workspace_href, reviewed_artifact_line, short_digest, validated_review_text,
};
#[cfg(target_arch = "wasm32")]
pub use crate::submission_action::SubmitForReview;
pub use crate::submission_refusal::{RefusalFindings, SubmissionRefusal};
