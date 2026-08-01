//! T-661 — the Mission Settings dialog (environment + flow + render prefs), split from
//! `eden_chrome.rs`.
//!
//! Terrain (readonly) + time/weather author through [`crate::eden_env::author_env`] (T-193 gate);
//! [`render_flow_section`] is the T-224 mission-flow block; [`render_prefs_section`] is the T-173 P6
//! basemap/hillshade/grid/world-layer controls. Time and weather additionally mirror to the
//! `missions` row through [`crate::eden_top_strip::RowMirror`] (T-192). Renders no DOM while closed;
//! the doc-reading halves are wasm-only.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::eden_env::ENV_UNCARRIED_NOTE;
use crate::ui::MaterialIcon;

#[cfg(target_arch = "wasm32")]
use crate::eden_env::{
    author_env, fmt_duration_secs, parse_flow_seconds, read_flow_jip, read_flow_seconds,
    FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS,
    SETTINGS_UNREAD_NOTE,
};
#[cfg(target_arch = "wasm32")]
use crate::eden_top_strip::RowMirror;

/// Mission Settings dialog (MissionSettingsDialog.tsx — environment half). Terrain (readonly) +
/// time / weather flow through [`author_env`] (one undo step each); the render-pref controls (map
/// style, grid, hillshade, world-layer toggles) are live below them since T-173 P6. Renders no DOM
/// while closed. T-159.26.
///
/// **T-192:** time and weather additionally mirror to the `missions` row through [`RowMirror`] on
/// commit.
///
/// **T-193:** the View Distance field and the Thermals toggle are gone, and [`ENV_UNCARRIED_NOTE`]
/// stands where they were. They were fully working controls authoring keys that no compiled
/// document, no row column and no mod script has ever read — [`CARRIED_ENV_KEYS`] carries the whole
/// argument for deleting them rather than trying to carry them through.
///
/// **T-224:** [`render_flow_section`] adds the mission-flow block — duration, briefing, safe start
/// and join-in-progress — and [`SETTINGS_UNREAD_NOTE`] says why respawn, spectator policy, night
/// vision and tickets are not beside them.
#[component]
pub fn MissionSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the suite Dialog behavior).
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    // T-192 — read the route id + auth store here, in the component body: the reactive owner is
    // live at setup and gone by the time a control's `on:change` fires.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    let ctrl = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-read env on undo/redo while open
        #[cfg(target_arch = "wasm32")]
        let env = crate::editor_ops::read_env();
        #[cfg(not(target_arch = "wasm32"))]
        let env = crate::dto::MissionEnv::default();
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
                                            // T-192 — same handler as the doc write on purpose; a
                                            // `change` listener would not survive this dialog's
                                            // rebuild-per-doc-tick. Partial values arrive as "" and
                                            // repeats are absorbed by the mirror's dedupe.
                                            row_mirror.set_time(&t);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                            // T-193 — Weather moved up beside Time. They are the two settings a
                            // compiled mission actually carries, and they were only ever apart
                            // because View Distance held this cell.
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
                        {render_flow_section(ctrl)}
                        {render_prefs_section(&env)}
                    </div>
                </div>
            </div>
        })
    }
}

/// T-224 — the mission-flow half of Mission Settings: how long the round runs, how long the
/// briefing and safe start are, and who may join once it has started.
///
/// Every control here writes through [`author_env`], the same one gate the Time and Weather controls
/// take, so the "does anything read this back?" question is asked once per key rather than once per
/// control. [`AUTHORED_FLOW_KEYS`] holds the answers, the compiled path each key becomes, and the
/// one hop (`flatten.rs`, not this slice's file) that is still hardcoded.
///
/// The four settings this dialog deliberately does NOT grow — respawn, spectator policy, night
/// vision and per-faction tickets — are covered by [`SETTINGS_UNREAD_NOTE`], which renders below.
///
/// **`change`, not `input`.** A duration box authored per keystroke would file `5`, `54`, `540`,
/// `5400` as four undo steps, and each of those bumps `doc_tick`, which rebuilds this whole subtree
/// out from under the caret. `change` fires on blur/Enter — one undo step per settled value, and the
/// rebuild lands after the author has already left the field. (Weather already uses `change` for the
/// same reason; Time uses `input` because a `<input type="time">` has no half-typed state worth
/// suppressing and T-192's row mirror rides that handler.)
///
/// Inert on the native view shell (no document), exactly like [`render_prefs_section`].
fn render_flow_section(ctrl: &'static str) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctrl;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";

        // Mission duration leads, out of stage order (briefing → safe start → round), because it is
        // the one an author comes to this dialog to set; the other two are the stage lengths around
        // it. Each row is (key, label, committed value, the sentence the field needs or "").
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
                                    // A refused value must not stay on screen. Leaving "-1" in the
                                    // box while the document still holds 5400 is the editor showing
                                    // the author a setting they do not have — the same lie as a
                                    // silently reverted one, told the other way round.
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
                            // The <select> can only emit its own options, so this can only fail if
                            // JIP_OPTIONS ever drifts from the schema enum — in which case refusing
                            // is right: TBD_MissionFlow.PolicyFromString maps an unknown string to
                            // ALWAYS, so a bad value holds the door open all round instead of erroring.
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

/// T-173 P6 — the render-pref half of Mission Settings, restored from the React
/// `MissionSettingsDialog`: basemap view (Satellite / Map), hillshade on/off + strength slider,
/// grid, and the 12 world-layer toggles. Per-mission prefs (hillshade / grid) persist to
/// `meta.environment`; per-user prefs (basemap view + layer toggles) persist to localStorage. Each
/// control applies live to the map host (no reload). On the native view-shell these are inert
/// (no engine), which is fine — the dialog is a wasm surface.
fn render_prefs_section(env: &crate::dto::MissionEnv) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = env;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::world_layer_prefs as wlp;
        let hillshade_on = env.show_hillshade;
        let hillshade_pct = (env.hillshade_opacity * 100.0).round() as i64;
        let grid_on = env.show_grid;
        let basemap = wlp::load_basemap_view();
        let prefs = wlp::load_prefs();
        let sect = "text-label-sm uppercase tracking-wider text-outline";

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
                                crate::world_assets::refresh_world_layers();
                            }
                            class="accent-primary"
                        />
                    </div>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Basemap"</span>
                <div class="flex gap-2">
                    {["satellite", "map"]
                        .into_iter()
                        .map(|v| {
                            let active = basemap == v;
                            let label = if v == "satellite" { "Satellite" } else { "Map" };
                            view! {
                                <button
                                    type="button"
                                    class=if active {
                                        "flex-1 rounded-md border border-primary/60 bg-primary/20 px-2.5 py-1.5 text-label-md text-primary"
                                    } else {
                                        "flex-1 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant transition-colors hover:border-primary/40"
                                    }
                                    on:click=move |_| {
                                        wlp::save_basemap_view(v);
                                        crate::world_assets::apply_basemap_view(v);
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Show hillshade"</span>
                    <input
                        type="checkbox"
                        prop:checked=hillshade_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showHillshade", on.into());
                            let op = crate::editor_ops::read_env().hillshade_opacity;
                            crate::world_assets::apply_hillshade(on, op);
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
                            crate::world_assets::apply_hillshade(true, op);
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
                            crate::world_assets::apply_grid(on);
                        }
                        class="accent-primary"
                    />
                </div>

                <span class=sect>"World layers"</span>
                <div class="flex flex-col gap-1">{layer_rows}</div>
            </div>
        }
        .into_any()
    }
}
