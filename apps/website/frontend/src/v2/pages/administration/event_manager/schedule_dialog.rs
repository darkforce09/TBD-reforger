//! The schedule form: creating an operation on the selected day.
//!
//! **Role:** the frosted form that overlays the calendar — start time, optional name, the missions
//! to attach, whether registration opens — and the publish action behind it.
//! **Position:** a dialog over the operations calendar, opened from the page heading or from the
//! empty day panel.
//! **Signals & state:** reads `selected` for the day being scheduled and the form's own `time`,
//! `name`, `open_reg` and `staged`; `form_open` gates the dialog and `publish_busy` the button. A
//! successful publish clears the form, shuts it, and refetches the operation list.
//! **Invariants:** the day comes from the calendar, not from a field in this form, so the operation
//! lands on the day the operator was looking at. Creation is two steps — the operation is created
//! first, then each staged mission is attached to it — and every attachment is given the operation's
//! own start time. An empty start time is refused before any request is made, and the busy flag
//! makes a second click while one is in flight a no-op.
#![allow(dead_code)]

use super::dates::{js_date, locale_date_string};
use super::mission_picker::staged_missions;
use super::state::Manager;
use crate::v2::core::ui::{cn, Dialog, MaterialIcon};
use leptos::prelude::*;

/// The Schedule Operation form.
///
/// Renders the day caption, the time and name fields, the staged-mission section, the registration
/// switch and the publish button.
pub(super) fn schedule_dialog(st: Manager) -> impl IntoView {
    let Manager {
        store,
        events,
        selected,
        name,
        time,
        open_reg,
        staged,
        form_open,
        publish_busy,
        ..
    } = st;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, events, staged);

    let on_publish = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let t = time.get_untracked();
            if t.is_empty() {
                toasts.error("Start time is required");
                return;
            }
            if publish_busy.get_untracked() {
                return;
            }
            publish_busy.set(true);
            let (y, m, d) = selected.get_untracked();
            // The selected calendar day plus the typed time, read back as an instant.
            let (hh, mm) = t
                .split_once(':')
                .map(|(h, m)| (h.parse().unwrap_or(0), m.parse().unwrap_or(0)))
                .unwrap_or((0, 0));
            let dt = js_sys::Date::new_with_year_month_day_hr_min(y as u32, m, d as i32, hh, mm);
            let start_iso = dt.to_iso_string().as_string().unwrap_or_default();
            let nm = name.get_untracked();
            let mut body = serde_json::json!({
                "start_time": start_iso,
                "registration_locked": !open_reg.get_untracked(),
            });
            if !nm.is_empty() {
                body["name_override"] = serde_json::Value::String(nm);
            }
            let to_attach = staged.get_untracked();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post::<serde_json::Value>(
                    store, "/events", body,
                )
                .await
                {
                    Ok(created) => {
                        let id = created
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let n = to_attach.len();
                        if !id.is_empty() {
                            for (mid, _) in to_attach {
                                let _ = crate::v2::core::api::client::api_post::<serde_json::Value>(
                                    store,
                                    &format!("/events/{id}/missions"),
                                    serde_json::json!({ "mission_id": mid, "start_time": start_iso }),
                                )
                                .await;
                            }
                        }
                        toasts.success(if n > 0 {
                            format!(
                                "Event published with {n} mission{}",
                                if n == 1 { "" } else { "s" }
                            )
                        } else {
                            "Event published".to_string()
                        });
                        name.set(String::new());
                        staged.set(Vec::new());
                        open_reg.set(true);
                        form_open.set(false);
                        events.refetch();
                    }
                    Err(_) => toasts.error("Failed to publish event"),
                }
                publish_busy.set(false);
            });
        }
    };

    view! {
        <Dialog open=form_open title="Schedule Operation">
            <p class="-mt-3 mb-4 text-label-md text-on-surface-variant">
                {move || {
                    let (y, m, d) = selected.get();
                    locale_date_string(
                        &js_date(y, m, d),
                        &[
                            ("weekday", "long"),
                            ("month", "long"),
                            ("day", "numeric"),
                            ("year", "numeric"),
                        ],
                    )
                }}
            </p>
            <label class="flex w-fit items-center gap-2 rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface focus-within:ring-1 focus-within:ring-primary/50">
                <MaterialIcon name="schedule" class="text-base text-on-surface-variant" />
                <input
                    type="time"
                    prop:value=move || time.get()
                    on:input=move |ev| time.set(event_target_value(&ev))
                    class="bg-transparent text-on-surface outline-none [color-scheme:dark]"
                />
            </label>

            <input
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
                placeholder="Operation name (e.g. Twin Theaters)"
                class="mt-3 w-full rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50"
            />


            {staged_missions(st)}

            // Registration status segmented control
            <div class="mt-6">
                <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                    "Registration"
                </p>
                <div class="inline-flex rounded-full bg-white/5 p-1">
                    {[true, false]
                        .into_iter()
                        .map(|is_open| {
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| open_reg.set(is_open)
                                    class=move || {
                                        cn(
                                            &[
                                                "rounded-full px-6 py-2 text-sm font-medium transition",
                                                if open_reg.get() == is_open {
                                                    if is_open {
                                                        "bg-success/20 text-success"
                                                    } else {
                                                        "bg-white/10 text-on-surface"
                                                    }
                                                } else {
                                                    "text-on-surface-variant hover:text-on-surface"
                                                },
                                            ],
                                        )
                                    }
                                >
                                    {if is_open { "Open" } else { "Locked" }}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            // Publish
            <button
                type="button"
                on:click=on_publish
                prop:disabled=move || publish_busy.get()
                class="mt-8 w-full rounded-full bg-action py-4 text-base font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
            >
                {move || if publish_busy.get() { "Publishing…" } else { "Publish Event" }}
            </button>
        </Dialog>
    }
}
