//! Mission dialog for the mission settings interface.

use super::*;

/// Renders mission environment, flow, and row settings.
#[component]
pub fn MissionSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
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
                blur_focused_control();
                open.set(false);
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    let ctrl = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
    let prefs_open = RwSignal::new(false);
    set_prefs_signal(prefs_open);
    let all_settings_open = RwSignal::new(false);
    set_all_settings_signal(all_settings_open);
    let shape = RwSignal::new(None::<RowShape>);
    #[cfg(target_arch = "wasm32")]
    {
        let loader = ShapeMirror::from_route();
        Effect::new(move |_| {
            if open.get() {
                loader.load(shape);
            }
        });
    }
    let body = move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-read env on undo/redo while open
        #[cfg(target_arch = "wasm32")]
        let env = crate::v2::apps::editor::bridge::host_state::editor_context::read_env();
        #[cfg(not(target_arch = "wasm32"))]
        let env = crate::v2::core::api::dto::MissionEnv::default();
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Mission Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Environment and flow for this mission."
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
                    <div class="flex flex-col gap-4">
                        <label class="flex flex-col gap-1">
                            <span class="text-label-sm uppercase tracking-wider text-outline">
                                "Terrain"
                            </span>
                            <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                                {env.terrain.clone()}
                            </div>
                        </label>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Time"
                                </span>
                                <input
                                    type="time"
                                    value=env.time.clone()
                                    on:input=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let t = event_target_value(&ev);
                                            author_env("time", t.as_str().into());
                                            row_mirror.set_time(&t);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Weather"
                                </span>
                                <select
                                    prop:value=env.weather.clone()
                                    on:change=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let w = event_target_value(&ev);
                                            author_env("weather", w.as_str().into());
                                            row_mirror.set_weather(&w);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                >
                                    <option value="clear">"Clear"</option>
                                    <option value="overcast">"Overcast"</option>
                                    <option value="heavy_rain">"Heavy Rain"</option>
                                    <option value="dense_fog">"Dense Fog"</option>
                                </select>
                            </label>
                        </div>
                        <p class="text-label-sm normal-case text-outline">{ENV_UNCARRIED_NOTE}</p>
                        {render_all_settings_pointer()}
                        {render_presentation_section(ctrl, shape)}
                        {render_shape_section(ctrl, shape)}
                        {render_flow_section(ctrl)}
                        {win_conditions_card(ctrl)}
                        {spawn_modules_panel(ctrl)}
                        {render_prefs_section(&env)}
                    </div>
                </div>
            </div>
        })
    };
    view! {
        {body}
        <EditorPreferencesDialog open=prefs_open />
        <AllSettingsDialog open=all_settings_open doc_tick=doc_tick />
    }
}

/// Renders the control that opens the full settings list.
pub(super) fn render_all_settings_pointer() -> AnyView {
    view! {
        <button
            type="button"
            class="mt-1 flex items-center justify-between gap-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-2 text-left transition-colors hover:border-primary/50"
            data-open-all-settings
            on:click=move |_| open_all_settings()
        >
            <span class="min-w-0">
                <span class="block text-label-md text-on-surface">"All settings in this mission"</span>
                <span class="block text-label-sm normal-case text-outline">
                    "One read-only list of every authored setting — including the ones that live on placed entities — against the defaults the schema declares."
                </span>
            </span>
            <MaterialIcon name="chevron_right" class="shrink-0 text-base text-outline" />
        </button>
    }
    .into_any()
}
