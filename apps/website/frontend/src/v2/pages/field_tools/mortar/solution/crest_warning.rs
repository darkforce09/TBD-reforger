//! The crest-clearance line: whether the lead gun's flight clears the terrain on its way.
//!
//! **Role:** words the solution's [`CrestClearance`], or says why there is none.
//! **Position:** a line of the solution panel (`super`); the profile comes from the map picker.
//! **Signals & state:** none; pure functions and a view over plain values.
//! **Invariants:** a blocked flight is a warning naming the first blocking distance; a clear one
//! states the smallest clearance and where it is; no profile is said plainly, never shown as clear.

use leptos::prelude::*;
use website_map_engine::data::scenario::ballistics::crest_clearance::CrestClearance;

/// The crest sentence and whether it warns.
pub(crate) fn crest_text(crest: Option<&CrestClearance>) -> (bool, String) {
    match crest {
        None => (
            false,
            "No crest check: the map heights along the line of fire are not loaded.".to_string(),
        ),
        Some(c) => match c.first_blocking_downrange_m {
            Some(blocking) => (
                true,
                format!(
                    "Crest warning: the flight passes below the terrain {blocking:.0} m from the \
                     lead gun (lowest clearance {:.1} m at {:.0} m).",
                    c.min_clearance_m, c.min_clearance_downrange_m
                ),
            ),
            None => (
                false,
                format!(
                    "Crest clear: lowest clearance {:.1} m at {:.0} m from the lead gun.",
                    c.min_clearance_m, c.min_clearance_downrange_m
                ),
            ),
        },
    }
}

/// The crest line.
pub(crate) fn crest_warning(crest: Option<CrestClearance>) -> impl IntoView {
    let (warns, text) = crest_text(crest.as_ref());
    let class = if warns {
        "text-sm font-semibold text-error"
    } else {
        "text-sm text-on-surface-variant"
    };
    view! {
        <p class=class data-mortar-crest=if warns { "blocked" } else { "clear" }>
            {text}
        </p>
    }
}
