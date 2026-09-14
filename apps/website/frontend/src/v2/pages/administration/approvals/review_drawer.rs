//! The review drawer: the mission being decided on, and the two decisions.
//!
//! **Role:** the submission's header, its own briefing and settings read from the mission itself,
//! the local scratch notes, and the action bar carrying the rejection reason.
//! **Position:** the detail pane of the approvals route, beside the pending queue.
//! **Signals & state:** owns `reason` (what the author will be told), `approve_busy` and
//! `reject_busy`, and the scratch notes. The mission's own record lives in a `LocalResource` keyed
//! on the selected submission. Both decisions refetch the queue on success.
//! **Invariants:** requesting changes **sends its reason**, and the control stays disabled while
//! the box is blank — an empty reason returns a mission to its author with no word on why. The
//! reason is trimmed, because whitespace reaches them as nothing at all. The briefing and the four
//! settings come from the mission being reviewed; a mission that supplied no briefing says so
//! rather than borrowing someone else's words. This is a component rather than a plain function
//! because it owns a fetch and the caller invokes it per selected row: a component gets its own
//! owner, so changing rows disposes the previous fetch instead of stacking them.
#![allow(dead_code)]

use super::submission_queue::terrain_label;
use crate::v2::core::api::dto::{ApprovalRow, MissionDetail};
use crate::v2::core::ui::{cn, MaterialIcon};
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

