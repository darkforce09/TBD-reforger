//! Event Hub (/events/:id) — ported from pages/events.tsx `EventHubPage` + `EventHubView` +
//! `MissionDossier` + the shared `OrbatSelector`. `<AuthGate>` → `useEvent(id)` → `GET /events/:id`
//! → a topo-glass hero (name, T-MINUS countdown, datetime, TS3 + event-bound modpack link) + per-mission
//! dossiers (the mission's briefing, faction dossiers, inline ORBAT selector).
//!
//! T-159.25: the FULL interactive surface is live — my_state badge, faction dossiers
//! (uniform placeholder + real armory), and the complete ORBAT selector
//! (faction tabs → squad list → slot rows) with the five mutations (register / withdraw /
//! reserve / release / assign via member typeahead), toasts carrying the backend `error` string
//! (T-127 U5), and `LocalResource` refetches standing in for the React query invalidations
//! (orbat + the parent event, via `on_change`).
//!
//! One knowing divergence: a hub refetch re-creates this component tree, so faction/squad
//! selection resets to the defaults after a mutation (React's useState survives because the
//! component instance persists). Register clears the selection in React too, so the visible
//! delta is squad-tab focus only.
//!
//! **T-392 — this page no longer invents mission intel.** The React port carried five demo
//! constants that rendered in the same type, in the same cards, as the real armory and ORBAT
//! rows: a maker, a duration, BLUFOR/OPFOR objective lists, a paragraph of briefing lore and a
//! per-faction vehicle roster. See [`briefing_text`] for the one that was a live defect and
//! [`meta_badges`] for the shape the rest were removed into.
#![allow(dead_code)]
use crate::datefmt::{countdown_label, format_local_datetime};
use crate::dto::{DataEnvelope, EventHub, EventMissionDossier, Member, ModpackDto, OrbatSquad};
use crate::nav::{has_min_role_authed, Role};
use crate::ui::{cn, AuthGate, MaterialIcon, DEFAULT_AVATAR};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

// A simple uniform-silhouette SVG so the frame always renders offline (events.tsx).
//
// T-392 KEPT THIS ONE, and deliberately. It is the only survivor of the placeholder family
// because it is *art* standing in for missing art, not a *value* standing in for a missing fact
// — the same line `docs/platform/frontend_data_provenance.md` already drew when it called
// `missions.rs` `thumbnail_url ?? PLACEHOLDER_ART` "a legitimate fallback" while listing this
// page's other five under "Mock values still on a real user's screen". A grey silhouette asserts
// nothing about a faction; "BLUFOR fields 4 BTR-70s" does. The ticket's own list omits it too.
const PLACEHOLDER_UNIFORM: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='80' height='120'><rect width='80' height='120' fill='%23242a3a'/><circle cx='40' cy='38' r='15' fill='%233a4252'/><rect x='18' y='56' width='44' height='56' rx='9' fill='%233a4252'/></svg>";

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

/// T-442 — Hub chip fetch target. Event-bound `modpack_id` wins; `/modpacks/current` only
/// when null/absent/blank. `ById` is resolved via `GET /modpacks` + select (no public
/// GET-by-id route; list is the equivalent).
#[derive(Debug, Clone, PartialEq, Eq)]
enum HubModpackFetch {
    ById(String),
    Current,
}

fn hub_modpack_fetch(modpack_id: Option<&str>) -> HubModpackFetch {
    match modpack_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => HubModpackFetch::ById(id.to_string()),
        None => HubModpackFetch::Current,
    }
}

