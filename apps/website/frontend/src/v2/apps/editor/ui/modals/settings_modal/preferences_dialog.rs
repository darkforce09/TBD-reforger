//! Preferences dialog for the mission settings interface.

use super::*;

/// Renders user-local map and layer preferences.
#[component]
pub(super) fn EditorPreferencesDialog(open: RwSignal<bool>) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            open.try_get_untracked().unwrap_or(false)
        });
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                open.set(false);
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    move || {
        if !open.get() {
            return None;
        }
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Editor Preferences"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Per-user editor settings — saved to this browser, not the mission."
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    {render_editor_prefs_body()}
                </div>
            </div>
        })
    }
}

/// Renders the user-local preference controls.
pub(super) fn render_editor_prefs_body() -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::v2::apps::editor::shell::world_layer_prefs::{self as wlp, WorldLayerPrefsView};
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let basemap = RwSignal::new(wlp::load_basemap_view());
        let prefs = wlp::load_prefs();

        let layer_rows = prefs
            .rows()
            .into_iter()
            .map(|(key, on, label)| {
                view! {
                    <div class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=on
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                let mut p = wlp::load_prefs();
                                p.set(key, checked);
                                wlp::save_prefs(&p);
                                website_map_engine::streaming::host::refresh_world_layers();
                            }
                            class="accent-primary"
                        />
                    </div>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="flex flex-col gap-4">
                <span class=sect>"Basemap"</span>
                <div class="flex gap-2">
                    {["satellite", "map"]
                        .into_iter()
                        .map(|v| {
                            let label = if v == "satellite" { "Satellite" } else { "Map" };
                            view! {
                                <button
                                    type="button"
                                    class=move || if basemap.get() == v {
                                        "flex-1 rounded-md border border-primary/60 bg-primary/20 px-2.5 py-1.5 text-label-md text-primary"
                                    } else {
                                        "flex-1 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant transition-colors hover:border-primary/40"
                                    }
                                    on:click=move |_| {
                                        wlp::save_basemap_view(v);
                                        website_map_engine::streaming::host::apply_basemap_view(v);
                                        basemap.set(v.to_string());
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                <span class=sect>"World layers"</span>
                <div class="flex flex-col gap-1">{layer_rows}</div>
            </div>
        }
        .into_any()
    }
}
