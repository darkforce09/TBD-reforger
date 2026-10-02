//! The signed-in half of the mortar page: the event, the save button and the saved list.
//!
//! **Role:** fetches the schedule and the selected event's saved fire missions, saves the last
//! solve against the selected event, and puts a saved row back into the inputs — the newest row
//! once per event, any row on a click.
//! **Position:** rendered by the page inside `AuthGate`; composes [`super::event_select`],
//! [`super::save_request`] and [`super::list::saved_list`].
//! **Signals & state:** the event choice, the save status, the hydrated-event set, and the event
//! list and saved-rows resources; writes the page's draft signals when a row loads.
//! **Invariants:** no account-scoped request leaves the page before the session admits this area;
//! a remembered event the schedule no longer lists is forgotten; a save refetches the list, so the
//! stored row appears without a reload; a new solve clears the previous save status.

use super::list::saved_list;
use super::restore::{gun_drafts, position_draft, restore, Restored};
use super::save_request::{save_status_text, SaveStatus};
use super::{
    event_select, hydration_step, read_event_pref, write_event_pref, EventOption, SavedFor,
};
use crate::v2::core::api::dto::SavedFire;
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::{DataEnvelope, Paginated};
use crate::v2::pages::field_tools::mortar::inputs::battery::GunDraft;
use crate::v2::pages::field_tools::mortar::inputs::positions::PositionDraft;
use crate::v2::pages::field_tools::mortar::inputs::weapon_and_shell::ArmamentSelection;
use crate::v2::pages::field_tools::mortar::inputs::wind::WindDraft;
use crate::v2::pages::field_tools::mortar::solution::SolveOutcome;
use leptos::prelude::*;
use std::collections::HashSet;

/// The page's draft signals a restored row writes.
#[derive(Clone, Copy)]
pub(crate) struct RestoreTargets {
    /// The target draft.
    pub(crate) target: RwSignal<PositionDraft>,
    /// The battery drafts.
    pub(crate) guns: RwSignal<Vec<GunDraft>>,
    /// Weapon, shell and charge.
    pub(crate) selection: RwSignal<ArmamentSelection>,
    /// The wind draft.
    pub(crate) wind: RwSignal<WindDraft>,
    /// The burst height text.
    pub(crate) burst_height: RwSignal<String>,
}

impl RestoreTargets {
    /// Writes `restored` into the drafts.
    fn apply(self, restored: &Restored) {
        self.target
            .update(|t| *t = position_draft(&restored.target, t));
        self.guns.update(|g| *g = gun_drafts(restored, g));
        if let Some(selection) = &restored.selection {
            self.selection.set(selection.clone());
        }
        if let Some(wind) = &restored.wind {
            self.wind.set(wind.clone());
        }
        if let Some(burst) = &restored.burst_height {
            self.burst_height.set(burst.clone());
        }
    }
}

/// The save area: event picker, save button and saved list.
#[component]
pub(crate) fn MortarSaveArea(
    restore_into: RestoreTargets,
    outcome: RwSignal<Option<SolveOutcome>>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let event_id = RwSignal::new(read_event_pref());
    let status = RwSignal::new(SaveStatus::Idle);
    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<EventOption>>(store, "/events")
                .await
                .ok()
                .map(|p| p.data)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None::<Vec<EventOption>>
        }
    });
    let saved = LocalResource::new(move || {
        let ev = event_id.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match ev {
                    Some(id) => crate::v2::core::api::client::api_get::<DataEnvelope<SavedFire>>(
                        store,
                        &format!("/events/{id}/fire-missions"),
                    )
                    .await
                    .ok()
                    .map(|d| SavedFor {
                        event_id: Some(id),
                        rows: d.data,
                    }),
                    None => Some(SavedFor {
                        event_id: None,
                        rows: Vec::new(),
                    }),
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = ev;
                None::<SavedFor>
            }
        }
    });
    Effect::new(move |_| {
        let Some(Some(rows)) = events.get() else {
            return;
        };
        let Some(want) = event_id.get() else { return };
        if !rows.iter().any(|e| e.id == want) {
            write_event_pref(None);
            event_id.set(None);
        }
    });
    let hydrated_for = StoredValue::new(HashSet::<String>::new());
    Effect::new(move |_| {
        let Some(Some(batch)) = saved.get() else {
            return;
        };
        let Some(ev) = event_id.get() else { return };
        let Some(restored) = hydration_step(&batch, &ev, &hydrated_for.get_value()) else {
            return;
        };
        hydrated_for.update_value(|seen| {
            seen.insert(ev);
        });
        if let Some(r) = restored {
            restore_into.apply(&r);
        }
    });
    Effect::new(move |_| {
        outcome.track();
        status.set(SaveStatus::Idle);
    });
    let load_row = move |row: SavedFire| {
        if let Some(r) = restore(&row) {
            restore_into.apply(&r);
        }
    };
    let on_save = move |_| {
        let Some(Ok(solved)) = outcome.get_untracked() else {
            return;
        };
        let Some(event) = event_id.get_untracked() else {
            return;
        };
        let body = super::save_request::save_body(&solved, Some(&event));
        status.set(SaveStatus::Saving);
        #[cfg(target_arch = "wasm32")]
        leptos::task::spawn_local(async move {
            match super::save_request::post_save(store, &body).await {
                Ok(created_at) => {
                    status.set(SaveStatus::Saved(created_at));
                    saved.refetch();
                }
                Err(text) => status.set(SaveStatus::Failed(text)),
            }
        });
        #[cfg(not(target_arch = "wasm32"))]
        let _ = body;
    };
    let can_save = move || {
        event_id.with(Option::is_some)
            && outcome.with(|o| matches!(o, Some(Ok(_))))
            && status.with(|s| *s != SaveStatus::Saving)
    };
    view! {
        <div class="flex flex-col gap-4 rounded-xl p-6 glass" data-mortar-save-area="">
            {event_select(events, event_id)}
            <div class="flex flex-wrap items-center gap-3">
                <button
                    type="button"
                    on:click=on_save
                    prop:disabled=move || !can_save()
                    class="rounded-lg border border-primary px-4 py-2 text-sm font-medium text-primary disabled:opacity-50"
                    data-mortar-save=""
                >
                    "Save Fire Mission"
                </button>
                <p class="text-xs text-on-surface-variant" data-mortar-save-status="">
                    {move || status.with(save_status_text)}
                </p>
            </div>
            {saved_list(saved, event_id, load_row)}
        </div>
    }
}
