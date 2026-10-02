//! Full dock rendering for the left dock.

use super::*;

macro_rules! full_dock {
    ($nodes:ident, $selected:ident, $active_layer:ident, $collapsed:ident, $layer_query:ident, $layer_nodes:ident, $layers_filtered_empty:ident, $doc_hits:ident, $sel_facets:ident, $tab:ident, $query:ident, $places:ident, $places_armed:ident, $bookmarks:ident, $renaming:ident, $rename_draft:ident, $rename_abandon:ident, $adding:ident, $add_draft:ident, $commit_bookmarks:ident, $arm_places:ident, $tab_btn:ident, $fly_row:ident, $hits_body:ident, $facets_row:ident, $places_body:ident, $full:ident, $stub:ident) => {
move || {
        view! {
            <aside class=DOCK_L>
                <div
                    class="group flex items-center justify-between gap-1 rounded px-1 py-0.5"
                    title="Drop a folder here to move it to the top level"
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        ev.stop_propagation();
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = engine_ops::complete_layer_drop_onto_root();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div class="flex min-w-0 items-center gap-1" role="tablist">
                        {collapse_chevron($collapsed, true)}
                        {$tab_btn(LeftTab::Layers, TAB_LABEL_LAYERS, "dock-left-tab-layers")}
                        {$tab_btn(LeftTab::Places, TAB_LABEL_PLACES, "dock-left-tab-places")}
                    </div>
                    {move || {
                        if $tab.get() == LeftTab::Places {
                            view! {
                                <button
                                    type="button"
                                    aria-label="Bookmark this view"
                                    data-testid="dock-left-bookmark-add"
                                    title="Bookmark this view (name the current camera position)"
                                    class="flex size-5 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        $add_draft.set($bookmarks.with_untracked(default_bookmark_name));
                                        $adding.set(true);
                                    }
                                >
                                    <MaterialIcon name="bookmark_add" class="block text-base" />
                                </button>
                            }
                                .into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    aria-label="New layer"
                                    title="New layer (child of the selected layer)"
                                    class="flex size-5 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let _ = outliner::create_layer();
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                >
                                    <MaterialIcon name="add" class="block text-base" />
                                </button>
                            }
                                .into_any()
                        }
                    }}
                </div>
                {move || {
                    ($tab.get() == LeftTab::Layers)
                        .then(|| {
                            let dest = $active_layer.with(|a| {
                                a.as_deref()
                                    .and_then(|id| $nodes.with(|ns| find_layer_label(ns, id)))
                            });
                            let (name, muted) = match dest {
                                Some(label) => (label, false),
                                None => match $nodes.with(|ns| first_folder_label(ns)) {
                                    Some(label) => (label, true),
                                    None => ("a new layer".to_string(), true),
                                },
                            };
                            let name_cls = if muted {
                                "min-w-0 flex-1 truncate font-medium text-on-surface-variant"
                            } else {
                                "min-w-0 flex-1 truncate font-medium text-primary"
                            };
                            view! {
                                <div
                                    class="mt-1 flex shrink-0 items-center gap-1 rounded bg-black/20 px-1.5 py-0.5 text-label-sm text-outline"
                                    data-testid="dock-left-drop-target"
                                    title="Placements (units and comments) file into this layer. Click a layer row to change it."
                                >
                                    <MaterialIcon
                                        name="place"
                                        class="block shrink-0 text-sm leading-none"
                                    />
                                    <span class="shrink-0">"Placing into:"</span>
                                    <span class=name_cls>{name}</span>
                                </div>
                            }
                        })
                }}
                {move || {
                    ($tab.get() == LeftTab::Layers)
                        .then(|| {
                            view! {
                                <input
                                    type="search"
                                    data-testid="dock-left-layers-filter"
                                    aria-label="Search the mission and filter editor layers"
                                    placeholder="Search mission — name, class:, mod:"
                                    prop:value=move || $layer_query.get()
                                    class="mt-1 w-full shrink-0 rounded border border-outline-variant/30 bg-black/20 px-1.5 py-0.5 text-label-sm text-on-surface outline-none placeholder:text-outline focus:border-primary/60"
                                    on:input=move |ev| $layer_query.set(event_target_value(&ev))
                                />
                            }
                        })
                }}
                {move || ($tab.get() == LeftTab::Layers).then($facets_row)}
                {move || ($tab.get() == LeftTab::Layers).then($hits_body)}
                {move || {
                    if $tab.get() == LeftTab::Places {
                        $places_body().into_any()
                    } else if $layers_filtered_empty.get() {
                        view! {
                            <p class="mt-2 px-1 text-label-sm text-outline">
                                "No layers match that filter."
                            </p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div
                                class="mt-1 min-h-0 flex-1 overflow-y-auto"
                                on:pointerup=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    engine_ops::cancel_layer_drag();
                                }
                            >
                                {virtual_tree(
                                    $layer_nodes,
                                    $selected,
                                    $active_layer,
                                    "editorLayers",
                                    "No objects placed yet.",
                                    false,
                                    true,
                                )}
                            </div>
                        }
                            .into_any()
                    }
                }}
            </aside>
        }
    }
    };
}
/// Expose the full dock view fragment.
pub(super) use full_dock;
