//! What an author is told when a submission for review is refused.
//!
//! **Role:** reads a refused submission into one of the reasons the backend names, words each one,
//! and carries the findings a compile refusal lists — each an authored path the author can go and
//! fix.
//! **Position:** read by the library dossier's submit action and by the review record's
//! resubmission of a mission that predates reviews.
//! **Signals & state:** none; pure over the refusal.
//! **Invariants:** a submission that does not compile answers 422 with `details.code` one of
//! `NO_PLACED_SLOTS`, `UNCOMPILABLE_VERSION`, `DOCUMENT_CONTRACT_VIOLATION` or
//! `UNSUPPORTED_AUTHORED_DATA`, with `details.finding_count` and at most twenty
//! `details.findings`; the count is kept so a capped list says how many it leaves out. A refusal
//! without a known reason — a mission not in a submittable state, a mission with no saved version —
//! falls back to the backend's own sentence.

use crate::v2::core::api::client::ApiRefusal;
use serde_json::Value;

/// Why a submission was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SubmissionRefusal {
    /// The current version places no slot, so there is nothing to play.
    NoPlacedSlots,
    /// The current version does not compile into a mission document.
    UncompilableVersion(RefusalFindings),
    /// The compiled document breaks the mission contract the game runtime loads.
    DocumentContractViolation(RefusalFindings),
    /// The version authors gameplay data the document cannot carry; each finding is an authored
    /// path.
    UnsupportedAuthoredData(RefusalFindings),
    /// Any other refusal, carrying the sentence to show.
    Other(String),
}

/// The findings a compile refusal lists, and how many there are in all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RefusalFindings {
    pub(crate) shown: Vec<String>,
    pub(crate) total: usize,
}

impl RefusalFindings {
    /// The findings and their total, read from a refusal's details.
    fn read(refusal: &ApiRefusal) -> Self {
        let details = refusal.details.as_ref();
        let shown: Vec<String> = details
            .and_then(|d| d.get("findings"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let total = details
            .and_then(|d| d.get("finding_count"))
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(shown.len())
            .max(shown.len());
        Self { shown, total }
    }

    /// How many findings the backend counted but did not list.
    pub(crate) fn unlisted(&self) -> usize {
        self.total - self.shown.len()
    }
}

impl SubmissionRefusal {
    /// Read a refused submission; `fallback` is shown when the backend sent no sentence.
    pub(crate) fn from_refusal(refusal: &ApiRefusal, fallback: &str) -> Self {
        match refusal.code() {
            Some("NO_PLACED_SLOTS") => Self::NoPlacedSlots,
            Some("UNCOMPILABLE_VERSION") => {
                Self::UncompilableVersion(RefusalFindings::read(refusal))
            }
            Some("DOCUMENT_CONTRACT_VIOLATION") => {
                Self::DocumentContractViolation(RefusalFindings::read(refusal))
            }
            Some("UNSUPPORTED_AUTHORED_DATA") => {
                Self::UnsupportedAuthoredData(RefusalFindings::read(refusal))
            }
            _ => Self::Other(refusal.message_or(fallback)),
        }
    }

    /// The sentence the author is shown.
    pub(crate) fn sentence(&self) -> String {
        match self {
            Self::NoPlacedSlots => "The current version places no slots, so there is nothing to \
                                    review. Place the playable slots in the Mission Creator, save \
                                    a version, and submit again."
                .to_string(),
            Self::UncompilableVersion(_) => "The current version does not compile into a mission \
                                             document. Fix what the compiler reports, save a \
                                             version, and submit again."
                .to_string(),
            Self::DocumentContractViolation(_) => "The compiled document breaks the mission \
                                                   contract the game server loads. Fix each \
                                                   finding, save a version, and submit again."
                .to_string(),
            Self::UnsupportedAuthoredData(_) => "The current version authors gameplay data the \
                                                 mission document cannot carry, so it would not \
                                                 play as authored. Remove or change each authored \
                                                 path below, save a version, and submit again."
                .to_string(),
            Self::Other(sentence) => sentence.clone(),
        }
    }

    /// The findings the refusal lists, when it lists any.
    pub(crate) fn findings(&self) -> Option<&RefusalFindings> {
        match self {
            Self::UncompilableVersion(f)
            | Self::DocumentContractViolation(f)
            | Self::UnsupportedAuthoredData(f) => Some(f),
            Self::NoPlacedSlots | Self::Other(_) => None,
        }
    }
}
