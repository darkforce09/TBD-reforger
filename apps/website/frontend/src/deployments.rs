//! My Deployments (/deployments) — ported from pages/operations.tsx `DeploymentsPage`. `<AuthGate>`
//! → `/deployments` Resource → `QueryState` → a two-pane service record: a left telemetry dossier
//! (identity from the auth store + the mock K/D / win-rate / fav-loadout constants + total deploys)
//! and a right pane (Active Orders banner + Combat History).
//!
//! **Empty-DB golden (unchanged):** with no upcoming and an empty history the "No Active Orders" +
//! "No Service History Compiled" states and the always-on dossier still render byte-for-byte as
//! before.
//!
//! **T-232:** both populated branches were literal `().into_any()`, so a caller with real
//! deployments (2 upcoming + 2 history rows against the live API) saw "Active Orders" over blank
//! space and an empty "Combat History" section. Both are now written: the Active-Orders banner
//! renders the caller's next deployment (countdown, assigned slot, ORBAT deep link) with the rest of
//! the queue beneath it, and Combat History is the service-record table the surface spec asks for
//! (Date / Operation / Role / Outcome / AAR).
//!
//! Neither branch fetches anything — `GET /me/deployments` returns both lists in one payload — so
//! there is no second Resource here to go stale (the T-226 hazard). Items stay `serde_json::Value`;
//! the fields read are pinned by the `registration_state` and `mission_outcome` enums.
#![allow(dead_code)]
use crate::auth::AuthStore;
use crate::datefmt::{countdown_label, format_local_datetime, format_short_date};
use crate::dto::Deployments;
use crate::ui::{badge_class, cn, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// `registration_state` → its `badge_class` variant.
fn state_variant(state: &str) -> &'static str {
    match state {
        "registered" | "attended" => "success",
        "waitlisted" => "warning",
        "withdrawn" | "no_show" => "error",
        _ => "neutral",
    }
}

/// `mission_outcome` → the service-record label the spec names ("MISSION SUCCESS / FAILED").
fn outcome_label(outcome: &str) -> &'static str {
    match outcome {
        "success" => "Mission Success",
        "failure" => "Failed",
        "aborted" => "Aborted",
        "pending" => "Pending",
        _ => "Unknown",
    }
}

fn outcome_variant(outcome: &str) -> &'static str {
    match outcome {
        "success" => "success",
        "failure" => "error",
        "aborted" => "warning",
        _ => "neutral",
    }
}

fn terrain_label(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => "—".into(),
    }
}

