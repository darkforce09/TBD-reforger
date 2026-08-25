//! Vehicle Database (/vehicles) — ported from pages/doctrine.tsx `VehicleDatabasePage`.
//! `<AuthGate>` → `GET /api/v1/vehicle-database` → a `GlassSplit`: faction-grouped list (master)
//! + IFF dossier (detail). Fields match the API row (`name`, `faction`, `armor_type`,
//! `amphibious`, `primary_threat`, `profile_image_url`) — the old 13-field mock dossier is gone.
//!
//! List items ride `DataEnvelope<Value>` (dto.rs already pins the vehicle-database golden that
//! way — no typed VehicleDatabase DTO consumer yet).
#![allow(dead_code)]
use crate::core::dto::DataEnvelope;
use crate::core::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use crate::core::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::Value;

const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
const BADGE_WARNING: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow";
const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// Amphibious Yes → warning chip; No / empty → success/neutral.
fn amphib_badge(amphibious: &str) -> &'static str {
    match amphibious.trim().to_ascii_lowercase().as_str() {
        "yes" | "y" | "true" => BADGE_WARNING,
        "no" | "n" | "false" => BADGE_SUCCESS,
        _ => BADGE_NEUTRAL,
    }
}

/// Distinct factions in first-seen order (API returns `ORDER BY name ASC`).
fn faction_order(vehicles: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for v in vehicles {
        let f = vstr(v, "faction");
        if !f.is_empty() && !out.iter().any(|x| x == &f) {
            out.push(f);
        }
    }
    out
}

#[component]
pub fn VehicleDatabasePage() -> impl IntoView {
    view! {
        <crate::core::ui::AuthGate>
            <VehiclesInner />
        </crate::core::ui::AuthGate>
    }
}

#[component]
fn VehiclesInner() -> impl IntoView {
    let store = expect_context::<crate::core::auth::AuthStore>();
    let vehicles = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<DataEnvelope<Value>>(store, "/vehicle-database")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                vehicles
                    .get()
                    .map(|opt| match opt {
                        Some(env) => board(env.data).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load vehicles."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn board(rows: Vec<Value>) -> impl IntoView {
    let selected_id = RwSignal::new(rows.first().map(|v| vstr(v, "id")).unwrap_or_default());
    let search = RwSignal::new(String::new());
    let rows_master = rows.clone();
    let rows_detail = rows;

    view! {
        <GlassSplit
            master_width="18rem"
            master_header=master_header(search).into_any()
            master=view! { {move || vehicle_list(selected_id, &search.get(), &rows_master)} }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let v = rows_detail
                        .iter()
                        .find(|r| vstr(r, "id") == id)
                        .cloned()
                        .or_else(|| rows_detail.first().cloned());
                    match v {
                        Some(row) => dossier(row).into_any(),
                        None => view! {
                            <div class="flex h-full items-center justify-center p-8">
                                <p class="font-mono text-sm text-on-surface-variant">
                                    "No vehicles in the database."
                                </p>
                            </div>
                        }
                        .into_any(),
                    }
                }}
            }
                .into_any()
        />
    }
}

fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "Vehicle Database"
            </p>
            <SidebarSearch placeholder="Search assets..." bind=search />
        </div>
    }
}

