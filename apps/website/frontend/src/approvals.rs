//! Mission Approvals (/admin/approvals) — ported from pages/admin.tsx `MissionApprovalsPage` +
//! `ReviewInspector`. `<AdminGate>` → `/approvals` Resource → a `SplitPane`: the pending queue
//! master + the `ReviewInspector` detail pane with the LIVE approve/reject mutations
//! (POST /approvals/:id/{approve,reject}); the queue refetches on success.
//!
//! T-218 — this surface stopped inventing things:
//!
//! - REJECT SENDS ITS REASON. `POST /approvals/:id/reject` takes `RejectInput { reason }` and
//!   writes it to `missions.rejection_reason`. This page posted `serde_json::json!({})`, so the
//!   column was overwritten with `""` on every rejection and the author was told their mission was
//!   returned with no word on why. The action bar now owns the reason field and the button will
//!   not fire without one — an empty reason is the bug, not a shortcut.
//!
//! - THE APPROVED / REJECTED TABS ARE GONE. They were `mock_approved()` / `mock_rejected()`: three
//!   invented missions by two invented authors, rendered as review history. The same mocks fed the
//!   tab counters, so an empty database advertised "Approved (2) / Rejected (1)" and a reviewer
//!   could open "Operation Iron Veil" and act on it. `GET /approvals` selects
//!   `status = 'pending_approval'` and nothing else, and no endpoint lists the other two states, so
//!   there is no honest tab to put there — deleted rather than gated, per T-195: a "demo data"
//!   ribbon is still a queue of missions that do not exist. They return with the endpoint (T-283).
//!
//! - THE TWO BUTTONS THAT LIED WENT WITH THEM. "Revoke Approval & Unpublish" and the rejected
//!   tab's "Approve & Publish" existed only in those mock tabs and only ever raised a toast —
//!   "Mission unpublished — pulled from the live server" over a mission that was still live.
//!
//! - THE BRIEFING AND STAT TILES ARE THE MISSION'S OWN. They were a fixed paragraph about
//!   contested farmland plus "BLUFOR Slots 32 / OPFOR Type Mechanized / Est. Duration ~90 min",
//!   printed under whichever real mission was selected. That is the one screen where fabricated
//!   facts do direct damage: it is the page where someone decides. They now come from
//!   `GET /missions/:id` (an admin passes `can_edit`, so a pending mission is readable), and a
//!   mission that supplied no briefing says so.
//!
//! STILL NOT REAL, AND LABELLED AS SUCH: the reviewer comment box is a local signal with no
//! backing table in any migration. That is T-283's ticket, so the box stays — but it now says on
//! screen that nothing it holds is saved or visible to the author, which is the part that mattered.
#![allow(dead_code)]
use crate::datefmt::{format_local_datetime, format_short_date};
use crate::dto::{ApprovalRow, MissionDetail, Paginated};
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;

fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `gameModeLabel` (lib/format.ts) — matches mission_overview.rs.
fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// `heavy_rain` → `Heavy rain`. The wire enums are snake_case; nothing else renders them here.
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

#[component]
pub fn MissionApprovalsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <MissionApprovalsInner />
        </AdminGate>
    }
}

#[component]
fn MissionApprovalsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let approvals = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<ApprovalRow>>(store, "/approvals")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<ApprovalRow>>
        }
    });
    let selected_id = RwSignal::new(None::<String>);
    let refetch = Callback::new(move |()| approvals.refetch());
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                approvals
                    .get()
                    .map(|opt| match opt {
                        Some(page) => {
                            board(page.data, page.total, selected_id, refetch).into_any()
                        }
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
    </Suspense>
    }
}

