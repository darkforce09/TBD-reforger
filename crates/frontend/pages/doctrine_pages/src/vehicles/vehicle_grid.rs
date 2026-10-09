//! The master half of the vehicle index: the search box, the administrator's add action and the
//! faction-grouped list.
//!
//! **Role:** renders one labelled group per faction, each holding the vehicles that survived the
//! search box, as rows that select the vehicle shown in the dossier pane; above them, the header
//! with the "Add vehicle" action for an administrator.
//! **Position:** the master pane of the vehicle index's split view, with its own header above it.
//! **Signals & state:** reads and writes the page's `search` signal through `SidebarSearch`,
//! writes the selected vehicle id back to the page's `selected_id` signal, and reads the page's
//! `is_admin` memo.
//! **Invariants:** groups appear in the order each faction was first seen, which is the name
//! order the API returns; a group whose vehicles all filtered out is not rendered. The add action
//! renders only while `is_admin` holds, on the section label's line, and both keep their text on
//! one line.

#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::vehicles::Vehicle;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use frontend_ui::split_pane::{ListDetailItem, SidebarSearch, search_matches};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The index header's section label: one line, cut with an ellipsis only when the column cannot
/// hold it beside the add action.
#[cfg(target_arch = "wasm32")]
const HEADER_TITLE_CLASS: &str = "min-w-0 truncate font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase";
/// The compact "Add vehicle" action: icon and label on one line, never shrunk, so it fits beside
/// the section label in the 18rem master column.
#[cfg(target_arch = "wasm32")]
const ADD_BUTTON_CLASS: &str = "flex shrink-0 items-center gap-1 whitespace-nowrap rounded-md border border-primary/30 bg-primary/10 px-2 py-1 text-xs font-medium text-primary transition-colors hover:bg-primary/20";

/// The distinct factions of `vehicles`, in the order each was first seen.
///
/// The API orders the list by name, so this is that order collapsed to one entry per faction.
/// Rows with no faction are skipped.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) fn faction_order(vehicles: &[Vehicle]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in vehicles {
        if !v.faction.is_empty() && !out.iter().any(|x| x == &v.faction) {
            out.push(v.faction.clone());
        }
    }
    out
}

/// The index header: the section label, the "Add vehicle" action while `is_admin` holds, and the
/// search box bound to `search`. `on_add` opens the vehicle form on a new vehicle.
#[cfg(target_arch = "wasm32")]
pub(super) fn master_header(
    search: RwSignal<String>,
    is_admin: Memo<bool>,
    on_add: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <div class="flex items-center justify-between gap-2">
                <p class=HEADER_TITLE_CLASS>"Vehicle Database"</p>
                {move || {
                    is_admin
                        .get()
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| on_add.run(())
                                    class=ADD_BUTTON_CLASS
                                >
                                    <MaterialIcon name="add" class="text-sm" />
                                    "Add vehicle"
                                </button>
                            }
                        })
                }}
            </div>
            <SidebarSearch placeholder="Search assets..." bind=search />
        </div>
    }
}

/// The grouped list of vehicles.
///
/// `selected_id` is the row the dossier is showing, `query` the live search text, and `vehicles`
/// the full list. A row matches when the query matches its name, its armour class or its
/// faction; clicking one writes its id to `selected_id`.
#[cfg(target_arch = "wasm32")]
pub(super) fn vehicle_list(
    selected_id: RwSignal<String>,
    query: &str,
    vehicles: &[Vehicle],
) -> impl IntoView + use<> {
    let query = query.to_string();
    faction_order(vehicles)
        .into_iter()
        .filter_map(move |faction| {
            let rows: Vec<Vehicle> = vehicles
                .iter()
                .filter(|v| v.faction == faction)
                .filter(|v| {
                    search_matches(
                        &query,
                        &format!("{} {} {}", v.name, v.armor_type, v.faction),
                    )
                })
                .cloned()
                .collect();
            if rows.is_empty() {
                return None;
            }
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {faction}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|v| {
                                let Vehicle { id, name, armor_type, .. } = v;
                                let active = id == selected_id.get().as_str();
                                view! {
                                    <ListDetailItem
                                        active=active
                                        title={view! { {name} }.into_any()}
                                        preview={view! {
                                            <span class="font-mono uppercase text-outline">
                                                {armor_type}
                                            </span>
                                        }
                                            .into_any()}
                                        on_click={Callback::new(move |()| selected_id.set(id.to_string()))}
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}
