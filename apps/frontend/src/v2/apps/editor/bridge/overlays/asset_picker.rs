//! Asset picker for editor overlays.
use super::*;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::{armed_placement, editor_context};

/// Map and screen anchors for an open asset picker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetPickerState {
    pub wx: f64,
    pub wy: f64,
    pub screen_x: f64,
    pub screen_y: f64,
}

/// Renders the asset picker at its anchored map position.
#[component]
pub(crate) fn AssetPickerOverlay(
    picker: RwSignal<Option<AssetPickerState>>,
    registry: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    active_side: RwSignal<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    Effect::new(move |_| {
        if picker.get().is_some() {
            query.set(String::new());
        }
    });
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            picker.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if picker.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_context::close_asset_picker();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        let state = picker.get()?;
        let items = registry.get().unwrap_or_default();
        let tree = crate::v2::apps::editor::arsenal::asset_catalog::build_catalog_tree(
            &items,
            &active_side.get(),
        );
        let mut leaves: Vec<(
            String,
            crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload,
        )> = Vec::new();
        fn collect(
            nodes: &[crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode],
            out: &mut Vec<(
                String,
                crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload,
            )>,
        ) {
            for n in nodes {
                if let Some(p) = &n.payload {
                    out.push((n.label.clone(), p.clone()));
                }
                collect(&n.children, out);
            }
        }
        collect(&tree, &mut leaves);
        let q = query.get().trim().to_lowercase();
        if !q.is_empty() {
            leaves.retain(|(label, _)| label.to_lowercase().contains(&q));
        }
        let pos = format!("left:{:.0}px;top:{:.0}px", state.screen_x, state.screen_y);
        let rows = leaves
            .into_iter()
            .map(|(label, payload)| {
                view! {
                    <button
                        class="block w-full truncate px-3 py-1.5 text-left text-sm text-on-surface hover:bg-primary/20"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            {
                                armed_placement::begin_place(payload.clone());
                                editor_context::close_asset_picker();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {label}
                    </button>
                }
            })
            .collect_view();
        Some(view! {
            <div
                class="fixed inset-0 z-40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_context::close_asset_picker();
                }
                on:contextmenu=move |ev| ev.prevent_default()
            ></div>
            <div
                class="glass animate-dialog-in fixed z-50 flex max-h-[22rem] w-64 flex-col overflow-hidden rounded-md border border-outline-variant/30 shadow-2xl outline-none"
                style=pos
                on:contextmenu=move |ev| ev.prevent_default()
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="border-b border-outline-variant/25 px-2 py-1.5">
                    <input
                        type="search"
                        class="w-full rounded bg-surface/40 px-2 py-1 text-sm text-on-surface outline-none placeholder:text-on-surface-variant"
                        placeholder="Place asset…"
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto py-1">{rows}</div>
            </div>
        })
    }
}
