//! Everything the operations calendar remembers, and the reads derived from it.
//!
//! **Role:** one copyable handle holding the calendar's position, the two dialogs' form fields,
//! the in-flight flags that keep a double click from sending a second request, and the three
//! fetches the screen runs. The panels take this handle instead of a parameter list.
//! **Position:** created once by the route component; every panel and every action reads it.
//! **Signals & state:** `view` is the month on screen and `selected` the highlighted day;
//! `selected_event` the operation the day panel has focus on. `events` lists every operation in
//! every state, `missions` the global library the attach pickers offer, and `hub` the attached
//! roster of the operation the edit dialog is open on. The `*_busy` flags are set around their
//! request and cleared when it settles.
//! **Invariants:** the resources are browser-only — a native build resolves each to nothing.
//! `hub` is keyed on the edit dialog being open, so closing it stops fetching, and it answers with
//! the event id it belongs to because a resource keeps serving its previous value while the next
//! run is in flight. Days are grouped by the **local** calendar day of each start time, so the
//! grid, the grouping and the forms all agree on which day an operation is on.
#![allow(dead_code)]

use super::dates::{day_key, iso_day_key};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::EventHub;
use crate::v2::core::api::dto::{EventListItem, EventMissionDossier, MissionCard, Paginated};
use crate::v2::core::auth::AuthStore;
use leptos::prelude::*;

/// The attached-mission roster of the operation the edit dialog is open on.
///
/// Three states rather than an option, because collapsing them loses the distinction between
/// "nothing to fetch", "the fetch failed" and "this answer belongs to the operation you were
/// looking at before". Rendered as one absence those become a spurious failure notice over a
/// request that is about to succeed and — worse — another operation's detach target under a
/// button that has already moved on.
#[derive(Clone, PartialEq)]
pub(super) enum Roster {
    /// The dialog is shut, or nothing is selected: there is no roster to show, and a run still in
    /// flight reads as loading rather than as a failure.
    Idle,
    /// The request failed; the dialog says so instead of showing an empty roster.
    Failed,
    /// The roster of the operation with this id.
    Loaded(String, Vec<EventMissionDossier>),
}

/// Every signal and fetch the operations calendar runs on.
#[derive(Clone, Copy)]
pub(super) struct Manager {
    /// The session, for every request this screen makes.
    pub(super) store: AuthStore,
    /// Today's [`day_key`], so the grid can mark the current day.
    pub(super) today_key: StoredValue<String>,
    /// The (year, zero-based month) the grid is showing.
    pub(super) view: RwSignal<(i32, i32)>,
    /// The (year, zero-based month, day) the day panel is showing.
    pub(super) selected: RwSignal<(i32, i32, u32)>,
    /// Every operation in every state, for the grid and the day panel.
    pub(super) events: LocalResource<Option<Paginated<EventListItem>>>,
    /// The global mission library, for both attach pickers.
    pub(super) missions: LocalResource<Option<Paginated<MissionCard>>>,
    /// The attached roster of the operation the edit dialog is open on.
    pub(super) hub: LocalResource<Roster>,
    /// The id of the operation the day panel has focus on.
    pub(super) selected_event: RwSignal<Option<String>>,
    /// Schedule form: the operation's name, blank for an unnamed one.
    pub(super) name: RwSignal<String>,
    /// Schedule form: the start time as `"HH:MM"`.
    pub(super) time: RwSignal<String>,
    /// Schedule form: whether registration opens immediately.
    pub(super) open_reg: RwSignal<bool>,
    /// Schedule form: the (mission id, title) pairs staged for attachment on publish.
    pub(super) staged: RwSignal<Vec<(String, String)>>,
    /// Whether the schedule form's attach picker is dropped down.
    pub(super) attach_open: RwSignal<bool>,
    /// Whether the schedule form is open.
    pub(super) form_open: RwSignal<bool>,
    /// Whether the delete confirmation is open.
    pub(super) confirm_open: RwSignal<bool>,
    /// Set while the publish request is in flight.
    pub(super) publish_busy: RwSignal<bool>,
    /// Set while the delete request is in flight.
    pub(super) delete_busy: RwSignal<bool>,
    /// Whether the edit form is open. Opening it is what starts the `hub` fetch.
    pub(super) edit_open: RwSignal<bool>,
    /// The row the edit form was seeded from; the save diffs against it.
    pub(super) edit_orig: RwSignal<Option<EventListItem>>,
    /// Edit form: the start date as `"YYYY-MM-DD"`.
    pub(super) edit_date: RwSignal<String>,
    /// Edit form: the start time as `"HH:MM"`.
    pub(super) edit_time: RwSignal<String>,
    /// Edit form: the operation's name.
    pub(super) edit_name: RwSignal<String>,
    /// Edit form: the briefing; blank clears it.
    pub(super) edit_briefing: RwSignal<String>,
    /// Edit form: the banner image address; blank clears it.
    pub(super) edit_banner: RwSignal<String>,
    /// Edit form: the slot ceiling, as typed.
    pub(super) edit_max_slots: RwSignal<String>,
    /// Edit form: whether registration is open.
    pub(super) edit_reg_open: RwSignal<bool>,
    /// Edit form: the lifecycle state picked.
    pub(super) edit_status: RwSignal<String>,
    /// Set while the edit save is in flight.
    pub(super) save_busy: RwSignal<bool>,
    /// The (attachment id, title) the detach confirmation is armed for.
    pub(super) detach_target: RwSignal<Option<(String, String)>>,
    /// Whether the detach confirmation is open.
    pub(super) detach_open: RwSignal<bool>,
    /// Set while the detach request is in flight.
    pub(super) detach_busy: RwSignal<bool>,
    /// Whether the edit form's attach picker is dropped down.
    pub(super) edit_attach_open: RwSignal<bool>,
    /// Set while an attach request from the edit form is in flight.
    pub(super) attach_busy: RwSignal<bool>,
}

