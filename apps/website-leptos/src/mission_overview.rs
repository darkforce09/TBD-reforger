//! Mission Overview (/missions/:id) — ported from pages/missions.tsx `MissionOverviewPage` +
//! `MissionDossierBody`. `<AuthGate>` → `useMission(id)` → `GET /missions/:id` → a `QueryState`
//! (Loading… / error / content) wrapping a PageHeader + a glass OpsCard dossier (badges, briefing,
//! a Weather/Time/Max-Players/Status detail grid, and — when present — the faction armory).
//!
//! **Gate scope:** the seeded golden `512d8658-…` (a fresh mission: auto-version v0.1.0, empty
//! armory, no briefing) → header + 3 badges + "No briefing provided." + the 4 details; the armory
//! section is hidden (no factions). The armory tabs/items are content-gated (need a golden with
//! loadouts). DTO round-trip is proven by the R-api gate (dto.rs `mission_detail`).
#![allow(dead_code)]
use crate::dto::MissionDetail;
use crate::ui::AuthGate;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde_json::Value;

// Badge variants (badge.tsx cva): the base `text-label-sm` is twMerge-dropped against the trailing
// text-{color}, same as the wiki neutral badge.
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
const BADGE_TERTIARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tertiary/30 bg-tertiary/10 text-tertiary";

/// `gameModeLabel` (lib/format.ts).
fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// `terrainLabel` (lib/format.ts) — capitalize first char; "—" when empty.
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
pub fn MissionOverviewPage() -> impl IntoView {
    view! {
        <AuthGate>
            <MissionOverviewInner />
        </AuthGate>
    }
}

#[component]
fn MissionOverviewInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let params = use_params_map();
    let mission = LocalResource::new(move || {
        let id = params
            .read()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<MissionDetail>
            }
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                mission
                    .get()
                    .map(|opt| match opt {
                        Some(m) => body(m).into_any(),
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

fn body(m: MissionDetail) -> impl IntoView {
    let version_suffix = m
        .current_version
        .as_ref()
        .map(|v| format!(" — v{}", v.semver))
        .unwrap_or_default();
    let subtitle = format!(
        "by {} — Terrain: {}{}",
        m.author_name,
        terrain_label(&m.terrain),
        version_suffix
    );
    view! {
        <div class="mx-auto w-full max-w-3xl">
            <header class="mb-8">
                <h1 class="mb-2 text-3xl font-bold text-on-surface">{m.title.clone()}</h1>
                <p class="max-w-3xl text-on-surface-variant">{subtitle}</p>
            </header>
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass">
                {dossier_body(&m)}
            </div>
        </div>
    }
}

/// Shared dossier content — used here and by the library slide-over (missions.rs), like React's
/// `MissionDossierBody`. T-159.25 adds the interactive Armory faction tabs (rows stay `Value`-read
/// so the R-api golden shape is untouched).
pub fn dossier_body(m: &MissionDetail) -> impl IntoView {
    let briefing = m
        .briefing
        .clone()
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "No briefing provided.".into());
    let v_badge = m
        .current_version
        .as_ref()
        .map(|v| view! { <span class=BADGE_TERTIARY>"v"{v.semver.clone()}</span> });
    // Armory faction tabs (React `factions = [...new Set(mission.armory.map(a => a.faction))]`).
    let armory = m.armory.clone();
    let factions: Vec<String> = {
        let mut seen = Vec::new();
        for a in &armory {
            if let Some(f) = a.get("faction").and_then(|v| v.as_str()) {
                if !seen.iter().any(|s: &String| s == f) {
                    seen.push(f.to_string());
                }
            }
        }
        seen
    };
    let faction_sel = RwSignal::new(None::<String>);
    // StoredValue keeps the resolver closure Copy (used by both the tabs and the rows renders).
    let default_faction = StoredValue::new(factions.first().cloned());
    let factions_for_tabs = factions.clone();
    let active_faction = move || faction_sel.get().or_else(|| default_faction.get_value());
    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap gap-2">
                <span class=BADGE_PRIMARY>{game_mode_label(&m.game_mode).to_string()}</span>
                <span class=BADGE_NEUTRAL>{terrain_label(&m.terrain)}</span>
                {v_badge}
            </div>

            <section>
                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                    "Tactical Briefing"
                </h3>
                <p class="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
                    {briefing}
                </p>
            </section>

            <dl class="grid grid-cols-1 gap-8 md:grid-cols-2">
                {detail("Weather", m.weather.clone())} {detail("Time", m.time_of_day.clone())}
                {detail("Max Players", m.max_players.to_string())}
                {detail("Status", m.status.clone())}
            </dl>

            {(!factions.is_empty())
                .then(move || {
                    let af_tabs = active_faction;
                    let af_rows = active_faction;
                    let armory_rows = armory.clone();
                    view! {
                        <section>
                            <h3 class="mb-2 text-label-md text-on-surface-variant uppercase">
                                "The Armory"
                            </h3>
                            <div class="mb-3 flex gap-2">
                                {factions_for_tabs
                                    .iter()
                                    .map(|f| {
                                        let f_click = f.clone();
                                        let f_active = f.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| faction_sel.set(Some(f_click.clone()))
                                                class=move || {
                                                    crate::ui::cn(
                                                        &[
                                                            "rounded-lg px-3 py-1.5 text-label-md",
                                                            if af_tabs().as_deref() == Some(f_active.as_str()) {
                                                                "bg-primary text-on-primary"
                                                            } else {
                                                                "bg-surface-container text-on-surface-variant"
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
                            <div class="grid gap-2">
                                {move || {
                                    let af = af_rows();
                                    armory_rows
                                        .iter()
                                        .filter(|a| {
                                            a.get("faction").and_then(|v| v.as_str())
                                                == af.as_deref()
                                        })
                                        .map(|item| {
                                            let name = item
                                                .get("item_name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string();
                                            let qty = match item.get("quantity") {
                                                Some(Value::Number(n)) => format!("x{n}"),
                                                _ => "∞".to_string(),
                                            };
                                            view! {
                                                <div class="flex justify-between rounded-lg border border-outline-variant/30 bg-surface-container p-3 text-label-md">
                                                    <span class="text-on-surface">{name}</span>
                                                    <span class="text-on-surface-variant">{qty}</span>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </section>
                    }
                })}
        </div>
    }
}

fn detail(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <dt class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </dt>
            <dd class="mt-1 text-headline-sm text-on-surface">{value}</dd>
        </div>
    }
}