fn event_hub_view(ev: EventHub, on_change: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let event_modpack_id = ev.modpack_id.clone();
    let modpack = LocalResource::new(move || {
        let event_modpack_id = event_modpack_id.clone();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match hub_modpack_fetch(event_modpack_id.as_deref()) {
                    HubModpackFetch::ById(id) => {
                        match crate::client::api_get::<DataEnvelope<ModpackDto>>(store, "/modpacks")
                            .await
                        {
                            Ok(env) => env.data.into_iter().find(|mp| mp.modpack.id == id),
                            Err(_) => None,
                        }
                    }
                    HubModpackFetch::Current => {
                        crate::client::api_get::<ModpackDto>(store, "/modpacks/current")
                            .await
                            .ok()
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, event_modpack_id);
                None::<ModpackDto>
            }
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

/// The Event Hub's rendering of a mission dossier's briefing — the authored prose, or the
/// explicit empty-state affordance when there is none.
///
/// **T-392 — this used to fall back to three sentences of invented lore.** That was harmless
/// while nothing could author a briefing; T-344/T-345 made briefings authorable end to end and
/// turned it into a live defect, because the fallback cannot tell *"nobody has written one yet"*
/// from *"the author deliberately cleared it"* and answers both with fiction that reads exactly
/// like real operational copy. Clearing a briefing was therefore impossible: `PATCH /missions/:id`
/// binds the field straight from the request (`api/handlers/missions.rs:505`), the read
/// `COALESCE`s NULL to `''`, and `EventMissionDossier` carries
/// `skip_serializing_if = "String::is_empty"` — so a cleared briefing arrives here as an *absent
/// key*, i.e. exactly the state the fallback fired on. Same shape as the T-373 finding.
///
/// **Not back-filled**, for T-373's reason: an absent value must stay distinguishable from a
/// default, and writing one into authored data destroys the author's intent permanently. This
/// only decides what to *show*; nothing here writes.
///
/// The affordance string is `mission_overview.rs:894`'s verbatim — the sibling mission-dossier
/// renderer, and the precedent the ticket named — rather than a new one invented for this page.
/// The `trim()` is `approvals.rs:389`'s ("The author submitted no briefing."), which is the
/// stricter of the two in-repo precedents: a briefing of `"\n\n "` is not authored content, and
/// rendering it verbatim leaves a "Mission Briefing" heading over blank space.
fn briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => "No briefing provided.".to_string(),
    }
}

/// Every meta badge the dossier header can honestly show, in render order.
///
/// **T-392** — this was a `Maker` chip and a `Duration` chip flanking `Terrain`, both hardcoded:
/// two fabricated literals wearing the same chip as the one real field, so every mission on the
/// site was attributed to the same person and declared the same length. `EventMissionDossier` has no
/// maker and no duration on the wire at all (`api/handlers/events.rs:952`; `dto.rs:781`), so
/// unlike the briefing there is nothing an author could ever do to displace them — which is also
/// why they are removed outright instead of given an empty-state affordance. An affordance is a
/// promise that the field is authorable; "Maker: —" on every mission forever would be noise
/// advertising a path that does not exist.
///
/// Returning the list, rather than inlining the one survivor into the `view!`, is what makes
/// *"every badge on this row came out of the dossier"* something a native test can assert — the
/// `view!` itself needs a DOM and cannot be exercised by `cargo test`.
fn meta_badges(m: &EventMissionDossier) -> Vec<(&'static str, String)> {
    vec![("Terrain", terrain_label(&m.terrain))]
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
    let badges = meta_badges(&m);
    let armory = m.armory_by_faction;
    let briefing = briefing_text(m.briefing.as_deref());
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
                        {terrain} " • " {mode} " • " {when}
                    </p>
                    <div class="mt-2 flex flex-wrap gap-2">
                        {badges
                            .into_iter()
                            .map(|(label, value)| meta_badge(label, value))
                            .collect_view()}
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
            // T-392 — the two objective panels that sat here are gone. They listed six
            // hardcoded objectives — a nuke, a VIP convoy, some FOBs — in the same card as the
            // real fill counts and the live ORBAT, so the people about to fly the operation read
            // them as its actual tasking. Nothing on the wire could ever have replaced them:
            // `EventMissionDossier` has no objectives field, and there is no authoring path that
            // reaches this endpoint — the briefing prose and markers T-344/T-345 made authorable
            // land in the mission CRDT document and compile to the mod, while the doc's own
            // `objectivesById` root (`map-engine-core` doc/store.rs:174) is still the closed
            // hydrate→emit loop with no mutator that T-345 found for the markers root. So this
            // is a removal, not an empty state: "No objectives provided." on every mission
            // forever would advertise an authoring path that does not exist.
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

            // T-392 — the "Assets" list that sat here is gone. It printed the same three
            // vehicles with the same counts under EVERY faction card, so BLUFOR's dossier
            // advertised a BTR-70 platoon; and it did so in a mono table one section above the
            // real, API-fed Armory list, which is what made it read as data. There is no
            // vehicles field on `EventMissionDossier` and no writer for one, so — as with the
            // objectives — this is a removal rather than an empty state. The Armory below is
            // the honest per-faction inventory this page actually has.
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
    // T-454 — reactive + authed: browse-mode `has_min_role(None)=>true` must NOT drive
    // reserve / release / assign affordances.
    let is_leader =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Leader));
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
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
                                        is_leader.get(),
                                        is_admin.get(),
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
                                is_leader.get(),
                                is_admin.get(),
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
                                is_leader.get(),
                                is_admin.get(),
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
                                        // T-284: leader/admin who can_manage gets a Clear control
                                        // → DELETE …/slots/:id/assign (same auth as Assign).
                                        if can_manage {
                                            let sid_clear = slot_id_assign.clone();
                                            let emid_clear = emid.clone();
                                            view! {
                                                <span class="flex items-center gap-2 text-on-surface-variant">
                                                    <img
                                                        src=DEFAULT_AVATAR
                                                        alt=""
                                                        class="h-6 w-6 rounded-full"
                                                    />
                                                    {assigned_label}
                                                    <button
                                                        type="button"
                                                        on:click=move |ev| {
                                                            ev.stop_propagation();
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                let toasts = crate::toast::use_toasts();
                                                                let path = format!(
                                                                    "/event-missions/{}/slots/{}/assign",
                                                                    emid_clear,
                                                                    sid_clear
                                                                );
                                                                leptos::task::spawn_local(async move {
                                                                    match crate::client::api_delete(
                                                                        store, &path,
                                                                    )
                                                                    .await
                                                                    {
                                                                        Ok(()) => {
                                                                            toasts.success(
                                                                                "Slot cleared",
                                                                            );
                                                                            changed.run(());
                                                                        }
                                                                        Err(e) => toasts.error(
                                                                            crate::client::api_error_message(
                                                                                &e,
                                                                                "Could not clear slot",
                                                                            ),
                                                                        ),
                                                                    }
                                                                });
                                                            }
                                                            #[cfg(not(target_arch = "wasm32"))]
                                                            let _ = (&emid_clear, &sid_clear);
                                                        }
                                                        class="rounded-lg border border-border-subtle px-3 py-1 text-xs text-error"
                                                    >
                                                        "Clear"
                                                    </button>
                                                </span>
                                            }
                                                .into_any()
                                        } else {
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
                                        }
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

