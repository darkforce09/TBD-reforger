//! Event Hub (/events/:id) — ported from pages/events.tsx `EventHubPage` + `EventHubView` +
//! `MissionDossier` + the shared `OrbatSelector`. `<AuthGate>` → `useEvent(id)` → `GET /events/:id`
//! → a topo-glass hero (name, T-MINUS countdown, datetime, TS3 + current-modpack link) + per-mission
//! dossiers (briefing lore, BLUFOR/OPFOR objectives, faction dossiers, inline ORBAT selector).
//!
//! T-159.25: the FULL interactive surface is live — my_state badge, faction dossiers
//! (uniform/vehicle placeholders + real armory), and the complete ORBAT selector
//! (faction tabs → squad list → slot rows) with the five mutations (register / withdraw /
//! reserve / release / assign via member typeahead), toasts carrying the backend `error` string
//! (T-127 U5), and `LocalResource` refetches standing in for the React query invalidations
//! (orbat + the parent event, via `on_change`).
//!
//! One knowing divergence: a hub refetch re-creates this component tree, so faction/squad
//! selection resets to the defaults after a mutation (React's useState survives because the
//! component instance persists). Register clears the selection in React too, so the visible
//! delta is squad-tab focus only.
#![allow(dead_code)]
use crate::datefmt::{countdown_label, format_local_datetime};
use crate::dto::{DataEnvelope, EventHub, EventMissionDossier, Member, ModpackDto, OrbatSquad};
use crate::nav::Role;
use crate::ui::{cn, AuthGate, MaterialIcon, DEFAULT_AVATAR};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

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
// A simple uniform-silhouette SVG so the frame always renders offline (events.tsx).
const PLACEHOLDER_UNIFORM: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='80' height='120'><rect width='80' height='120' fill='%23242a3a'/><circle cx='40' cy='38' r='15' fill='%233a4252'/><rect x='18' y='56' width='44' height='56' rx='9' fill='%233a4252'/></svg>";
const PLACEHOLDER_VEHICLES: [(&str, u32); 3] = [("BTR-70 APC", 4), ("UAZ-469", 6), ("Mi-8 Hip", 2)];

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

/// Faction render order (events.tsx `FACTION_SIDE_RANK`): BLUFOR → OPFOR → INDFOR → alphabetical.
/// The React regexes use `\b` word boundaries; here the name is tokenized on non-alphanumerics and
/// single-word markers match whole tokens (multi-word markers match as substrings) — same verdicts
/// on every real faction name the platform has seen.
fn faction_side(name: &str) -> u8 {
    let lower = name.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has = |t: &str| tokens.contains(&t);
    if [
        "blufor", "bluefor", "nato", "usmc", "usa", "west", "western",
    ]
    .iter()
    .any(|t| has(t))
        || lower.contains("us army")
    {
        0
    } else if [
        "opfor", "ussr", "soviet", "russia", "csat", "east", "eastern",
    ]
    .iter()
    .any(|t| has(t))
    {
        1
    } else if [
        "indfor",
        "independent",
        "guer",
        "guerrilla",
        "resistance",
        "civ",
        "civilian",
    ]
    .iter()
    .any(|t| has(t))
    {
        2
    } else {
        99
    }
}

