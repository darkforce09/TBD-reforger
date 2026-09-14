//! The edit form: changing an operation that already exists.
//!
//! **Role:** the frosted form that rewrites a published operation — start date and time, name,
//! briefing, banner, slot ceiling, attached missions, lifecycle state and registration — and the
//! save action behind it.
//! **Position:** a dialog over the operations calendar, opened from the day panel.
//! **Signals & state:** every field is an `edit_*` signal, seeded when the form opens; `edit_orig`
//! holds the row they were seeded from. `edit_open` gates the dialog and `save_busy` the button. A
//! successful save shuts the form and refetches the operation list.
//! **Invariants:** the save sends **only the fields that changed**, diffed against `edit_orig`.
//! Every field the endpoint takes is optional and present means write, so posting the whole form
//! back would re-send a start time on a save that only renamed the operation — and a start time in
//! the body is what the server's pre-start guard measures. Blanking the briefing or the banner is
//! itself a change: the empty string is sent and clears the field, while leaving the key out leaves
//! it alone. Start times are compared as instants, not as text. A save with nothing changed sends
//! no request at all. The date field exists because reopening a started operation requires pushing
//! its start into the future in the same request, which is otherwise unexpressible.
#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use super::dates::{combine_iso, parse_date_value, same_instant, split_hm};
use super::lifecycle::{can_transition, EVENT_STATUSES};
use super::mission_picker::attached_missions;
use super::state::Manager;
use crate::v2::core::ui::{cn, Dialog, MaterialIcon};
use leptos::prelude::*;

