//! CreateMissionDialog — the features/mission-creator/CreateMissionDialog.tsx port (T-159.25).
//! Transient "New Mission" dialog launched from the Mission Library (T-048): define environment,
//! `POST /missions`, then navigate to the 2D editor at /missions/:id/edit. The form resets on
//! every close (clean slate on reopen — the macOS Mail pattern).
#![allow(dead_code)]
use crate::ui::{cn, Dialog};
use leptos::prelude::*;

// macOS pill controls — match the Event Manager create dialog (admin.tsx).
const PILL: &str = "w-full rounded-full bg-white/5 px-5 py-3 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none transition focus:ring-1 focus:ring-primary/50";

const DEFAULT_TERRAIN: &str = "everon";
const DEFAULT_MODE: &str = "pve_coop";
const DEFAULT_WEATHER: &str = "clear";
const DEFAULT_TIME: &str = "14:00";
const DEFAULT_MAX: i64 = 64;

fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[component]
pub fn CreateMissionDialog(open: RwSignal<bool>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // The store feeds only the wasm-gated submit body.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let title = RwSignal::new(String::new());
    let terrain = RwSignal::new(DEFAULT_TERRAIN.to_string());
    let game_mode = RwSignal::new(DEFAULT_MODE.to_string());
    let weather = RwSignal::new(DEFAULT_WEATHER.to_string());
    let time_of_day = RwSignal::new(DEFAULT_TIME.to_string());
    let max_players = RwSignal::new(DEFAULT_MAX);
    let busy = RwSignal::new(false);

    let reset = move || {
        title.set(String::new());
        terrain.set(DEFAULT_TERRAIN.to_string());
        game_mode.set(DEFAULT_MODE.to_string());
        weather.set(DEFAULT_WEATHER.to_string());
        time_of_day.set(DEFAULT_TIME.to_string());
        max_players.set(DEFAULT_MAX);
    };
    // Reset to a clean slate whenever the dialog closes (handleOpenChange).
    Effect::new(move |_| {
        if !open.get() {
            reset();
        }
    });

    // handleSubmit: validate title, POST /missions, toast, close, navigate to the editor.
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
            let t = title.get_untracked().trim().to_string();
            if t.is_empty() {
                toasts.error("Title is required");
                return;
            }
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let body = serde_json::json!({
                "title": t,
                "terrain": terrain.get_untracked(),
                "game_mode": game_mode.get_untracked(),
                "weather": weather.get_untracked(),
                "time_of_day": time_of_day.get_untracked(),
                "max_players": max_players.get_untracked(),
            });
            leptos::task::spawn_local(async move {
                match crate::client::api_post::<serde_json::Value>(store, "/missions", body).await
                {
                    Ok(data) => {
                        toasts.success("Mission created");
                        open.set(false);
                        if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                            // navigate(`/missions/${id}/edit`) — a full-page load matches the
                            // lazy editor route boundary well enough here.
                            if let Some(win) = web_sys::window() {
                                let _ = win
                                    .location()
                                    .set_href(&format!("/missions/{id}/edit"));
                            }
                        }
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Failed to create mission",
                    )),
                }
                busy.set(false);
            });
        }
    };

    view! {
        <Dialog
            open=open
            title="New Mission"
            description="Define terrain and environment before opening the 2D editor."
            class="max-w-lg"
        >
            <form on:submit=on_submit class="space-y-5">
                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">
                        "Operation Designation"
                    </label>
                    <input
                        type="text"
                        placeholder="Enter operation designation..."
                        prop:value=move || title.get()
                        on:input=move |ev| title.set(event_target_value(&ev))
                        autofocus
                        class=PILL
                    />
                </div>

                <div>
                    <p class="mb-2 text-label-md text-on-surface-variant">"Terrain"</p>
                    <div class="grid gap-3 sm:grid-cols-2">
                        {["everon", "arland"]
                            .into_iter()
                            .map(|t| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=move |_| terrain.set(t.to_string())
                                        class=move || {
                                            cn(
                                                &[
                                                    "rounded-xl border p-4 text-left text-label-md font-semibold transition",
                                                    if terrain.get() == t {
                                                        "border-primary bg-primary/10 text-on-surface"
                                                    } else {
                                                        "border-white/10 bg-white/5 text-on-surface-variant hover:bg-white/10"
                                                    },
                                                ],
                                            )
                                        }
                                    >
                                        {terrain_label(t)}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>

                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">
                        "Game Mode"
                    </label>
                    <select
                        prop:value=move || game_mode.get()
                        on:change=move |ev| game_mode.set(event_target_value(&ev))
                        class=PILL
                    >
                        <option value="pve_coop">"Co-op PvE"</option>
                        <option value="pvp">"PvP"</option>
                        <option value="zeus">"Zeus"</option>
                    </select>
                </div>

                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">
                        "Insertion Time"
                    </label>
                    <input
                        type="time"
                        prop:value=move || time_of_day.get()
                        on:input=move |ev| time_of_day.set(event_target_value(&ev))
                        class=PILL
                    />
                </div>

                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">"Weather"</label>
                    <select
                        prop:value=move || weather.get()
                        on:change=move |ev| weather.set(event_target_value(&ev))
                        class=PILL
                    >
                        <option value="clear">"Clear (Default)"</option>
                        <option value="overcast">"Overcast"</option>
                        <option value="heavy_rain">"Heavy Rain"</option>
                        <option value="dense_fog">"Dense Fog"</option>
                    </select>
                </div>

                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">
                        "Max Players"
                    </label>
                    <select
                        prop:value=move || max_players.get().to_string()
                        on:change=move |ev| {
                            max_players.set(event_target_value(&ev).parse().unwrap_or(DEFAULT_MAX))
                        }
                        class=PILL
                    >
                        {[16i64, 32, 48, 64, 96, 128]
                            .into_iter()
                            .map(|n| {
                                view! {
                                    <option value=n.to_string()>{n} " Operators"</option>
                                }
                            })
                            .collect_view()}
                    </select>
                </div>

                <button
                    type="submit"
                    prop:disabled=move || busy.get()
                    class="w-full rounded-full bg-primary py-3 text-label-md font-semibold text-on-primary transition hover:bg-primary/90 disabled:opacity-50"
                >
                    {move || if busy.get() { "Creating…" } else { "Create Mission Draft" }}
                </button>
            </form>
        </Dialog>
    }
}
