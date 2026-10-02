//! The review drawer: the mission being decided on, the artifact under review, its history, and the
//! decision.
//!
//! **Role:** the submission's header; the notice a stale decision left; the mission's own briefing
//! and settings; the provenance and compile findings of the artifact under review with the link to
//! its read-only review workspace; every earlier review and the thread with a comment box; and the
//! decision form. A mission that predates reviews shows why nothing can be decided instead.
//! **Position:** the detail pane of the approvals route, beside the pending queue.
//! **Signals & state:** the mission detail, the review history and the artifact each live in a
//! `LocalResource` keyed on the selected submission; posting a comment re-reads the history. The
//! decision form owns its own state and writes the desk.
//! **Invariants:** a decision is offered only while a review is under way, and it names that
//! review's artifact. This is a component rather than a plain function because it owns fetches and
//! the caller invokes it per selected row: a component gets its own owner, so changing rows
//! disposes the previous fetches instead of stacking them.
#![allow(dead_code)]

use super::page::ApprovalsDesk;
use super::review_briefing::briefing_and_settings;
use super::review_decision::decision_form;
use super::submission_queue::terrain_label;
use crate::v2::core::api::dto::{
    ApprovalRow, MissionArtifact, MissionDetail, MissionReviewHistory,
};
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::datefmt::format_local_datetime;
use crate::v2::pages::mission_hub::mission_review::artifact_provenance_view::artifact_provenance;
use crate::v2::pages::mission_hub::mission_review::comment_composer::ReviewCommentComposer;
use crate::v2::pages::mission_hub::mission_review::review_history_view::{
    review_list, review_thread,
};
use crate::v2::pages::mission_hub::mission_review::review_wording::{
    review_workspace_href, short_digest,
};
use leptos::prelude::*;

/// A read that either answered or failed with the sentence to show.
type Read<T> = Result<T, String>;

/// The review surface for one pending submission.
#[component]
pub(super) fn ReviewInspector(row: ApprovalRow, desk: ApprovalsDesk) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));
    let mid = StoredValue::new(row.mission_id.clone());
    let under_review = row.artifact_id.clone();
    let history_reload = RwSignal::new(0u32);

    // The mission's own briefing and settings: an administrator may read a pending mission, so the
    // words under the header belong to the mission being reviewed.
    let detail = LocalResource::new(move || {
        let id = mid.get_value();
        async move { read_detail(store, id).await }
    });
    let history = LocalResource::new(move || {
        let _ = history_reload.get();
        let id = mid.get_value();
        async move { read_history(store, id).await }
    });
    let artifact_id = StoredValue::new(under_review.clone());
    let artifact = LocalResource::new(move || {
        let id = mid.get_value();
        let artifact = artifact_id.get_value();
        async move {
            match artifact {
                Some(artifact) => Some(read_artifact(store, id, artifact).await),
                None => None,
            }
        }
    });
    let posted = Callback::new(move |_| history_reload.update(|n| *n = n.wrapping_add(1)));
    let notice = move || {
        desk.notice
            .get()
            .filter(|(mission, _)| *mission == mid.get_value())
            .map(|(_, text)| text)
    };

    view! {
        <div class="flex min-h-full flex-col">
            {drawer_header(&row)}
            <div class="space-y-8 px-8 py-7">
                {move || {
                    notice()
                        .map(|text| {
                            view! {
                                <p
                                    role="alert"
                                    class="rounded-xl border border-tactical-yellow/40 bg-tactical-yellow/10 px-4 py-3 text-sm text-on-surface"
                                >
                                    {text}
                                </p>
                            }
                        })
                }}
                {under_review.is_none().then(predates_reviews_notice)}
                <section>
                    <Suspense fallback=move || {
                        view! { <p class="text-body-md text-on-surface-variant">"Loading briefing…"</p> }
                    }>{move || detail.get().map(|read| briefing_and_settings(read.ok()))}</Suspense>
                </section>
                {under_review
                    .clone()
                    .map(|artifact_id| {
                        let semver = row.version_semver.clone().unwrap_or_default();
                        let href = review_workspace_href(&row.mission_id, &artifact_id);
                        view! {
                            <section class="space-y-3">
                                <div class="flex flex-wrap items-center justify-between gap-2">
                                    <h2 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                                        "Artifact under review"
                                    </h2>
                                    <a
                                        href=href
                                        target="_blank"
                                        rel="noopener"
                                        data-testid="approvals-open-review-workspace"
                                        class="flex items-center gap-1.5 rounded-full border border-primary/40 bg-primary/10 px-4 py-2 text-label-md text-primary transition hover:bg-primary/20"
                                    >
                                        <MaterialIcon name="open_in_new" class="text-[16px]" />
                                        "Open the read-only review workspace"
                                    </a>
                                </div>
                                <Suspense fallback=move || {
                                    view! { <p class="text-sm text-on-surface-variant">"Loading the artifact…"</p> }
                                }>
                                    {move || {
                                        artifact
                                            .get()
                                            .flatten()
                                            .map(|read| artifact_section(read, semver.clone()))
                                    }}
                                </Suspense>
                            </section>
                        }
                    })}
                <section class="space-y-4">
                    <h2 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                        "Review record"
                    </h2>
                    <Suspense fallback=move || {
                        view! { <p class="text-sm text-on-surface-variant">"Loading the review record…"</p> }
                    }>
                        {move || {
                            history
                                .get()
                                .map(|read| {
                                    history_section(read, mid.get_value(), me.get_value())
                                })
                        }}
                    </Suspense>
                    <ReviewCommentComposer
                        mission_id=row.mission_id.clone()
                        artifact_id=under_review.clone()
                        on_posted=posted
                        placeholder="Comment on the artifact under review — its author reads the thread."
                    />
                </section>
            </div>
            {under_review
                .map(|artifact_id| decision_form(row.mission_id.clone(), artifact_id, desk))}
        </div>
    }
}

