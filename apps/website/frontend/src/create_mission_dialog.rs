//! CreateMissionDialog — the features/mission-creator/CreateMissionDialog.tsx port (T-159.25).
//! Transient "New Mission" dialog launched from the Mission Library (T-048): define environment,
//! `POST /missions`, then navigate to the 2D editor at /missions/:id/edit. The form resets on
//! every close (clean slate on reopen — the macOS Mail pattern).
//!
//! **T-671 — the briefing.** `POST /missions` has always taken a `briefing` field
//! (`handlers/missions.rs::CreateMissionInput`, bound straight into the INSERT) and this dialog has
//! never sent one, so every mission created here started with an empty library blurb and no surface
//! could fill it afterwards either. [`BRIEFING_HINT`] is the field that closes that half; the editor
//! half is `eden_settings::render_presentation_section`.
//!
//! **There is deliberately no thumbnail field here.** `create_mission` hardcodes `thumbnail_url` to
//! `''` and `CreateMissionInput` has no such member — PATCH is the column's only HTTP writer, by
//! T-413's design. A control here would post a key the handler drops on the floor, which is worse
//! than no control: it looks saved.
#![allow(dead_code)]
use crate::ui::{cn, Dialog};
use leptos::prelude::*;

// macOS pill controls — match the Event Manager create dialog (admin.tsx).
const PILL: &str = "w-full rounded-full bg-white/5 px-5 py-3 text-label-md text-on-surface placeholder:text-on-surface-variant/60 outline-none transition focus:ring-1 focus:ring-primary/50";

/// T-671 — what the briefing is for, said where it is typed. Same distinction the editor's
/// `eden_settings::BRIEFING_NOTE` draws: this is the library blurb (`missions.briefing`), not the
/// per-faction in-game briefing screen (`$defs/briefings`), which is authored on a faction.
///
/// It is optional on purpose — `CreateMissionInput::briefing` is `#[serde(default)]` and the column
/// takes `''` — because requiring an operation summary before the map has been opened would make the
/// new-mission button harder to press, and the editor can fill it in later.
const BRIEFING_HINT: &str =
    "Optional — the library blurb shown before anyone joins. You can write it later in the editor \
     (Mission Settings ▸ Presentation).";

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
    // T-671 — the library blurb. Empty is a valid mission (`#[serde(default)]` server-side).
    let briefing = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let reset = move || {
        title.set(String::new());
        terrain.set(DEFAULT_TERRAIN.to_string());
        game_mode.set(DEFAULT_MODE.to_string());
        weather.set(DEFAULT_WEATHER.to_string());
        time_of_day.set(DEFAULT_TIME.to_string());
        max_players.set(DEFAULT_MAX);
        briefing.set(String::new());
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
                // T-671 — trimmed, so a box the author only tabbed through stores `''` rather than
                // a whitespace blurb that renders as a blank card caption but is not empty to any
                // `is_empty()` check downstream (`mission_overview.rs`, `approvals.rs`).
                "briefing": briefing.get_untracked().trim(),
            });
            leptos::task::spawn_local(async move {
                match crate::client::api_post::<serde_json::Value>(store, "/missions", body).await {
                    Ok(data) => {
                        toasts.success("Mission created");
                        open.set(false);
                        if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                            // navigate(`/missions/${id}/edit`) — a full-page load matches the
                            // lazy editor route boundary well enough here.
                            if let Some(win) = web_sys::window() {
                                let _ = win.location().set_href(&format!("/missions/{id}/edit"));
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

                // T-671 — the library blurb. Last, and labelled optional, because it is the only
                // field here that can be filled in later without reopening this dialog.
                <div>
                    <label class="mb-2 block text-label-md text-on-surface-variant">"Briefing"</label>
                    <textarea
                        rows="4"
                        placeholder="What is this operation, who is involved, and what does winning look like?"
                        prop:value=move || briefing.get()
                        on:input=move |ev| briefing.set(event_target_value(&ev))
                        class=cn(&[PILL, "rounded-2xl resize-y leading-relaxed"])
                    ></textarea>
                    <p class="mt-2 text-label-sm text-on-surface-variant/70">{BRIEFING_HINT}</p>
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

// T-671 — the create half. Source scans, because both claims are about the SHAPE of the request this
// dialog builds: that `briefing` is on it, and that `thumbnail_url` is not. Needles are assembled from
// fragments and `live_source` truncates at the first `#[cfg(test)]`, so this module cannot become its
// own haystack (T-759).
#[cfg(test)]
mod t671_create_carries_the_briefing {
    use crate::arsenal::class_r_scrub::{live_source, only_body};

    /// The briefing is typed here and it reaches `POST /missions`. `CreateMissionInput::briefing`
    /// binds straight into the INSERT, so a control that does not make it onto the body is a field
    /// that looks authored and stores `''`.
    ///
    /// Perturbation this catches: adding the textarea without the body key (or removing the key and
    /// leaving the box), which is exactly the defect this ticket found — an accepting API with no
    /// caller.
    #[test]
    fn the_post_body_carries_the_authored_briefing() {
        let src = live_source(include_str!("create_mission_dialog.rs"));
        let submit = only_body(&src, "fn CreateMissionDialog");
        let key = format!("brief{}", "ing");
        assert!(
            submit.contains(&format!("\"{key}\":")),
            "T-671: POST /missions must carry the authored briefing"
        );
        assert!(
            submit.contains(&format!("<text{}", "area")),
            "T-671: the create dialog must offer somewhere to write it"
        );
        // Trimmed on the way out — a box that was only tabbed through must store `''`, not a
        // whitespace blurb that renders blank but is not empty to any downstream `is_empty()`.
        // Scoped to what follows the KEY: `.trim()` appears elsewhere in this body (the title
        // guard), so an unscoped needle would stay green with the briefing untrimmed.
        let at = submit.find(&format!("\"{key}\":")).expect("checked above") + key.len();
        assert!(
            submit[at..at + 80.min(submit.len() - at)].contains(&format!("tri{}", "m")),
            "T-671: the briefing must be trimmed before it is posted"
        );
    }

    /// **No thumbnail control here.** `create_mission` hardcodes `thumbnail_url` to `''` and
    /// `CreateMissionInput` has no such member, so a field on this form would post a key the handler
    /// drops — a control that looks saved and saves nothing. PATCH is the column's only HTTP writer
    /// (T-413), and the editor's Mission Settings is where it is authored.
    #[test]
    fn the_create_form_offers_no_thumbnail_it_cannot_store() {
        let src = live_source(include_str!("create_mission_dialog.rs"));
        let body = only_body(&src, "fn CreateMissionDialog");
        assert!(
            !body.contains(&format!("thumbnail{}", "_url")),
            "T-671: POST /missions does not accept thumbnail_url — do not post a key it drops"
        );
    }
}