/// A game mode's wire value as the label the review surface shows.
fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// A snake_case wire enum as a readable label, or a dash when it is empty.
fn enum_label(v: &str) -> String {
    if v.is_empty() {
        return "—".into();
    }
    let spaced = v.replace('_', " ");
    let mut c = spaced.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The review surface for one pending submission.
///
/// Renders the header, the mission's briefing and settings, the scratch notes, and the action bar
/// with the rejection reason.
#[component]
pub(super) fn ReviewInspector(row: ApprovalRow, refetch: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &refetch);
    let mid = StoredValue::new(row.mission_id.clone());
    let approve_busy = RwSignal::new(false);
    let reject_busy = RwSignal::new(false);
    // The rejection reason. It is read when the decision is sent, and the control stays disabled
    // while it is blank, so a mission cannot be returned with nothing said.
    let reason = RwSignal::new(String::new());
    let reason_blank = move || reason.get().trim().is_empty();

    // The mission's own briefing and settings. An administrator may read a pending mission, so
    // the words under the header belong to the mission being reviewed.
    let detail = LocalResource::new(move || {
        let id = mid.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::v2::core::api::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<MissionDetail>
            }
        }
    });

    // A local note feed: there is no review-comments table behind it. Kept, and captioned on
    // screen so nobody mistakes it for something the author will see.
    let comments = RwSignal::new(Vec::<String>::new());
    let draft = RwSignal::new(String::new());
    let post_comment = move || {
        let body = draft.get_untracked().trim().to_string();
        if body.is_empty() {
            return;
        }
        comments.update(|c| c.push(body));
        draft.set(String::new());
    };

    let on_approve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if approve_busy.get_untracked() {
                return;
            }
            approve_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/approvals/{}/approve", mid.get_value());
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post_ok(store, &path, serde_json::json!({}))
                    .await
                {
                    Ok(()) => {
                        toasts.success("Mission approved & published");
                        refetch.run(());
                    }
                    Err(_) => toasts.error("Approval failed"),
                }
                approve_busy.set(false);
            });
        }
    };
    let on_reject = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if reject_busy.get_untracked() {
                return;
            }
            // Trimmed, because a reason of " " reaches the author as no reason at all.
            let body = reason.get_untracked().trim().to_string();
            if body.is_empty() {
                return;
            }
            reject_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/approvals/{}/reject", mid.get_value());
            leptos::task::spawn_local(async move {
                let payload = serde_json::json!({ "reason": body });
                match crate::v2::core::api::client::api_post_ok(store, &path, payload).await {
                    Ok(()) => {
                        toasts.success("Changes requested — the author gets your reason");
                        reason.set(String::new());
                        refetch.run(());
                    }
                    Err(_) => toasts.error("Request failed"),
                }
                reject_busy.set(false);
            });
        }
    };
    let stub_toast = move |msg: &'static str| {
        move |_| {
            #[cfg(target_arch = "wasm32")]
            crate::v2::core::ui::toast::use_toasts().success(msg);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = msg;
        }
    };

    view! {
        <div class="flex min-h-full flex-col">
            // Cinematic header
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
                        <span class="rounded-full bg-white/10 px-3 py-1 font-mono text-code-md text-on-surface backdrop-blur-md">
                            {format_local_datetime(&row.submitted_at)}
                        </span>
                    </div>
                    <h1 class="text-headline-lg text-on-surface drop-shadow-lg">
                        {row.title.clone()}
                    </h1>
                </div>
            </div>

            // Briefing + settings, read off the mission itself.
            <div class="px-8 py-7">
                <Suspense fallback=move || {
                    view! {
                        <p class="text-body-md text-on-surface-variant">"Loading briefing…"</p>
                    }
                }>
                    {move || {
                        detail
                            .get()
                            .map(|opt| match opt {
                                Some(m) => {
                                    let briefing = m.briefing.clone().unwrap_or_default();
                                    let body = if briefing.trim().is_empty() {
                                        view! {
                                            <p class="text-body-md text-outline italic">
                                                "The author submitted no briefing."
                                            </p>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <p class="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
                                                {briefing}
                                            </p>
                                        }
                                            .into_any()
                                    };
                                    view! {
                                        {body}
                                        <div class="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
                                            {stat_tile("Max Players", m.max_players.to_string())}
                                            {stat_tile(
                                                "Game Mode",
                                                game_mode_label(&m.game_mode).to_string(),
                                            )} {stat_tile("Weather", enum_label(&m.weather))}
                                            {stat_tile("Time of Day", m.time_of_day.clone())}
                                        </div>
                                    }
                                        .into_any()
                                }
                                None => {
                                    view! {
                                        <p class="text-body-md text-error">
                                            "Could not load this mission's briefing — review it in the Mission Library before deciding."
                                        </p>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>

                <button
                    type="button"
                    on:click=stub_toast("Tactical Planner (2D editor) is coming soon")
                    class="mt-6 flex w-full items-center justify-center gap-2 rounded-xl border border-primary/40 bg-primary/10 py-3.5 text-label-md font-medium text-primary transition hover:bg-primary/20"
                >
                    <MaterialIcon name="search" class="text-[20px]" />
                    "Launch Tactical Planner for Deep Review"
                </button>

                <div class="mt-8">
                    <h2 class="mb-1 text-label-md font-semibold tracking-wide text-on-surface uppercase">
                        "Scratch Notes"
                    </h2>
                    <p class="mb-4 text-label-sm text-outline">
                        "Local to this browser tab. Not saved, not sent to the author, and gone when you navigate away — the review-comments API does not exist yet (T-283). Put anything the author must act on in the rejection reason below."
                    </p>
                    <div class="flex flex-col gap-3">
                        {move || {
                            comments
                                .get()
                                .into_iter()
                                .map(|body| {
                                    view! {
                                        <div class="rounded-xl border border-dashed border-white/10 bg-white/[0.02] px-4 py-3 text-label-md text-on-surface-variant">
                                            <span class="mr-2 font-semibold text-on-surface">
                                                "You"
                                            </span>
                                            {body}
                                            <span class="ml-2 font-mono text-label-sm text-outline">
                                                "(unsaved)"
                                            </span>
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>

                    <div class="mt-5 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 py-1.5 pr-1.5 pl-5 backdrop-blur-md focus-within:border-primary/40">
                        <input
                            prop:value=move || draft.get()
                            on:input=move |ev| draft.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    ev.prevent_default();
                                    post_comment();
                                }
                            }
                            placeholder="Note to self (not saved)…"
                            class="flex-1 bg-transparent text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none"
                        />
                        <button
                            type="button"
                            on:click=move |_| post_comment()
                            aria-label="Add scratch note"
                            class="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80"
                        >
                            <MaterialIcon name="arrow_upward" class="text-[20px]" />
                        </button>
                    </div>
                </div>
            </div>

            // Sticky action bar. The reason lives here rather than in a modal because it is an
            // input to one of the two buttons beside it, and the button is inert without it.
            <div class="sticky bottom-0 mt-auto flex flex-col gap-3 border-t border-white/5 bg-surface-container/40 p-6 backdrop-blur-xl">
                <label
                    for="reject-reason"
                    class="text-label-sm font-medium tracking-wide text-on-surface-variant uppercase"
                >
                    "Reason for requesting changes"
                </label>
                <textarea
                    id="reject-reason"
                    rows="2"
                    prop:value=move || reason.get()
                    on:input=move |ev| reason.set(event_target_value(&ev))
                    placeholder="What does the author need to fix? This is saved on the mission and is the only thing they are told."
                    class="w-full resize-y rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-label-md text-on-surface outline-none transition placeholder:text-on-surface-variant/60 focus:border-primary/40"
                />
                <div class="flex items-center justify-end gap-3">
                    <span class=move || {
                        cn(
                            &[
                                "mr-auto text-label-sm text-outline",
                                if reason_blank() { "" } else { "invisible" },
                            ],
                        )
                    }>"A reason is required to return a mission."</span>
                    <button
                        type="button"
                        prop:disabled=move || reject_busy.get() || reason_blank()
                        on:click=on_reject
                        title="Returns the mission to its author with the reason above"
                        class="rounded-full border border-tactical-yellow/40 bg-tactical-yellow/5 px-6 py-3 text-label-md font-medium text-tactical-yellow transition hover:bg-tactical-yellow/10 disabled:cursor-not-allowed disabled:opacity-40"
                    >
                        "Request Changes"
                    </button>
                    <button
                        type="button"
                        prop:disabled=move || approve_busy.get()
                        on:click=on_approve
                        class="rounded-full bg-emerald-600 px-7 py-3 text-label-md font-bold text-white shadow-[0_0_20px_rgba(16,185,129,0.3)] transition hover:bg-emerald-500 disabled:opacity-50"
                    >
                        "Approve & Publish"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// One of the four setting tiles under the briefing.
fn stat_tile(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
            <p class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 truncate text-headline-sm text-on-surface">{value}</p>
        </div>
    }
}
