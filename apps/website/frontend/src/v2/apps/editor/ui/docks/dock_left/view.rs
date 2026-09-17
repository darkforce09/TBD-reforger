//! Left editor dock view.

use super::*;

mod full_dock;
mod places_body;
use full_dock::full_dock;
use places_body::places_body;

/// Left dock — the live **Editor Layers** outliner (spec O1). Click a folder to make it the drop
/// target, a slot to select it (no camera move — React parity).
///
///
/// tab-strip chevron both flip it (see [`collapse_chevron`]).
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_context::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    /// observe it).
    collapsed: RwSignal<bool>,
) -> impl IntoView {
    let layer_query = RwSignal::new(String::new());
    let layer_nodes = RwSignal::new(Vec::<OutlinerNode>::new());
    Effect::new(move |_| {
        let q = layer_query.get();
        layer_nodes.set(nodes.with(|ns| filter_outliner(ns, &q)));
    });
    let layers_filtered_empty = Memo::new(move |_| {
        !layer_query.with(|q| q.trim().is_empty()) && layer_nodes.with(Vec::is_empty)
    });

    let doc_hits = RwSignal::new(Vec::<DocHit>::new());
    Effect::new(move |_| {
        let _tick = nodes.with(Vec::len);
        let q = layer_query.get();
        doc_hits.set(search_document(&document_rows(), &q));
    });

    let sel_facets = RwSignal::new(Vec::<SelectionFacet>::new());
    Effect::new(move |_| {
        let n = selected.with(Vec::len);
        if n < 2 {
            sel_facets.set(Vec::new());
            return;
        }
        sel_facets.set(selection_facets(&selection_rows()));
    });

    let tab = RwSignal::new(LeftTab::Layers);
    let query = RwSignal::new(String::new());
    let places = RwSignal::new(Vec::<NamedPlace>::new());
    let places_armed = RwSignal::new(false);
    let bookmarks = RwSignal::new(load_bookmarks());
    let renaming = RwSignal::new(Option::<String>::None);
    let rename_draft = RwSignal::new(String::new());
    let rename_abandon = RwSignal::new(false);
    let adding = RwSignal::new(false);
    let add_draft = RwSignal::new(String::new());

    let commit_bookmarks = move |next: Bookmarks| {
        save_bookmarks(&next);
        bookmarks.set(next);
    };

    let arm_places = move || {
        if places_armed.get_untracked() {
            return;
        }
        places_armed.set(true);
        places.set(load_named_places());
    };

    let tab_btn = move |t: LeftTab, label: &'static str, testid: &'static str| {
        view! {
            <button
                type="button"
                role="tab"
                data-testid=testid
                aria-selected=move || (tab.get() == t).to_string()
                class=move || {
                    if tab.get() == t {
                        "shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase text-on-surface"
                    } else {
                        "shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase text-outline transition-colors hover:text-on-surface"
                    }
                }
                on:click=move |ev: web_sys::MouseEvent| {
                    ev.stop_propagation();
                    tab.set(t);
                    if t == LeftTab::Places {
                        arm_places();
                    }
                }
            >
                {label}
            </button>
        }
    };

    let fly_row = move |label: String,
                        title: String,
                        x: f64,
                        y: f64,
                        zoom: Option<f64>,
                        actions: AnyView| {
        view! {
            <div class="group flex items-center gap-1 rounded px-1 py-0.5 hover:bg-white/5">
                <button
                    type="button"
                    title=title
                    class="min-w-0 flex-1 cursor-pointer truncate text-left text-label-sm text-on-surface"
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        fly_to(x, y, zoom);
                    }
                >
                    {label}
                </button>
                {actions}
            </div>
        }
    };

    let places_body = places_body!(
        nodes,
        selected,
        active_layer,
        collapsed,
        layer_query,
        layer_nodes,
        layers_filtered_empty,
        doc_hits,
        sel_facets,
        tab,
        query,
        places,
        places_armed,
        bookmarks,
        renaming,
        rename_draft,
        rename_abandon,
        adding,
        add_draft,
        commit_bookmarks,
        arm_places,
        tab_btn,
        fly_row,
        hits_body,
        facets_row,
        places_body,
        full,
        stub
    );

    let hits_body = move || {
        let q = layer_query.get();
        if q.trim().is_empty() {
            return ().into_any();
        }
        let hits = doc_hits.get();
        if hits.is_empty() {
            let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
                &q,
                "entities in this mission",
            );
            return view! {
                <p class="mt-1 px-1 text-label-sm text-outline" data-testid="dock-left-search-empty">
                    {msg}
                </p>
            }
            .into_any();
        }
        let total = hits.len();
        let shown = total.min(MAX_DOC_HITS);
        view! {
            <section class="mt-1 flex shrink-0 flex-col" data-testid="dock-left-search-results">
                <h3 class="px-1 text-label-sm font-semibold uppercase text-outline">
                    {if shown == total {
                        format!("Found {total}")
                    } else {
                        format!("Found {total} — showing {shown}")
                    }}
                </h3>
                <div class="max-h-40 overflow-y-auto">
                    {hits
                        .into_iter()
                        .take(MAX_DOC_HITS)
                        .map(|hit| {
                            let routable = hit_is_routable(&hit);
                            let kind = hit.entity.kind;
                            let id = hit.entity.id.clone();
                            let badge = kind.noun();
                            let label = hit.entity.label.clone();
                            let matched = if hit.field == "faction" {
                                hit.entity.faction.clone()
                            } else {
                                hit.entity
                                    .text
                                    .iter()
                                    .find(|(f, _)| *f == hit.field)
                                    .map_or_else(String::new, |(_, v)| v.clone())
                            };
                            let body = view! {
                                <MaterialIcon
                                    name=kind.icon()
                                    class="block shrink-0 text-sm text-outline"
                                />
                                <span class="min-w-0 flex-1 truncate text-left text-label-sm">
                                    {label.clone()}
                                </span>
                                <span class="shrink-0 text-label-sm lowercase text-outline">
                                    {badge}
                                </span>
                            };
                            if routable {
                                let title = format!(
                                    "Select this {badge} — matched {} \"{matched}\" ({id})",
                                    hit.field,
                                );
                                let click_id = id.clone();
                                view! {
                                    <button
                                        type="button"
                                        title=title
                                        data-testid="dock-left-search-hit"
                                        class="flex w-full cursor-pointer items-center gap-1 rounded px-1 py-0.5 text-on-surface hover:bg-white/5"
                                        on:click=move |ev: web_sys::MouseEvent| {
                                            ev.stop_propagation();
                                            crate::v2::apps::editor::ui::inspector::validation_panel::route_select_by_subject_id(
                                                &click_id,
                                            );
                                        }
                                    >
                                        {body}
                                    </button>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div
                                        aria-disabled="true"
                                        title=unselectable_reason(kind)
                                        data-testid="dock-left-search-hit-inert"
                                        class="flex w-full items-center gap-1 rounded px-1 py-0.5 text-outline"
                                    >
                                        {body}
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        }
        .into_any()
    };

    let facets_row = move || {
        let facets = sel_facets.get();
        if facets.is_empty() {
            return ().into_any();
        }
        view! {
            <section class="mt-1 flex shrink-0 flex-col" data-testid="dock-left-selection-filter">
                <h3 class="px-1 text-label-sm font-semibold uppercase text-outline">
                    "Filter selection"
                </h3>
                <div class="flex flex-wrap gap-1 px-1 py-0.5">
                    {facets
                        .into_iter()
                        .map(|f| {
                            let n = f.ids.len();
                            let title = format!(
                                "Keep only the {n} selected by {}: {}",
                                f.axis.to_lowercase(),
                                f.label,
                            );
                            let ids = f.ids.clone();
                            view! {
                                <button
                                    type="button"
                                    title=title
                                    data-testid="dock-left-selection-facet"
                                    class="shrink-0 cursor-pointer rounded border border-outline-variant/30 px-1.5 py-0.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        apply_selection(ids.clone());
                                    }
                                >
                                    {format!("{} ({n})", f.label)}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        }
        .into_any()
    };

    let full = full_dock!(
        nodes,
        selected,
        active_layer,
        collapsed,
        layer_query,
        layer_nodes,
        layers_filtered_empty,
        doc_hits,
        sel_facets,
        tab,
        query,
        places,
        places_armed,
        bookmarks,
        renaming,
        rename_draft,
        rename_abandon,
        adding,
        add_draft,
        commit_bookmarks,
        arm_places,
        tab_btn,
        fly_row,
        hits_body,
        facets_row,
        places_body,
        full,
        stub
    );
    let stub = move || {
        view! {
            <div
                class="pointer-events-auto flex items-start bg-surface-container-lowest/55 backdrop-blur-xl"
                style=format!("width:{STUB_PX}px;height:{STUB_PX}px")
            >
                {collapse_chevron(collapsed, true)}
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
