//! The composer that adds a comment to a mission's review thread.
//!
//! **Role:** a text box and a post control that send one comment, about the artifact under review
//! when there is one, and hand the posted comment back to the thread that shows it.
//! **Position:** under the review thread, in the approvals drawer and in the mission hub's review
//! record.
//! **Signals & state:** owns the draft, the in-flight flag and the problem last reported.
//! **Invariants:** a comment is trimmed and checked as the backend checks it — not blank, at most
//! 8000 bytes — before it is sent, and the draft is kept until the backend accepts it, so a refused
//! comment is not lost. Posting is browser-only.

use super::review_wording::validated_review_text;
use crate::v2::core::api::dto::ReviewComment;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// A comment box for one mission's review thread.
#[component]
pub(crate) fn ReviewCommentComposer(
    /// The mission whose thread the comment joins.
    mission_id: String,
    /// The artifact the comment is about, when it is about one.
    artifact_id: Option<String>,
    /// Runs with the comment once the backend has stored it.
    on_posted: Callback<ReviewComment>,
    /// What the empty box invites.
    placeholder: &'static str,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let draft = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let problem = RwSignal::new(None::<String>);
    let target = StoredValue::new((mission_id, artifact_id));
    let post = move |_| {
        let body = match validated_review_text(&draft.get_untracked(), "comment") {
            Ok(body) => body,
            Err(why) => {
                problem.set(Some(why));
                return;
            }
        };
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::core::api::dto::ReviewCommentRequest;
            use crate::v2::core::api::endpoints::mission_reviews::post_review_comment;
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let (mission, artifact_id) = target.get_value();
            let request = ReviewCommentRequest { body, artifact_id };
            leptos::task::spawn_local(async move {
                match post_review_comment(store, &mission, &request).await {
                    Ok(comment) => {
                        let _ = draft.try_set(String::new());
                        let _ = problem.try_set(None);
                        on_posted.run(comment);
                    }
                    Err(refusal) => {
                        let _ = problem
                            .try_set(Some(refusal.message_or("The comment could not be posted")));
                    }
                }
                let _ = busy.try_set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (body, store, target, on_posted);
    };
    view! {
        <div class="space-y-2">
            <textarea
                aria-label="Comment"
                rows="3"
                placeholder=placeholder
                prop:value=move || draft.get()
                on:input=move |ev| draft.set(event_target_value(&ev))
                class="w-full resize-y rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-label-md text-on-surface outline-none transition placeholder:text-on-surface-variant/60 focus:border-primary/40"
            ></textarea>
            <div class="flex items-center justify-between gap-3">
                <p class="text-label-sm text-error-alert">{move || problem.get()}</p>
                <button
                    type="button"
                    on:click=post
                    prop:disabled=move || busy.get() || draft.with(|d| d.trim().is_empty())
                    class="flex shrink-0 items-center gap-1.5 rounded-full bg-primary px-4 py-2 text-label-md font-medium text-on-primary transition hover:bg-primary/80 disabled:opacity-50"
                >
                    <MaterialIcon name="send" class="text-[16px]" />
                    "Post comment"
                </button>
            </div>
        </div>
    }
}
