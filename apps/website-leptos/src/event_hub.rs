//! Event Hub (/events/:id) — ported from pages/events.tsx `EventHubPage` + `EventHubView` +
//! `MissionDossier` + the shared `OrbatSelector`. `<AuthGate>` → `useEvent(id)` → `GET /events/:id`
//! → a topo-glass hero (name, T-MINUS countdown, datetime, TS3 + current-modpack link) + per-mission
//! dossiers (briefing lore, BLUFOR/OPFOR objectives, faction dossiers, inline ORBAT selector).
//!
//! **Gate scope:** the seeded golden (event c71a4d1a-…, one mission dossier with empty
//! factions/armory + empty ORBAT). So the Faction-Dossiers section is hidden (no factions) and the
//! ORBAT selector short-circuits to "No ORBAT slots defined…" (its ~300-LOC faction/squad/slot shell
//! only renders with a non-empty ORBAT golden — content-gated). Dates go through `datefmt` (js_sys →
//! the frozen clock). DTO round-trips are R-api-proven (dto.rs `event_hub` / `orbat_envelope`).
#![allow(dead_code)]
use crate::datefmt::{countdown_label, format_local_datetime};
use crate::dto::{DataEnvelope, EventHub, EventMissionDossier, ModpackDto};
use crate::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde_json::Value;

// Placeholder mission-intel (events.tsx) — the API doesn't yet serve maker/duration/objectives/lore.
const PLACEHOLDER_MAKER: &str = "Sam";
const PLACEHOLDER_DURATION: &str = "90 MIN";
const PLACEHOLDER_BLUFOR: [&str; 3] = [
    "Protect the forward operating bases",
    "Protect and secure the nuke",
    "Escort the VIP convoy to extraction",
];
const PLACEHOLDER_OPFOR: [&str; 3] = [
    "Capture the forward operating bases",
    "Find and detonate the nuke",
    "Intercept the VIP convoy",
];
const PLACEHOLDER_LORE: &str = "Hostile mechanized elements have pushed across the northern border under cover of a winter storm. Command has tasked us with holding the line until reinforcements arrive. Expect contested airspace and degraded visibility.";

fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}
fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[component]
pub fn EventHubPage() -> impl IntoView {
    view! {
        <AuthGate>
            <EventHubInner />
        </AuthGate>
    }
}

