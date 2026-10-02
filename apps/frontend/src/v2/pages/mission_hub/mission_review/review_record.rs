//! A mission's review record, as its author and administrators read it in the mission hub.
//!
//! **Role:** reads the mission's review history and renders it — the approved artifact first, then
//! every review with its decision, conditions or rejection reason and its review workspace link,
//! then the thread — with a reply box under the thread.
//! **Position:** under the dossier on `/missions/:id` and in the library's dossier sheet, for the
//! mission's author and for administrators, the two the backend serves the history to.
//! **Signals & state:** owns whether the history is wanted and a reload counter; the history itself
//! lives in a `LocalResource` keyed on both, so a posted reply re-reads it.
//! **Invariants:** the history is read without being asked for only when the mission's row shows a
//! review has happened — submitted, decided or approved; a draft that never left its author's
//! hands shows a control to look instead of a request nobody needs. A reply concerns the artifact
//! of the newest review. A mission awaiting approval with no review under way predates reviews:
//! nobody can decide it until it is resubmitted, so the record offers exactly that. Every read is
//! browser-only.

use super::comment_composer::ReviewCommentComposer;
use super::review_history_view::{review_list, review_thread};
use super::review_wording::{has_review_trace, pending_review, reviewed_artifact_line};
use super::submission_action::SubmitForReview;
use crate::v2::core::api::dto::MissionReviewHistory;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The review history as read: not asked for, in flight, refused with a reason, or answered.
#[derive(Clone, PartialEq)]
enum HistoryRead {
    NotAsked,
    Failed(String),
    Loaded(MissionReviewHistory),
}

/// The approved artifact, named by the review that decided it when the history holds one.
pub(crate) fn approved_artifact_line(
    history: &MissionReviewHistory,
    approved_artifact_id: &str,
) -> String {
    match history
        .reviews
        .iter()
        .find(|r| r.artifact_id == approved_artifact_id && r.state.starts_with("approved"))
    {
        Some(review) => format!(
            "Approved artifact: {} — deployments run exactly these bytes.",
            reviewed_artifact_line(&review.semver, &review.artifact_digest)
        ),
        None => format!(
            "Approved artifact {approved_artifact_id} — deployments run exactly these bytes."
        ),
    }
}

/// The review record of one mission.
#[component]
pub(crate) fn MissionReviewRecord(
    mission_id: String,
    /// The mission's status, as its row carries it.
    status: String,
    /// When the mission was last decided, as its row carries it.
    reviewed_at: Option<String>,
    /// The artifact the latest approval decided.
    approved_artifact_id: Option<String>,
    /// Runs once a mission that predates reviews has been resubmitted.
    on_resubmitted: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));
    let wanted = RwSignal::new(has_review_trace(
        &status,
        reviewed_at.as_deref(),
        approved_artifact_id.as_deref(),
    ));
    let reload = RwSignal::new(0u32);
    let mission = StoredValue::new(mission_id);
    let approved = StoredValue::new(approved_artifact_id);
    let awaiting_approval = status == "pending_approval";
    let history = LocalResource::new(move || {
        let asked = wanted.get();
        let _ = reload.get();
        let id = mission.get_value();
        async move {
            if !asked {
                return HistoryRead::NotAsked;
            }
            #[cfg(target_arch = "wasm32")]
            {
                use crate::v2::core::api::endpoints::mission_reviews::load_review_history;
                match load_review_history(store, &id).await {
                    Ok(history) => HistoryRead::Loaded(history),
                    Err(e) => HistoryRead::Failed(crate::v2::core::api::client::api_error_message(
                        &e,
                        "The review record could not be read",
                    )),
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                HistoryRead::Failed("The review record is read in the browser".to_string())
            }
        }
    });
    let posted = Callback::new(move |_| reload.update(|n| *n = n.wrapping_add(1)));
    let resubmitted = Callback::new(move |()| {
        reload.update(|n| *n = n.wrapping_add(1));
        on_resubmitted.run(());
    });

    view! {
        <section class="space-y-4" data-testid="mission-review-record">
            <h3 class="flex items-center gap-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                <MaterialIcon name="fact_check" class="text-[18px]" />
                "Review record"
            </h3>
            <Suspense fallback=move || {
                view! { <p class="text-sm text-on-surface-variant">"Loading the review record…"</p> }
            }>
                {move || {
                    history
                        .get()
                        .map(|read| match read {
                            HistoryRead::NotAsked => {
                                view! {
                                    <div class="flex flex-wrap items-center gap-3 text-sm text-on-surface-variant">
                                        <p>"This mission has not been submitted for review."</p>
                                        <button
                                            type="button"
                                            on:click=move |_| wanted.set(true)
                                            class="rounded-full border border-white/10 px-3 py-1 text-xs text-primary transition hover:bg-white/5"
                                        >
                                            "Look for earlier reviews"
                                        </button>
                                    </div>
                                }
                                    .into_any()
                            }
                            HistoryRead::Failed(why) => {
                                view! { <p class="text-sm text-error-alert">{why}</p> }.into_any()
                            }
                            HistoryRead::Loaded(history) => {
                                let predates_reviews = awaiting_approval
                                    && pending_review(&history).is_none();
                                view! {
                                    {predates_reviews
                                        .then(|| resubmission_notice(mission.get_value(), resubmitted))}
                                    {record_body(
                                        mission.get_value(),
                                        history,
                                        approved.get_value(),
                                        me.get_value(),
                                        posted,
                                    )}
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

/// A mission submitted before reviews existed: nothing is under review, so nothing can be decided
/// until its current version is compiled by a resubmission.
fn resubmission_notice(mission_id: String, on_submitted: Callback<()>) -> impl IntoView {
    view! {
        <div class="space-y-3 rounded-xl border border-tactical-yellow/30 bg-tactical-yellow/10 p-4 text-sm">
            <p class="text-on-surface">
                "This mission was submitted before reviews existed, so no artifact is under review and no reviewer can decide it. Resubmit it to compile its current version into an artifact a reviewer can decide."
            </p>
            <SubmitForReview
                mission_id=mission_id
                label="Resubmit for review"
                on_submitted=on_submitted
            />
        </div>
    }
}

/// The record once read: the approved artifact, the reviews, the thread and the reply box.
fn record_body(
    mission_id: String,
    history: MissionReviewHistory,
    approved_artifact_id: Option<String>,
    me: Option<String>,
    posted: Callback<crate::v2::core::api::dto::ReviewComment>,
) -> impl IntoView {
    let approved_line = approved_artifact_id
        .as_deref()
        .map(|approved| approved_artifact_line(&history, approved));
    let reply_artifact = history.reviews.first().map(|r| r.artifact_id.clone());
    view! {
        {approved_line
            .map(|line| {
                view! {
                    <p class="rounded-lg border border-success/30 bg-success/10 px-3 py-2 text-sm text-success">
                        {line}
                    </p>
                }
            })}
        {review_list(&mission_id, &history, approved_artifact_id.as_deref(), me.as_deref())}
        <div class="space-y-2">
            <h4 class="font-mono text-label-sm tracking-widest text-outline uppercase">"Thread"</h4>
            {review_thread(&history.comments, me.as_deref())}
        </div>
        <ReviewCommentComposer
            mission_id=mission_id
            artifact_id=reply_artifact
            on_posted=posted
            placeholder="Reply to the review — your comment joins the thread about the newest artifact."
        />
    }
}
