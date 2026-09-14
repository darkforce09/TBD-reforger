//! The month grid and the day panel: where an operation is read, selected and opened for editing.
//!
//! **Role:** the screen's body — the heading with the schedule action, the month calendar with one
//! cell per day, and the panel listing the selected day's operations with the edit and delete
//! controls under it.
//! **Position:** the whole of the operations calendar route below its own header, above the four
//! dialogs.
//! **Signals & state:** reads `view` for the month on screen, `selected` for the highlighted day
//! and `selected_event` for the operation in focus; writes all three. Opening the edit form seeds
//! every `edit_*` field from the selected row and then sets `edit_open`, which is what starts the
//! roster fetch.
//! **Invariants:** the grid is padded with attribute-less blanks — leading ones for the weekday the
//! month starts on, trailing ones to a whole number of weeks — so the columns stay aligned. Cells
//! show at most three operation marks. The edit form is seeded from the list row rather than from a
//! fetch, because the row already carries every field the save can send, so the form is populated
//! the instant it opens.
#![allow(dead_code)]

use super::dates::{
    day_key, iso_date_value, iso_time_value, js_date, locale_date_string, MONTH_NAMES, WEEKDAYS,
};
use super::lifecycle::status_badge;
use super::state::Manager;
use crate::v2::core::ui::{badge_class, cn, MaterialIcon};
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