/// T-392 — the durable half. **The reason the placeholder family shipped is that nothing
/// asserted the empty case**: a test that only exercises a populated briefing passes just as
/// happily against a fallback that invents one.
///
/// These are native (`main.rs` compiles `event_hub` unconditionally, unlike the `wasm32`-gated
/// editor modules), so they cannot drive a `view!` — Leptos CSR needs a DOM. That is exactly why
/// the two decisions this ticket changed were lifted into [`briefing_text`] and [`meta_badges`]
/// rather than left inline: a pure function is the part of a render decision `cargo test` can
/// reach. What is left over — sections deleted outright, with no value to return — is guarded by
/// [`tests::no_fabricated_mission_intel_survives_in_this_module`].
#[cfg(test)]
mod tests {
    use super::*;

    /// The event hub as the dev stack actually served it (the same capture `dto.rs`'s
    /// `event_hub` golden round-trips). **Its one mission carries no `briefing` key at all** —
    /// the backend omits the field when the column is empty — so the single recorded real
    /// response is itself a live instance of the defect, not a hypothetical one.
    const EVENT_HUB_GOLDEN: &str = include_str!(
        "../tests/fixtures/api/GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
    );

    fn golden_dossier() -> EventMissionDossier {
        let hub: EventHub = serde_json::from_str(EVENT_HUB_GOLDEN).expect("golden parses");
        hub.missions
            .into_iter()
            .next()
            .expect("golden has a mission")
    }