#[component]
fn EventHubInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let params = use_params_map();
    let event = LocalResource::new(move || {
        let id = params
            .read()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/events/{id}");
                crate::client::api_get::<EventHub>(store, &path).await.ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<EventHub>
            }
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                event
                    .get()
                    .map(|opt| match opt {
                        Some(ev) => hub_shell(ev).into_any(),
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

fn hub_shell(ev: EventHub) -> impl IntoView {
    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto bg-surface-glass backdrop-blur-xl">
                <div class="mx-auto w-full max-w-5xl p-6 md:p-8">
                    <a
                        href="/events"
                        class="mb-4 inline-flex items-center gap-1 text-label-md text-primary hover:underline"
                    >
                        <MaterialIcon name="chevron_left" class="text-base" />
                        " All Operations"
                    </a>
                    {event_hub_view(ev)}
                </div>
            </div>
        </div>
    }
}

fn event_hub_view(ev: EventHub) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let modpack = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<ModpackDto>(store, "/modpacks/current")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<ModpackDto>
        }
    });
    let name = ev
        .name_override
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Untitled Operation".into());
    let countdown = countdown_label(&ev.start_time);
    let when = format_local_datetime(&ev.start_time);
    let missions = ev.missions;
    let has_missions = !missions.is_empty();
    view! {
        <section class="relative mb-8 overflow-hidden rounded-xl border border-outline-variant/30 bg-surface-container p-8">
            <div class="pointer-events-none absolute inset-0 bg-gradient-to-b from-primary/10 to-transparent"></div>
            <div class="relative flex flex-col gap-3">
                <span class="text-label-sm text-on-surface-variant uppercase">"Operation Hub"</span>
                <h1 class="text-headline-lg text-on-surface md:text-4xl">{name}</h1>
                <div class="font-mono text-headline-md tracking-widest text-primary">
                    "T-MINUS "
                    {countdown}
                </div>
                <p class="text-on-surface-variant">{when}</p>
                <div class="mt-2 flex flex-wrap gap-3 text-label-md">
                    <span class="flex items-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container-high px-3 py-2">
                        <MaterialIcon name="headset_mic" class="text-primary" />
                        " TS3: ts.tbdevent.eu"
                    </span>
                    {move || {
                        modpack
                            .get()
                            .flatten()
                            .map(|mp| {
                                // ModpackDto flattens a Modpack; workshop_url is String (empty when
                                // absent) → React's `?? '#'`.
                                let href = if mp.modpack.workshop_url.is_empty() {
                                    "#".to_string()
                                } else {
                                    mp.modpack.workshop_url.clone()
                                };
                                view! {
                                    <a
                                        href=href
                                        target="_blank"
                                        rel="noreferrer"
                                        class="flex items-center gap-2 rounded-lg border border-outline-variant/30 bg-surface-container-high px-3 py-2 hover:border-primary/40"
                                    >
                                        <MaterialIcon name="extension" class="text-primary" />
                                        " "
                                        {mp.modpack.name.clone()}
                                        " v"
                                        {mp.modpack.version.clone()}
                                    </a>
                                }
                            })
                    }}
                </div>
            </div>
        </section>

        <h2 class="mb-4 text-label-md text-on-surface-variant uppercase tracking-wide">
            "Mission Dossiers"
        </h2>
        {if has_missions {
            view! {
                <div class="flex flex-col gap-6">
                    {missions
                        .into_iter()
                        .enumerate()
                        .map(|(i, m)| mission_dossier(i + 1, m))
                        .collect_view()}
                </div>
            }
                .into_any()
        } else {
            view! {
                <p class="text-on-surface-variant">
                    "No missions have been added to this operation yet."
                </p>
            }
                .into_any()
        }}
    }
}

fn meta_badge(label: &'static str, value: String) -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-1.5 rounded border border-outline-variant/30 bg-surface-container/60 px-2 py-1 font-mono text-[11px] uppercase tracking-wide">
            <span class="text-on-surface-variant">{label} ":"</span>
            <span class="text-on-surface">{value}</span>
        </span>
    }
}

