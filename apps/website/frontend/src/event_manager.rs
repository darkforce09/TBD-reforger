//! Operations Calendar (/admin/events) — ported from pages/admin.tsx `EventManagerPage`. `<AdminGate>`
//! → a month calendar (leading blanks + day cells padded to whole weeks) + a per-day operations panel
//! + the Schedule Operation flow.
//!
//! T-159.25: fully interactive — live `/events?scope=all` + `/missions?scope=global` Resources,
//! month paging, day selection, per-day operation list + selection, the frosted create Dialog
//! (time/name/staged-mission attach dropdown/registration segmented control → POST /events, then
//! POST /events/:id/missions per staged mission), and delete with the Aegis confirm
//! (DELETE /events/:id). Calendar date math stays on `js_sys::Date` (freeze.js parity).
//!
//! T-226: the create half shipped without its edit half — `PATCH /events/:id` and
//! `DELETE /events/:id/missions/:emid` both existed and worked with **no caller anywhere in the
//! SPA**, so once an operation was published its time, name, briefing, capacity, registration
//! lock and (since T-225) its lifecycle **status** were frozen forever, and an attached mission
//! could never be removed. Both are now driven from the selected operation: an **Edit Operation**
//! Dialog (same frosted vocabulary as create) PATCHes only the fields that actually changed, and
//! its "Attached Missions" roster detaches through the same Aegis confirm the delete uses —
//! detaching drops that mission's ORBAT slots *and* every registration on it, which is exactly as
//! destructive as deleting the operation.
#![allow(dead_code)]
use crate::datefmt::format_local_datetime;
use crate::dto::{EventHub, EventListItem, EventMissionDossier, MissionCard, Paginated};
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

/// `combineDateTime(date, time).toISOString()` — a LOCAL wall-clock (y, m0, d, hh:mm) as an
/// instant. Shared by publish and edit so a rescheduled operation lands on the same instant a
/// freshly-published one would: the calendar, `iso_day_key` and this constructor all agree on the
/// browser's zone, and only the ISO string that goes on the wire is UTC.
fn combine_iso(y: i32, m0: i32, d: u32, hh: i32, mm: i32) -> String {
    js_sys::Date::new_with_year_month_day_hr_min(y as u32, m0, d as i32, hh, mm)
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

/// `"HH:MM"` → `(hh, mm)`, lenient exactly like the original publish handler (an unparseable
/// component reads 0) — callers reject the empty string before getting here.
fn split_hm(t: &str) -> (i32, i32) {
    t.split_once(':')
        .map(|(h, m)| (h.parse().unwrap_or(0), m.parse().unwrap_or(0)))
        .unwrap_or((0, 0))
}

/// `"YYYY-MM-DD"` (an `<input type="date">` value) → `(year, month0, day)`. `None` on anything
/// the browser would not have produced, which is how a cleared date field is rejected.
fn parse_date_value(s: &str) -> Option<(i32, i32, u32)> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: i32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m - 1, d))
}

/// An instant → the LOCAL `"YYYY-MM-DD"` an `<input type="date">` wants. Empty on an invalid ISO
/// string (the field then reads blank rather than showing a NaN date).
fn iso_date_value(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// An instant → the LOCAL `"HH:MM"` an `<input type="time">` wants.
fn iso_time_value(iso: &str) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

/// Whether two ISO strings name the same instant.
///
/// The edit form must compare INSTANTS, not strings: the row's `start_time` comes back from
/// Postgres (`…:00Z`, no sub-second part) while the form rebuilds it through
/// `Date.toISOString()` (`…:00.000Z`). A string compare would call an untouched start time
/// "changed" on every single save, and a `start_time` in the PATCH body is not inert — it is the
/// value the backend's pre-start guard measures against `now()`.
fn same_instant(a: &str, b: &str) -> bool {
    let (a, b) = (
        js_sys::Date::new(&wasm_bindgen::JsValue::from_str(a)).get_time(),
        js_sys::Date::new(&wasm_bindgen::JsValue::from_str(b)).get_time(),
    );
    !a.is_nan() && !b.is_nan() && a == b
}

/// The six event lifecycle states (T-225), wire value → label.
const EVENT_STATUSES: [(&str, &str); 6] = [
    ("scheduled", "Scheduled"),
    ("open", "Open"),
    ("locked", "Locked"),
    ("live", "Live"),
    ("completed", "Completed"),
    ("cancelled", "Cancelled"),
];

/// Legal `from → to` moves — a mirror of `handlers::events::can_transition`, which stays the
/// source of truth. This exists only so the status picker cannot *offer* a move the server will
/// refuse; every rule the client cannot evaluate is still enforced server-side and surfaced
/// verbatim. In particular the "postponed" rule — going back to a pre-start state requires the
/// post-PATCH `start_time` to be in the FUTURE, measured by Postgres' clock, not the browser's —
/// is deliberately not replicated: the 409 it raises carries the instruction to reschedule in the
/// same request, and `api_error_message` shows it as-is.
fn can_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "scheduled" => matches!(to, "open" | "locked" | "live" | "cancelled"),
        "open" => matches!(to, "locked" | "live" | "cancelled"),
        "locked" => matches!(to, "open" | "live" | "cancelled"),
        "live" => matches!(to, "open" | "locked" | "completed" | "cancelled"),
        // `completed` and `cancelled` are terminal — an operation that was fought or called off
        // is a matter of record. An unknown status (a state added server-side before this table
        // learns about it) is treated the same way: offer nothing rather than guess.
        _ => false,
    }
}