/// The cinematic header: status, terrain, author, the version and artifact, and when it was
/// submitted.
fn drawer_header(row: &ApprovalRow) -> impl IntoView {
    let badges = [
        row.version_semver
            .as_ref()
            .map(|semver| format!("v{semver}")),
        row.artifact_digest
            .as_ref()
            .map(|digest| format!("artifact {}", short_digest(digest))),
    ];
    view! {
        <div class="relative h-64 shrink-0 bg-topo-map bg-cover bg-center">
            <div class="absolute inset-0 bg-gradient-to-t from-surface-glass to-transparent"></div>
            <div class="absolute inset-x-0 bottom-0 p-8">
                <div class="mb-3 flex flex-wrap items-center gap-2">
                    <span class="rounded-full border border-tactical-yellow/40 bg-tactical-yellow/20 px-3 py-1 text-label-sm font-medium text-tactical-yellow backdrop-blur-md">
                        "Pending review"
                    </span>
                    <span class="rounded-full bg-white/10 px-3 py-1 text-label-sm text-on-surface backdrop-blur-md">
                        {terrain_label(&row.terrain)}
                    </span>
                    <span class="rounded-full bg-white/10 px-3 py-1 text-label-sm text-on-surface backdrop-blur-md">
                        {row.author_name.clone()}
                    </span>
                    {badges
                        .into_iter()
                        .flatten()
                        .map(|badge| {
                            view! {
                                <span class="rounded-full bg-white/10 px-3 py-1 font-mono text-code-md text-on-surface backdrop-blur-md">
                                    {badge}
                                </span>
                            }
                        })
                        .collect_view()}
                    <span class="rounded-full bg-white/10 px-3 py-1 font-mono text-code-md text-on-surface backdrop-blur-md">
                        {format_local_datetime(&row.submitted_at)}
                    </span>
                </div>
                <h1 class="text-headline-lg text-on-surface drop-shadow-lg">{row.title.clone()}</h1>
            </div>
        </div>
    }
}

/// Why a mission that predates reviews cannot be decided here.
fn predates_reviews_notice() -> impl IntoView {
    view! {
        <p class="rounded-xl border border-tactical-yellow/40 bg-tactical-yellow/10 px-4 py-3 text-sm text-on-surface">
            "This mission was submitted before reviews existed, so no artifact is under review and there is nothing to decide. Its author resubmits it from the mission hub, which compiles its current version into an artifact that appears here."
        </p>
    }
}

/// The artifact's provenance and findings, or why it could not be read.
fn artifact_section(read: Read<MissionArtifact>, semver: String) -> impl IntoView {
    match read {
        Ok(artifact) => artifact_provenance(&artifact, &semver).into_any(),
        Err(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
    }
}

/// Every review and the thread, or why the record could not be read.
fn history_section(
    read: Read<MissionReviewHistory>,
    mission_id: String,
    me: Option<String>,
) -> impl IntoView {
    match read {
        Ok(history) => view! {
            {review_list(&mission_id, &history, None, me.as_deref())}
            <h3 class="font-mono text-label-sm tracking-widest text-outline uppercase">"Thread"</h3>
            {review_thread(&history.comments, me.as_deref())}
        }
        .into_any(),
        Err(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
    }
}

/// The mission's detail.
async fn read_detail(store: crate::v2::core::auth::AuthStore, id: String) -> Read<MissionDetail> {
    #[cfg(target_arch = "wasm32")]
    {
        let path = format!(
            "/missions/{}",
            crate::v2::core::api::endpoints::encode_path_segment(&id)
        );
        crate::v2::core::api::client::api_get::<MissionDetail>(store, &path)
            .await
            .map_err(|e| crate::v2::core::api::client::api_error_message(&e, "Unreadable"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (store, id);
        Err("The mission is read in the browser".to_string())
    }
}

/// The mission's review history.
async fn read_history(
    store: crate::v2::core::auth::AuthStore,
    id: String,
) -> Read<MissionReviewHistory> {
    #[cfg(target_arch = "wasm32")]
    {
        use crate::v2::core::api::endpoints::mission_reviews::load_review_history;
        load_review_history(store, &id).await.map_err(|e| {
            crate::v2::core::api::client::api_error_message(
                &e,
                "The review record could not be read",
            )
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (store, id);
        Err("The review record is read in the browser".to_string())
    }
}

/// The artifact under review.
async fn read_artifact(
    store: crate::v2::core::auth::AuthStore,
    id: String,
    artifact: String,
) -> Read<MissionArtifact> {
    #[cfg(target_arch = "wasm32")]
    {
        use crate::v2::core::api::endpoints::mission_reviews::load_mission_artifact;
        load_mission_artifact(store, &id, &artifact)
            .await
            .map_err(|e| {
                crate::v2::core::api::client::api_error_message(
                    &e,
                    "The artifact under review could not be read",
                )
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (store, id, artifact);
        Err("The artifact is read in the browser".to_string())
    }
}
