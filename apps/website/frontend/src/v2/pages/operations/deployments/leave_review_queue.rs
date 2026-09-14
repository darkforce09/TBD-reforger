//! The administrator's leave review queue.
//!
//! **Role:** renders the pending leave requests with an approve and a deny control on each, and
//! the reviewer's name on the ones already decided.
//! **Position:** the last section of the right-hand column, rendered only for an administrator.
//! **Signals & state:** owns the queue resource, and refetches it after each decision.
//! **Invariants:** the controls appear only on a request still pending; a decided one shows who
//! decided it. Both decisions are browser-only paths and do nothing in a native build.
#![allow(dead_code)]

use super::leave_of_absence::leave_status_variant;
use super::table_head::ServiceHead;
use crate::v2::core::api::dto::{LeaveRequest, Paginated};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::badge_class;
use crate::v2::core::utils::datefmt::format_short_date;
use leptos::prelude::*;

// The decision request bodies are only built in the browser.
#[cfg(target_arch = "wasm32")]
use serde_json::Value;

/// The review queue, on this page rather than behind a route of its own.
#[component]
pub(super) fn AdminLeaveQueue() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let queue = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<LeaveRequest>>(
                store,
                "/admin/leave-requests",
            )
            .await
            .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<LeaveRequest>>
        }
    });

    view! {
        <section class="border-t border-white/10 p-8">
            <div class="mb-4 flex flex-wrap items-baseline justify-between gap-2">
                <h2 class="font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "LOA Review Queue"
                </h2>
                <span class="font-mono text-[10px] tracking-widest text-tactical-yellow/80 uppercase">
                    "Admin"
                </span>
            </div>
            <Suspense fallback=move || {
                view! {
                    <p class="font-mono text-xs text-on-surface-variant">"Loading review queue…"</p>
                }
            }>
                {move || {
                    queue.get().map(|opt| match opt {
                        Some(page) => admin_leave_table(page.data, queue).into_any(),
                        None => {
                            view! {
                                <p class="font-mono text-xs text-error">
                                    "Failed to load LOA review queue."
                                </p>
                            }
                                .into_any()
                        }
                    })
                }}
            </Suspense>
        </section>
    }
}

/// The queue as a table, or the empty-state box when nothing is waiting.
fn admin_leave_table(
    rows: Vec<LeaveRequest>,
    queue: LocalResource<Option<Paginated<LeaveRequest>>>,
) -> impl IntoView {
    let store = expect_context::<AuthStore>();
    if rows.is_empty() {
        return view! {
            <div class="rounded-xl border border-white/10 px-4 py-6 text-center">
                <p class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                    "No leave requests in the queue"
                </p>
            </div>
        }
        .into_any();
    }
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[44rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Member" />
                        <ServiceHead label="Starts" />
                        <ServiceHead label="Ends" />
                        <ServiceHead label="Reason" />
                        <ServiceHead label="Status" />
                        <ServiceHead label="Review" />
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            let id = r.id.clone();
                            let id_deny = r.id.clone();
                            let status_label = r.status.clone();
                            let variant = leave_status_variant(&r.status);
                            let pending = r.status == "pending";
                            let reason = if r.reason.is_empty() {
                                "—".to_string()
                            } else {
                                r.reason.clone()
                            };
                            let on_approve = move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let toasts = crate::v2::core::ui::toast::use_toasts();
                                    let path = format!("/admin/leave-requests/{id}");
                                    leptos::task::spawn_local(async move {
                                        match crate::v2::core::api::client::api_patch::<Value>(
                                            store,
                                            &path,
                                            serde_json::json!({"status":"approved"}),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                toasts.success("LOA approved");
                                                queue.refetch();
                                            }
                                            Err(e) => toasts.error(
                                                crate::v2::core::api::client::api_error_message(
                                                    &e,
                                                    "Failed to approve LOA",
                                                ),
                                            ),
                                        }
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (&store, &queue, &id);
                                }
                            };
                            let on_deny = move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let toasts = crate::v2::core::ui::toast::use_toasts();
                                    let path = format!("/admin/leave-requests/{id_deny}");
                                    leptos::task::spawn_local(async move {
                                        match crate::v2::core::api::client::api_patch::<Value>(
                                            store,
                                            &path,
                                            serde_json::json!({"status":"denied"}),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                toasts.success("LOA denied");
                                                queue.refetch();
                                            }
                                            Err(e) => toasts.error(
                                                crate::v2::core::api::client::api_error_message(
                                                    &e,
                                                    "Failed to deny LOA",
                                                ),
                                            ),
                                        }
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (&store, &queue, &id_deny);
                                }
                            };
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                                        {r.discord_id.clone()}
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.starts_on)}
                                    </td>
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.ends_on)}
                                    </td>
                                    <td class="px-4 py-3 text-on-surface">{reason}</td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(variant)>{status_label}</span>
                                    </td>
                                    <td class="px-4 py-3">
                                        {if pending {
                                            view! {
                                                <div class="flex flex-wrap gap-2">
                                                    <button
                                                        type="button"
                                                        on:click=on_approve
                                                        class="rounded-full bg-emerald-600/90 px-3 py-1.5 font-mono text-[10px] tracking-widest text-white uppercase transition hover:bg-emerald-500"
                                                    >
                                                        "Approve"
                                                    </button>
                                                    <button
                                                        type="button"
                                                        on:click=on_deny
                                                        class="rounded-full border border-error/40 bg-error/10 px-3 py-1.5 font-mono text-[10px] tracking-widest text-error uppercase transition hover:bg-error/20"
                                                    >
                                                        "Deny"
                                                    </button>
                                                </div>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <span class="font-mono text-xs text-outline">
                                                    {r.reviewed_by
                                                        .clone()
                                                        .unwrap_or_else(|| "—".into())}
                                                </span>
                                            }
                                                .into_any()
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}