/// The Edit Operation form.
///
/// Renders the date and time fields, the name, briefing, banner and slot-ceiling fields, the
/// attached-mission section, the lifecycle picker, the registration switch and the save button.
pub(super) fn edit_dialog(st: Manager) -> impl IntoView {
    let Manager {
        store,
        events,
        edit_open,
        edit_orig,
        edit_date,
        edit_time,
        edit_name,
        edit_briefing,
        edit_banner,
        edit_max_slots,
        edit_reg_open,
        edit_status,
        save_busy,
        ..
    } = st;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, events);

    let on_save_edit = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let Some(orig) = edit_orig.get_untracked() else {
                return;
            };
            if save_busy.get_untracked() {
                return;
            }
            let Some((y, m0, d)) = parse_date_value(&edit_date.get_untracked()) else {
                toasts.error("Start date is required");
                return;
            };
            let t = edit_time.get_untracked();
            if t.is_empty() {
                toasts.error("Start time is required");
                return;
            }
            let (hh, mm) = split_hm(&t);
            let start_iso = combine_iso(y, m0, d, hh, mm);

            let mut body = serde_json::Map::new();
            if !same_instant(&start_iso, &orig.start_time) {
                body.insert("start_time".into(), start_iso.into());
            }
            let nm = edit_name.get_untracked();
            if nm != orig.name_override.clone().unwrap_or_default() {
                body.insert("name_override".into(), nm.into());
            }
            let br = edit_briefing.get_untracked();
            if br != orig.briefing.clone().unwrap_or_default() {
                body.insert("briefing".into(), br.into());
            }
            let bn = edit_banner.get_untracked();
            if bn != orig.banner_image_url.clone().unwrap_or_default() {
                body.insert("banner_image_url".into(), bn.into());
            }
            let raw_slots = edit_max_slots.get_untracked();
            let slots = raw_slots.trim();
            let slots: i64 = if slots.is_empty() {
                0
            } else {
                match slots.parse::<i64>() {
                    Ok(v) if v >= 0 => v,
                    _ => {
                        toasts.error("Max slots must be a whole number of 0 or more");
                        return;
                    }
                }
            };
            if slots != orig.max_slots {
                body.insert("max_slots".into(), slots.into());
            }
            let locked = !edit_reg_open.get_untracked();
            if locked != orig.registration_locked {
                body.insert("registration_locked".into(), locked.into());
            }
            let st = edit_status.get_untracked();
            if st != orig.status {
                body.insert("status".into(), st.into());
            }
            if body.is_empty() {
                edit_open.set(false);
                toasts.message("No changes to save");
                return;
            }

            save_busy.set(true);
            let path = format!("/events/{}", orig.id);
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_patch::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::Value::Object(body),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success("Operation updated");
                        edit_open.set(false);
                        events.refetch();
                    }
                    // The transition 409s carry the server's own sentence (including "reschedule
                    // it in the same request to postpone it"), which is more useful than anything
                    // this page could invent — show it verbatim.
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not update operation",
                    )),
                }
                save_busy.set(false);
            });
        }
    };

    view! {
        <Dialog open=edit_open title="Edit Operation">
            <div class="flex flex-wrap items-center gap-3">
                <label class="flex w-fit items-center gap-2 rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface focus-within:ring-1 focus-within:ring-primary/50">
                    <MaterialIcon
                        name="calendar_month"
                        class="text-base text-on-surface-variant"
                    />
                    <input
                        type="date"
                        aria-label="Start date"
                        prop:value=move || edit_date.get()
                        on:input=move |ev| edit_date.set(event_target_value(&ev))
                        class="bg-transparent text-on-surface outline-none [color-scheme:dark]"
                    />
                </label>
                <label class="flex w-fit items-center gap-2 rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface focus-within:ring-1 focus-within:ring-primary/50">
                    <MaterialIcon name="schedule" class="text-base text-on-surface-variant" />
                    <input
                        type="time"
                        aria-label="Start time"
                        prop:value=move || edit_time.get()
                        on:input=move |ev| edit_time.set(event_target_value(&ev))
                        class="bg-transparent text-on-surface outline-none [color-scheme:dark]"
                    />
                </label>
            </div>

            <input
                aria-label="Operation name"
                prop:value=move || edit_name.get()
                on:input=move |ev| edit_name.set(event_target_value(&ev))
                placeholder="Operation name (e.g. Twin Theaters)"
                class="mt-3 w-full rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50"
            />

            <textarea
                aria-label="Briefing"
                rows="4"
                prop:value=move || edit_briefing.get()
                on:input=move |ev| edit_briefing.set(event_target_value(&ev))
                placeholder="Briefing (Markdown supported)"
                class="mt-3 w-full resize-y rounded-2xl bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50"
            ></textarea>

            <div class="mt-3 flex flex-wrap gap-3">
                <input
                    aria-label="Banner image URL"
                    prop:value=move || edit_banner.get()
                    on:input=move |ev| edit_banner.set(event_target_value(&ev))
                    placeholder="Banner image URL"
                    class="min-w-0 flex-1 rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50"
                />
                <input
                    type="number"
                    min="0"
                    aria-label="Max slots"
                    prop:value=move || edit_max_slots.get()
                    on:input=move |ev| edit_max_slots.set(event_target_value(&ev))
                    placeholder="Max slots"
                    class="w-32 rounded-full bg-white/5 px-5 py-3 font-mono text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50 [color-scheme:dark]"
                />
            </div>

            {attached_missions(st)}

            // Lifecycle status — only the transitions the server will actually accept are
            // offered; the rules the browser cannot evaluate stay server-side.
            <div class="mt-6">
                <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                    "Status"
                </p>
                {move || {
                    let from = edit_orig.get().map(|o| o.status).unwrap_or_default();
                    let opts: Vec<(&str, &str)> = EVENT_STATUSES
                        .iter()
                        .copied()
                        .filter(|(v, _)| can_transition(&from, v))
                        .collect();
                    let terminal = opts.len() <= 1;
                    view! {
                        <select
                            aria-label="Lifecycle status"
                            prop:value=move || edit_status.get()
                            prop:disabled=terminal
                            on:change=move |ev| edit_status.set(event_target_value(&ev))
                            class="w-full rounded-full border border-white/10 bg-white/5 px-5 py-3 text-sm text-on-surface outline-none focus:border-primary/50 disabled:opacity-50"
                        >
                            {opts
                                .into_iter()
                                .map(|(v, l)| view! { <option value=v>{l}</option> })
                                .collect_view()}
                        </select>
                        {terminal
                            .then(|| {
                                view! {
                                    <p class="mt-2 px-1 text-xs text-on-surface-variant/70">
                                        "Completed and cancelled operations are terminal — rerunning one is a new operation, not an edit."
                                    </p>
                                }
                            })}
                    }
                }}
            </div>

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
                                    on:click=move |_| edit_reg_open.set(is_open)
                                    class=move || {
                                        cn(
                                            &[
                                                "rounded-full px-6 py-2 text-sm font-medium transition",
                                                if edit_reg_open.get() == is_open {
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

            <button
                type="button"
                on:click=on_save_edit
                prop:disabled=move || save_busy.get()
                class="mt-8 w-full rounded-full bg-action py-4 text-base font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
            >
                {move || if save_busy.get() { "Saving…" } else { "Save Changes" }}
            </button>
        </Dialog>
    }
}
