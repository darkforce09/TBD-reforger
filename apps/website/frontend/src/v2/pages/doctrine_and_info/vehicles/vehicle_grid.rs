//! The master half of the vehicle index: the search box and the faction-grouped list.
//!
//! **Role:** renders one labelled group per faction, each holding the vehicles that survived the
//! search box, as rows that select the vehicle shown in the dossier pane.
//! **Position:** the master pane of the vehicle index's split view, with its own header above it.
//! **Signals & state:** reads and writes the page's `search` signal through `SidebarSearch`, and
//! writes the selected vehicle id back to the page's `selected_id` signal.
//! **Invariants:** groups appear in the order each faction was first seen, which is the name
//! order the API returns; a group whose vehicles all filtered out is not rendered.

use super::helpers::vstr;
use crate::v2::core::ui::split_pane::{ListDetailItem, SidebarSearch};
use leptos::prelude::*;
use serde_json::Value;

#[cfg(test)]
#[path = "tests/vehicles.rs"]
mod tests;

/// The distinct factions of `vehicles`, in the order each was first seen.
///
/// The API orders the list by name, so this is that order collapsed to one entry per faction.
/// Rows with no faction are skipped.
pub(super) fn faction_order(vehicles: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for v in vehicles {
        let f = vstr(v, "faction");
        if !f.is_empty() && !out.iter().any(|x| x == &f) {
            out.push(f);
        }
    }
    out
}

/// The index header: the section label and the search box bound to `search`.
pub(super) fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "Vehicle Database"
            </p>
            <SidebarSearch placeholder="Search assets..." bind=search />
        </div>
    }
}

/// The grouped list of vehicles.
///
/// `selected_id` is the row the dossier is showing, `query` the live search text, and `vehicles`
/// the full list. A row matches when the query matches its name, its armour class or its
/// faction; clicking one writes its id to `selected_id`.
pub(super) fn vehicle_list(
    selected_id: RwSignal<String>,
    query: &str,
    vehicles: &[Value],
) -> impl IntoView {
    let query = query.to_string();
    faction_order(vehicles)
        .into_iter()
        .filter_map(move |faction| {
            let rows: Vec<Value> = vehicles
                .iter()
                .filter(|v| vstr(v, "faction") == faction)
                .filter(|v| {
                    crate::v2::core::ui::split_pane::search_matches(
                        &query,
                        &format!(
                            "{} {} {}",
                            vstr(v, "name"),
                            vstr(v, "armor_type"),
                            vstr(v, "faction")
                        ),
                    )
                })
                .cloned()
                .collect();
            if rows.is_empty() {
                return None;
            }
            let faction_label = faction.clone();
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {faction_label}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|v| {
                                let id = vstr(&v, "id");
                                let name = vstr(&v, "name");
                                let class = vstr(&v, "armor_type");
                                let id_click = id.clone();
                                view! {
                                    <ListDetailItem
                                        active=id == selected_id.get()
                                        title=view! { {name} }.into_any()
                                        preview=view! {
                                            <span class="font-mono uppercase text-outline">
                                                {class}
                                            </span>
                                        }
                                            .into_any()
                                        on_click=Callback::new(move |()| {
                                            selected_id.set(id_click.clone())
                                        })
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