    /// The state that could not be reached before: an author clears the box, `PATCH` stores
    /// `''`, the wire omits the key, and this arrives as `None`.
    #[test]
    fn a_cleared_briefing_renders_the_empty_state_and_never_the_invented_lore() {
        for cleared in [None, Some(""), Some("   \n\n  ")] {
            let out = briefing_text(cleared);
            assert_eq!(
                out, "No briefing provided.",
                "an authored-empty briefing must render the affordance, not prose ({cleared:?})"
            );
            // The specific claim: not one word of the removed lore, under any name.
            assert!(
                !out.contains("Hostile mechanized"),
                "the placeholder lore came back for {cleared:?}: {out}"
            );
            assert!(
                !out.contains("winter storm") && !out.contains("contested airspace"),
                "the placeholder lore came back for {cleared:?}: {out}"
            );
        }
    }

    /// The golden is the empty case — proof this is the live wire shape, not a synthetic one.
    #[test]
    fn the_recorded_wire_response_hits_the_empty_case() {
        let m = golden_dossier();
        assert!(
            m.briefing.is_none(),
            "fixture drifted: this test is only meaningful while the golden omits `briefing`"
        );
        assert_eq!(
            briefing_text(m.briefing.as_deref()),
            "No briefing provided."
        );
    }

    /// The other half of the contract: a real briefing is still rendered verbatim, newlines and
    /// all (the `<p>` is `whitespace-pre-line`, and T-344 kept paragraph breaks intact through
    /// the document core specifically so they would survive to a reader).
    #[test]
    fn an_authored_briefing_is_rendered_verbatim() {
        let authored = "Hold the ridge.\n\nSecond wave at H+20.";
        assert_eq!(briefing_text(Some(authored)), authored);
        // Leading/trailing space is not evidence of emptiness — only all-whitespace is.
        assert_eq!(briefing_text(Some(" Hold. ")), " Hold. ");
    }

    /// Every badge on the dossier header must come out of the dossier. This is the assertion the
    /// fabricated `Maker`/`Duration` chips could not have survived: it pins the whole list, so
    /// re-adding a hardcoded chip fails here rather than shipping.
    #[test]
    fn meta_badges_are_all_dossier_derived() {
        let m = golden_dossier();
        assert_eq!(meta_badges(&m), vec![("Terrain", "Everon".to_string())]);

        // Change the dossier, and every badge changes with it — nothing is pinned to a literal.
        let mut other = golden_dossier();
        other.terrain = "arland".into();
        assert_eq!(meta_badges(&other), vec![("Terrain", "Arland".to_string())]);
    }

    /// The guard for the sections this ticket deleted outright — the objective panels and the
    /// per-faction vehicle list. They returned no value, so there is no seam to call; the honest
    /// assertion is over the module source itself: nothing here may name the old constants or
    /// carry their copy, whatever a future slice decides to call it.
    ///
    /// Each needle is assembled with `concat!` so this test does not match itself: only the
    /// fragments appear in the source, never the joined string. That constraint binds the prose
    /// too — quoting the deleted copy in a comment anywhere in this module fails this test, which
    /// is the point. If you are here because it went red on a comment you just wrote, paraphrase.
    #[test]
    fn no_fabricated_mission_intel_survives_in_this_module() {
        const SRC: &str = include_str!("event_hub.rs");
        let banned = [
            concat!("PLACEHOLDER_", "MAKER"),
            concat!("PLACEHOLDER_", "DURATION"),
            concat!("PLACEHOLDER_", "BLUFOR"),
            concat!("PLACEHOLDER_", "OPFOR"),
            concat!("PLACEHOLDER_", "LORE"),
            concat!("PLACEHOLDER_", "VEHICLES"),
            // The copy itself, in case it returns under a different name.
            concat!("Hostile mechanized", " elements"),
            concat!("Protect and secure", " the nuke"),
            concat!("Find and detonate", " the nuke"),
            concat!("Escort the VIP", " convoy"),
            concat!("BTR-70", " APC"),
            concat!("Mi-8", " Hip"),
            concat!("90", " MIN"),
        ];
        for needle in banned {
            assert!(
                !SRC.contains(needle),
                "fabricated mission intel is back in event_hub.rs: {needle:?}. \
                 The Event Hub may only render what the dossier serves — see T-392."
            );
        }
    }

