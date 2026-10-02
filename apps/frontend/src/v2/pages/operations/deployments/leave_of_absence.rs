//! The caller's leave of absence: the form that files one, and the list of the ones on file.
//!
//! **Role:** renders the leave panel — the date and reason fields, the submit button, and the
//! table of the caller's own requests — and owns the date rules the form checks before posting
//! and the shared form-control recipe.
//! **Position:** a section of the right-hand column, below the combat history.
//! **Signals & state:** owns the three field signals, a busy flag, a form-error signal, and the
//! resource holding the caller's own requests, which it refetches after a successful submit.
//! **Invariants:** the date rules mirror the backend's, and only a bare `YYYY-MM-DD` is posted —
//! the wire *response* form carries a time and a zone, and must never be sent back. The submit
//! is a browser-only path and does nothing in a native build.
#![allow(dead_code)]

use super::table_head::ServiceHead;
use crate::v2::core::api::dto::{CreateLeaveInput, DataEnvelope, LeaveRequest};
use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::{badge_class, MaterialIcon};
use crate::v2::core::utils::datefmt::format_short_date;
use leptos::prelude::*;
use serde_json::Value;

/// The pill-shaped form-control recipe the three fields share.
const LOA_INPUT: &str = "w-full rounded-full bg-white/5 px-5 py-2.5 font-mono text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none transition focus:ring-1 focus:ring-primary/50";

/// The backend's leave date rules, checked before the request goes out: both dates present, both
/// a bare `YYYY-MM-DD`, and the end on or after the start.
pub(super) fn validate_loa_range(starts_on: &str, ends_on: &str) -> Result<(), &'static str> {
    if starts_on.is_empty() || ends_on.is_empty() {
        return Err("starts_on and ends_on are required");
    }
    if !is_ymd(starts_on) || !is_ymd(ends_on) {
        return Err("dates must be YYYY-MM-DD");
    }
    if ends_on < starts_on {
        return Err("ends_on must be on or after starts_on");
    }
    Ok(())
}

/// Whether `s` is exactly a bare `YYYY-MM-DD` date.
fn is_ymd(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[0..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// The badge variant a leave request's status is shown in.
pub(super) fn leave_status_variant(status: &str) -> &'static str {
    match status {
        "approved" => "success",
        "denied" => "error",
        "pending" => "warning",
        _ => "neutral",
    }
}

/// The request body the form posts.
///
/// A named helper rather than an inline value, so the shape is exercised natively: the submit
/// closure around it is browser-only.
pub(super) fn create_leave_body(starts_on: String, ends_on: String, reason: String) -> Value {
    serde_json::to_value(CreateLeaveInput {
        starts_on,
        ends_on,
        reason,
    })
    .unwrap_or(Value::Null)
}

/// The caller's leave panel: file a request, and see the ones already on file.
#[component]
pub(super) fn LeaveOfAbsencePanel() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let mine = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<DataEnvelope<LeaveRequest>>(
                store,
                "/me/leave-requests",
            )
            .await
            .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<LeaveRequest>>
        }
    });
    let starts_on = RwSignal::new(String::new());
    let ends_on = RwSignal::new(String::new());
    let reason = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let form_error = RwSignal::new(None::<String>);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let start = starts_on.get_untracked().trim().to_string();
            let end = ends_on.get_untracked().trim().to_string();
            let why = reason.get_untracked().trim().to_string();
            if let Err(msg) = validate_loa_range(&start, &end) {
                form_error.set(Some(msg.to_string()));
                toasts.error(msg);
                return;
            }
            form_error.set(None);
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let body = create_leave_body(start, end, why);
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post::<LeaveRequest>(
                    store,
                    "/me/leave-requests",
                    body,
                )
                .await
                {
                    Ok(_) => {
                        toasts.success("Leave request submitted");
                        starts_on.set(String::new());
                        ends_on.set(String::new());
                        reason.set(String::new());
                        mine.refetch();
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Failed to submit leave request",
                    )),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <section class="border-t border-white/10 p-8">
            <div class="mb-4 flex flex-wrap items-baseline justify-between gap-2">
                <h2 class="font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "Leave of Absence"
                </h2>
                <span class="font-mono text-[10px] tracking-widest text-on-surface-variant/70 uppercase">
                    "Submit Leave of Absence"
                </span>
            </div>
            <form on:submit=on_submit class="mb-6 grid gap-4 md:grid-cols-4">
                <div>
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Starts on"
                    </label>
                    <input
                        type="date"
                        required
                        prop:value=move || starts_on.get()
                        on:input=move |ev| starts_on.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div>
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Ends on"
                    </label>
                    <input
                        type="date"
                        required
                        prop:value=move || ends_on.get()
                        on:input=move |ev| ends_on.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div class="md:col-span-2">
                    <label class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                        "Reason"
                    </label>
                    <input
                        type="text"
                        placeholder="Optional reason…"
                        prop:value=move || reason.get()
                        on:input=move |ev| reason.set(event_target_value(&ev))
                        class=LOA_INPUT
                    />
                </div>
                <div class="md:col-span-4 flex flex-wrap items-center gap-3">
                    <button
                        type="submit"
                        prop:disabled=move || busy.get()
                        class="inline-flex items-center gap-2 rounded-full border border-primary/50 bg-primary/15 px-5 py-2.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/25 disabled:cursor-not-allowed disabled:opacity-50"
                    >
                        <MaterialIcon name="event_busy" class="text-base" />
                        {move || {
                            if busy.get() { "Submitting…" } else { "Submit Leave of Absence" }
                        }}
                    </button>
                    {move || {
                        form_error
                            .get()
                            .map(|e| {
                                view! {
                                    <span class="font-mono text-xs text-error">{e}</span>
                                }
                            })
                    }}
                </div>
            </form>
            <Suspense fallback=move || {
                view! {
                    <p class="font-mono text-xs text-on-surface-variant">"Loading leave requests…"</p>
                }
            }>
                {move || {
                    mine.get().map(|opt| match opt {
                        Some(env) => leave_rows(&env.data).into_any(),
                        None => {
                            view! {
                                <p class="font-mono text-xs text-error">
                                    "Failed to load leave requests."
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

/// The caller's own leave requests as a table, or the empty-state box when there are none.
fn leave_rows(rows: &[LeaveRequest]) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <div class="rounded-xl border border-white/10 px-4 py-6 text-center">
                <p class="font-mono text-xs tracking-widest text-on-surface-variant uppercase">
                    "No leave requests on file"
                </p>
            </div>
        }
        .into_any();
    }
    let rows = rows.to_vec();
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[36rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Starts" />
                        <ServiceHead label="Ends" />
                        <ServiceHead label="Reason" />
                        <ServiceHead label="Status" />
                        <ServiceHead label="Filed" />
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .into_iter()
                        .map(|r| {
                            let status_label = r.status.clone();
                            let variant = leave_status_variant(&r.status);
                            let reason = if r.reason.is_empty() {
                                "—".to_string()
                            } else {
                                r.reason.clone()
                            };
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
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
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&r.created_at)}
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