/// "BLUFOR · Command · Platoon Leader" from whichever of `faction` / `squad` / `role` the backend
/// actually filled — an unassigned registration carries none of the three, and joining blindly would
/// render a row of bare separators.
fn slot_line(u: &Value) -> String {
    ["faction", "squad", "role"]
        .iter()
        .map(|k| vstr(u, k))
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

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
    let has_active = !d.upcoming.is_empty();
    let has_history = !d.service_history.is_empty();
    let upcoming = d.upcoming.clone();
    let history = d.service_history.clone();

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
                                active_orders(upcoming).into_any()
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
                            service_record(history).into_any()
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

/// The Active Orders banner. `upcoming` is ordered soonest-first by the backend, so `[0]` is the
/// hero order and the rest queue beneath it — the spec's "Awaiting Deployment" list without a
/// second heading, since the banner already carries one.
fn active_orders(upcoming: Vec<Value>) -> impl IntoView {
    let mut it = upcoming.into_iter();
    let Some(next) = it.next() else {
        // `has_active` is checked by the caller; this arm exists so the function is total.
        return ().into_any();
    };
    let rest: Vec<Value> = it.collect();
    let name = vstr(&next, "name");
    let name = if name.is_empty() {
        "Untitled Operation".to_string()
    } else {
        name
    };
    let start = vstr(&next, "start_time");
    let when = format_local_datetime(&start);
    let countdown = countdown_label(&start);
    let terrain = terrain_label(&vstr(&next, "terrain"));
    let state = vstr(&next, "state");
    let slot = slot_line(&next);
    let event_id = vstr(&next, "event_id");
    let emid = vstr(&next, "event_mission_id");
    // Item 9 of the surface spec — `/events/:id/missions/:emid/orbat`, the route `router.rs:105`
    // registers. Rendered only when both ids are on the wire; a registration with no
    // `event_mission_id` has no ORBAT to modify.
    let orbat_href = (!event_id.is_empty() && !emid.is_empty())
        .then(|| format!("/events/{event_id}/missions/{emid}/orbat"));
    let hub_href = (!event_id.is_empty()).then(|| format!("/events/{event_id}"));
    view! {
        <div class="flex flex-col gap-5">
            <div class="flex flex-col gap-3">
                <div class="flex flex-wrap items-baseline gap-x-4 gap-y-1">
                    <h3 class="text-3xl font-black uppercase tracking-tight text-on-surface">
                        {name}
                    </h3>
                    {(!state.is_empty())
                        .then(|| {
                            view! { <span class=badge_class(state_variant(&state))>{state.clone()}</span> }
                        })}
                </div>
                <div class="flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                    <span>{when}</span>
                    <span class="text-primary">"T-MINUS "{countdown}</span>
                    <span>{terrain}</span>
                </div>
                {(!slot.is_empty())
                    .then(|| {
                        view! {
                            <span class="w-fit rounded-md border border-primary/30 bg-primary/10 px-2.5 py-1 font-mono text-xs tracking-widest text-primary uppercase">
                                "Assigned slot: "
                                {slot.clone()}
                            </span>
                        }
                    })}
            </div>
            <div class="flex flex-wrap gap-3">
                {orbat_href
                    .map(|href| {
                        view! {
                            <a
                                href=href
                                class="inline-flex items-center gap-2 rounded-full border border-primary/50 bg-surface/50 px-5 py-2.5 font-mono text-xs tracking-widest text-primary uppercase transition hover:bg-primary/20"
                            >
                                <MaterialIcon name="tune" class="text-base" />
                                "Modify Assignment"
                            </a>
                        }
                    })}
                {hub_href
                    .map(|href| {
                        view! {
                            <a
                                href=href
                                class="inline-flex items-center gap-2 rounded-full border border-white/10 px-5 py-2.5 font-mono text-xs tracking-widest text-on-surface uppercase transition hover:bg-white/5"
                            >
                                <MaterialIcon name="open_in_new" class="text-base" />
                                "Operation Hub"
                            </a>
                        }
                    })}
            </div>
            {(!rest.is_empty())
                .then(|| {
                    view! {
                        <div class="flex flex-col gap-2 border-t border-white/10 pt-4">
                            <span class="font-mono text-[10px] uppercase tracking-widest text-on-surface-variant">
                                "Also Awaiting Deployment"
                            </span>
                            {rest
                                .into_iter()
                                .map(|u| {
                                    let n = vstr(&u, "name");
                                    let n = if n.is_empty() {
                                        "Untitled Operation".to_string()
                                    } else {
                                        n
                                    };
                                    let s = vstr(&u, "start_time");
                                    let st = vstr(&u, "state");
                                    let eid = vstr(&u, "event_id");
                                    let row = view! {
                                        <>
                                            <span class="min-w-0 flex-1 truncate text-on-surface">{n}</span>
                                            <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                {format_local_datetime(&s)}
                                            </span>
                                            {(!st.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <span class=badge_class(state_variant(&st))>{st.clone()}</span>
                                                    }
                                                })}
                                        </>
                                    };
                                    // A row is a link when it has an event to link to, and inert
                                    // markup when it does not — never an anchor to "/events/".
                                    if eid.is_empty() {
                                        view! {
                                            <div class="flex items-center gap-3 rounded-lg border border-white/10 px-4 py-2.5 text-sm">
                                                {row}
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        view! {
                                            <a
                                                href=format!("/events/{eid}")
                                                class="flex items-center gap-3 rounded-lg border border-white/10 px-4 py-2.5 text-sm transition hover:bg-white/[0.03]"
                                            >
                                                {row}
                                            </a>
                                        }
                                            .into_any()
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                })}
        </div>
    }
    .into_any()
}

/// The Service Record table — spec items 11–18. A real `<table>` rather than a div grid: it is
/// tabular data, and the wide-table roster (`personnel.rs`) is the platform precedent for one.
fn service_record(history: Vec<Value>) -> impl IntoView {
    view! {
        <div class="custom-scrollbar overflow-x-auto rounded-xl border border-white/10">
            <table class="w-full min-w-[40rem] border-collapse text-left text-sm">
                <thead>
                    <tr class="border-b border-white/10 bg-surface-container-lowest/40">
                        <ServiceHead label="Date" />
                        <ServiceHead label="Operation" />
                        <ServiceHead label="Role Played" />
                        <ServiceHead label="Outcome" />
                        <ServiceHead label="AAR" />
                    </tr>
                </thead>
                <tbody>
                    {history
                        .into_iter()
                        .map(|h| {
                            let date = vstr(&h, "date");
                            let operation = vstr(&h, "operation");
                            let operation = if operation.is_empty() {
                                "—".to_string()
                            } else {
                                operation
                            };
                            let role = vstr(&h, "role");
                            let role = if role.is_empty() { "—".to_string() } else { role };
                            let outcome = vstr(&h, "outcome");
                            let replay = vstr(&h, "aar_replay_url");
                            view! {
                                <tr class="border-b border-white/5 transition last:border-b-0 hover:bg-white/[0.02]">
                                    <td class="px-4 py-3 font-mono text-xs whitespace-nowrap text-on-surface-variant">
                                        {format_short_date(&date)}
                                    </td>
                                    <td class="px-4 py-3 font-medium text-on-surface">{operation}</td>
                                    <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                                        {role}
                                    </td>
                                    <td class="px-4 py-3">
                                        <span class=badge_class(
                                            outcome_variant(&outcome),
                                        )>{outcome_label(&outcome)}</span>
                                    </td>
                                    <td class="px-4 py-3">
                                        {if replay.is_empty() {
                                            view! {
                                                <span class="font-mono text-xs text-outline">"—"</span>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <a
                                                    href=replay
                                                    target="_blank"
                                                    rel="noreferrer"
                                                    class="inline-flex items-center gap-1 font-mono text-xs tracking-wider text-primary uppercase transition hover:underline"
                                                >
                                                    <MaterialIcon name="play_circle" class="text-base" />
                                                    "View Replay"
                                                </a>
                                            }
                                                .into_any()
                                        }}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn ServiceHead(label: &'static str) -> impl IntoView {
    view! {
        <th class="px-4 py-3 font-mono text-[10px] font-normal tracking-widest text-on-surface-variant uppercase">
            {label}
        </th>
    }
}