    /// Strip `//` / `/* */` so bans cannot false-red on doc comments (T-457).
    fn strip_rust_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut chars = src.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '/' {
                match chars.peek() {
                    Some('/') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '\n' {
                                out.push('\n');
                                break;
                            }
                        }
                        continue;
                    }
                    Some('*') => {
                        chars.next();
                        while let Some(n) = chars.next() {
                            if n == '*' && matches!(chars.peek(), Some('/')) {
                                chars.next();
                                break;
                            }
                        }
                        continue;
                    }
                    _ => {}
                }
            }
            out.push(c);
        }
        out
    }

    fn collapse_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// T-442 Class-R — Hub chip must prefer `event.modpack_id` when set; unconditional-only
    /// `GET /modpacks/current` as the sole fetch is the pre-fix defect.
    #[test]
    fn hub_chip_prefers_event_modpack_id() {
        assert_eq!(
            hub_modpack_fetch(Some("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee")),
            HubModpackFetch::ById("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee".into())
        );
        assert_eq!(hub_modpack_fetch(None), HubModpackFetch::Current);
        assert_eq!(hub_modpack_fetch(Some("")), HubModpackFetch::Current);
        assert_eq!(hub_modpack_fetch(Some("  \t")), HubModpackFetch::Current);

        const SRC: &str = include_str!("event_hub.rs");
        let production = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");
        let code = collapse_ws(&strip_rust_comments(production));
        assert!(
            code.contains("hub_modpack_fetch(event_modpack_id.as_deref())"),
            "Hub chip must route the event's modpack_id through hub_modpack_fetch"
        );
        assert!(
            code.contains("HubModpackFetch::ById") && code.contains("HubModpackFetch::Current"),
            "both ById (event modpack) and Current (fallback) arms must exist in production"
        );
        // Ban pre-T-442 defect: the only modpack URL in the chip path is /modpacks/current.
        // ById uses the list envelope (GET /modpacks) — not a substring of /modpacks/current.
        assert!(
            code.contains("api_get::<DataEnvelope<ModpackDto>>(store, \"/modpacks\")"),
            "event-bound path must hit GET /modpacks (list equivalent of /modpacks/:id)"
        );
        assert!(
            code.contains("\"/modpacks/current\""),
            "null/absent modpack_id must still fall back to /modpacks/current"
        );
    }

    /// T-454 / T-457 Class-R — ORBAT reserve/release/assign must not use browse-mode
    /// `has_min_role(None)=>true`. Binds to live `is_leader`/`is_admin` Memo assignments;
    /// bans free `has_min_role(` in production.
    #[test]
    fn orbat_affordances_use_authed_reactive_role() {
        const SRC: &str = include_str!("event_hub.rs");
        let production = SRC
            .split("mod tests {")
            .next()
            .expect("tests module marker");
        let code = collapse_ws(&strip_rust_comments(production));
        assert!(
            code.contains(
                "let is_leader = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Leader))"
            ),
            "is_leader must be the Memo that re-reads AuthStore.user via has_min_role_authed \
             (dead Memo + browse-mode has_min_role is a fail)"
        );
        assert!(
            code.contains(
                "let is_admin = Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin))"
            ),
            "is_admin must be the Memo that re-reads AuthStore.user via has_min_role_authed \
             (dead Memo + browse-mode has_min_role is a fail)"
        );
        let masked = code.replace("has_min_role_authed", "HAS_MIN_ROLE_AUTHED");
        assert!(
            !masked.contains("has_min_role("),
            "production must not call browse-mode has_min_role( — use has_min_role_authed only"
        );
        let one_shot_leader = format!("store.has_min_role({}::Leader)", "Role");
        let one_shot_admin = format!("store.has_min_role({}::Admin)", "Role");
        assert!(
            !code.contains(&one_shot_leader) && !code.contains(&one_shot_admin),
            "one-shot store.has_min_role freezes pre-bootstrap None as leader/admin"
        );
    }
}
