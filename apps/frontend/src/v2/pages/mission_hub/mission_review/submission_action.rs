//! The control that submits a mission for review, and what its author is told when it is refused.
//!
//! **Role:** one button that compiles the mission's current version into an artifact and opens its
//! review, and the panel under it that names why a refused submission was refused — with every
//! finding the compile listed.
//! **Position:** in the library dossier's Manage row, and in the review record of a mission that
//! predates reviews, which must be resubmitted before anyone can decide it.
//! **Signals & state:** owns the in-flight flag and the last refusal.
//! **Invariants:** the refusal stays on screen until the next attempt, because its findings are a
//! list the author works through rather than a notice to glance at. The submission is
//! browser-only.

use super::submission_refusal::SubmissionRefusal;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// Submit one mission for review.
#[component]
pub(crate) fn SubmitForReview(
    mission_id: String,
    /// The button's words: "Submit for review", or "Resubmit for review" on a returned mission.
    label: &'static str,
    /// Runs once the backend has opened the review.
    on_submitted: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let busy = RwSignal::new(false);
    let refusal = RwSignal::new(None::<SubmissionRefusal>);
    let mission = StoredValue::new(mission_id);
    let submit = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::endpoints::mission_reviews::submit_mission_for_review;
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            refusal.set(None);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let id = mission.get_value();
            leptos::task::spawn_local(async move {
                match submit_mission_for_review(store, &id).await {
                    Ok(_) => {
                        toasts.success(
                            "Submitted for review — a reviewer now decides the compiled artifact",
                        );
                        on_submitted.run(());
                    }
                    Err(refused) => {
                        let _ = refusal.try_set(Some(SubmissionRefusal::from_refusal(
                            &refused,
                            "The mission could not be submitted for review",
                        )));
                    }
                }
                let _ = busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (store, mission, on_submitted);
    };
    view! {
        <div class="flex flex-col gap-2">
            <button
                type="button"
                on:click=submit
                prop:disabled=move || busy.get()
                data-testid="mission-submit-for-review"
                class="flex w-fit items-center gap-2 rounded-lg border border-primary/30 bg-primary/15 px-4 py-2 text-label-md font-semibold text-primary transition-colors hover:bg-primary/25 disabled:opacity-60"
            >
                <MaterialIcon name="send" class="text-[16px]" />
                {label}
            </button>
            {move || refusal.get().map(refusal_panel)}
        </div>
    }
}

/// Why the submission was refused, and every finding the compile listed.
fn refusal_panel(refusal: SubmissionRefusal) -> impl IntoView {
    let findings = refusal.findings().cloned();
    view! {
        <div
            role="alert"
            data-testid="submission-refusal"
            class="rounded-xl border border-error-alert/30 bg-error-alert/10 p-4 text-sm"
        >
            <p class="text-on-surface">{refusal.sentence()}</p>
            {findings
                .filter(|f| !f.shown.is_empty())
                .map(|findings| {
                    let unlisted = findings.unlisted();
                    view! {
                        <ul class="mt-3 max-h-48 list-disc space-y-1 overflow-y-auto pl-5 font-mono text-code-md text-error-alert">
                            {findings
                                .shown
                                .into_iter()
                                .map(|finding| view! { <li class="break-all">{finding}</li> })
                                .collect_view()}
                        </ul>
                        {(unlisted > 0)
                            .then(|| {
                                view! {
                                    <p class="mt-2 text-xs text-on-surface-variant">
                                        {format!("… and {unlisted} more the compiler counted.")}
                                    </p>
                                }
                            })}
                    }
                })}
        </div>
    }
}
