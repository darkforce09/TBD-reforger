//! Operations Calendar (/admin/events) — ported from pages/admin.tsx `EventManagerPage`. `<AdminGate>`
//! → a month calendar (leading blanks + day cells padded to whole weeks) + a per-day operations panel
//! + the Schedule Operation flow.
//!
//! T-159.25: fully interactive — live `/events?scope=all` + `/missions?scope=global` Resources,
//! month paging, day selection, per-day operation list + selection, the frosted create Dialog
//! (time/name/staged-mission attach dropdown/registration segmented control → POST /events, then
//! POST /events/:id/missions per staged mission), and delete with the Aegis confirm
//! (DELETE /events/:id). Calendar date math stays on `js_sys::Date` (freeze.js parity).
#![allow(dead_code)]
use crate::datefmt::format_local_datetime;
use crate::dto::{EventListItem, MissionCard, Paginated};
use crate::ui::{badge_class, cn, AdminGate, Dialog, MaterialIcon};
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

/// Local YYYY-MM-DD key for a (year, month0, day) triple — admin.tsx `dayKey` (no UTC drift).
fn day_key(y: i32, m0: i32, d: u32) -> String {
    format!("{y:04}-{:02}-{d:02}", m0 + 1)
}

/// `dayKey(new Date(iso))` — the event's LOCAL calendar day.
fn iso_day_key(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    day_key(d.get_full_year() as i32, d.get_month() as i32, d.get_date())
}

fn js_date(y: i32, m0: i32, d: u32) -> js_sys::Date {
    js_sys::Date::new_with_year_month_day(y as u32, m0, d as i32)
}

fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "—".into(),
    }
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
    let store = expect_context::<crate::auth::AuthStore>();
    // today = selectedDate = viewMonth basis — new Date() under the frozen clock.
    let today = js_sys::Date::new_0();
    let today_key = day_key(
        today.get_full_year() as i32,
        today.get_month() as i32,
        today.get_date(),
    );
    let today_key = StoredValue::new(today_key);
    // (year, month0) of the visible month; (year, month0, day) of the selected date.
    let view = RwSignal::new((today.get_full_year() as i32, today.get_month() as i32));
    let selected = RwSignal::new((
        today.get_full_year() as i32,
        today.get_month() as i32,
        today.get_date(),
    ));

    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<EventListItem>>(store, "/events?scope=all")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<EventListItem>>
        }
    });
    let missions = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<MissionCard>>(store, "/missions?scope=global")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<MissionCard>>
        }
    });

    // Form state (Schedule Operation dialog).
    let name = RwSignal::new(String::new());
    let time = RwSignal::new("19:00".to_string());
    let open_reg = RwSignal::new(true);
    let staged = RwSignal::new(Vec::<(String, String)>::new()); // (id, title)
    let attach_open = RwSignal::new(false);
    let form_open = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);
    let selected_event = RwSignal::new(None::<String>);
    let publish_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);

    let shift_month = move |delta: i32| {
        view.update(|(y, m)| {
            let total = *y * 12 + *m + delta;
            *y = total.div_euclid(12);
            *m = total.rem_euclid(12);
        });
    };

    // Group events by local day (recomputed reactively from the Resource).
    let events_by_day = move || {
        let mut map = std::collections::HashMap::<String, Vec<EventListItem>>::new();
        if let Some(Some(page)) = events.get() {
            for e in page.data {
                map.entry(iso_day_key(&e.start_time)).or_default().push(e);
            }
        }
        map
    };
    let day_ops = move || {
        let (y, m, d) = selected.get();
        events_by_day()
            .remove(&day_key(y, m, d))
            .unwrap_or_default()
    };

    let select_day = move |y: i32, m: i32, d: u32| {
        selected.set((y, m, d));
        attach_open.set(false);
        let ops = events_by_day()
            .remove(&day_key(y, m, d))
            .unwrap_or_default();
        selected_event.set(ops.first().map(|o| o.id.clone()));
    };

    // handlePublish: POST /events, then POST /events/:id/missions per staged mission.
    let on_publish = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
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
            // combineDateTime(selectedDate, time).toISOString()
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
                match crate::client::api_post::<serde_json::Value>(store, "/events", body).await {
                    Ok(created) => {
                        let id = created
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let n = to_attach.len();
                        if !id.is_empty() {
                            for (mid, _) in to_attach {
                                let _ = crate::client::api_post::<serde_json::Value>(
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

    // confirmDelete: DELETE /events/:id through the Aegis confirm (F2F-07).
    let on_confirm_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(id) = selected_event.get_untracked() else {
                return;
            };
            confirm_open.set(false);
            if delete_busy.get_untracked() {
                return;
            }
            delete_busy.set(true);
            let toasts = crate::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::client::api_delete(store, &format!("/events/{id}")).await {
                    Ok(()) => {
                        toasts.success("Operation deleted");
                        selected_event.set(None);
                        events.refetch();
                    }
                    Err(_) => toasts.error("Failed to delete operation"),
                }
                delete_busy.set(false);
            });
        }
    };

    view! {
        <div class="mx-auto h-full w-full max-w-5xl">
            // Header — primary action opens the frosted create form over the calendar.
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
                            // Two text nodes ("November" + " 2026"), matching React's
                            // `{monthName} {year}` JSX (the frozen V golden pins the node split).
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
                                // Plain pad divs — React renders them attribute-less and the frozen
                                // V golden pins that (no synthetic ids).
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
                                                    <span class=badge>{label}</span>
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
                                    <button
                                        type="button"
                                        on:click=move |_| confirm_open.set(true)
                                        prop:disabled=move || delete_busy.get()
                                        class="mt-4 w-full rounded-full py-3 text-sm font-medium text-error-alert transition hover:bg-error-alert/10 disabled:cursor-not-allowed disabled:opacity-40"
                                    >
                                        "Delete Selected Operation"
                                    </button>
                                }
                            })
                    }}
                </div>
            </div>

            // Destructive confirm for operation delete (F2F-07) — Aegis Dialog.
            <Dialog
                open=confirm_open
                title="Delete this operation?"
                description="The operation, its attached missions' ORBATs, and all registrations are removed. This cannot be undone."
            >
                <div class="flex justify-end gap-2">
                    <button
                        type="button"
                        on:click=move |_| confirm_open.set(false)
                        class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        on:click=on_confirm_delete
                        prop:disabled=move || delete_busy.get()
                        class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                    >
                        "Delete operation"
                    </button>
                </div>
            </Dialog>

            // Frosted create form — overlays the calendar, preserving context.
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

                // Mission multi-select
                <div class="mt-6">
                    <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                        "Missions"
                    </p>
                    <div class="space-y-2">
                        {move || {
                            let list = staged.get();
                            if list.is_empty() {
                                view! {
                                    <p class="px-1 text-sm text-on-surface-variant/70">
                                        "No missions attached yet."
                                    </p>
                                }
                                    .into_any()
                            } else {
                                list.into_iter()
                                    .map(|(id, title)| {
                                        let title_label = title.clone();
                                        view! {
                                            <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                                <MaterialIcon name="map" class="text-on-surface-variant" />
                                                <span class="flex-1 text-sm text-on-surface">
                                                    {title_label}
                                                </span>
                                                <button
                                                    type="button"
                                                    on:click=move |_| {
                                                        staged.update(|s| s.retain(|(sid, _)| sid != &id))
                                                    }
                                                    aria-label=format!("Remove {title}")
                                                    class="flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                                >
                                                    <MaterialIcon name="close" class="text-base" />
                                                </button>
                                            </div>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        }}
                    </div>

                    // + Attach Mission dropdown
                    <div class="relative mt-2">
                        <button
                            type="button"
                            on:click=move |_| attach_open.update(|o| *o = !*o)
                            class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5"
                        >
                            <MaterialIcon name="add" class="text-base" />
                            "Attach Mission"
                        </button>
                        {move || {
                            attach_open
                                .get()
                                .then(|| {
                                    let available: Vec<MissionCard> = missions
                                        .get()
                                        .flatten()
                                        .map(|p| p.data)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .filter(|m| {
                                            !staged.get().iter().any(|(id, _)| id == &m.id)
                                        })
                                        .collect();
                                    view! {
                                        <div class="absolute z-10 mt-2 max-h-64 w-full overflow-y-auto rounded-xl border border-white/10 bg-surface-container-high/95 p-1 shadow-2xl backdrop-blur-xl">
                                            {if available.is_empty() {
                                                view! {
                                                    <p class="px-3 py-2 text-sm text-on-surface-variant">
                                                        "No more missions in the library."
                                                    </p>
                                                }
                                                    .into_any()
                                            } else {
                                                available
                                                    .into_iter()
                                                    .map(|m| {
                                                        let id = m.id.clone();
                                                        let title = m.title.clone();
                                                        let terrain = terrain_label(&m.terrain);
                                                        view! {
                                                            <button
                                                                type="button"
                                                                on:click=move |_| {
                                                                    staged.update(|s| s.push((id.clone(), title.clone())));
                                                                    attach_open.set(false);
                                                                }
                                                                class="flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm text-on-surface transition hover:bg-white/5"
                                                            >
                                                                <span class="truncate">{m.title.clone()}</span>
                                                                <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                                    {terrain}
                                                                </span>
                                                            </button>
                                                        }
                                                    })
                                                    .collect_view()
                                                    .into_any()
                                            }}
                                        </div>
                                    }
                                })
                        }}
                    </div>
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
        </div>
    }
}
