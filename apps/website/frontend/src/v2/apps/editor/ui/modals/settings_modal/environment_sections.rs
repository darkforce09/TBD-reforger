//! Environment sections for the mission settings interface.

use super::*;

/// Renders authored mission flow settings.
pub(super) fn render_flow_section(ctrl: &'static str) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctrl;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";

        let rows: [(&str, &str, i64, &str); 3] = [
            (
                "timeLimitSeconds",
                "Mission duration",
                read_flow_seconds("timeLimitSeconds", FLOW_DEFAULT_TIMELIMIT_S),
                "0 means no time limit — the round will not end on the clock.",
            ),
            (
                "briefingSeconds",
                "Briefing",
                read_flow_seconds("briefingSeconds", FLOW_DEFAULT_BRIEFING_S),
                "Announced on entering the briefing; the stage is advanced by an admin, not by this \
                 clock.",
            ),
            (
                "safeStartSeconds",
                "Safe start",
                read_flow_seconds("safeStartSeconds", FLOW_DEFAULT_SAFESTART_S),
                "",
            ),
        ];

        let duration_rows = rows
            .into_iter()
            .map(|(key, label, committed, hint_text)| {
                view! {
                    <label class="flex flex-col gap-1">
                        <span class=sect>
                            {format!("{label} — {}", fmt_duration_secs(committed))}
                        </span>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            value=committed.to_string()
                            on:change=move |ev| {
                                let raw = event_target_value(&ev);
                                if let Some(secs) = parse_flow_seconds(&raw) {
                                    author_env(key, secs.into());
                                } else {
                                    leptos::logging::warn!(
                                        "refusing meta.environment.{key} = {raw:?}: flow durations are whole seconds >= 0"
                                    );
                                    if let Some(input) = ev
                                        .target()
                                        .and_then(|t| {
                                            wasm_bindgen::JsCast::dyn_into::<
                                                web_sys::HtmlInputElement,
                                            >(t)
                                                .ok()
                                        })
                                    {
                                        input.set_value(&committed.to_string());
                                    }
                                }
                            }
                            class=ctrl
                        />
                        {(!hint_text.is_empty())
                            .then(|| view! { <span class=hint>{hint_text}</span> })}
                    </label>
                }
            })
            .collect::<Vec<_>>();

        let jip = read_flow_jip();
        let jip_options = JIP_OPTIONS
            .into_iter()
            .map(|(value, label)| view! { <option value=value>{label}</option> })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Mission flow"</span>
                {duration_rows}
                <label class="flex flex-col gap-1">
                    <span class=sect>"Join in progress"</span>
                    <select
                        prop:value=jip
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            if JIP_OPTIONS.iter().any(|(k, _)| *k == v) {
                                author_env("jip", v.as_str().into());
                            } else {
                                leptos::logging::error!("refusing meta.environment.jip = {v:?}");
                            }
                        }
                        class=ctrl
                    >
                        {jip_options}
                    </select>
                    <span class=hint>
                        "Whether a player who connects after the mission has started may deploy."
                    </span>
                </label>
                <p class=hint>{SETTINGS_UNREAD_NOTE}</p>
            </div>
        }
        .into_any()
    }
}

/// Renders document display preferences.
pub(super) fn render_prefs_section(env: &crate::v2::core::api::dto::MissionEnv) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = env;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let hillshade_on = env.show_hillshade;
        let hillshade_pct = (env.hillshade_opacity * 100.0).round() as i64;
        let grid_on = env.show_grid;
        let sect = "text-label-sm uppercase tracking-wider text-outline";

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Show hillshade"</span>
                    <input
                        type="checkbox"
                        prop:checked=hillshade_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showHillshade", on.into());
                            let op = crate::v2::apps::editor::bridge::host_state::editor_context::read_env().hillshade_opacity;
                            website_map_engine::streaming::host::apply_hillshade(on, op);
                        }
                        class="accent-primary"
                    />
                </div>
                <label class="flex flex-col gap-1" class:opacity-40=move || !hillshade_on>
                    <span class=sect>{move || format!("Hillshade strength — {hillshade_pct}%")}</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="1"
                        prop:disabled=!hillshade_on
                        value=hillshade_pct.to_string()
                        on:input=move |ev| {
                            let pct: f64 = event_target_value(&ev).parse().unwrap_or(40.0);
                            let op = (pct / 100.0).clamp(0.0, 1.0);
                            author_env("hillshadeOpacity", op.into());
                            website_map_engine::streaming::host::apply_hillshade(true, op);
                        }
                        class="accent-primary"
                    />
                </label>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Grid"</span>
                    <input
                        type="checkbox"
                        prop:checked=grid_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showGrid", on.into());
                            website_map_engine::streaming::host::apply_grid(on);
                        }
                        class="accent-primary"
                    />
                </div>

                <button
                    type="button"
                    class="mt-1 flex items-center justify-between gap-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-2 text-left transition-colors hover:border-primary/50"
                    on:click=move |_| open_editor_preferences()
                >
                    <span class="min-w-0">
                        <span class="block text-label-md text-on-surface">
                            "Editor preferences moved"
                        </span>
                        <span class="block text-label-sm normal-case text-outline">
                            "Basemap view and world layers are now per-user editor preferences."
                        </span>
                    </span>
                    <MaterialIcon name="chevron_right" class="shrink-0 text-base text-outline" />
                </button>
            </div>
        }
        .into_any()
    }
}