fn vehicle_list(selected_id: RwSignal<String>, query: &str, vehicles: &[Value]) -> impl IntoView {
    let query = query.to_string();
    faction_order(vehicles)
        .into_iter()
        .filter_map(move |faction| {
            let rows: Vec<Value> = vehicles
                .iter()
                .filter(|v| vstr(v, "faction") == faction)
                .filter(|v| {
                    crate::core::split_pane::search_matches(
                        &query,
                        &format!(
                            "{} {} {}",
                            vstr(v, "name"),
                            vstr(v, "armor_type"),
                            vstr(v, "faction")
                        ),
                    )
                })
                .cloned()
                .collect();
            if rows.is_empty() {
                return None;
            }
            let faction_label = faction.clone();
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {faction_label}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|v| {
                                let id = vstr(&v, "id");
                                let name = vstr(&v, "name");
                                let class = vstr(&v, "armor_type");
                                let id_click = id.clone();
                                view! {
                                    <ListDetailItem
                                        active=id == selected_id.get()
                                        title=view! { {name} }.into_any()
                                        preview=view! {
                                            <span class="font-mono uppercase text-outline">
                                                {class}
                                            </span>
                                        }
                                            .into_any()
                                        on_click=Callback::new(move |()| {
                                            selected_id.set(id_click.clone())
                                        })
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}

fn dossier(v: Value) -> impl IntoView {
    let name = vstr(&v, "name");
    let faction = vstr(&v, "faction");
    let armor = vstr(&v, "armor_type");
    let amphib = vstr(&v, "amphibious");
    let threat = vstr(&v, "primary_threat");
    let image = vstr(&v, "profile_image_url");

    let hero = if image.is_empty() {
        view! {
            <div class="flex h-full w-full items-center justify-center bg-surface-container-low">
                <MaterialIcon name="directions_car" class="text-7xl text-outline" />
            </div>
        }
        .into_any()
    } else {
        view! { <img src=image alt="" class="h-full w-full object-cover" /> }.into_any()
    };

    let amphib_label = if amphib.is_empty() {
        "—".to_string()
    } else {
        amphib.clone()
    };
    let threat_body = if threat.is_empty() {
        "No primary threat recorded.".to_string()
    } else {
        threat.clone()
    };

    view! {
        <div>
            <div class="relative h-72 w-full overflow-hidden">
                {hero}
                <div class="absolute inset-0 bg-gradient-to-t from-surface-dim to-transparent"></div>
                <div class="absolute right-8 bottom-6 left-8">
                    <div class="mb-3 flex flex-wrap items-center gap-2">
                        <span class=BADGE_NEUTRAL>"ARMOR: "{armor.clone()}</span>
                        {if !amphib.is_empty() {
                            view! {
                                <span class=amphib_badge(&amphib)>"AMPHIB: "{amphib.clone()}</span>
                            }
                                .into_any()
                        } else {
                            ().into_any()
                        }}
                        <span class=BADGE_PRIMARY>{faction.clone()}</span>
                    </div>
                    <h1 class="text-4xl font-black tracking-tighter text-white uppercase">
                        {name}
                    </h1>
                </div>
            </div>
            <div class="space-y-8 p-8 md:p-12">
                <div class="rounded-2xl border-l-4 border-tactical-yellow bg-tactical-yellow/10 p-4 shadow-lg backdrop-blur-md">
                    <p class="mb-1 font-mono text-xs font-bold tracking-widest text-tactical-yellow uppercase">
                        "Primary Threat"
                    </p>
                    <p class="text-body-md leading-relaxed text-on-surface-variant">
                        {threat_body}
                    </p>
                </div>
                <div>
                    {section_title("Telemetry")}
                    <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
                        {vehicle_stat("Faction", faction)}
                        {vehicle_stat("Armor", armor)}
                        {vehicle_stat("Amphibious", amphib_label)}
                    </div>
                </div>
            </div>
        </div>
    }
}

fn section_title(t: &'static str) -> impl IntoView {
    view! {
        <h2 class="mb-3 font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
            {t}
        </h2>
    }
}

fn vehicle_stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <p class="font-mono text-[11px] tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 font-mono text-base text-white">{value}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::faction_order;
    use serde_json::json;

    #[test]
    fn factions_preserve_first_seen_order() {
        let rows = vec![
            json!({"name": "BTR-70", "faction": "USSR"}),
            json!({"name": "M113A3", "faction": "US Army"}),
            json!({"name": "UAZ-469", "faction": "USSR"}),
        ];
        assert_eq!(
            faction_order(&rows),
            vec!["USSR".to_string(), "US Army".to_string()]
        );
    }
}