fn sort_factions(mut factions: Vec<String>) -> Vec<String> {
    factions.sort_by(|a, b| faction_side(a).cmp(&faction_side(b)).then_with(|| a.cmp(b)));
    factions
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
    // The React query invalidations on register/withdraw hit ['events'] → the hub re-renders with
    // the new my_state; here the selector calls back into an event refetch.
    let on_change = Callback::new(move |()| event.refetch());
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                event
                    .get()
                    .map(|opt| match opt {
                        Some(ev) => hub_shell(ev, on_change).into_any(),
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

fn hub_shell(ev: EventHub, on_change: Callback<()>) -> impl IntoView {
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
                    {event_hub_view(ev, on_change)}
                </div>
            </div>
        </div>
    }
}

fn event_hub_view(ev: EventHub, on_change: Callback<()>) -> impl IntoView {
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
                        .map(|(i, m)| mission_dossier(i + 1, m, on_change))
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

fn mission_dossier(index: usize, m: EventMissionDossier, on_change: Callback<()>) -> impl IntoView {
    // OpsCard(className="bg-surface-container-high"): twMerge drops the default bg-surface-container
    // vs the override bg-surface-container-high (same bg-color group). Inlined resolved (our cn()
    // doesn't group these two hyphenated bg names — a real cn() gap tracked to the tw_merge task).
    let card = "relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 border border-border-subtle bg-surface-container-high";
    // Faction Dossiers render for every listed faction (armory keyed by faction fills the cards).
    let faction_list = sort_factions(if m.factions.is_empty() {
        m.armory_by_faction
            .iter()
            .map(|f| f.faction.clone())
            .collect()
    } else {
        m.factions.clone()
    });
    let armory = m.armory_by_faction;
    let briefing = m
        .briefing
        .clone()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| PLACEHOLDER_LORE.into());
    let terrain = terrain_label(&m.terrain);
    let mode = game_mode_label(&m.game_mode).to_string();
    let when = format_local_datetime(&m.start_time);
    let my_state = m.my_state.clone();
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
                    {my_state
                        .clone()
                        .map(|s| {
                            view! {
                                <span class="rounded bg-success-muted px-2 py-0.5 text-xs font-semibold text-success">
                                    {s.to_uppercase()}
                                </span>
                            }
                        })}
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

            // Faction Dossiers — uniforms/assets placeholders + real armory per faction (T-159.25).
            {(!faction_list.is_empty())
                .then(|| {
                    view! {
                        <section class="mt-4">
                            <h4 class="mb-2 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                                "Faction Dossiers"
                            </h4>
                            <div class="grid gap-3 md:grid-cols-2">
                                {faction_list
                                    .iter()
                                    .map(|faction| {
                                        let items = armory
                                            .iter()
                                            .find(|f| &f.faction == faction)
                                            .map(|f| f.items.clone())
                                            .unwrap_or_default();
                                        faction_dossier_card(faction.clone(), items)
                                    })
                                    .collect_view()}
                            </div>
                        </section>
                    }
                })}

            <div class="mt-4">
                <OrbatSelector
                    emid=m.event_mission_id.clone()
                    my_state=my_state
                    on_change=on_change
                />
            </div>
        </div>
    }
}

fn faction_dossier_card(faction: String, items: Vec<crate::dto::ArmoryItem>) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border-subtle bg-surface-container p-3">
            <h5 class="mb-3 text-sm font-semibold">{faction}</h5>

            <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                "Uniforms"
            </span>
            <div class="mb-3 flex gap-2">
                {(0..3)
                    .map(|_| {
                        view! {
                            <img
                                src=PLACEHOLDER_UNIFORM
                                alt=""
                                class="aspect-[2/3] w-12 rounded-md border border-white/10 object-cover"
                            />
                        }
                    })
                    .collect_view()}
            </div>

            <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                "Assets"
            </span>
            <ul class="mb-3 overflow-hidden rounded-md bg-surface-container/50 font-mono text-xs">
                {PLACEHOLDER_VEHICLES
                    .iter()
                    .map(|(name, qty)| {
                        view! {
                            <li class="flex items-center justify-between border-b border-white/5 px-2 py-1.5 last:border-0">
                                <span class="text-on-surface-variant">{*name}</span>
                                <span class="text-tactical-yellow">"x" {*qty}</span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>

            {(!items.is_empty())
                .then(|| {
                    view! {
                        <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                            "Armory"
                        </span>
                        <ul class="space-y-1 text-sm">
                            {items
                                .into_iter()
                                .map(|it| {
                                    let qty = it
                                        .quantity
                                        .map(|q| format!("x{q}"))
                                        .unwrap_or_else(|| "∞".to_string());
                                    view! {
                                        <li class="flex justify-between text-on-surface-variant">
                                            <span>{it.item_name}</span>
                                            <span>{qty}</span>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                })}
        </div>
    }
}

/* ───────────────────────── ORBAT selector (full port, T-159.25) ───────────────────────── */

/// Busy flags per mutation (React's per-hook `isPending`).
#[derive(Clone, Copy)]
struct OrbatBusy {
    register: RwSignal<bool>,
    withdraw: RwSignal<bool>,
    reserve: RwSignal<bool>,
    release: RwSignal<bool>,
}

/// The inline ORBAT split-pane selector — the events.tsx `OrbatSelector` port: faction tabs →
/// squad list → slot rows, with register / withdraw / reserve / release / assign live against the
/// backend. Reused by the standalone /events/:id/missions/:emid/orbat route.
#[component]
pub fn OrbatSelector(
    emid: String,
    my_state: Option<String>,
    #[prop(optional)] on_change: Option<Callback<()>>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let emid_res = emid.clone();
    let orbat = LocalResource::new(move || {
        let emid = emid_res.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/event-missions/{emid}/orbat");
                crate::client::api_get::<DataEnvelope<OrbatSquad>>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, emid);
                None::<DataEnvelope<OrbatSquad>>
            }
        }
    });
    // Selection state (React useState). Faction/squad hold Options resolved against the live list.
    let faction_sel = RwSignal::new(None::<String>);
    let squad_sel = RwSignal::new(None::<String>);
    let selected_slot = RwSignal::new(None::<String>);
    let assigning = RwSignal::new(None::<String>);
    let busy = OrbatBusy {
        register: RwSignal::new(false),
        withdraw: RwSignal::new(false),
        reserve: RwSignal::new(false),
        release: RwSignal::new(false),
    };
    // A mutation refetches the ORBAT here and bubbles to the hub (events/dashboard invalidation).
    let changed = Callback::new(move |()| {
        orbat.refetch();
        if let Some(cb) = on_change {
            cb.run(());
        }
    });

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-sm text-on-surface-variant">"Loading ORBAT…"</p> }
        }>
            {move || {
                let emid = emid.clone();
                let my_state = my_state.clone();
                orbat
                    .get()
                    .map(move |opt| {
                        let squads = opt.map(|e| e.data).unwrap_or_default();
                        if squads.is_empty() {
                            view! {
                                <p class="text-sm text-on-surface-variant">
                                    "No ORBAT slots defined for this mission."
                                </p>
                            }
                                .into_any()
                        } else {
                            selector_shell(
                                    emid.clone(),
                                    my_state.clone(),
                                    squads,
                                    faction_sel,
                                    squad_sel,
                                    selected_slot,
                                    assigning,
                                    busy,
                                    changed,
                                )
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The non-empty selector body. Split from the component so the Suspense closure stays readable.
#[allow(clippy::too_many_arguments)]
fn selector_shell(
    emid: String,
    my_state: Option<String>,
    squads: Vec<OrbatSquad>,
    faction_sel: RwSignal<Option<String>>,
    squad_sel: RwSignal<Option<String>>,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    busy: OrbatBusy,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let factions = sort_factions(
        squads
            .iter()
            .map(|s| s.faction.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
    );
    let is_leader = store.has_min_role(Role::Leader);
    let is_admin = store.has_min_role(Role::Admin);
    // Non-Copy captures ride StoredValues so every closure below (used repeatedly inside reactive
    // renders) stays Copy.
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));
    let my_state = StoredValue::new(my_state);

    let factions_for_tabs = factions.clone();
    let squads_sv = StoredValue::new(squads);
    let factions_sv = StoredValue::new(factions);

    // Resolved active faction/squad (React: `faction ?? factions[0]`, `squadKey find ?? [0]`).
    let active = move || {
        let factions = factions_sv.get_value();
        let af = faction_sel
            .get()
            .filter(|f| factions.contains(f))
            .or_else(|| factions.first().cloned());
        let fsquads: Vec<OrbatSquad> = squads_sv
            .get_value()
            .into_iter()
            .filter(|s| Some(&s.faction) == af.as_ref())
            .collect();
        let asq = squad_sel
            .get()
            .and_then(|k| fsquads.iter().find(|s| s.squad == k).cloned())
            .or_else(|| fsquads.first().cloned());
        (af, fsquads, asq)
    };

    let pick_squad = move |squad: String| {
        squad_sel.set(Some(squad));
        selected_slot.set(None);
        assigning.set(None);
    };

    // register (POST …/register {slot_id}) — useRegisterMission port. The emid rides a
    // StoredValue so the handler is Copy (it is used inside a reactive footer closure).
    let emid_reg = StoredValue::new(emid.clone());
    let on_register = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(slot) = selected_slot.get_untracked() else {
                return;
            };
            if busy.register.get_untracked() {
                return;
            }
            busy.register.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/event-missions/{}/register", emid_reg.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(
                    store,
                    &path,
                    serde_json::json!({ "slot_id": slot }),
                )
                .await
                {
                    Ok(()) => {
                        toasts.success("Registered for deployment");
                        selected_slot.set(None);
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not claim that slot",
                    )),
                }
                busy.register.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = emid_reg;
    };

    // withdraw (DELETE …/register) — useWithdrawMission port.
    let emid_wd = emid.clone();
    let on_withdraw = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.withdraw.get_untracked() {
                return;
            }
            busy.withdraw.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/event-missions/{emid_wd}/register");
            leptos::task::spawn_local(async move {
                match crate::client::api_delete(store, &path).await {
                    Ok(()) => {
                        toasts.success("Withdrawn from mission");
                        changed.run(());
                    }
                    Err(_) => toasts.error("Could not withdraw"),
                }
                busy.withdraw.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &emid_wd;
    };

    let emid_rsv = emid.clone();
    let emid_rel = emid.clone();
    let emid_assign = emid.clone();

    view! {
        <div class="grid overflow-hidden rounded-xl border border-border-subtle md:grid-cols-[240px_1fr]">
            // Left: navigation sidebar
            <aside class="border-b border-border-subtle bg-surface-container p-4 md:border-b-0 md:border-r">
                {(factions_for_tabs.len() > 1)
                    .then(|| {
                        let tabs = factions_for_tabs.clone();
                        view! {
                            <div class="mb-4 flex rounded-lg bg-surface p-1">
                                {tabs
                                    .into_iter()
                                    .map(|f| {
                                        let f_click = f.clone();
                                        let f_active = f.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    faction_sel.set(Some(f_click.clone()));
                                                    squad_sel.set(None);
                                                    selected_slot.set(None);
                                                    assigning.set(None);
                                                }
                                                class=move || {
                                                    let (af, _, _) = active();
                                                    cn(
                                                        &[
                                                            "flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors",
                                                            if af.as_deref() == Some(f_active.as_str()) {
                                                                "bg-primary text-on-primary"
                                                            } else {
                                                                "text-on-surface-variant"
                                                            },
                                                        ],
                                                    )
                                                }
                                            >
                                                {f.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                    })}
                <ul class="space-y-1">
                    {move || {
                        let (_, fsquads, asq) = active();
                        fsquads
                            .into_iter()
                            .map(|s| {
                                let is_active = asq.as_ref().map(|a| a.squad == s.squad).unwrap_or(false);
                                let squad_name = s.squad.clone();
                                let full = s.filled >= s.total;
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            on:click=move |_| pick_squad(squad_name.clone())
                                            class=cn(
                                                &[
                                                    "flex w-full items-center justify-between rounded-lg px-3 py-2 text-left text-sm transition-colors",
                                                    if is_active {
                                                        "bg-primary/10 text-on-surface"
                                                    } else {
                                                        "text-on-surface-variant hover:bg-surface-container-high"
                                                    },
                                                ],
                                            )
                                        >
                                            <span class="flex items-center gap-1.5">
                                                {s.reserved_by
                                                    .is_some()
                                                    .then(|| {
                                                        view! {
                                                            <MaterialIcon
                                                                name="lock"
                                                                class="text-sm text-on-surface-variant"
                                                            />
                                                        }
                                                    })}
                                                <span class="font-medium text-on-surface">
                                                    {s.squad.clone()}
                                                </span>
                                                {s.callsign
                                                    .clone()
                                                    .filter(|c| !c.is_empty())
                                                    .map(|c| view! { <span class="ml-1 text-xs">{c}</span> })}
                                            </span>
                                            <span class=cn(
                                                &[
                                                    "font-mono text-xs",
                                                    if full { "text-error" } else { "text-on-surface-variant" },
                                                ],
                                            )>{s.filled} "/" {s.total}</span>
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ul>
            </aside>

            // Right: slot detail pane
            <section class="flex min-h-[18rem] flex-col bg-surface-container-high">
                <div class="flex-1 p-4">
                    {move || {
                        let (_, _, asq) = active();
                        match asq {
                            Some(sq) => {
                                squad_pane(
                                        emid_assign.clone(),
                                        sq,
                                        me.get_value(),
                                        is_leader,
                                        is_admin,
                                        my_state.get_value(),
                                        selected_slot,
                                        assigning,
                                        busy,
                                        changed,
                                        emid_rsv.clone(),
                                        emid_rel.clone(),
                                    )
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <p class="text-on-surface-variant">
                                        "Select a squad to view its slots."
                                    </p>
                                }
                                    .into_any()
                            }
                        }
                    }}
                </div>

                // Footer action bar
                <div class="flex items-center justify-between gap-3 border-t border-border-subtle bg-surface-container p-4">
                    <div class="text-sm text-on-surface-variant">
                        {move || {
                            let (_, _, asq) = active();
                            footer_message(
                                my_state.get_value(),
                                asq,
                                me.get_value(),
                                is_leader,
                                is_admin,
                            )
                        }}
                    </div>
                    <div class="flex gap-2">
                        {my_state
                            .get_value()
                            .map(|_| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=on_withdraw
                                        prop:disabled=move || busy.withdraw.get()
                                        class="rounded-lg border border-error/50 px-4 py-2 text-sm text-error disabled:opacity-50"
                                    >
                                        "Withdraw"
                                    </button>
                                }
                            })}
                        {move || {
                            let (_, _, asq) = active();
                            let (_, _, self_register) = squad_flags(
                                asq.as_ref(),
                                me.get_value(),
                                is_leader,
                                is_admin,
                            );
                            (my_state.get_value().is_none() && self_register)
                                .then(|| {
                                    view! {
                                        <button
                                            type="button"
                                            on:click=on_register
                                            prop:disabled=move || {
                                                selected_slot.get().is_none() || busy.register.get()
                                            }
                                            class="rounded-lg bg-primary px-6 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                                        >
                                            "Register for Deployment"
                                        </button>
                                    }
                                })
                        }}
                    </div>
                </div>
            </section>
        </div>
    }
}

/// (can_manage, locked_for_me, self_register) — the events.tsx reservation flags.
fn squad_flags(
    sq: Option<&OrbatSquad>,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
) -> (bool, bool, bool) {
    let _ = is_leader;
    let Some(sq) = sq else {
        return (false, false, false);
    };
    let reserved_by = sq.reserved_by.clone().filter(|r| !r.is_empty());
    let i_am_reserver = reserved_by.is_some() && reserved_by == me;
    let can_manage = is_admin || i_am_reserver;
    let locked_for_me = reserved_by.is_some() && !can_manage;
    let self_register = !can_manage && !locked_for_me;
    (can_manage, locked_for_me, self_register)
}

fn footer_message(
    my_state: Option<String>,
    asq: Option<OrbatSquad>,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
) -> String {
    if let Some(s) = my_state {
        return format!("You are {s} for this mission.");
    }
    let (can_manage, locked_for_me, _) = squad_flags(asq.as_ref(), me, is_leader, is_admin);
    if locked_for_me {
        "This squad is reserved by a leader.".to_string()
    } else if can_manage {
        "Assign members to fill this squad.".to_string()
    } else {
        "Select an open slot to deploy.".to_string()
    }
}

/// The active squad's header (reserve/release) + slot list (+ per-slot assign picker).
#[allow(clippy::too_many_arguments)]
fn squad_pane(
    emid: String,
    sq: OrbatSquad,
    me: Option<String>,
    is_leader: bool,
    is_admin: bool,
    my_state: Option<String>,
    selected_slot: RwSignal<Option<String>>,
    assigning: RwSignal<Option<String>>,
    busy: OrbatBusy,
    changed: Callback<()>,
    emid_rsv: String,
    emid_rel: String,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let _ = my_state;
    // The store/callback feed only the wasm-gated mutation closures.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &changed);
    let (can_manage, locked_for_me, self_register) =
        squad_flags(Some(&sq), me.clone(), is_leader, is_admin);
    let reserved_by = sq.reserved_by.clone().filter(|r| !r.is_empty());
    let i_am_reserver = reserved_by.is_some() && reserved_by == me;
    let squad_name = sq.squad.clone();
    let callsign = sq.callsign.clone().filter(|c| !c.is_empty());

    // reserve (POST …/squads/reserve {squad}) — useReserveSquad port.
    let squad_rsv = squad_name.clone();
    let on_reserve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.reserve.get_untracked() {
                return;
            }
            busy.reserve.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/event-missions/{emid_rsv}/squads/reserve");
            let squad = squad_rsv.clone();
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(
                    store,
                    &path,
                    serde_json::json!({ "squad": squad }),
                )
                .await
                {
                    Ok(()) => {
                        toasts.success(format!("Reserved {squad}"));
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not reserve squad",
                    )),
                }
                busy.reserve.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&emid_rsv, &squad_rsv);
    };

    // release (POST …/squads/release {squad}) — useReleaseSquad port.
    let squad_rel = squad_name.clone();
    let on_release = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.release.get_untracked() {
                return;
            }
            busy.release.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/event-missions/{emid_rel}/squads/release");
            let squad = squad_rel.clone();
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(
                    store,
                    &path,
                    serde_json::json!({ "squad": squad }),
                )
                .await
                {
                    Ok(()) => {
                        toasts.success("Squad released");
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not release squad",
                    )),
                }
                busy.release.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&emid_rel, &squad_rel);
    };

    view! {
        <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
            <h4 class="font-semibold">
                {squad_name.clone()}
                {callsign
                    .map(|c| {
                        view! {
                            <span class="text-sm font-normal text-on-surface-variant">
                                " | "
                                {c}
                            </span>
                        }
                    })}
            </h4>
            <div class="flex items-center gap-2">
                {if reserved_by.is_some() {
                    let holder = sq
                        .reserved_by_name
                        .clone()
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| "a leader".into());
                    view! {
                        <span class="flex items-center gap-1 rounded bg-surface-container-highest px-2 py-0.5 text-xs text-on-surface-variant">
                            <MaterialIcon name="lock" class="text-sm" />
                            "Reserved by "
                            {holder}
                        </span>
                        {(i_am_reserver || is_admin)
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=on_release
                                        prop:disabled=move || busy.release.get()
                                        class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-on-surface-variant disabled:opacity-50"
                                    >
                                        "Release"
                                    </button>
                                }
                            })}
                    }
                        .into_any()
                } else if is_leader {
                    view! {
                        <button
                            type="button"
                            on:click=on_reserve
                            prop:disabled=move || busy.reserve.get()
                            class="flex items-center gap-1 rounded-lg bg-primary px-3 py-1 text-xs font-medium text-on-primary disabled:opacity-50"
                        >
                            <MaterialIcon name="lock" class="text-sm" />
                            " Reserve Squad"
                        </button>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
            </div>
        </div>

        <ul class="overflow-hidden rounded-lg border border-border-subtle divide-y divide-border-subtle">
            {sq
                .slots
                .iter()
                .map(|slot| {
                    let taken = slot
                        .assigned_to
                        .clone()
                        .filter(|a| !a.is_empty())
                        .is_some();
                    let clickable = self_register && !taken;
                    let slot_id = slot.id.clone();
                    let slot_id_sel = slot.id.clone();
                    let slot_id_assign = slot.id.clone();
                    let slot_id_picker = slot.id.clone();
                    let assigned_label = slot
                        .assigned_name
                        .clone()
                        .filter(|n| !n.is_empty())
                        .or_else(|| slot.assigned_to.clone())
                        .unwrap_or_default();
                    let row_class = move || {
                        let selected = selected_slot.get().as_deref() == Some(slot_id_sel.as_str());
                        cn(
                            &[
                                "flex items-center justify-between gap-3 px-4 py-2 text-sm",
                                if clickable { "cursor-pointer" } else { "" },
                                if selected { "bg-primary/10" } else { "" },
                                if clickable && !selected { "hover:bg-surface-container" } else { "" },
                            ],
                        )
                    };
                    let on_row = move |_| {
                        if !clickable {
                            return;
                        }
                        let cur = selected_slot.get_untracked();
                        selected_slot
                            .set(
                                if cur.as_deref() == Some(slot_id.as_str()) {
                                    None
                                } else {
                                    Some(slot_id.clone())
                                },
                            );
                    };
                    view! {
                        <li>
                            <div on:click=on_row class=row_class>
                                <span class="flex items-center gap-2">
                                    <span class="text-on-surface-variant tabular-nums">
                                        {slot.number}
                                        ":"
                                    </span>
                                    <span class="font-medium">{slot.role.clone()}</span>
                                    {slot
                                        .loadout
                                        .clone()
                                        .filter(|l| !l.is_empty())
                                        .map(|l| {
                                            view! {
                                                <span class="text-on-surface-variant">"(" {l} ")"</span>
                                            }
                                        })}
                                    {slot
                                        .tag
                                        .clone()
                                        .filter(|t| !t.is_empty())
                                        .map(|t| {
                                            view! {
                                                <span class="rounded bg-surface-container-highest px-1.5 py-0.5 text-[10px] font-semibold text-on-surface-variant">
                                                    {t}
                                                </span>
                                            }
                                        })}
                                </span>
                                <span class="shrink-0">
                                    {if taken {
                                        view! {
                                            <span class="flex items-center gap-2 text-on-surface-variant">
                                                <img
                                                    src=DEFAULT_AVATAR
                                                    alt=""
                                                    class="h-6 w-6 rounded-full"
                                                />
                                                {assigned_label}
                                            </span>
                                        }
                                            .into_any()
                                    } else if can_manage {
                                        let sid = slot_id_assign.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let cur = assigning.get_untracked();
                                                    assigning
                                                        .set(
                                                            if cur.as_deref() == Some(sid.as_str()) {
                                                                None
                                                            } else {
                                                                Some(sid.clone())
                                                            },
                                                        );
                                                }
                                                class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-primary"
                                            >
                                                {
                                                    let sid = slot_id_assign.clone();
                                                    move || {
                                                        if assigning.get().as_deref() == Some(sid.as_str()) {
                                                            "Cancel"
                                                        } else {
                                                            "Assign"
                                                        }
                                                    }
                                                }
                                            </button>
                                        }
                                            .into_any()
                                    } else if locked_for_me {
                                        view! {
                                            <span class="text-xs text-on-surface-variant">"Reserved"</span>
                                        }
                                            .into_any()
                                    } else {
                                        let sid = slot.id.clone();
                                        view! {
                                            <span class=move || {
                                                let selected = selected_slot.get().as_deref()
                                                    == Some(sid.as_str());
                                                cn(
                                                    &[
                                                        "flex items-center gap-2",
                                                        if selected { "text-primary" } else { "text-success" },
                                                    ],
                                                )
                                            }>
                                                <span class="h-2 w-2 rounded-full bg-current"></span>
                                                {
                                                    let sid = slot.id.clone();
                                                    move || {
                                                        if selected_slot.get().as_deref() == Some(sid.as_str()) {
                                                            "Selected"
                                                        } else {
                                                            "Available"
                                                        }
                                                    }
                                                }
                                            </span>
                                        }
                                            .into_any()
                                    }}
                                </span>
                            </div>
                            {(can_manage && !taken)
                                .then(|| {
                                    let sid = slot_id_picker.clone();
                                    let emid = emid.clone();
                                    move || {
                                        (assigning.get().as_deref() == Some(sid.as_str()))
                                            .then(|| {
                                                view! {
                                                    <AssignPicker
                                                        emid=emid.clone()
                                                        slot_id=sid.clone()
                                                        assigning=assigning
                                                        changed=changed
                                                    />
                                                }
                                            })
                                    }
                                })}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// The leader's inline member typeahead for filling a reserved squad's slot (events.tsx
/// `AssignPicker`): `GET /members?q=` per keystroke → click a member → `PUT
/// /event-missions/:emid/slots/:slotId/assign {discord_id}`.
#[component]
fn AssignPicker(
    emid: String,
    slot_id: String,
    assigning: RwSignal<Option<String>>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // StoredValues so `on_pick` is Copy (it's used inside the reactive members-list closure).
    let emid = StoredValue::new(emid);
    let slot_id = StoredValue::new(slot_id);
    let q = RwSignal::new(String::new());
    let members = LocalResource::new(move || {
        let q = q.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!(
                    "/members?q={}",
                    js_sys::encode_uri_component(&q)
                        .as_string()
                        .unwrap_or_default()
                );
                crate::client::api_get::<DataEnvelope<Member>>(store, &path)
                    .await
                    .ok()
                    .map(|e| e.data)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, q);
                None::<Vec<Member>>
            }
        }
    });
    let assign_busy = RwSignal::new(false);
    // All of these feed only the wasm-gated assign PUT.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (assign_busy, store, emid, slot_id, assigning, &changed);
    let on_pick = move |m: Member| {
        #[cfg(target_arch = "wasm32")]
        {
            if assign_busy.get_untracked() {
                return;
            }
            assign_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!(
                "/event-missions/{}/slots/{}/assign",
                emid.get_value(),
                slot_id.get_value()
            );
            leptos::task::spawn_local(async move {
                match crate::client::api_put::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "discord_id": m.discord_id }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success(format!("Assigned {}", m.username));
                        assigning.set(None);
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not assign member",
                    )),
                }
                assign_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = m;
    };

    view! {
        <div class="border-t border-border-subtle bg-surface p-2">
            <input
                autofocus
                prop:value=move || q.get()
                on:input=move |ev| q.set(event_target_value(&ev))
                placeholder="Search members…"
                class="w-full rounded-lg border border-border-subtle bg-surface-container px-3 py-1.5 text-sm"
            />
            <ul class="mt-2 max-h-40 overflow-y-auto">
                {move || {
                    members
                        .get()
                        .flatten()
                        .map(|list| {
                            if list.is_empty() {
                                view! {
                                    <li class="px-2 py-1 text-xs text-on-surface-variant">
                                        "No matching members."
                                    </li>
                                }
                                    .into_any()
                            } else {
                                list.into_iter()
                                    .map(|m| {
                                        let avatar = m
                                            .avatar_url
                                            .clone()
                                            .filter(|a| !a.is_empty())
                                            .unwrap_or_else(|| DEFAULT_AVATAR.to_string());
                                        let username = m.username.clone();
                                        let pick = m.clone();
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    on:click=move |_| on_pick(pick.clone())
                                                    class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-surface-container-high"
                                                >
                                                    <img src=avatar alt="" class="h-5 w-5 rounded-full" />
                                                    {username}
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        })
                }}
            </ul>
        </div>
    }
}
