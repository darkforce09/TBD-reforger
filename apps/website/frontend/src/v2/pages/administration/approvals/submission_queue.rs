//! The pending queue: every mission waiting on a decision, and the pane it sits beside.
//!
//! **Role:** the queue heading with its count, one selectable row per submission, and the split
//! that puts the queue beside the review drawer.
//! **Position:** the whole body of the approvals route.
//! **Signals & state:** reads and writes `selected_id`; the fetched rows are parked in a stored
//! value that both the list and the drawer read.
//! **Invariants:** the count is the server's total, not the length of the page on screen — the
//! queue is paginated, so a page of a longer backlog must not report the page's size. Selection
//! falls back to the first row, so a queue with anything in it always has something open. An empty
//! queue shows the empty pane rather than a drawer over nothing.
#![allow(dead_code)]

use super::review_drawer::ReviewInspector;
use crate::v2::core::api::dto::ApprovalRow;
use crate::v2::core::ui::split_pane::{SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::{cn, MaterialIcon};
use crate::v2::core::utils::datefmt::format_short_date;
use leptos::prelude::*;

/// A terrain's wire name with its first letter capitalised, or a dash when there is none.
pub(super) fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The queue beside the review drawer.
pub(super) fn board(
    pending: Vec<ApprovalRow>,
    total: i64,
    selected_id: RwSignal<Option<String>>,
    refetch: Callback<()>,
) -> impl IntoView {
    let rows_sv = StoredValue::new(pending);
    // The server's total, not the length of this page: the queue is paginated, so one page of a
    // longer backlog must not report the page's size.
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