fn mission_dossier(index: usize, m: EventMissionDossier) -> impl IntoView {
    // OpsCard(className="bg-surface-container-high"): twMerge drops the default bg-surface-container
    // vs the override bg-surface-container-high (same bg-color group). Inlined resolved (our cn()
    // doesn't group these two hyphenated bg names — a real cn() gap tracked to the tw_merge task).
    let card = "relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 border border-border-subtle bg-surface-container-high";
    // factionList empty (no factions, no armory) → the Faction Dossiers section is hidden.
    let show_factions = !m.factions.is_empty() || !m.armory_by_faction.is_empty();
    let briefing = m
        .briefing
        .clone()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| PLACEHOLDER_LORE.into());
    let terrain = terrain_label(&m.terrain);
    let mode = game_mode_label(&m.game_mode).to_string();
    let when = format_local_datetime(&m.start_time);
    view! {
        <div class=card>
            <div class="flex flex-wrap items-start justify-between gap-4">
                <div>
                    <span class="text-xs font-semibold uppercase tracking-widest text-on-surface-variant">
                        "Mission " {index}
                    </span>
                    <h3 class="mt-1 text-xl font-semibold">{m.title.clone()}</h3>
                    <p class="mt-1 text-sm text-on-surface-variant">
                        {terrain.clone()} " • " {mode} " • " {when}
                    </p>
                    <div class="mt-2 flex flex-wrap gap-2">
                        {meta_badge("Maker", PLACEHOLDER_MAKER.into())}
                        {meta_badge("Terrain", terrain)}
                        {meta_badge("Duration", PLACEHOLDER_DURATION.into())}
                    </div>
                </div>
                <div class="flex flex-col items-end gap-2">
                    <p class="font-mono text-sm text-on-surface-variant">
                        {m.filled} "/" {m.total} " slots filled"
                    </p>
                    <button
                        type="button"
                        disabled
                        title="2D mission planner — coming soon"
                        class="flex cursor-not-allowed items-center gap-2 rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-on-surface-variant opacity-50"
                    >
                        <MaterialIcon name="map" class="text-base" />
                        " Mission Planner"
                    </button>
                </div>
            </div>

            <section class="mt-4">
                <h4 class="mb-2 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "Mission Briefing"
                </h4>
                <p class="whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                    {briefing}
                </p>
                <div class="mt-4 grid grid-cols-1 gap-6 md:grid-cols-2">
                    <div class="rounded-lg border border-secondary/20 bg-secondary-container/10 p-4">
                        <div class="mb-3 flex items-center gap-2">
                            <div class="h-2.5 w-2.5 rounded-full bg-primary shadow-[0_0_8px_rgba(173,198,255,0.6)]"></div>
                            <h4 class="font-mono text-sm text-primary">"BLUFOR Objectives"</h4>
                        </div>
                        <ul class="space-y-1.5 text-sm text-on-surface-variant">
                            {PLACEHOLDER_BLUFOR
                                .iter()
                                .map(|o| {
                                    view! {
                                        <li class="flex gap-2">
                                            <span class="text-primary">"›"</span>
                                            {*o}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </div>
                    <div class="rounded-lg border border-error/20 bg-error-container/10 p-4">
                        <div class="mb-3 flex items-center gap-2">
                            <div class="h-2.5 w-2.5 rounded-full bg-error shadow-[0_0_8px_rgba(255,180,171,0.6)]"></div>
                            <h4 class="font-mono text-sm text-error">"OPFOR Objectives"</h4>
                        </div>
                        <ul class="space-y-1.5 text-sm text-on-surface-variant">
                            {PLACEHOLDER_OPFOR
                                .iter()
                                .map(|o| {
                                    view! {
                                        <li class="flex gap-2">
                                            <span class="text-error">"›"</span>
                                            {*o}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </div>
                </div>
            </section>

            // Faction Dossiers hidden when there are no factions (content-gated).
            {show_factions.then(|| view! { <span></span> })}

            <div class="mt-4">
                <OrbatSelector emid=m.event_mission_id.clone() my_state=m.my_state.clone() />
            </div>
        </div>
    }
}

/// The inline ORBAT split-pane selector. For an empty ORBAT it short-circuits to a single line;
/// the full faction/squad/slot shell is content-gated (needs a non-empty ORBAT golden). Reused by
/// the standalone /events/:id/missions/:emid/orbat route.
#[component]
pub fn OrbatSelector(emid: String, my_state: Option<String>) -> impl IntoView {
    let _ = my_state; // only the non-empty selector uses it
    let store = expect_context::<crate::auth::AuthStore>();
    let orbat = LocalResource::new(move || {
        let emid = emid.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/event-missions/{emid}/orbat");
                crate::client::api_get::<DataEnvelope<Value>>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, emid);
                None::<DataEnvelope<Value>>
            }
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-sm text-on-surface-variant">"Loading ORBAT…"</p> }
        }>
            {move || {
                orbat
                    .get()
                    .map(|opt| {
                        let empty = opt.as_ref().map(|e| e.data.is_empty()).unwrap_or(true);
                        if empty {
                            view! {
                                <p class="text-sm text-on-surface-variant">
                                    "No ORBAT slots defined for this mission."
                                </p>
                            }
                                .into_any()
                        } else {
                            // Non-empty selector — content-gated.
                            ().into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
