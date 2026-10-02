//! The mission's tactical briefing section.
//!
//! **Role:** the one block of prose on the dossier — the operation summary an author writes for
//! the library, shown before anyone joins.
//! **Position:** a section of the shared dossier body, between the badges and the detail grid.
//! **Signals & state:** none; it renders the briefing it is handed.
//! **Invariants:** a whitespace-only briefing is not authored content and takes the empty
//! affordance, rather than leaving a heading over blank pre-wrapped space.

use super::dossier_body::tactical_briefing_text;
use crate::v2::core::api::dto::MissionDetail;
use leptos::prelude::*;

/// The briefing section, with its empty affordance already applied.
pub(super) fn briefing_section(m: &MissionDetail) -> impl IntoView {
    let briefing = tactical_briefing_text(m.briefing.as_deref());
    view! {
            <section>
                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                    "Tactical Briefing"
                </h3>
                <p class="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
                    {briefing}
                </p>
            </section>
    }
}
