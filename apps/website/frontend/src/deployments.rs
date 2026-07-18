//! My Deployments (/deployments) — ported from pages/operations.tsx `DeploymentsPage`. `<AuthGate>`
//! → `/deployments` Resource → `QueryState` → a two-pane service record: a left telemetry dossier
//! (identity from the auth store + the mock K/D / win-rate / fav-loadout constants + total deploys)
//! and a right pane (Active Orders banner + Combat History).
//!
//! **Gate scope (this slice):** the empty-DB `/deployments` golden (no upcoming, empty history) → the
//! "No Active Orders" + "No Service History Compiled" states + the always-on dossier (mock stats are
//! deterministic client constants) are byte-exact-verified. The populated Active-Order card + the
//! combat-history timeline are content-golden gated; the upcoming/history item types stay
//! `serde_json::Value` until then.
#![allow(dead_code)]
use crate::auth::AuthStore;
use crate::dto::Deployments;
use crate::ui::{cn, MaterialIcon};
use leptos::prelude::*;

// Client-side constants (used until telemetry serves real numbers) — byte-identical to operations.tsx.
const MOCK_KD: &str = "2.45";
const MOCK_WIN_RATE: &str = "68%";
const FAV_WEAPON_NAME: &str = "M4A1 Block II";
const FAV_WEAPON_IMG: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='56'><rect width='120' height='56' fill='%23242a3a'/><rect x='12' y='25' width='86' height='6' rx='2' fill='%23adc6ff'/><rect x='80' y='22' width='11' height='18' rx='2' fill='%233a4252'/><rect x='30' y='31' width='10' height='12' rx='2' fill='%233a4252'/></svg>";
const FAV_ASSET_NAME: &str = "M1A2 Abrams";
const FAV_ASSET_IMG: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='56'><rect width='120' height='56' fill='%23242a3a'/><rect x='22' y='28' width='76' height='14' rx='3' fill='%233a4252'/><rect x='44' y='20' width='30' height='10' rx='2' fill='%233a4252'/><rect x='70' y='30' width='34' height='4' rx='2' fill='%23adc6ff'/><circle cx='36' cy='44' r='5' fill='%23adc6ff'/><circle cx='84' cy='44' r='5' fill='%23adc6ff'/></svg>";
const BANNER_IMG: &str = "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='400' height='200'><rect width='400' height='200' fill='%23151b2b'/><g stroke='%23adc6ff' stroke-width='0.5' opacity='0.5'><path d='M0 40 H400 M0 80 H400 M0 120 H400 M0 160 H400 M50 0 V200 M120 0 V200 M190 0 V200 M260 0 V200 M330 0 V200'/></g><circle cx='190' cy='100' r='26' fill='none' stroke='%23facc15' stroke-width='1.5'/><path d='M190 66 V134 M156 100 H224' stroke='%23facc15' stroke-width='1'/></svg>";

#[component]
pub fn DeploymentsPage() -> impl IntoView {
    view! {
        <crate::ui::AuthGate>
            <DeploymentsInner />
        </crate::ui::AuthGate>
    }
}