impl Manager {
    /// Build the screen's state: today's position, empty forms, and the three fetches.
    ///
    /// Must be called from inside the route component, so the resources belong to that owner and
    /// are disposed with it.
    pub(super) fn new() -> Self {
        let store = expect_context::<AuthStore>();
        let today = js_sys::Date::new_0();
        let today_key = day_key(
            today.get_full_year() as i32,
            today.get_month() as i32,
            today.get_date(),
        );
        let today_key = StoredValue::new(today_key);
        let view = RwSignal::new((today.get_full_year() as i32, today.get_month() as i32));
        let selected = RwSignal::new((
            today.get_full_year() as i32,
            today.get_month() as i32,
            today.get_date(),
        ));

        let events = LocalResource::new(move || async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::v2::core::api::client::api_get::<Paginated<EventListItem>>(
                    store,
                    "/events?scope=all",
                )
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
                crate::v2::core::api::client::api_get::<Paginated<MissionCard>>(
                    store,
                    "/missions?scope=global",
                )
                .await
                .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = store;
                None::<Paginated<MissionCard>>
            }
        });

        let edit_open = RwSignal::new(false);
        let selected_event = RwSignal::new(None::<String>);

        // The attachment id the detach request keys on exists only on the hub payload — the list
        // row carries a count and nothing else. Keyed on the dialog rather than on selection so
        // clicking through a day's operations does not fire a request per click.
        let hub = LocalResource::new(move || {
            let id = edit_open.get().then(|| selected_event.get()).flatten();
            async move {
                #[cfg(target_arch = "wasm32")]
                {
                    match id {
                        Some(id) => {
                            match crate::v2::core::api::client::api_get::<EventHub>(
                                store,
                                &format!("/events/{id}"),
                            )
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

        Self {
            store,
            today_key,
            view,
            selected,
            events,
            missions,
            hub,
            selected_event,
            name: RwSignal::new(String::new()),
            time: RwSignal::new("19:00".to_string()),
            open_reg: RwSignal::new(true),
            staged: RwSignal::new(Vec::<(String, String)>::new()),
            attach_open: RwSignal::new(false),
            form_open: RwSignal::new(false),
            confirm_open: RwSignal::new(false),
            publish_busy: RwSignal::new(false),
            delete_busy: RwSignal::new(false),
            edit_open,
            edit_orig: RwSignal::new(None::<EventListItem>),
            edit_date: RwSignal::new(String::new()),
            edit_time: RwSignal::new(String::new()),
            edit_name: RwSignal::new(String::new()),
            edit_briefing: RwSignal::new(String::new()),
            edit_banner: RwSignal::new(String::new()),
            edit_max_slots: RwSignal::new(String::new()),
            edit_reg_open: RwSignal::new(true),
            edit_status: RwSignal::new(String::new()),
            save_busy: RwSignal::new(false),
            detach_target: RwSignal::new(None::<(String, String)>),
            detach_open: RwSignal::new(false),
            detach_busy: RwSignal::new(false),
            edit_attach_open: RwSignal::new(false),
            attach_busy: RwSignal::new(false),
        }
    }

    /// Move the grid `delta` months, carrying into the year.
    pub(super) fn shift_month(self, delta: i32) {
        self.view.update(|(y, m)| {
            let total = *y * 12 + *m + delta;
            *y = total.div_euclid(12);
            *m = total.rem_euclid(12);
        });
    }

    /// Every fetched operation, grouped by the local calendar day it starts on.
    ///
    /// Reads the resource, so it re-runs when the list is refetched.
    pub(super) fn events_by_day(self) -> std::collections::HashMap<String, Vec<EventListItem>> {
        let mut map = std::collections::HashMap::<String, Vec<EventListItem>>::new();
        if let Some(Some(page)) = self.events.get() {
            for e in page.data {
                map.entry(iso_day_key(&e.start_time)).or_default().push(e);
            }
        }
        map
    }

    /// The operations on the selected day, in the order the API returned them.
    pub(super) fn day_ops(self) -> Vec<EventListItem> {
        let (y, m, d) = self.selected.get();
        self.events_by_day()
            .remove(&day_key(y, m, d))
            .unwrap_or_default()
    }

    /// Select a day: move the day panel there, shut the schedule form's picker, and give focus to
    /// that day's first operation (or to none, when the day is empty).
    pub(super) fn select_day(self, y: i32, m: i32, d: u32) {
        self.selected.set((y, m, d));
        self.attach_open.set(false);
        let ops = self
            .events_by_day()
            .remove(&day_key(y, m, d))
            .unwrap_or_default();
        self.selected_event.set(ops.first().map(|o| o.id.clone()));
    }
}
