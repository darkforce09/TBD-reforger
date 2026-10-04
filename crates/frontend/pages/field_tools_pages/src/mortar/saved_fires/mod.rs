//! Saving a fire mission against an event, and reading the saved ones back.
//!
//! **Role:** the event picker and its remembered choice, the fetched batch tagged with the event
//! it answers for, and the hydration rule; `restore` unpacks a stored row into drafts, `list`
//! renders the saved fire missions, `save_request` builds and posts the save, and `save_area`
//! composes them for a signed-in viewer.
//! **Position:** the signed-in half of `/tools/mortar`, rendered inside the page's `AuthGate`.
//! **Signals & state:** the save area owns the event choice, the event list and saved-row
//! resources, and the set of events already hydrated; `read_event_pref` and
//! `write_event_pref` read and write `localStorage` under `EVENT_PREF_KEY` in the browser
//! build only.
//! **Invariants:** a fetched batch is tagged with the event it was fetched for, and nothing acts on
//! a batch whose tag is not the selected event; hydration happens at most once per event; a row
//! with no coordinates restores as nothing rather than as the origin.

pub mod connection_gate;
pub(crate) mod list;
pub(crate) mod restore;
pub mod save_area;
pub(crate) mod save_request;

#[cfg(target_arch = "wasm32")]
use crate::mortar::inputs::INPUT_CLASS;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::SavedFire;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(any(target_arch = "wasm32", test))]
use restore::{Restored, restore};
#[cfg(any(target_arch = "wasm32", test))]
use serde::Deserialize;
#[cfg(any(target_arch = "wasm32", test))]
use std::collections::HashSet;

/// The `localStorage` key holding the event the operator last saved to, so a reload reopens that
/// event's fire missions rather than the first event's.
#[cfg(target_arch = "wasm32")]
pub(crate) const EVENT_PREF_KEY: &str = "tbd-mortar-event";

/// A fetched batch of fire missions, tagged with the event it was fetched for.
///
/// `LocalResource` keeps serving its previous value while the next key is in flight, so the moment
/// the operator picks an event the hydration effect sees the new event with the previous event's
/// rows. The tag lets [`hydration_step`] refuse that batch instead of latching the new event over
/// rows that are not its own.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SavedFor {
    /// The event the batch answers for; `None` for "no event".
    pub(crate) event_id: Option<String>,
    /// The event's saved fire missions, oldest first.
    pub(crate) rows: Vec<SavedFire>,
}

/// One scheduled event, narrowed to what the picker needs, so a schedule field this picker does
/// not use can never fail its decode.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct EventOption {
    /// The event id.
    pub(crate) id: String,
    /// The event's display name; nullable.
    #[serde(default)]
    pub(crate) name_override: Option<String>,
    /// Start time, RFC 3339.
    pub(crate) start_time: String,
}

#[cfg(any(target_arch = "wasm32", test))]
impl EventOption {
    /// The display name, or "Untitled Operation" for a missing or blank one. The date is composed
    /// by the view, because the date formatter needs the browser.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn name(&self) -> &str {
        match self.name_override.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => n,
            _ => "Untitled Operation",
        }
    }
}

/// What the load-time hydration does with `batch` for the selected event `want`.
///
/// `None`: nothing — the batch answers for another event, or `want` is already in
/// `already_hydrated` (a refetch after a save, or a return to an event whose drafts may have been
/// edited since). `Some(restored)`: latch `want` and apply `restored`, itself `None` when the event
/// has nothing saved or its newest row has no coordinates. The latch is a set, so returning to an
/// event never re-hydrates it.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn hydration_step(
    batch: &SavedFor,
    want: &str,
    already_hydrated: &HashSet<String>,
) -> Option<Option<Restored>> {
    if batch.event_id.as_deref() != Some(want) {
        return None;
    }
    if already_hydrated.contains(want) {
        return None;
    }
    Some(batch.rows.last().and_then(restore))
}

/// The event the operator last saved to, in the browser build.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_event_pref() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(EVENT_PREF_KEY)
        .ok()?
        .filter(|s| !s.trim().is_empty())
}

/// Remembers the event being saved to, or forgets it for "none"; a blank id clears the key.
#[cfg(target_arch = "wasm32")]
pub(crate) fn write_event_pref(id: Option<&str>) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        match id {
            Some(id) if !id.trim().is_empty() => {
                let _ = storage.set_item(EVENT_PREF_KEY, id);
            }
            _ => {
                let _ = storage.remove_item(EVENT_PREF_KEY);
            }
        }
    }
}

/// The event picker: the schedule, plus "none", which means a solution is computed but not saved.
/// The option set is rebuilt when the event list lands, so a remembered event matches an option
/// that exists by the time the value is applied.
#[cfg(target_arch = "wasm32")]
pub(crate) fn event_select(
    events: LocalResource<Option<Vec<EventOption>>>,
    event_id: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
        let rows = events.get().flatten().unwrap_or_default();
        view! {
            <label class="text-sm">
                "Event"
                <select
                    prop:value=move || event_id.get().unwrap_or_default()
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        let v = (!v.is_empty()).then_some(v);
                        write_event_pref(v.as_deref());
                        event_id.set(v);
                    }
                    class=INPUT_CLASS
                    data-mortar-input="event"
                >
                    <option value="">"— none (not saved) —"</option>
                    {rows
                        .into_iter()
                        .map(|e| {
                            let label = format!(
                                "{} — {}",
                                e.name(),
                                frontend_ui::datefmt::format_short_date(&e.start_time),
                            );
                            view! { <option value=e.id.clone()>{label}</option> }
                        })
                        .collect_view()}
                </select>
            </label>
        }
    }
}

#[cfg(test)]
#[path = "../tests/saved_fires.rs"]
mod tests;
