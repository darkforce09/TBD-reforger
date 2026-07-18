//! Mission Approvals (/admin/approvals) — ported from pages/admin.tsx `MissionApprovalsPage` +
//! `ReviewInspector`. `<AdminGate>` → `/approvals` Resource → `QueryState` → a `SplitPane`: a
//! Pending/Approved/Rejected segmented control + the queue master + the review detail pane.
//!
//! T-159.25: fully interactive — live tab switching (Approved/Rejected history stays on the React
//! MOCK rows until list endpoints exist), row selection, and the `ReviewInspector` with the mock
//! stats/feed shell + the LIVE approve/reject mutations (POST /approvals/:id/{approve,reject}) on
//! the sticky action bar; queue refetches on success.
#![allow(dead_code)]
use crate::datefmt::{format_local_datetime, format_short_date};
use crate::dto::{ApprovalRow, Paginated};
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;

// Mock approved/rejected history so the audit trail + revoke flow render before list endpoints for
// those states exist (admin.tsx MOCK_APPROVED / MOCK_REJECTED). Pending stays on live data.
fn mock_approved() -> Vec<ApprovalRow> {
    vec![
        ApprovalRow {
            mission_id: "apr-1".into(),
            title: "Operation Iron Veil".into(),
            terrain: "everon".into(),
            author_id: "u-mike".into(),
            author_name: "Mission Maker Mike".into(),
            submitted_at: "2026-06-12T18:00:00Z".into(),
        },
        ApprovalRow {
            mission_id: "apr-2".into(),
            title: "Checkpoint Zulu".into(),
            terrain: "arland".into(),
            author_id: "u-sarah".into(),
            author_name: "Sarah Chen".into(),
            submitted_at: "2026-06-09T12:30:00Z".into(),
        },
    ]
}
fn mock_rejected() -> Vec<ApprovalRow> {
    vec![ApprovalRow {
        mission_id: "rej-1".into(),
        title: "Night of the Long Knives".into(),
        terrain: "everon".into(),
        author_id: "u-mike".into(),
        author_name: "Mission Maker Mike".into(),
        submitted_at: "2026-06-05T09:15:00Z".into(),
    }]
}

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
    // 0 = pending, 1 = approved (mock), 2 = rejected (mock).
    let tab = RwSignal::new(0usize);
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
                        Some(page) => board(page.data, tab, selected_id, refetch).into_any(),
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
    tab: RwSignal<usize>,
    selected_id: RwSignal<Option<String>>,
    refetch: Callback<()>,
) -> impl IntoView {
    let pending_count = pending.len();
    let rows_sv = StoredValue::new(pending);
    let rows_for = move |t: usize| -> Vec<ApprovalRow> {
        match t {
            0 => rows_sv.get_value(),
            1 => mock_approved(),
            _ => mock_rejected(),
        }
    };
    // TABS row — counts render `(n)` only when > 0, active tab tracks the signal.
    let tab_btn = move |i: usize, label: &'static str, count: usize| {
        view! {
            <button
                type="button"
                on:click=move |_| {
                    tab.set(i);
                    selected_id.set(None);
                }
                class=move || {
                    if tab.get() == i {
                        "flex-1 rounded-full py-2 text-center font-medium whitespace-nowrap transition-all bg-action text-on-action shadow-md"
                    } else {
                        "flex-1 rounded-full py-2 text-center font-medium whitespace-nowrap transition-all text-white/50 hover:text-white"
                    }
                }
            >
                {label}
                {(count > 0)
                    .then(|| {
                        view! {
                            <span class="ml-1.5 font-mono text-code-md opacity-70">
                                "(" {count} ")"
                            </span>
                        }
                    })}
            </button>
        }
    };
    let master_header = view! {
        <div class="flex w-full items-center rounded-full bg-white/5 p-1">
            {tab_btn(0, "Pending", pending_count)}
            {tab_btn(1, "Approved", mock_approved().len())}
            {tab_btn(2, "Rejected", mock_rejected().len())}
        </div>
    }
    .into_any();

    let selected = move || {
        let rows = rows_for(tab.get());
        selected_id
            .get()
            .and_then(|id| rows.iter().find(|r| r.mission_id == id).cloned())
            .or_else(|| rows.first().cloned())
    };

    let master = view! {
        {move || {
            let t = tab.get();
            let rows = rows_for(t);
            if rows.is_empty() {
                let msg = if t == 0 {
                    "No pending approvals.".to_string()
                } else if t == 1 {
                    "No approved missions.".to_string()
                } else {
                    "No rejected missions.".to_string()
                };
                view! { <p class="px-1 py-4 text-label-md text-on-surface-variant">{msg}</p> }
                    .into_any()
            } else {
                let sel = selected();
                rows.into_iter()
                    .map(|r| {
                        let active = sel.as_ref().map(|s| s.mission_id == r.mission_id).unwrap_or(false);
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
        {move || {
            let t = tab.get();
            match selected() {
                Some(row) => review_inspector(row, t, refetch).into_any(),
                None => {
                    let msg = if t == 0 {
                        "Queue clear — no pending approvals."
                    } else if t == 1 {
                        "No approved missions to show."
                    } else {
                        "No rejected missions to show."
                    };
                    view! {
                        <SplitPaneEmpty
                            icon=view! { <MaterialIcon name="task_alt" class="text-4xl" /> }
                                .into_any()
                            message=msg
                        />
                    }
                        .into_any()
                }
            }
        }}
    }
    .into_any();

    view! { <SplitPane master_header=master_header master=master detail=detail /> }
}

/// The GitHub-PR-meets-chat review surface (admin.tsx `ReviewInspector`): cinematic header,
/// mock briefing/stats/feed shell, comment box (local), sticky approve/reject bar. Only the
/// pending-tab actions hit the API.
fn review_inspector(row: ApprovalRow, status_tab: usize, refetch: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &refetch);
    let mid = StoredValue::new(row.mission_id.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = mid;
    let approve_busy = RwSignal::new(false);
    let reject_busy = RwSignal::new(false);
    // Local comment feed — mock until a review-comments API lands.
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
            reject_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/approvals/{}/reject", mid.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, serde_json::json!({})).await {
                    Ok(()) => {
                        toasts.success("Changes requested — returned to author");
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
                        {(status_tab == 1)
                            .then(|| {
                                view! {
                                    <span class="rounded-full border border-success/40 bg-success/20 px-3 py-1 text-label-sm font-medium text-success backdrop-blur-md">
                                        "Published"
                                    </span>
                                }
                            })}
                        {(status_tab == 2)
                            .then(|| {
                                view! {
                                    <span class="rounded-full border border-error-alert/40 bg-error-alert/20 px-3 py-1 text-label-sm font-medium text-error-alert backdrop-blur-md">
                                        "Rejected"
                                    </span>
                                }
                            })}
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

            // Briefing + stats (mock shell until the Git versioning backend lands)
            <div class="px-8 py-7">
                <p class="text-body-md text-on-surface-variant">
                    "Combined-arms assault across contested farmland. BLUFOR pushes from the south to seize the northern town while a mechanized OPFOR garrison holds the objective. Review the latest push before approving for the live mission database."
                </p>
                <div class="mt-6 grid grid-cols-3 gap-3">
                    {stat_tile("BLUFOR Slots", "32")} {stat_tile("OPFOR Type", "Mechanized")}
                    {stat_tile("Est. Duration", "~90 min")}
                </div>

                <button
                    type="button"
                    on:click=stub_toast("Tactical Planner (2D editor) is coming soon")
                    class="mt-6 flex w-full items-center justify-center gap-2 rounded-xl border border-primary/40 bg-primary/10 py-3.5 text-label-md font-medium text-primary transition hover:bg-primary/20"
                >
                    <MaterialIcon name="search" class="text-[20px]" />
                    "Launch Tactical Planner for Deep Review"
                </button>

                <div class="mt-8">
                    <h2 class="mb-4 text-label-md font-semibold tracking-wide text-on-surface uppercase">
                        "Activity & Version History"
                    </h2>
                    <div class="flex flex-col gap-3">
                        {move || {
                            comments
                                .get()
                                .into_iter()
                                .map(|body| {
                                    view! {
                                        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3 text-label-md text-on-surface-variant">
                                            <span class="mr-2 font-semibold text-on-surface">
                                                "You"
                                            </span>
                                            {body}
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
                            placeholder="Leave a review comment…"
                            class="flex-1 bg-transparent text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none"
                        />
                        <button
                            type="button"
                            on:click=move |_| post_comment()
                            aria-label="Send comment"
                            class="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80"
                        >
                            <MaterialIcon name="arrow_upward" class="text-[20px]" />
                        </button>
                    </div>
                </div>
            </div>

            // Sticky action bar — adapts to mission status
            <div class="sticky bottom-0 mt-auto flex items-center justify-end gap-3 border-t border-white/5 bg-surface-container/40 p-6 backdrop-blur-xl">
                {if status_tab == 1 {
                    view! {
                        <button
                            type="button"
                            on:click=stub_toast("Mission unpublished — pulled from the live server")
                            class="rounded-full border border-error-alert/50 bg-error-alert/10 px-7 py-3 text-label-md font-bold text-error-alert shadow-[0_0_20px_rgba(239,68,68,0.2)] transition hover:bg-error-alert/20"
                        >
                            "Revoke Approval & Unpublish"
                        </button>
                    }
                        .into_any()
                } else if status_tab == 2 {
                    view! {
                        <button
                            type="button"
                            on:click=stub_toast("Mission re-approved & published")
                            class="rounded-full bg-emerald-600 px-7 py-3 text-label-md font-bold text-white shadow-[0_0_20px_rgba(16,185,129,0.3)] transition hover:bg-emerald-500"
                        >
                            "Approve & Publish"
                        </button>
                    }
                        .into_any()
                } else {
                    view! {
                        <button
                            type="button"
                            prop:disabled=move || reject_busy.get()
                            on:click=on_reject
                            class="rounded-full border border-tactical-yellow/40 bg-tactical-yellow/5 px-6 py-3 text-label-md font-medium text-tactical-yellow transition hover:bg-tactical-yellow/10 disabled:opacity-50"
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
                    }
                        .into_any()
                }}
            </div>
        </div>
    }
}

fn stat_tile(label: &'static str, value: &'static str) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
            <p class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 text-headline-sm text-on-surface">{value}</p>
        </div>
    }
}
