//! Attributes modal asset type picker behavior.

use super::*;

/// Renders searchable catalog selection with an advanced asset-id field.
#[cfg(target_arch = "wasm32")]
pub(super) fn type_picker(
    label: &'static str,
    value: String,
    gate: Gate,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    on_commit: impl Fn(String) + Copy + Send + 'static,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let advanced = RwSignal::new(false);
    let current = StoredValue::new(value.clone());
    let trigger_text = move || {
        let id = current.get_value();
        if gate.differs() {
            return "Multiple values".to_string();
        }
        if id.is_empty() {
            return "Faction default".to_string();
        }
        registry_items
            .get()
            .as_deref()
            .and_then(|items| {
                crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(items, &id)
            })
            .map_or(id, |it| it.display_name.clone())
    };
    let pick = move |asset_id: String| {
        on_commit(asset_id);
        open.set(false);
    };
    view! {
        <div class="flex flex-col gap-1">
            {field_label(label, gate)}
            <div class="relative">
                <button
                    type="button"
                    aria-label=label
                    aria-haspopup="listbox"
                    data-testid="type-picker-trigger"
                    disabled=move || gate.locked()
                    on:click=move |_| {
                        if !gate.locked_now() {
                            if !open.get_untracked() {
                                query.set(String::new());
                            }
                            open.update(|o| *o = !*o);
                        }
                    }
                    class=move || {
                        let lock = if gate.locked() { CONTROL_LOCKED } else { "" };
                        format!("{CONTROL} flex items-center justify-between text-left{lock}")
                    }
                >
                    <span class=move || {
                        let id = current.get_value();
                        if gate.differs() || id.is_empty() {
                            "truncate text-on-surface-variant"
                        } else {
                            "truncate text-on-surface"
                        }
                    }>{trigger_text}</span>
                    <crate::v2::core::ui::MaterialIcon name="search" />
                </button>
                {move || {
                    open.get().then(|| {
                        let items = registry_items.get();
                        let body = match items {
                            None => view! {
                                <p
                                    class="px-3 py-4 text-label-sm text-on-surface-variant"
                                    data-testid="type-picker-loading"
                                >
                                    "Loading the asset catalog…"
                                </p>
                            }.into_any(),
                            Some(items) => {
                                let full = crate::v2::apps::editor::arsenal::asset_catalog::build_picker_catalog_tree(&items);
                                if crate::v2::apps::editor::arsenal::asset_catalog::catalog_leaf_count(&full) == 0 {
                                    view! {
                                        <div
                                            class="flex flex-col gap-2 px-3 py-3"
                                            data-testid="type-picker-empty"
                                        >
                                            <p class="text-label-sm text-error">
                                                "No modpack is configured, so the asset catalog is empty. Set a current modpack, then retry."
                                            </p>
                                            <button
                                                type="button"
                                                data-testid="type-picker-retry"
                                                class="self-start rounded border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface transition hover:bg-surface-container-high"
                                                on:click=move |_| {
                                                    if let Some(w) = web_sys::window() {
                                                        let _ = w.location().reload();
                                                    }
                                                }
                                            >
                                                "Retry"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    let q = query.get();
                                    let filtered = crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&full, &q);
                                    let mut leaves: Vec<(String, String)> = Vec::new();
                                    fn collect(
                                        nodes: &[crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode],
                                        out: &mut Vec<(String, String)>,
                                    ) {
                                        for n in nodes {
                                            if let Some(p) = &n.payload {
                                                out.push((n.label.clone(), p.asset_id.clone()));
                                            }
                                            collect(&n.children, out);
                                        }
                                    }
                                    collect(&filtered, &mut leaves);
                                    let no_match = !q.trim().is_empty() && leaves.is_empty();
                                    let empty_msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(&q, "assets");
                                    let rows = leaves
                                        .into_iter()
                                        .map(|(lbl, id)| {
                                            let idc = id.clone();
                                            view! {
                                                <button
                                                    type="button"
                                                    data-testid="type-picker-leaf"
                                                    class="block w-full truncate px-3 py-1.5 text-left text-label-md text-on-surface hover:bg-primary/20"
                                                    title=id
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        pick(idc.clone());
                                                    }
                                                >
                                                    {lbl}
                                                </button>
                                            }
                                        })
                                        .collect_view();
                                    view! {
                                        <div class="min-h-0 flex-1 overflow-y-auto py-1">
                                            <button
                                                type="button"
                                                data-testid="type-picker-clear"
                                                class="block w-full truncate border-b border-outline-variant/20 px-3 py-1.5 text-left text-label-md text-on-surface-variant hover:bg-primary/20"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    pick(String::new());
                                                }
                                            >
                                                "Faction default (clear)"
                                            </button>
                                            {no_match
                                                .then(|| view! {
                                                    <p
                                                        class="px-3 py-2 text-label-sm text-on-surface-variant"
                                                        data-testid="type-picker-nomatch"
                                                    >
                                                        {empty_msg}
                                                    </p>
                                                })}
                                            {rows}
                                        </div>
                                    }.into_any()
                                }
                            }
                        };
                        view! {
                            <div
                                class="absolute inset-0 z-40"
                                on:click=move |_| open.set(false)
                            ></div>
                            <div
                                class="glass absolute left-0 right-0 top-full z-50 mt-1 flex max-h-64 flex-col overflow-hidden rounded-md border border-outline-variant/30 shadow-2xl"
                                data-testid="type-picker-popover"
                            >
                                <div class="border-b border-outline-variant/25 p-1.5">
                                    <input
                                        type="search"
                                        autofocus
                                        aria-label="Search asset types"
                                        data-testid="type-picker-search"
                                        class="w-full rounded bg-surface/40 px-2 py-1 text-label-md text-on-surface outline-none placeholder:text-on-surface-variant"
                                        placeholder="Search types…"
                                        on:input=move |ev| query.set(event_target_value(&ev))
                                        on:keydown=move |ev| {
                                            if ev.key() == "Escape" {
                                                ev.stop_propagation();
                                                open.set(false);
                                            }
                                        }
                                    />
                                </div>
                                {body}
                            </div>
                        }
                    })
                }}
            </div>
            {(!gate.differs()).then(|| view! {
                <button
                    type="button"
                    data-testid="type-picker-advanced-toggle"
                    class="self-start text-label-sm normal-case text-primary hover:underline"
                    on:click=move |_| advanced.update(|a| *a = !*a)
                >
                    {move || if advanced.get() { "Hide advanced" } else { "Advanced: enter an asset id" }}
                </button>
                <div class=move || if advanced.get() { "" } else { "hidden" }>
                    {text_field(
                        label,
                        current.get_value(),
                        "Asset id — empty uses the faction default",
                        gate,
                        on_commit,
                    )}
                </div>
            })}
        </div>
    }
}
