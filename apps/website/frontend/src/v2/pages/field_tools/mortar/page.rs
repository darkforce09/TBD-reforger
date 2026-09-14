//! The mortar calculator route: the inputs, the request, and the panels that show the answer.
//!
//! **Role:** owns every signal the page shares, the two fetches behind it (the operation list and
//! the fire missions saved against the selected operation), the reconciliation and hydration
//! effects, the request the Calculate button makes, and the arrangement of the panels.
//! **Position:** the `/tools/mortar` route, behind the authentication gate, inside the navigation
//! frame.
//! **Signals & state:** `fp_x`/`fp_y`/`tgt_x`/`tgt_y`, `weapon`, `event_id`, `solution` and `busy`
//! signals; two `LocalResource`s; a `StoredValue` set of the operations already hydrated; the
//! authentication store and the toast queue from context.
//! **Invariants:** the solver runs on the server, so nothing here computes ballistics. With an
//! operation selected, one request both computes and persists, so the operator is never shown
//! numbers that failed to save; with none, the page solves and says the result will not outlive
//! the tab. Every request path is `wasm32`-only; natively both resources resolve to `None` and the
//! page renders its empty states.
#![allow(dead_code)]

use super::firing_solution::firing_solution;
use super::map_picker::{coordinate_inputs, terrain_preview};
use super::saved_fires::{
    hydration_step, read_event_pref, restore, saved_list, write_event_pref, EventOption, SavedFire,
    SavedFor, Shown,
};
#[cfg(target_arch = "wasm32")]
use super::saved_fires::{save_body, SaveResponse};
use super::weapon_selector::{operation_select, weapon_select, WEAPONS};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::{DataEnvelope, FireSolution, Paginated};
use crate::v2::core::ui::{AuthGate, PageHeader};
use leptos::prelude::*;
use std::collections::HashSet;

/// The inputs card's class. The grid beats the base panel's `flex`, and the wide breakpoint is
/// three columns because the card holds six controls — two pickers and four coordinates — which
/// lay out as two rows of three rather than one row plus two orphans.
const CARD_INPUTS: &str = "relative flex-col overflow-hidden rounded-xl p-6 glass grid gap-4 sm:grid-cols-2 lg:grid-cols-3";

/// The mortar calculator, behind the authentication gate.
#[component]
pub fn MortarCalculatorPage() -> impl IntoView {
    view! {
        <AuthGate>
            <MortarInner />
        </AuthGate>
    }
}

