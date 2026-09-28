//! The validation report: what the last upload answered, and every calibration case it failed.
//!
//! **Role:** renders the outcome headline in its tone and, for a calibration refusal, the table of
//! failed cases with what differs in each.
//! **Position:** below the upload form of the `/admin/ballistics-catalogs` route; reads the
//! outcome the form writes.
//! **Signals & state:** reads [`UploadDesk::outcome`](super::upload_form::UploadDesk) only.
//! **Invariants:** nothing renders before a send, and every failure the answer carried is listed —
//! the table is never truncated, because each row names a case the catalog has to be fixed for.
#![allow(dead_code)]

use super::view_model::{
    outcome_failures, outcome_headline, outcome_key, OutcomeTone, UploadOutcome,
};
use crate::v2::core::api::dto::ballistics_catalogs::CalibrationFailure;
use crate::v2::core::ui::cn;
use leptos::prelude::*;

/// The border, background and text classes of a tone.
pub(super) fn tone_classes(tone: OutcomeTone) -> &'static str {
    match tone {
        OutcomeTone::Success => "border-success/30 bg-success/10 text-success",
        OutcomeTone::Failure => "border-error/40 bg-error/10 text-error",
        OutcomeTone::Warning => {
            "border-tactical-yellow/40 bg-tactical-yellow/10 text-tactical-yellow"
        }
    }
}

/// The report panel: empty until a send finishes.
pub(super) fn report_panel(outcome: RwSignal<Option<UploadOutcome>>) -> impl IntoView {
    move || outcome.get().map(|outcome| report_view(&outcome))
}

/// One outcome: the headline, then the failed cases when there are any.
fn report_view(outcome: &UploadOutcome) -> impl IntoView {
    let (tone, headline) = outcome_headline(outcome);
    let failures = outcome_failures(outcome).to_vec();
    view! {
        <section
            class="space-y-3"
            data-testid="ballistics-catalog-report"
            data-upload-outcome=outcome_key(outcome)
        >
            <p
                role="status"
                class=cn(&["rounded-xl border px-4 py-3 text-label-md", tone_classes(tone)])
            >
                {headline}
            </p>
            {(!failures.is_empty()).then(|| failure_table(failures))}
        </section>
    }
}

/// The failed cases, one row each.
fn failure_table(failures: Vec<CalibrationFailure>) -> impl IntoView {
    view! {
        <div class="max-h-[28rem] overflow-auto rounded-xl border border-white/10">
            <table class="w-full text-label-md">
                <thead class="sticky top-0 z-10 bg-surface-container-high/80 text-label-sm text-on-surface-variant uppercase backdrop-blur-md">
                    <tr>
                        <th class="px-4 py-2 text-left font-medium">"Case"</th>
                        <th class="px-4 py-2 text-left font-medium">"What differs"</th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-white/5">
                    {failures
                        .into_iter()
                        .map(|failure| {
                            view! {
                                <tr>
                                    <td class="px-4 py-2 align-top font-mono text-code-md text-on-surface break-all">
                                        {failure.case_id}
                                    </td>
                                    <td class="px-4 py-2 text-on-surface-variant">
                                        {failure.reason}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
