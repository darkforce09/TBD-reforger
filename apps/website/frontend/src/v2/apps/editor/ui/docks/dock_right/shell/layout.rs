//! Right dock shell behavior.

use super::*;

/// Render the right dock palettes and editor controls. The Factions and Vehicles
/// trees share registry data; the collapse latch controls the corner stub.
#[component]
pub fn DockRight(
    catalog: RwSignal<CatalogState>,
    /// the `kind == "vehicle"` half of the same registry fetch.
    vehicle_catalog: RwSignal<CatalogState>,
    /// the raw registry rows, for the placed-vehicle cargo picker's labels and options.
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    /// terminal `/registry` failure (distinct from `registry_items == None` = still loading).
    registry_failed: RwSignal<bool>,
    /// bump to re-kick the cold `/registry` fetch (Favourites Retry).
    registry_fetch_gen: RwSignal<u64>,
    /// the doc-change tick the placed-vehicle list re-reads on.
    doc_tick: RwSignal<u64>,
    fm_open: RwSignal<bool>,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
    /// collapse latch (owned by `mission_editor`; `R`/chevron toggle it, the accessor + reflow
    /// observe it).
    collapsed: RwSignal<bool>,
) -> impl IntoView {
    let palette_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    Effect::new(move |_| {
        let _ = active_side.get(); // Re-seed when chips flip the filtered tree.
        if let CatalogState::Ready(nodes) = catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            palette_collapsed.set(set);
        }
    });
    let tab = RwSignal::new(0usize);
    let search = RwSignal::new(String::new());
    let no_collapse = RwSignal::new(std::collections::HashSet::<String>::new());
    let vehicle_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let vehicle_seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if vehicle_seeded.get_value() {
            return;
        }
        if let CatalogState::Ready(nodes) = vehicle_catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            vehicle_collapsed.set(set);
            vehicle_seeded.set_value(true);
        }
    });
    let vehicle_search = RwSignal::new(String::new());
    #[cfg(target_arch = "wasm32")]
    let place_with_crew = RwSignal::new(editor_context::place_with_crew());
    let object_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let object_search = RwSignal::new(String::new());
    let zone_selected = RwSignal::new(None::<String>);
    install_select_zone(std::rc::Rc::new(move |id: &str| {
        zone_selected.set(Some(id.to_string()));
        tab.set(ZONES_TAB);
        collapsed.set(false);
    }));
    let comp_editing = RwSignal::new(None::<String>);
    let trigger_selected = RwSignal::new(None::<String>);
    let marker_selected = RwSignal::new(None::<(String, String)>);
    let favourites = RwSignal::new(load_favourites());
    let recent_placed = RwSignal::new(Vec::<RecentPlaced>::new());
    install_recent_recorder(std::rc::Rc::new(move |asset_id: String, label: String| {
        record_recent(recent_placed, asset_id, label);
    }));
    let history_open = RwSignal::new(false);
    let faction_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    Effect::new(move |_| {
        let side = active_side.get();
        if let Some(items) = registry_items.get() {
            let nodes = crate::v2::apps::editor::arsenal::asset_catalog::build_faction_catalog_tree(
                &items, &side,
            );
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            faction_collapsed.set(set);
        }
    });
    let registry_no_modpack = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    {
        let auth = use_context::<crate::v2::core::auth::AuthStore>();
        Effect::new(move |_| {
            if !registry_failed.get() {
                registry_no_modpack.set(false);
                return;
            }
            let Some(auth) = auth else {
                return;
            };
            leptos::task::spawn_local(async move {
                let got: Result<
                    crate::v2::core::api::dto::RegistryResponse,
                    crate::v2::core::api::client::ApiErr,
                > = crate::v2::core::api::client::api_get(auth, "/registry?limit=1&offset=0").await;
                registry_no_modpack.set(matches!(got, Err((404, _))));
            });
        });
    }
    let tab_btn = move |i: usize, label: &'static str| {
        let icon = tab_icon(i);
        view! {
            <button
                type="button"
                role="tab"
                title=label
                aria-label=label
                aria-selected=move || (tab.get() == i).to_string()
                class=move || if tab.get() == i { TAB_CELL_ON } else { TAB_CELL_OFF }
                on:click=move |_| tab.set(i)
            >
                <MaterialIcon name=icon class="block text-sm leading-none" />
            </button>
        }
    };
    let full = move || {
        view! {
            <aside class=DOCK_R>
                <div class=TAB_STRIP>
                    <div class=TAB_GROUP role="tablist">
                        {tab_btn(0, "Factions")}
                        {tab_btn(1, "Vehicles")}
                        {tab_btn(ZONES_TAB, "Zones")}
                        {tab_btn(4, "Compositions")}
                        {tab_btn(5, "Triggers")}
                        {tab_btn(6, "Favourites")}
                        {tab_btn(2, "Markers")}
                    </div>
                    <div class=TAB_GROUP>
                        <button
                            type="button"
                            title="Manage factions"
                            aria-label="Manage factions"
                            on:click=move |_| fm_open.set(true)
                            class=TAB_CELL_VERB
                        >
                            <MaterialIcon name="tune" class="block text-sm leading-none" />
                        </button>
                        {collapse_chevron(collapsed, false)}
                    </div>
                </div>
                {move || match tab.get() {
                    0 => factions_panel(FactionsPanelSignals {
                        active_side,
                        objects_mode,
                        object_search,
                        search,
                        catalog,
                        registry_items,
                        object_collapsed,
                        no_collapse,
                        favourites,
                        registry_no_modpack,
                        registry_fetch_gen,
                        faction_collapsed,
                        recent_placed,
                    }),
                    1 => view! {
                        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Vehicles"</h3>
                        <p class="mt-0.5 text-label-sm normal-case text-outline">
                            "Every placeable vehicle, across factions — a filtered view of the catalog. Drag one onto the map to place it."
                        </p>
                        <input
                            type="search"
                            aria-label="Search vehicles"
                            placeholder=format!("Search vehicles{SEARCH_PLACEHOLDER_GRAMMAR}")
                            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                            on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                        />
                        {move || {
                            (!matches!(vehicle_catalog.get(), CatalogState::Failed))
                                .then(search_grammar_hint)
                        }}
                        {crew_place_toggle(
                            #[cfg(target_arch = "wasm32")]
                            place_with_crew,
                        )}
                        <div class="mt-2">
                            {move || {
                                if objects_mode.get() {
                                    return view! {
                                        <p class="text-label-sm text-outline">
                                            "Objects place from the Factions tab while the Objects chip is selected."
                                        </p>
                                    }
                                        .into_any();
                                }
                                match vehicle_catalog.get() {
                                    CatalogState::Loading => {
                                        view! {
                                            <p class="text-label-sm text-outline">"Loading vehicles…"</p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Failed => catalog_failure_view(
                                        "vehicle catalog",
                                        registry_no_modpack,
                                        registry_fetch_gen,
                                    ),
                                    CatalogState::Ready(nodes) if nodes.is_empty() => {
                                        view! {
                                            <p class="text-label-sm text-outline">
                                                "No placeable vehicles."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Ready(nodes) => {
                                        let q = vehicle_search.get();
                                        if q.trim().is_empty() {
                                            vehicle_collapsed.track();
                                            faction_palette_rows(
                                                &nodes,
                                                0,
                                                &[],
                                                &[],
                                                vehicle_collapsed,
                                                favourites,
                                                registry_items,
                                                recent_placed,
                                            )
                                        } else {
                                            let filtered =
                                                crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                                            if filtered.is_empty() {
                                                let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
                                                    &q, "vehicles",
                                                );
                                                view! {
                                                    <p class="text-label-sm text-outline">{msg}</p>
                                                }
                                                    .into_any()
                                            } else {
                                                faction_palette_rows(
                                                    &filtered,
                                                    0,
                                                    &[],
                                                    &[],
                                                    no_collapse,
                                                    favourites,
                                                    registry_items,
                                                    recent_placed,
                                                )
                                            }
                                        }
                                    }
                                }
                            }}
                        </div>
                    }
                        .into_any(),
                    ZONES_TAB => zones_panel(doc_tick, zone_selected),
                    4 => compositions_panel(doc_tick, comp_editing),
                    5 => triggers_panel(doc_tick, trigger_selected),
                    6 => view! {
                        <div class="mt-2 flex items-center gap-0.5" role="tablist" aria-label="Assets and history">
                            <button
                                type="button"
                                role="tab"
                                aria-selected=move || (!history_open.get()).to_string()
                                class=move || if history_open.get() { SUBTAB_OFF } else { SUBTAB_ON }
                                on:click=move |_| history_open.set(false)
                            >
                                "Favourites"
                            </button>
                            <button
                                type="button"
                                role="tab"
                                aria-selected=move || history_open.get().to_string()
                                class=move || if history_open.get() { SUBTAB_ON } else { SUBTAB_OFF }
                                on:click=move |_| history_open.set(true)
                            >
                                "Recently placed"
                            </button>
                        </div>
                        {move || {
                            if history_open.get() {
                                recently_placed_panel(recent_placed, registry_items)
                            } else {
                                favourites_panel(
                                    favourites,
                                    registry_items,
                                    registry_failed,
                                    registry_fetch_gen,
                                )
                            }
                        }}
                    }
                        .into_any(),
                    2 => markers_panel(doc_tick, marker_selected),
                    _ => ().into_any(),
                }}
            </aside>
        }
    };
    let stub = move || {
        view! {
            <div
                class="pointer-events-auto flex items-start justify-end bg-surface-container-lowest/55 backdrop-blur-xl"
                style=format!("width:{STUB_PX}px;height:{STUB_PX}px")
            >
                {collapse_chevron(collapsed, false)}
            </div>
        }
    };
    move || {
        if collapsed.get() {
            stub().into_any()
        } else {
            full().into_any()
        }
    }
}
