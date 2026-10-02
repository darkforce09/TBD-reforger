//! The reviews of one mission and its thread, as the author and the reviewer both read them.
//!
//! **Role:** renders every review — its state, the version and artifact it decides, who submitted
//! it and when, how it ended, the conditions or rejection reason it carries, whether its artifact is
//! the one deployments run, and the link to its read-only review workspace — and then the thread,
//! every comment in the order it was written.
//! **Position:** inside the mission hub's review record and the approvals drawer.
//! **Signals & state:** none; the history is handed in already read.
//! **Invariants:** reviews are listed newest first, as the backend sends them, and the thread in
//! writing order. The approved artifact is marked on every review that decided it, because a
//! mission rejected and resubmitted can carry the same artifact through more than one review.

use super::review_wording::{
    account_label, comment_kind_label, decision_comment, decision_line, review_state_label,
    review_state_tone, review_workspace_href, reviewed_artifact_line, short_digest,
    submission_line,
};
use crate::v2::core::api::dto::{MissionReview, MissionReviewHistory, ReviewComment};
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::utc_timestamp::utc_label;
use leptos::prelude::*;

/// Every review of the mission, newest first.
pub(crate) fn review_list(
    mission_id: &str,
    history: &MissionReviewHistory,
    approved_artifact_id: Option<&str>,
    me: Option<&str>,
) -> impl IntoView {
    if history.reviews.is_empty() {
        return view! {
            <p class="text-sm text-on-surface-variant">"No review has been opened yet."</p>
        }
        .into_any();
    }
    view! {
        <ol class="space-y-3" data-testid="review-history">
            {history
                .reviews
                .iter()
                .map(|review| {
                    let approved = approved_artifact_id == Some(review.artifact_id.as_str())
                        && review.state.starts_with("approved");
                    review_card(mission_id, history, review, approved, me)
                })
                .collect_view()}
        </ol>
    }
    .into_any()
}

/// One review.
fn review_card(
    mission_id: &str,
    history: &MissionReviewHistory,
    review: &MissionReview,
    approved: bool,
    me: Option<&str>,
) -> impl IntoView {
    let decision = decision_comment(history, &review.id)
        .map(|(kind, body)| (comment_kind_label(kind), body.to_string()));
    let workspace = review_workspace_href(mission_id, &review.artifact_id);
    view! {
        <li class="rounded-xl border border-white/10 bg-white/[0.02] p-4 text-sm">
            <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="flex flex-wrap items-center gap-2">
                    <span class=badge_class(review_state_tone(&review.state))>
                        {review_state_label(&review.state)}
                    </span>
                    <span class="font-mono text-code-md text-on-surface">
                        {reviewed_artifact_line(&review.semver, &review.artifact_digest)}
                    </span>
                    {approved
                        .then(|| {
                            view! {
                                <span class=badge_class("success")>
                                    <MaterialIcon name="verified" class="text-[14px]" />
                                    "Approved artifact — deployments run it"
                                </span>
                            }
                        })}
                </div>
                <a
                    href=workspace
                    target="_blank"
                    rel="noopener"
                    class="flex items-center gap-1 rounded-full border border-white/10 px-3 py-1 text-xs text-primary transition hover:bg-white/5"
                >
                    <MaterialIcon name="open_in_new" class="text-[14px]" />
                    "Open review workspace"
                </a>
            </div>
            <p class="mt-2 text-xs text-on-surface-variant">{submission_line(review, me)}</p>
            <p class="text-xs text-on-surface-variant">{decision_line(review, me)}</p>
            {decision
                .map(|(label, body)| {
                    view! {
                        <div class="mt-3 rounded-lg border border-white/10 bg-black/20 px-3 py-2">
                            <p class="font-mono text-label-sm tracking-wider text-outline uppercase">
                                {label}
                            </p>
                            <p class="mt-1 whitespace-pre-line text-on-surface">{body}</p>
                        </div>
                    }
                })}
        </li>
    }
}

/// The thread: every comment in the order it was written.
pub(crate) fn review_thread(comments: &[ReviewComment], me: Option<&str>) -> impl IntoView {
    if comments.is_empty() {
        return view! {
            <p class="text-sm text-on-surface-variant">"Nobody has commented yet."</p>
        }
        .into_any();
    }
    view! {
        <ol class="space-y-2" data-testid="review-thread">
            {comments.iter().map(|comment| thread_entry(comment, me)).collect_view()}
        </ol>
    }
    .into_any()
}

/// One comment: who wrote it, when, what kind of entry it is, and the artifact it concerns.
fn thread_entry(comment: &ReviewComment, me: Option<&str>) -> impl IntoView {
    let author = if comment.author_name.trim().is_empty() || me == Some(comment.author_id.as_str())
    {
        account_label(&comment.author_id, me)
    } else {
        comment.author_name.clone()
    };
    let about = comment
        .artifact_id
        .as_deref()
        .map(|artifact| format!(" · about artifact {}", short_digest(artifact)));
    let tone = match comment.kind.as_str() {
        "rejection" => "border-error-alert/30",
        "approval_conditions" => "border-tertiary/30",
        _ => "border-white/10",
    };
    view! {
        <li class=format!("rounded-lg border {tone} bg-white/[0.02] px-3 py-2 text-sm")>
            <p class="text-xs text-on-surface-variant">
                <span class="font-semibold text-on-surface">{author}</span>
                " · "
                {comment_kind_label(&comment.kind)}
                " · "
                {utc_label(&comment.created_at)}
                {about}
            </p>
            <p class="mt-1 whitespace-pre-line text-on-surface">{comment.body.clone()}</p>
        </li>
    }
}
