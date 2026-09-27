//! Why a wiki save was refused, read from the refusal, and the sentences that tell the author.
//!
//! **Role:** classifies a refused `PUT /wiki/{slug}` — a revision conflict, refused markup, an
//! oversized body or request, or any other refusal — and words each for the author, with one line
//! per refused construct.
//! **Position:** fed by the save submission with the [`ApiRefusal`] the request returned; read
//! by the problem view under the save button and in the revision view.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the structured `details` decide the kind when they parse as a
//! [`WikiSaveRefusal`]; otherwise the status does (409 is a conflict, 413 an oversized request).
//! A conflict offers a reload, nothing else does. A draft save's reload discards the draft; a
//! restore's reload keeps it.

use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::wiki::{WikiMarkupFinding, WikiSaveRefusal, WikiSaveRefusalCode};

/// The largest body the server keeps, in bytes, as the refusal sentence names it.
const BODY_LIMIT_TEXT: &str = "262 144 bytes";

/// Which write was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum SaveOrigin {
    /// The editor's save of a draft.
    Draft,
    /// The revision view's restore of an older revision.
    Restore,
}

/// What stopped a save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) enum SaveProblem {
    /// 409: the page moved on from the revision the save started from.
    RevisionConflict {
        /// The revision the save named as its base.
        base_revision: Option<i64>,
        /// The page's revision now, when the server named it.
        current_revision: Option<i64>,
    },
    /// 422: the markup holds constructs the wiki refuses; one finding each.
    MarkupRefused {
        /// Every refused construct, with its line.
        findings: Vec<WikiMarkupFinding>,
    },
    /// 400 `wiki_body_too_large`: the markdown is over the size limit.
    BodyTooLarge,
    /// 413: the request is larger than the server reads at all.
    RequestTooLarge,
    /// Any other refusal, as the sentence to show.
    Refused {
        /// The sentence shown to the author.
        message: String,
    },
}

/// A refused write: which one, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct SaveFailure {
    /// The write that was refused.
    pub(in super::super) origin: SaveOrigin,
    /// Why.
    pub(in super::super) problem: SaveProblem,
}

/// The wiki's reason inside `refusal`, when its `details` carry one.
fn wiki_details(refusal: &ApiRefusal) -> Option<WikiSaveRefusal> {
    let details = refusal.details.clone()?;
    serde_json::from_value(serde_json::Value::Object(details)).ok()
}

impl SaveProblem {
    /// Classifies `refusal`, the answer to a save whose base was `base_revision`.
    pub(in super::super) fn from_refusal(refusal: &ApiRefusal, base_revision: Option<i64>) -> Self {
        let details = wiki_details(refusal);
        let code = details.as_ref().map(|details| details.code);
        match (refusal.status, code) {
            (_, Some(WikiSaveRefusalCode::RevisionConflict)) | (409, _) => Self::RevisionConflict {
                base_revision,
                current_revision: details.and_then(|details| details.current_revision),
            },
            (_, Some(WikiSaveRefusalCode::MarkupRefused)) => Self::MarkupRefused {
                findings: details
                    .and_then(|details| details.findings)
                    .unwrap_or_default(),
            },
            (_, Some(WikiSaveRefusalCode::BodyTooLarge)) => Self::BodyTooLarge,
            (413, _) => Self::RequestTooLarge,
            _ => Self::Refused {
                message: refusal_sentence(refusal),
            },
        }
    }

    /// The sentence that says what happened.
    pub(in super::super) fn headline(&self) -> String {
        match self {
            Self::RevisionConflict {
                base_revision,
                current_revision,
            } => match (base_revision, current_revision) {
                (Some(base), Some(current)) => format!(
                    "Not saved: this manual changed while you were working. You started from \
                     revision {base}; it is now at revision {current}."
                ),
                (_, Some(current)) => format!(
                    "Not saved: this manual changed while you were working; it is now at \
                     revision {current}."
                ),
                _ => "Not saved: this manual changed while you were working.".to_string(),
            },
            Self::MarkupRefused { findings } => match findings.len() {
                1 => "Not saved: the markup has 1 problem.".to_string(),
                count => format!("Not saved: the markup has {count} problems."),
            },
            Self::BodyTooLarge => format!(
                "Not saved: the text is over the {BODY_LIMIT_TEXT} a manual may hold. Split it \
                 into more than one manual."
            ),
            Self::RequestTooLarge => {
                "Not saved: the request is larger than the server accepts.".to_string()
            }
            Self::Refused { message } => message.clone(),
        }
    }

    /// One line per refused construct: `Line <n>: <detail>`; empty for every other problem.
    pub(in super::super) fn finding_lines(&self) -> Vec<String> {
        match self {
            Self::MarkupRefused { findings } => findings
                .iter()
                .map(|finding| format!("Line {}: {}", finding.line, finding.detail))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl SaveFailure {
    /// The label of the reload a conflict offers, or `None` for a problem a reload cannot fix.
    pub(in super::super) fn reload_label(&self) -> Option<String> {
        let SaveProblem::RevisionConflict {
            current_revision, ..
        } = &self.problem
        else {
            return None;
        };
        let target = current_revision.map_or_else(
            || "the latest revision".to_string(),
            |revision| format!("revision {revision}"),
        );
        Some(match self.origin {
            SaveOrigin::Draft => format!("Discard my draft and load {target}"),
            SaveOrigin::Restore => format!("Load {target}"),
        })
    }
}

/// The sentence for a refusal the wiki gives no reason for.
fn refusal_sentence(refusal: &ApiRefusal) -> String {
    match refusal.status {
        0 => "Not saved: the request did not reach the server. Check the connection and try again."
            .to_string(),
        401 => "Not saved: your session has ended. Sign in again, then save.".to_string(),
        403 => "Not saved: only an administrator can save a manual.".to_string(),
        _ => refusal.message_or("Failed to save wiki page"),
    }
}

#[cfg(test)]
#[path = "tests/save_refusal.rs"]
mod tests;