/// The calculator itself: the signals, the two fetches, the effects that reconcile and hydrate
/// them, the solve request, and the panels.
#[component]
fn MortarInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let fp_x = RwSignal::new(1000.0);
    let fp_y = RwSignal::new(2000.0);
    let tgt_x = RwSignal::new(2200.0);
    let tgt_y = RwSignal::new(1800.0);
    let weapon = RwSignal::new(WEAPONS[0].to_string());
    // Seeded from localStorage so a reload lands on the operation the operator was working, not on
    // whichever one sorts first.
    let event_id = RwSignal::new(read_event_pref());
    let solution = RwSignal::new(None::<Shown>);
    let busy = RwSignal::new(false);

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
            let _ = store;
            None::<Vec<EventOption>>
        }
    });

    // The reload half of the round trip. Re-keys on the selected operation, so switching
    // operations swaps the history rather than merging two gun lines' work.
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
                let _ = (store, ev);
                None::<SavedFor>
            }
        }
    });

    // Reconcile the remembered operation against the live schedule.
    //
    // **`fire_missions.event_id` has no foreign key**, deliberately: there is no 23503 handler and
    // a 500 on an ingest path loses data. So a stale id out of `localStorage` is not rejected by
    // anything: the save route would happily write the row against an operation that no longer
    // exists, and the list route would happily return `{"data":[]}` for it. The fire mission would
    // be saved, reported saved, and unreachable. Nothing validates the id for us, so this does.
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

    // Hydrate the card from the newest saved fire mission, ONCE per operation.
    let hydrated_for = StoredValue::new(HashSet::<String>::new());
    Effect::new(move |_| {
        // Only act on a fetch that actually answered. Latching on a failed load would make the
        // page's one hydration attempt the one that read nothing.
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
            fp_x.set(r.fp.0);
            fp_y.set(r.fp.1);
            tgt_x.set(r.tgt.0);
            tgt_y.set(r.tgt.1);
            weapon.set(r.shown.weapon_system.clone());
            solution.set(Some(r.shown));
        }
    });

    // Load any saved fire mission back into the form — the newest one runs automatically on load,
    // the rest are one click away.
    let load_row = move |row: SavedFire| {
        if let Some(r) = restore(&row) {
            fp_x.set(r.fp.0);
            fp_y.set(r.fp.1);
            tgt_x.set(r.tgt.0);
            tgt_y.set(r.tgt.1);
            weapon.set(r.shown.weapon_system.clone());
            solution.set(Some(r.shown));
        }
    };

    let on_solve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let ev = event_id.get_untracked();
            let body = save_body(
                &weapon.get_untracked(),
                (fp_x.get_untracked(), fp_y.get_untracked()),
                (tgt_x.get_untracked(), tgt_y.get_untracked()),
                ev.as_deref(),
            );
            leptos::task::spawn_local(async move {
                match ev {
                    // With an operation: ONE call that computes and persists
                    // (`POST /fire-missions`). Solving first and saving second would leave a
                    // window where the operator is looking at numbers that failed to save.
                    Some(_) => {
                        match crate::v2::core::api::client::api_post::<SaveResponse>(
                            store,
                            "/fire-missions",
                            body,
                        )
                        .await
                        {
                            Ok(r) => {
                                let mut shown = Shown::from(&r.solution);
                                shown.saved_at = Some(r.fire_mission.created_at.clone());
                                solution.set(Some(shown));
                                saved.refetch();
                                toasts.success("Firing solution saved to the operation");
                            }
                            Err(e) => {
                                toasts.error(crate::v2::core::api::client::api_error_message(
                                    &e,
                                    "Could not compute firing solution",
                                ))
                            }
                        }
                    }
                    // No operation: solve only, and say so. There is no endpoint that persists a
                    // fire mission the operator can find again without one.
                    None => {
                        match crate::v2::core::api::client::api_post::<FireSolution>(
                            store,
                            "/fire-missions/solve",
                            body,
                        )
                        .await
                        {
                            Ok(s) => {
                                solution.set(Some(Shown::from(&s)));
                                toasts
                                    .message("Not saved — pick an operation to keep this solution");
                            }
                            Err(e) => {
                                toasts.error(crate::v2::core::api::client::api_error_message(
                                    &e,
                                    "Could not compute firing solution",
                                ))
                            }
                        }
                    }
                }
                busy.set(false);
            });
        }
    };
    view! {
        <div class="relative flex h-full w-full flex-col overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full flex-col gap-4 bg-surface-glass p-6 backdrop-blur-xl md:p-8">
                <PageHeader
                    title="Mortar Calculator"
                    subtitle="Enter grid coordinates, pick a tube, and save the solution to an operation."
                />
                <div class=CARD_INPUTS>
                    {weapon_select(weapon)} {operation_select(events, event_id)}
                    {coordinate_inputs(fp_x, fp_y, tgt_x, tgt_y)}
                </div>
                <button
                    type="button"
                    on:click=on_solve
                    prop:disabled=move || busy.get()
                    class="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                    {move || {
                        if busy.get() {
                            "Computing…".to_string()
                        } else if event_id.get().is_some() {
                            "Calculate & Save".to_string()
                        } else {
                            "Calculate Solution".to_string()
                        }
                    }}
                </button>
                <div class="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-border-subtle bg-surface-container-lowest">
                    {terrain_preview(fp_x, fp_y, tgt_x, tgt_y)}
                    {saved_list(saved, event_id, load_row)}
                    {firing_solution(solution, weapon)}
                </div>
            </div>
        </div>
    }
}
