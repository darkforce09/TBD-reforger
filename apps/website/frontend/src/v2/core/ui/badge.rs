//! Status pill classes, by variant name.
//!
//! **Role:** maps a variant name to the border, fill and text colours of a small status pill.
//! **Position:** used inline wherever a row or card carries a status.
//! **Signals & state:** none.
//! **Invariants:** an unknown variant falls back to the neutral pill rather than panicking, so a
//! status string that arrives from the backend can be passed straight through.

/// The full class string for a status pill of the named variant.
///
/// Known variants are the primary, tertiary, warning, success and error tones; anything else returns
/// the neutral pill.
#[allow(dead_code)]
pub fn badge_class(variant: &str) -> String {
    let v = match variant {
        "primary" => "border-primary/30 bg-primary/10 text-primary",
        "tertiary" => "border-tertiary/30 bg-tertiary/10 text-tertiary",
        "warning" => "border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow",
        "success" => "border-success/30 bg-success/15 text-success",
        "error" => "border-error-alert/30 bg-error-alert/10 text-error-alert",
        _ => "border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant",
    };
    format!("inline-flex items-center gap-1 rounded border px-2 py-0.5 text-label-sm uppercase whitespace-nowrap {v}")
}
