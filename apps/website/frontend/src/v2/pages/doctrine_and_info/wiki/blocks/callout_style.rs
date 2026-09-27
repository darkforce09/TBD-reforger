//! How each of the seven callout kinds looks: the box colour and the label above its text.
//!
//! **Role:** maps a [`WikiCalloutKind`] to its box class, its label class and its label.
//! **Position:** read by the block mapper when it renders a callout block.
//! **Signals & state:** none; one pure function over constants.
//! **Invariants:** three colour families only — the informational primary blue (note, tip,
//! info), the tactical yellow (important, warning) and the error red (caution, critical) — and
//! every kind has its own label, so two kinds never read the same.

use crate::v2::core::api::dto::wiki::WikiCalloutKind;

/// The box of an informational callout: note, tip and info.
const INFORMATION_BOX_CLASS: &str = "my-6 rounded-2xl border border-l-4 border-primary bg-primary/10 p-4 shadow-lg backdrop-blur-md";
/// The label of an informational callout.
const INFORMATION_LABEL_CLASS: &str =
    "mb-1 font-mono text-xs font-bold tracking-widest text-primary uppercase";
/// The box of an attention callout: important and warning.
const ATTENTION_BOX_CLASS: &str = "my-6 rounded-2xl border border-l-4 border-tactical-yellow bg-tactical-yellow/10 p-4 shadow-lg backdrop-blur-md";
/// The label of an attention callout.
const ATTENTION_LABEL_CLASS: &str =
    "mb-1 font-mono text-xs font-bold tracking-widest text-tactical-yellow uppercase";
/// The box of a danger callout: caution and critical.
const DANGER_BOX_CLASS: &str =
    "my-6 rounded-2xl border border-l-4 border-error bg-error/10 p-4 shadow-lg backdrop-blur-md";
/// The label of a danger callout.
const DANGER_LABEL_CLASS: &str =
    "mb-1 font-mono text-xs font-bold tracking-widest text-error-alert uppercase";

/// The look of one callout kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CalloutStyle {
    /// The class of the outer box.
    pub(super) box_class: &'static str,
    /// The class of the label line.
    pub(super) label_class: &'static str,
    /// The label shown above the callout's text.
    pub(super) label: &'static str,
}

/// The look of a callout of `kind`.
pub(super) fn callout_style(kind: WikiCalloutKind) -> CalloutStyle {
    let (box_class, label_class) = match kind {
        WikiCalloutKind::Note | WikiCalloutKind::Tip | WikiCalloutKind::Info => {
            (INFORMATION_BOX_CLASS, INFORMATION_LABEL_CLASS)
        }
        WikiCalloutKind::Important | WikiCalloutKind::Warning => {
            (ATTENTION_BOX_CLASS, ATTENTION_LABEL_CLASS)
        }
        WikiCalloutKind::Caution | WikiCalloutKind::Critical => {
            (DANGER_BOX_CLASS, DANGER_LABEL_CLASS)
        }
    };
    let label = match kind {
        WikiCalloutKind::Note => "NOTE",
        WikiCalloutKind::Tip => "PRO-TIP",
        WikiCalloutKind::Important => "IMPORTANT",
        WikiCalloutKind::Info => "INFO",
        WikiCalloutKind::Warning => "WARNING",
        WikiCalloutKind::Caution => "CAUTION",
        WikiCalloutKind::Critical => "CRITICAL RULE",
    };
    CalloutStyle {
        box_class,
        label_class,
        label,
    }
}
