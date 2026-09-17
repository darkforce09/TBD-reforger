//! Factions and Objects tab rendering.

use super::*;

/// Signals shared by the Factions and Objects tab view.
#[derive(Clone, Copy)]
pub(super) struct FactionsPanelSignals {
    pub(super) active_side: RwSignal<String>,
    pub(super) objects_mode: RwSignal<bool>,
    pub(super) object_search: RwSignal<String>,
    pub(super) search: RwSignal<String>,
    pub(super) catalog: RwSignal<CatalogState>,
    pub(super) registry_items: RwSignal<Option<Vec<RegistryItem>>>,
    pub(super) object_collapsed: RwSignal<std::collections::HashSet<String>>,
    pub(super) no_collapse: RwSignal<std::collections::HashSet<String>>,
    pub(super) favourites: RwSignal<Favourites>,
    pub(super) registry_no_modpack: RwSignal<bool>,
    pub(super) registry_fetch_gen: RwSignal<u64>,
    pub(super) faction_collapsed: RwSignal<std::collections::HashSet<String>>,
    pub(super) recent_placed: RwSignal<Vec<RecentPlaced>>,
}

/// Render side chips, search, and the merged faction asset tree.
pub(super) fn factions_panel(signals: FactionsPanelSignals) -> AnyView {
    let FactionsPanelSignals {
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
    } = signals;
    view! {
        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Asset Browser"</h3>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Drag a role onto the map to place its slot."
        </p>
        <div
            class="mt-2 flex items-center gap-1.5"
            role="group"
            aria-label="Eden side"
        >
            {EDEN_SIDE_CHIPS
                .iter()
                .filter_map(|label| EdenChip::from_label(label))
                .map(|chip| {
                    let fill = chip.fill_class();
                    view! {
                        <button
                            type="button"
                            aria-label=chip.label()
                            aria-pressed=move || {
                                eden_chip_selected(
                                    chip,
                                    &active_side.get(),
                                    objects_mode.get(),
                                )
                            }
                            class=move || {
                                let selected = eden_chip_selected(
                                    chip,
                                    &active_side.get(),
                                    objects_mode.get(),
                                );
                                if selected {
                                    format!(
                                        "{fill} h-5 w-8 shrink-0 rounded-sm ring-2 ring-offset-1 ring-offset-surface-container-lowest ring-white/90 opacity-100"
                                    )
                                } else {
                                    format!(
                                        "{fill} h-5 w-8 shrink-0 rounded-sm opacity-45 transition-opacity hover:opacity-75"
                                    )
                                }
                            }
                            on:click=move |_| {
                                apply_eden_chip(chip, active_side, objects_mode)
                            }
                        />
                    }
                })
                .collect_view()}
            <Show when=move || custom_chip_visible(
                EdenSubmode::from_tab(0, objects_mode.get()),
            )>
                <button
                    type="button"
                    disabled=true
                    aria-label=EDEN_CUSTOM_CHIP
                    title="Custom groups arrive in T-078"
                    class="flex h-5 shrink-0 items-center rounded-sm border border-outline-variant/60 px-1.5 text-[10px] font-semibold uppercase tracking-wide text-outline opacity-45"
                >
                    {EDEN_CUSTOM_CHIP}
                </button>
            </Show>
        </div>
        <input
            type="search"
            aria-label=move || {
                if objects_mode.get() {
                    "Search objects"
                } else {
                    "Search assets"
                }
            }
            placeholder=move || {
                if objects_mode.get() {
                    format!("Search objects{SEARCH_PLACEHOLDER_GRAMMAR}")
                } else {
                    format!("Search assets{SEARCH_PLACEHOLDER_GRAMMAR}")
                }
            }
            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
            on:input=move |ev| {
                let v = event_target_value(&ev);
                if objects_mode.get_untracked() {
                    object_search.set(v);
                } else {
                    search.set(v);
                }
            }
        />
        {move || {
            (objects_mode.get()
                || !matches!(catalog.get(), CatalogState::Failed))
                .then(search_grammar_hint)
        }}
        <div class="mt-2">
            {move || {
                if objects_mode.get() {
                    let items = registry_items.get().unwrap_or_default();
                    let nodes =
                        crate::v2::apps::editor::arsenal::asset_catalog::build_object_catalog_tree(&items);
                    if nodes.is_empty() {
                        return view! {
                            <p class="text-label-sm text-outline">
                                "No placeable objects in the registry."
                            </p>
                        }
                        .into_any();
                    }
                    let q = object_search.get();
                    if q.trim().is_empty() {
                        object_collapsed.track();
                        return palette_rows(
                            &nodes,
                            0,
                            &[],
                            &[],
                            object_collapsed,
                            PaletteKind::Object,
                            favourites,
                        );
                    }
                    let filtered = crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                    if filtered.is_empty() {
                        let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(&q, "objects");
                        return view! {
                            <p class="text-label-sm text-outline">{msg}</p>
                        }
                        .into_any();
                    }
                    return palette_rows(
                        &filtered,
                        0,
                        &[],
                        &[],
                        no_collapse,
                        PaletteKind::Object,
                        favourites,
                    );
                }
                match catalog.get() {
                    CatalogState::Loading => {
                        view! {
                            <p class="text-label-sm text-outline">"Loading assets…"</p>
                        }
                            .into_any()
                    }
                    CatalogState::Failed => catalog_failure_view(
                        "asset catalog",
                        registry_no_modpack,
                        registry_fetch_gen,
                    ),
                    CatalogState::Ready(_) => {
                        let items = registry_items.get().unwrap_or_default();
                        let side = active_side.get();
                        let nodes = crate::v2::apps::editor::arsenal::asset_catalog::build_faction_catalog_tree(
                            &items, &side,
                        );
                        if nodes.is_empty() {
                            return view! {
                                <p class="text-label-sm text-outline">"No placeable assets."</p>
                            }
                            .into_any();
                        }
                        let q = search.get();
                        if q.trim().is_empty() {
                            faction_collapsed.track();
                            faction_palette_rows(
                                &nodes,
                                0,
                                &[],
                                &[],
                                faction_collapsed,
                                favourites,
                                registry_items,
                                recent_placed,
                            )
                        } else {
                            let filtered =
                                crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                            if filtered.is_empty() {
                                let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
                                    &q, "assets",
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
    .into_any()
}
