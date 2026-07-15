//! Mission Library (/missions) — ported from pages/missions.tsx `MissionLibraryPage`. `<AuthGate>` →
//! `/missions` Resource → a scope-tabbed, filterable library: a featured hero + a mission grid.
//!
//! **Gate scope (this slice):** the empty-DB `/missions` golden (Paginated empty) on the default
//! Global tab (mission_maker+ store user) → header + scope tabs + New Mission + the search/filter
//! toolbar + "No missions found." (Global scope → no empty-create CTA), no featured hero. Byte-exact.
//! The featured hero, the mission grid cards, and the dossier Sheet / create dialog are
//! content-golden/behavior gated (need a seeded golden + interactivity).
#![allow(dead_code)]
use crate::dto::Paginated;
use crate::nav::Role;
use crate::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

const SELECT_CLASS: &str = "rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
// SCOPES: (label, active-on-load). Global is scopeIdx 0.
const SCOPES: [&str; 3] = ["Global Missions", "My Missions", "Bookmarked"];

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
fn game_mode_label(m: &str) -> &str {
    match m {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

#[component]
pub fn MissionLibraryPage() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // isMaker: admin (and guest browse-mode) both satisfy mission_maker → the New Mission affordances.
    let is_maker = store.has_min_role(Role::MissionMaker);
    let missions = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<Value>>(store, "/missions")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<Value>>
        }
    });
    view! {
        <AuthGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto bg-surface-glass backdrop-blur-xl">
                    <div class="p-6 md:p-8">
                        {library_header(is_maker)}
                        <Suspense fallback=move || {
                            view! { <p class="text-on-surface-variant">"Loading…"</p> }
                        }>
                            {move || {
                                missions
                                    .get()
                                    .map(|opt| match opt {
                                        Some(page) => body(page.data).into_any(),
                                        None => {
                                            view! {
                                                <p class="text-error">"Failed to load data."</p>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    </div>
                </div>
            </div>
        </AuthGate>
    }
}

fn library_header(is_maker: bool) -> impl IntoView {
    view! {
        <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
            <div>
                <h1 class="text-4xl font-bold tracking-tight text-on-surface uppercase">
                    "Mission Library"
                </h1>
                <p class="mt-1 text-body-md text-on-surface-variant">
                    "Browse, filter, and deploy active operations across the theater."
                </p>
                <div class="mt-5 inline-flex gap-1 rounded-full border border-white/5 bg-black/20 p-1">
                    {SCOPES
                        .iter()
                        .enumerate()
                        .map(|(i, label)| {
                            // cn(): text-label-md twMerge-dropped vs the trailing text-{color}.
                            let class = if i == 0 {
                                "rounded-full px-4 py-1.5 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                            } else {
                                "rounded-full px-4 py-1.5 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                            };
                            view! {
                                <button type="button" class=class>
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            {is_maker
                .then(|| {
                    view! {
                        <button
                            type="button"
                            title="New Mission (Ctrl+N)"
                            class="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                        >
                            <MaterialIcon name="add" class="text-[18px]" />
                            "New Mission"
                        </button>
                    }
                })}
        </header>
    }
}

fn body(missions: Vec<Value>) -> impl IntoView {
    // featured = globalData[0] — None on the empty golden → no hero. Grid empty + Global scope →
    // "No missions found." (the maker empty-create CTA is My-Missions-scope only).
    let _featured = missions.first();
    view! {
        <>
            <div class="mb-6 flex flex-wrap items-center gap-2 rounded-2xl border border-white/5 bg-black/20 p-2 backdrop-blur-md">
                <input
                    type="search"
                    placeholder="Search operations..."
                    value=""
                    class="min-w-[200px] flex-1 rounded-lg border border-white/10 bg-black/30 px-4 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
                />
                <select class=SELECT_CLASS>
                    <option value="">"All Terrains"</option>
                    <option value="everon">{terrain_label("everon")}</option>
                    <option value="arland">{terrain_label("arland")}</option>
                </select>
                <select class=SELECT_CLASS>
                    <option value="">"All Modes"</option>
                    <option value="pve_coop">{game_mode_label("pve_coop")}</option>
                    <option value="pvp">{game_mode_label("pvp")}</option>
                    <option value="zeus">{game_mode_label("zeus")}</option>
                </select>
                <select class=SELECT_CLASS>
                    <option value="">"All Players"</option>
                    <option value="1-8">"1–8"</option>
                    <option value="9-16">"9–16"</option>
                    <option value="17-32">"17–32"</option>
                    <option value="33-64">"33–64"</option>
                </select>
            </div>
            {if missions.is_empty() {
                view! { <p class="py-12 text-center text-on-surface-variant">"No missions found."</p> }
                    .into_any()
            } else {
                // Populated grid — content-golden gated.
                ().into_any()
            }}
        </>
    }
}