/// The Edit dialog's attached-mission roster. Distinguishes "nothing to fetch" from "the fetch
/// failed" — see the `hub` Resource for why collapsing them is a visible bug — and carries the
/// event id it belongs to, because a Resource keeps serving its LAST value while the next run is
/// in flight: without the id, opening Edit on operation B shows operation A's missions until B's
/// hub lands, and a fast click would send B's id with A's `event_mission_id`.
#[derive(Clone, PartialEq)]
enum Roster {
    /// The dialog is shut (or nothing is selected): there is no roster to show, and any run still
    /// in flight should read as loading rather than as a failure.
    Idle,
    Failed,
    Loaded(String, Vec<EventMissionDossier>),
}

/// Badge variant for a lifecycle status, so the day list shows where an operation *is* and not
/// just whether registration happens to be locked.
fn status_badge(status: &str) -> String {
    badge_class(match status {
        "open" => "success",
        "locked" => "warning",
        "live" => "primary",
        "completed" => "tertiary",
        "cancelled" => "error",
        _ => "neutral",
    })
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

    // Form state (Edit Operation dialog — T-226). `edit_orig` is the row the dialog was opened
    // from: the PATCH diffs against it so an edit to one field cannot quietly rewrite the other
    // six, and so a save with nothing changed sends no request at all.
    let edit_open = RwSignal::new(false);
    let edit_orig = RwSignal::new(None::<EventListItem>);
    let edit_date = RwSignal::new(String::new());
    let edit_time = RwSignal::new(String::new());
    let edit_name = RwSignal::new(String::new());
    let edit_briefing = RwSignal::new(String::new());
    let edit_banner = RwSignal::new(String::new());
    let edit_max_slots = RwSignal::new(String::new());
    let edit_reg_open = RwSignal::new(true);
    let edit_status = RwSignal::new(String::new());
    let save_busy = RwSignal::new(false);
    // (event_mission_id, title) of the mission the detach confirm is armed for.
    let detach_target = RwSignal::new(None::<(String, String)>);
    let detach_open = RwSignal::new(false);
    let detach_busy = RwSignal::new(false);

    // The attached-mission roster is NOT on the list row — `mission_count` is a number, and the
    // `event_mission_id` that `DELETE /events/:id/missions/:emid` keys on exists only on the hub
    // payload. Keyed on the edit dialog rather than on selection so clicking through a day's
    // operations does not fire a hub GET per click; the roster is only ever shown in the dialog.
    //
    // `Roster` is a three-state enum rather than an `Option<EventHub>` because a Resource keeps
    // serving its LAST value while the next run is in flight, and `Option` would collapse three
    // different situations into one `None`: "the dialog is shut, nothing to fetch", "the fetch
    // failed", and "this value belongs to the operation you were looking at BEFORE". Rendered as
    // one `None` those become a spurious "Could not load attached missions." over a request that
    // is about to succeed, and — worse — another operation's `event_mission_id` under a Detach
    // button whose event id has already moved on.
    let hub = LocalResource::new(move || {
        let id = edit_open.get().then(|| selected_event.get()).flatten();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match id {
                    Some(id) => {
                        match crate::client::api_get::<EventHub>(store, &format!("/events/{id}"))
                            .await
                        {
                            Ok(h) => Roster::Loaded(id, h.missions),
                            Err(_) => Roster::Failed,
                        }
                    }
                    None => Roster::Idle,
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                Roster::Idle
            }
        }
    });

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

    // Seed the Edit dialog from the selected row. Deliberately reads the row from the day list
    // instead of the hub: `EventListItem` already carries every field `PATCH /events/:id` accepts,
    // so the form is populated the instant it opens rather than after a round trip.
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
        // Flipping this is what drives the `hub` Resource — measured in the browser: the hub GET
        // is in flight ~immediately after this set, on the first open as on every later one. No
        // manual refetch is needed here (one IS needed after a detach, where no source changes).
        edit_open.set(true);
    };

    // handleSaveEdit: PATCH /events/:id with ONLY the fields that changed.
    //
    // Every field on the input is `Option`, and "present" means "write this" — so posting the
    // whole form back would re-send `start_time` on a save that only renamed the operation, and
    // `start_time` is the value the backend's pre-start guard measures. Diffing keeps a rename a
    // rename. `status` is diffed for the same reason plus one more: `from == to` is a legal no-op
    // server-side, but leaving it out means a rename can never be the thing that trips a 409.
    let on_save_edit = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
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
                match crate::client::api_patch::<serde_json::Value>(
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
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not update operation",
                    )),
                }
                save_busy.set(false);
            });
        }
    };

    // confirmDetach: DELETE /events/:id/missions/:emid behind the same Aegis confirm the operation
    // delete uses — the backend drops the mission's ORBAT slots and every registration on it, so
    // this is destructive in exactly the way an unconfirmed click must never be.
    let on_confirm_detach = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let (Some(id), Some((emid, _))) = (
                selected_event.get_untracked(),
                detach_target.get_untracked(),
            ) else {
                return;
            };
            detach_open.set(false);
            detach_target.set(None);
            if detach_busy.get_untracked() {
                return;
            }
            detach_busy.set(true);
            let toasts = crate::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::client::api_delete(store, &format!("/events/{id}/missions/{emid}"))
                    .await
                {
                    Ok(()) => {
                        toasts.success("Mission detached");
                        // Both: the roster loses a row and the day list's mission_count drops.
                        hub.refetch();
                        events.refetch();
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not detach mission",
                    )),
                }
                detach_busy.set(false);
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
                                            // T-226: the lifecycle status is now settable, so it has to be
                                            // VISIBLE — otherwise the only place an admin can read the state
                                            // they just set is the form they set it in.
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

            // Edit Operation (T-226) — the missing PATCH caller. Same frosted vocabulary as
            // create; the DATE field is the one addition, and it is not decoration: the backend
            // refuses a move back to a pre-start state unless the same request pushes the start
            // time into the future, so "postpone and reopen" is only expressible with it.
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

                // Attached missions — the detach half of the ticket. `event_mission_id` is only
                // on the hub payload, so this section is what the `hub` Resource exists for.
                <div class="mt-6">
                    <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                        "Attached Missions"
                    </p>
                    <div class="space-y-2">
                        {move || {
                            let current = selected_event.get();
                            match hub.get() {
                                Some(Roster::Failed) => {
                                    view! {
                                        <p class="px-1 text-sm text-error-alert">
                                            "Could not load attached missions."
                                        </p>
                                    }
                                        .into_any()
                                }
                                Some(Roster::Loaded(id, ms))
                                    if Some(id.as_str()) == current.as_deref() && ms.is_empty() =>
                                {
                                    view! {
                                        <p class="px-1 text-sm text-on-surface-variant/70">
                                            "No missions attached."
                                        </p>
                                    }
                                        .into_any()
                                }
                                Some(Roster::Loaded(id, ms)) if Some(id.as_str()) == current.as_deref() => {
                                    ms
                                        .into_iter()
                                        .map(|m| {
                                            let emid = m.event_mission_id.clone();
                                            let title = m.title.clone();
                                            let aria = format!("Detach {}", m.title);
                                            let meta = format!(
                                                "{} · {}/{} filled",
                                                format_local_datetime(&m.start_time),
                                                m.filled,
                                                m.total,
                                            );
                                            view! {
                                                <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                                    <MaterialIcon name="map" class="text-on-surface-variant" />
                                                    <div class="min-w-0 flex-1">
                                                        <p class="truncate text-sm text-on-surface">{m.title.clone()}</p>
                                                        <p class="mt-0.5 font-mono text-xs text-on-surface-variant">
                                                            {meta}
                                                        </p>
                                                    </div>
                                                    <button
                                                        type="button"
                                                        on:click=move |_| {
                                                            detach_target.set(Some((emid.clone(), title.clone())));
                                                            detach_open.set(true);
                                                        }
                                                        prop:disabled=move || detach_busy.get()
                                                        aria-label=aria
                                                        class="flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert disabled:opacity-40"
                                                    >
                                                        <MaterialIcon name="link_off" class="text-base" />
                                                    </button>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }
                                // Never resolved, the shut-dialog `Idle`, or a roster still
                                // belonging to the PREVIOUS operation — all of them mean the
                                // answer for THIS operation has not arrived yet.
                                _ => {
                                    view! {
                                        <p class="px-1 text-sm text-on-surface-variant/70">"Loading…"</p>
                                    }
                                        .into_any()
                                }
                            }
                        }}
                    </div>
                </div>

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

            // Destructive confirm for mission detach (T-226). LAST in the tree on purpose: it and
            // the edit Dialog share z-50, so DOM order is what puts the confirm on top of the form
            // that launched it.
            <Dialog
                open=detach_open
                title="Detach this mission?"
                description="The mission's ORBAT slots and every registration on it are deleted. The mission itself stays in the library. This cannot be undone."
            >
                <p class="mb-4 truncate text-sm text-on-surface">
                    {move || detach_target.get().map(|(_, title)| title)}
                </p>
                <div class="flex justify-end gap-2">
                    <button
                        type="button"
                        on:click=move |_| {
                            detach_open.set(false);
                            detach_target.set(None);
                        }
                        class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        on:click=on_confirm_detach
                        prop:disabled=move || detach_busy.get()
                        class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                    >
                        "Detach mission"
                    </button>
                </div>
            </Dialog>
        </div>
    }
}