#[component]
fn DeploymentsInner() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let data = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Deployments>(store, "/me/deployments")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Deployments>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                data.get()
                    .map(|opt| match opt {
                        Some(d) => dossier(d).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

#[component]
fn TelemetryStat(
    label: &'static str,
    value: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let p_class = cn(&["text-[5rem] font-bold leading-none tracking-tighter", class]);
    view! {
        <div>
            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                {label}
            </span>
            <p class=p_class>{value}</p>
        </div>
    }
}

#[component]
fn FavLoadout(label: &'static str, name: &'static str, img: &'static str) -> impl IntoView {
    view! {
        <div>
            <span class="mb-1.5 block font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                {label}
            </span>
            <div class="flex items-center gap-3 rounded-lg border border-white/10 bg-surface-container/50 p-2">
                <img
                    src=img
                    alt=""
                    class="h-10 w-20 shrink-0 rounded border border-white/10 object-cover"
                />
                <span class="font-mono text-sm text-on-surface">{name}</span>
            </div>
        </div>
    }
}

fn dossier(d: Deployments) -> impl IntoView {
    let user = expect_context::<AuthStore>().user.get();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let role = user.as_ref().map(|u| u.role.as_str()).unwrap_or_default();
    // upcoming[0] / service_history populated views are content-golden gated (empty golden → empty).
    let has_active = !d.upcoming.is_empty();
    let has_history = !d.service_history.is_empty();

    view! {
        <div class="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
            <div class="flex h-full w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl lg:flex-row">
                // ── Left: telemetry dossier ──
                <aside class="custom-scrollbar flex shrink-0 flex-col gap-8 overflow-y-auto border-b border-white/10 bg-surface-container-lowest/40 p-8 lg:w-[30%] lg:border-b-0 lg:border-r">
                    <header>
                        <div class="mb-6 flex h-16 w-16 items-center justify-center text-primary">
                            <MaterialIcon name="military_tech" class="text-[4rem] leading-none" />
                        </div>
                        <h2 class="text-4xl font-black uppercase leading-none tracking-tighter text-on-surface">
                            {username}
                        </h2>
                        <span class="mt-1 block font-mono text-sm uppercase tracking-widest text-primary">
                            {role}
                        </span>
                    </header>
                    <div class="space-y-6">
                        <TelemetryStat label="K/D Ratio" value=MOCK_KD class="text-primary" />
                        <TelemetryStat label="Win Rate" value=MOCK_WIN_RATE class="text-success" />
                    </div>
                    <div class="space-y-5 border-t border-white/10 pt-6">
                        <div>
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Total Deployments"
                            </span>
                            <p class="font-mono text-3xl font-bold text-on-surface">
                                {d.total_operations}
                            </p>
                        </div>
                        <FavLoadout label="Fav Weapon" name=FAV_WEAPON_NAME img=FAV_WEAPON_IMG />
                        <FavLoadout label="Fav Asset" name=FAV_ASSET_NAME img=FAV_ASSET_IMG />
                    </div>
                </aside>

                // ── Right: active orders + combat history ──
                <main class="custom-scrollbar flex min-h-0 flex-1 flex-col overflow-y-auto bg-surface-container-highest/10">
                    <section class="relative shrink-0 overflow-hidden border-b border-white/10">
                        <img
                            src=BANNER_IMG
                            alt=""
                            class="absolute inset-0 h-full w-full object-cover opacity-30 mix-blend-luminosity"
                        />
                        <div class="absolute inset-0 bg-gradient-to-r from-surface-container-lowest/80 to-transparent"></div>
                        <div class="relative z-10 flex min-h-[240px] flex-col justify-center gap-3 p-8">
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Active Orders"
                            </span>
                            {if has_active {
                                // Populated active-order card is content-golden gated.
                                ().into_any()
                            } else {
                                view! {
                                    <div class="flex flex-col items-center justify-center gap-3 py-4 text-center">
                                        <MaterialIcon
                                            name="track_changes"
                                            class="text-7xl text-on-surface-variant/40 animate-pulse drop-shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                                        />
                                        <h3 class="text-3xl font-black uppercase tracking-tight text-on-surface-variant/60">
                                            "No Active Orders"
                                        </h3>
                                        <p class="font-mono text-sm text-on-surface-variant">
                                            "Stand by for deployment tasking."
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }}
                        </div>
                    </section>
                    <section class="p-8">
                        <h2 class="mb-4 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                            "Combat History"
                        </h2>
                        {if has_history {
                            // Populated combat-history timeline is content-golden gated.
                            ().into_any()
                        } else {
                            view! {
                                <div class="bg-grid-overlay flex min-h-[200px] items-center justify-center rounded-xl border border-white/10 shadow-[inset_0_0_30px_rgba(173,198,255,0.06)]">
                                    <p class="font-mono text-code-md uppercase tracking-widest text-on-surface-variant">
                                        "No Service History Compiled"
                                    </p>
                                </div>
                            }
                                .into_any()
                        }}
                    </section>
                </main>
            </div>
        </div>
    }
}