fn board(
    pending: Vec<ApprovalRow>,
    total: i64,
    selected_id: RwSignal<Option<String>>,
    refetch: Callback<()>,
) -> impl IntoView {
    let rows_sv = StoredValue::new(pending);
    // The count is the server's `total`, not the length of this page — the queue is paginated and
    // a 20-row page of a 40-row backlog must not read "20". The old counter was
    // `mock_approved().len()`, which is why an empty database claimed two approvals.
    let master_header = view! {
        <div class="flex w-full items-center justify-between gap-2">
            <h2 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                "Pending Review"
            </h2>
            <span class="font-mono text-code-md text-on-surface-variant tabular-nums">{total}</span>
        </div>
    }
    .into_any();

    let selected = move || {
        let rows = rows_sv.get_value();
        selected_id
            .get()
            .and_then(|id| rows.iter().find(|r| r.mission_id == id).cloned())
            .or_else(|| rows.first().cloned())
    };

    let master = view! {
        {move || {
            let rows = rows_sv.get_value();
            if rows.is_empty() {
                view! {
                    <p class="px-1 py-4 text-label-md text-on-surface-variant">
                        "No pending approvals."
                    </p>
                }
                    .into_any()
            } else {
                let sel = selected();
                rows.into_iter()
                    .map(|r| {
                        let active = sel
                            .as_ref()
                            .map(|s| s.mission_id == r.mission_id)
                            .unwrap_or(false);
                        let rid = r.mission_id.clone();
                        view! {
                            <button
                                type="button"
                                on:click=move |_| selected_id.set(Some(rid.clone()))
                                class=cn(
                                    &[
                                        "group w-full rounded-r-xl border-l-4 px-4 py-3 text-left transition-all duration-200",
                                        if active {
                                            "border-primary bg-primary/15 shadow-[inset_0_0_18px_rgba(173,198,255,0.15)]"
                                        } else {
                                            "border-transparent hover:bg-white/[0.03]"
                                        },
                                    ],
                                )
                            >
                                <span class=cn(
                                    &[
                                        "font-mono text-code-md",
                                        if active { "text-primary" } else { "text-outline" },
                                    ],
                                )>"[" {format_short_date(&r.submitted_at)} "]"</span>
                                <h3 class=cn(
                                    &[
                                        "mt-1 truncate text-label-md font-semibold",
                                        if active {
                                            "text-on-surface"
                                        } else {
                                            "text-on-surface-variant group-hover:text-on-surface"
                                        },
                                    ],
                                )>{r.title.clone()}</h3>
                                <p class="mt-0.5 truncate text-label-sm text-on-surface-variant">
                                    "By " {r.author_name.clone()} " · " {terrain_label(&r.terrain)}
                                </p>
                            </button>
                        }
                    })
                    .collect_view()
                    .into_any()
            }
        }}
    }
        .into_any();

    let detail = view! {
        {move || match selected() {
            Some(row) => view! { <ReviewInspector row=row refetch=refetch /> }.into_any(),
            None => {
                view! {
                    <SplitPaneEmpty
                        icon=view! { <MaterialIcon name="task_alt" class="text-4xl" /> }.into_any()
                        message="Queue clear — no pending approvals."
                    />
                }
                    .into_any()
            }
        }}
    }
    .into_any();

    view! { <SplitPane master_header=master_header master=master detail=detail /> }
}

/// The GitHub-PR-meets-chat review surface (admin.tsx `ReviewInspector`): cinematic header, the
/// mission's real briefing + stats, the (local, labelled) comment box, and the sticky action bar
/// carrying the rejection reason. Every row reaching here is `pending_approval`, so both actions
/// hit the API.
///
/// A component rather than a plain `fn` (which is what it was) because it now owns a
/// `LocalResource`, and the caller invokes it from inside a reactive closure — one per selected
/// row. A component gets its own owner, so switching rows disposes the previous fetch and its
/// signals instead of stacking them on the enclosing effect. Same reason `MissionDossierSheet`
/// (missions.rs) is a component.
#[component]
fn ReviewInspector(row: ApprovalRow, refetch: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &refetch);
    let mid = StoredValue::new(row.mission_id.clone());
    let approve_busy = RwSignal::new(false);
    let reject_busy = RwSignal::new(false);
    // The rejection reason. This is the whole ticket: it is read at POST time and sent as
    // `RejectInput { reason }`, and "Request Changes" stays disabled while it is blank so a
    // reviewer cannot repeat the silent-discard by accident.
    let reason = RwSignal::new(String::new());
    let reason_blank = move || reason.get().trim().is_empty();

    // The mission's own briefing + settings. Admins pass `can_edit`, so a pending mission is
    // readable; this replaces the fixed "contested farmland" paragraph that used to print under
    // every mission in the queue.
    let detail = LocalResource::new(move || {
        let id = mid.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::client::api_get::<MissionDetail>(store, &path)
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

    // Local comment feed — no comments table exists in any migration (T-283). Kept so the slice
    // does not delete a queued feature out from under it, captioned so nobody mistakes it for one.
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
            let toasts = crate::toast::use_toasts();
            let path = format!("/approvals/{}/approve", mid.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, serde_json::json!({})).await {
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
            let toasts = crate::toast::use_toasts();
            let path = format!("/approvals/{}/reject", mid.get_value());
            leptos::task::spawn_local(async move {
                let payload = serde_json::json!({ "reason": body });
                match crate::client::api_post_ok(store, &path, payload).await {
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
            crate::toast::use_toasts().success(msg);
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
