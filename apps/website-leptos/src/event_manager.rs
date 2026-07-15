//! Operations Calendar (/admin/events) — ported from pages/admin.tsx `EventManagerPage`. `<AdminGate>`
//! → a month calendar (leading blanks + day cells padded to whole weeks) + a per-day operations panel
//! + a "Schedule Operation" button. The two Dialogs (create form / delete confirm) start closed, so a
//! base-ui `open={false}` renders nothing in the DOM — omitted here (they're behavior, a follow-up).
//!
//! **Date math parity:** every calendar/day-panel date goes through `js_sys::Date` — the SAME JS `Date`
//! freeze.js patches to the fixed epoch on both apps. So the grid (leading offset, days-in-month, the
//! selected/today cell) and the `toLocaleDateString(undefined, …)` header match React by construction,
//! and in production the calendar shows the real current month. No `chrono`; no hardcoded month.
//!
//! **Gate scope:** the default render — empty `/events` + `/missions` goldens → today's month, today
//! selected, the day panel's empty state, no selected-event delete button, both dialogs closed.
#![allow(dead_code)]
use crate::ui::{AdminGate, MaterialIcon};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// Mirror `date.toLocaleDateString(undefined, options)` exactly — same receiver, undefined locale,
/// same options object — by calling the JS method reflectively (js_sys `to_locale_date_string` can't
/// express the undefined locale). Browser-only; not exercised by native tests.
fn locale_date_string(date: &js_sys::Date, options: &[(&str, &str)]) -> String {
    let opts = js_sys::Object::new();
    for (k, v) in options {
        let _ = js_sys::Reflect::set(&opts, &(*k).into(), &(*v).into());
    }
    let f = match js_sys::Reflect::get(date, &"toLocaleDateString".into()) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let f: js_sys::Function = match f.dyn_into() {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    f.call2(date, &wasm_bindgen::JsValue::UNDEFINED, &opts)
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

#[component]
pub fn EventManagerPage() -> impl IntoView {
    view! {
        <AdminGate>
            <EventManagerInner />
        </AdminGate>
    }
}

#[component]
fn EventManagerInner() -> impl IntoView {
    // today = selectedDate = viewMonth basis — new Date() under the frozen clock.
    let today = js_sys::Date::new_0();
    let year = today.get_full_year();
    let month = today.get_month() as i32; // 0-11
    let sel_day = today.get_date();
    let leading = js_sys::Date::new_with_year_month_day(year, month, 1).get_day();
    let days_in_month = js_sys::Date::new_with_year_month_day(year, month + 1, 0).get_date();
    let panel_date = locale_date_string(
        &today,
        &[
            ("weekday", "short"),
            ("month", "short"),
            ("day", "numeric"),
            ("year", "numeric"),
        ],
    );

    // leading blanks + each day (1..=daysInMonth) padded to whole weeks.
    let mut cells: Vec<Option<u32>> = Vec::new();
    for _ in 0..leading {
        cells.push(None);
    }
    for d in 1..=days_in_month {
        cells.push(Some(d));
    }
    while cells.len() % 7 != 0 {
        cells.push(None);
    }

    let month_name = MONTH_NAMES[month as usize];
    view! {
        <div class="mx-auto h-full w-full max-w-5xl">
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
                            {month_name}
                            " "
                            {year}
                        </h2>
                        <div class="flex items-center gap-1">
                            <button
                                type="button"
                                aria-label="Previous month"
                                class="flex size-9 items-center justify-center rounded-full text-on-surface-variant transition hover:bg-white/5 hover:text-white"
                            >
                                <MaterialIcon name="chevron_left" />
                            </button>
                            <button
                                type="button"
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
                        {cells
                            .into_iter()
                            .map(|cell| match cell {
                                None => view! { <div></div> }.into_any(),
                                Some(d) => {
                                    let is_selected = d == sel_day;
                                    // isToday == isSelected here (today == selectedDate), so the
                                    // `!isSelected && isToday` class is always the empty arm.
                                    let class = crate::ui::cn(
                                        &[
                                            "flex aspect-square flex-col items-center justify-center gap-1.5 rounded-xl text-sm transition",
                                            if is_selected {
                                                "bg-action text-on-action shadow-[0_0_20px_rgba(59,130,246,0.4)]"
                                            } else {
                                                "text-on-surface hover:bg-white/5"
                                            },
                                            "",
                                        ],
                                    );
                                    view! {
                                        <button type="button" class=class>
                                            <span>{d}</span>
                                            <span class="flex h-1 items-center gap-0.5"></span>
                                        </button>
                                    }
                                        .into_any()
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                // ── Right: scheduled operations for the selected day ──
                <div class="lg:col-span-4 lg:border-l lg:border-white/5 lg:pl-8">
                    <p class="font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                        {panel_date}
                    </p>
                    <h2 class="mt-1 mb-4 text-lg font-bold tracking-tight text-white">
                        "Scheduled Operations"
                    </h2>
                    // dayOps empty (empty /events golden) → the empty state.
                    <p class="text-sm text-on-surface-variant">
                        "No operations scheduled. "
                        <button type="button" class="text-primary hover:underline">
                            "Schedule one."
                        </button>
                    </p>
                // selectedEventId null → no "Delete Selected Operation" button.
                </div>
            </div>
        // Both Dialogs (formOpen / confirmDeleteOpen) start closed → not in the DOM.
        </div>
    }
}