/// The calendar and the selected day's operations.
///
/// Renders the page heading, the month grid, and the day panel with its per-operation rows; the
/// two controls at the foot of the panel open the edit form and arm the delete confirmation.
pub(super) fn event_table(st: Manager) -> impl IntoView {
    let Manager {
        view,
        selected,
        selected_event,
        today_key,
        form_open,
        confirm_open,
        delete_busy,
        edit_orig,
        edit_date,
        edit_time,
        edit_name,
        edit_briefing,
        edit_banner,
        edit_max_slots,
        edit_reg_open,
        edit_status,
        edit_attach_open,
        edit_open,
        ..
    } = st;
    let shift_month = move |delta: i32| st.shift_month(delta);
    let select_day = move |y: i32, m: i32, d: u32| st.select_day(y, m, d);
    let events_by_day = move || st.events_by_day();
    let day_ops = move || st.day_ops();

    // The edit form is seeded from the list row rather than from the roster fetch: the row already
    // carries every field the save can send, so the form is populated the instant it opens. Setting
    // `edit_open` last is what starts the roster request; no manual refetch is needed here.
    let open_edit = move |_| {
        let Some(id) = selected_event.get_untracked() else {
            return;
        };
        let Some(op) = day_ops().into_iter().find(|o| o.id == id) else {
            return;
        };
        edit_date.set(iso_date_value(&op.start_time));
        edit_time.set(iso_time_value(&op.start_time));
        edit_name.set(op.name_override.clone().unwrap_or_default());
        edit_briefing.set(op.briefing.clone().unwrap_or_default());
        edit_banner.set(op.banner_image_url.clone().unwrap_or_default());
        edit_max_slots.set(op.max_slots.to_string());
        edit_reg_open.set(!op.registration_locked);
        edit_status.set(op.status.clone());
        edit_orig.set(Some(op));
        edit_attach_open.set(false);
        edit_open.set(true);
    };

    view! {
        <div class="mb-6 flex flex-wrap items-center justify-between gap-4">
            <div>
                <h1 class="text-headline-md tracking-tight text-on-surface">
                    "Operations Calendar"
                </h1>
                <p class="mt-1 text-sm text-on-surface-variant">
                    "Schedule operations for any day. ORBATs generate from each attached mission."
                </p>
            </div>
            <button
                type="button"
                on:click=move |_| form_open.set(true)
                class="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
            >
                <MaterialIcon name="add" class="text-[18px]" />
                "Schedule Operation"
            </button>
        </div>

        <div class="grid grid-cols-1 gap-8 lg:grid-cols-12">
            // ── Left: tactical calendar ──
            <div class="lg:col-span-8">
                <div class="mb-6 flex items-center justify-between">
                    <h2 class="text-2xl font-bold tracking-tight text-white">
                        // Deliberately two text nodes, not one interpolation: the rendered
                        // markup is pinned with the month and the year as separate nodes.
                        {move || MONTH_NAMES[view.get().1 as usize].to_string()}
                        {move || format!(" {}", view.get().0)}
                    </h2>
                    <div class="flex items-center gap-1">
                        <button
                            type="button"
                            on:click=move |_| shift_month(-1)
                            aria-label="Previous month"
                            class="flex size-9 items-center justify-center rounded-full text-on-surface-variant transition hover:bg-white/5 hover:text-white"
                        >
                            <MaterialIcon name="chevron_left" />
                        </button>
                        <button
                            type="button"
                            on:click=move |_| shift_month(1)
                            aria-label="Next month"
                            class="flex size-9 items-center justify-center rounded-full text-on-surface-variant transition hover:bg-white/5 hover:text-white"
                        >
                            <MaterialIcon name="chevron_right" />
                        </button>
                    </div>
                </div>

                <div class="mb-2 grid grid-cols-7 gap-1">
                    {WEEKDAYS
                        .iter()
                        .map(|w| {
                            view! {
                                <div class="py-2 text-center font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                                    {*w}
                                </div>
                            }
                        })
                        .collect_view()}
                </div>

                <div class="grid grid-cols-7 gap-1">
                    {move || {
                        let (y, m) = view.get();
                        let leading = js_date(y, m, 1).get_day() as usize;
                        let days_in_month = js_sys::Date::new_with_year_month_day(
                                y as u32,
                                m + 1,
                                0,
                            )
                            .get_date();
                        let by_day = events_by_day();
                        let sel = selected.get();
                        let mut cells: Vec<leptos::prelude::AnyView> = Vec::new();
                        for _ in 0..leading {
                            // Attribute-less pads: the rendered markup is pinned with no
                            // identifiers on them.
                            cells.push(view! { <div></div> }.into_any());
                        }
                        for d in 1..=days_in_month {
                            let key = day_key(y, m, d);
                            let is_selected = sel == (y, m, d);
                            let is_today = key == today_key.get_value();
                            let ops = by_day.get(&key).map(|v| v.len()).unwrap_or(0).min(3);
                            cells
                                .push(
                                    view! {
                                        <button
                                            type="button"
                                            on:click=move |_| select_day(y, m, d)
                                            class=cn(
                                                &[
                                                    "flex aspect-square flex-col items-center justify-center gap-1.5 rounded-xl text-sm transition",
                                                    if is_selected {
                                                        "bg-action text-on-action shadow-[0_0_20px_rgba(59,130,246,0.4)]"
                                                    } else {
                                                        "text-on-surface hover:bg-white/5"
                                                    },
                                                    if !is_selected && is_today {
                                                        "font-bold text-primary"
                                                    } else {
                                                        ""
                                                    },
                                                ],
                                            )
                                        >
                                            <span>{d}</span>
                                            <span class="flex h-1 items-center gap-0.5">
                                                {(0..ops)
                                                    .map(|_| {
                                                        view! {
                                                            <span class=if is_selected {
                                                                "h-1 w-4 rounded-full bg-white/70"
                                                            } else {
                                                                "h-1 w-4 rounded-full bg-primary/50"
                                                            }></span>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </span>
                                        </button>
                                    }
                                        .into_any(),
                                );
                        }
                        while cells.len() % 7 != 0 {
                            cells.push(view! { <div></div> }.into_any());
                        }
                        cells.collect_view()
                    }}
                </div>
            </div>

            // ── Right: scheduled operations for the selected day ──
            <div class="lg:col-span-4 lg:border-l lg:border-white/5 lg:pl-8">
                <p class="font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                    {move || {
                        let (y, m, d) = selected.get();
                        locale_date_string(
                            &js_date(y, m, d),
                            &[
                                ("weekday", "short"),
                                ("month", "short"),
                                ("day", "numeric"),
                                ("year", "numeric"),
                            ],
                        )
                    }}
                </p>
                <h2 class="mt-1 mb-4 text-lg font-bold tracking-tight text-white">
                    "Scheduled Operations"
                </h2>

                {move || {
                    let ops = day_ops();
                    if ops.is_empty() {
                        view! {
                            <p class="text-sm text-on-surface-variant">
                                "No operations scheduled. "
                                <button
                                    type="button"
                                    on:click=move |_| form_open.set(true)
                                    class="text-primary hover:underline"
                                >
                                    "Schedule one."
                                </button>
                            </p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="space-y-2">
                                {ops
                                    .into_iter()
                                    .map(|op| {
                                        let oid = op.id.clone();
                                        let active = move || {
                                            selected_event.get().as_deref() == Some(oid.as_str())
                                        };
                                        let oid_click = op.id.clone();
                                        let title = op
                                            .name_override
                                            .clone()
                                            .filter(|n| !n.is_empty())
                                            .unwrap_or_else(|| "Untitled Operation".into());
                                        let meta = format!(
                                            "{} · {} mission{} · {}/{}",
                                            format_local_datetime(&op.start_time),
                                            op.mission_count,
                                            if op.mission_count == 1 { "" } else { "s" },
                                            op.filled,
                                            op.total_slots,
                                        );
                                        let (badge, label) = if op.registration_locked {
                                            (badge_class("neutral"), "Locked")
                                        } else {
                                            (badge_class("success"), "Open")
                                        };
                                        // The lifecycle state is settable, so it has to be
                                        // readable here too — otherwise the only place it can be
                                        // seen is the form that sets it.
                                        let st_class = status_badge(&op.status);
                                        let st_label = op.status.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| selected_event.set(Some(oid_click.clone()))
                                                class=move || {
                                                    cn(
                                                        &[
                                                            "flex w-full items-center justify-between gap-3 rounded-xl border px-4 py-3 text-left transition",
                                                            if active() {
                                                                "border-primary/60 bg-primary/15"
                                                            } else {
                                                                "border-white/10 hover:bg-white/[0.03]"
                                                            },
                                                        ],
                                                    )
                                                }
                                            >
                                                <div class="min-w-0">
                                                    <p class="truncate text-sm font-medium text-on-surface">
                                                        {title}
                                                    </p>
                                                    <p class="mt-0.5 font-mono text-xs text-on-surface-variant">
                                                        {meta}
                                                    </p>
                                                </div>
                                                <span class="flex shrink-0 items-center gap-1.5">
                                                    <span class=st_class>{st_label}</span>
                                                    <span class=badge>{label}</span>
                                                </span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }}

                {move || {
                    selected_event
                        .get()
                        .map(|_| {
                            view! {
                                <div class="mt-4 space-y-2">
                                    <button
                                        type="button"
                                        on:click=open_edit
                                        class="w-full rounded-full border border-white/10 py-3 text-sm font-medium text-on-surface transition hover:bg-white/5"
                                    >
                                        "Edit Selected Operation"
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| confirm_open.set(true)
                                        prop:disabled=move || delete_busy.get()
                                        class="w-full rounded-full py-3 text-sm font-medium text-error-alert transition hover:bg-error-alert/10 disabled:cursor-not-allowed disabled:opacity-40"
                                    >
                                        "Delete Selected Operation"
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
        </div>
    }
}
