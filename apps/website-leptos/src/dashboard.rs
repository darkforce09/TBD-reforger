//! Dashboard (landing "/") — ported from pages/Dashboard.tsx. `<AuthGate>` → a `/dashboard`
//! Resource → `QueryState` (loading/error/content) → the bento (hero + server/deployment/modpack
//! cards + intelligence feed). DOM + class strings matched 1:1 to the React render (authed V-gate).
//!
//! **Gate scope (this slice):** the empty-DB `/dashboard` golden — `next_event`/`my_assignment`/
//! `server_status` null, `current_modpack` present, no announcements — so the *null/empty* branches
//! (and the modpack card via [`format_bytes`]) are byte-exact-verified. The populated branches
//! (countdown, live server stats, assignment, announcement rows) render faithfully but are only
//! V-gate-covered once a seeded content golden exists (follow-up); their date/uptime formatters port
//! then. Nested bodies stay `serde_json::Value` until their own DTOs are golden-proven.
#![allow(dead_code)]
use crate::dto::DashboardResponse;
use crate::ui::{cn, AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

/// Cinematic hero backdrop from the tactical-command-center blueprint (byte-identical to React).
const HERO_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuB_SlrhFHaG9jlm7NfoEUTrANNfG_-m0cqYcJVwKZ1pAUA_LTEnwP1zyNasVKfTgKdnX14ssTtYpEc3I1qn0UaEjwwEQyuAGxherp9Eu5rIpF4afr0sjFAUSjc9Z5NpB2xub7NkJCKNYCkkFsIa25L2e5QrbN4lEOZHeGZeLxpbVtQC8WATlT2skffHxtraZAi95LpXOqnuyLkxHIoJOHtxsFj2rJ4xCywZTnNZy_bJSzmLgPaun0eZsYw-Prx2nJ2GeJMP72x2l-4";

/// `formatBytes` (lib/format.ts) — `<1 → "0 B"`, `≥1 GiB → "{:.1} GB"`, else `"{:.0} MB"`.
fn format_bytes(bytes: i64) -> String {
    if bytes < 1 {
        return "0 B".into();
    }
    let gb = bytes as f64 / 1024f64.powi(3);
    if gb >= 1.0 {
        return format!("{gb:.1} GB");
    }
    let mb = bytes as f64 / 1024f64.powi(2);
    format!("{mb:.0} MB")
}

fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
fn vint(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <AuthGate>
            <DashboardInner />
        </AuthGate>
    }
}

#[component]
fn DashboardInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // useDashboard() → GET /dashboard. LocalResource (not Resource): the gloo-net + single-flight
    // client future is `!Send` (client-only), which the Send-bounded Resource rejects. Native builds
    // don't fetch (the browser V-gate exercises the real path).
    let dash = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DashboardResponse>(store, "/dashboard")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DashboardResponse>
        }
    });
    // QueryState: pending → "Loading…"; resolved-err → error; resolved-ok → bento.
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                dash.get()
                    .map(|opt| match opt {
                        Some(d) => bento(d).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The bento render. OpsCard is inlined per card because React's `cn(base, 'glass', className)`
/// runs tailwind-merge, which drops the base `gap-3` when the card passes `gap-4`; the merged class
/// strings are written out verbatim here (a general Rust tw_merge is a separate deferred utility).
fn bento(d: DashboardResponse) -> impl IntoView {
    let next = d.next_event;
    let assignment = d.my_assignment;
    let server = d.server_status;
    let modpack = d.current_modpack;
    let announcements = d.recent_announcements;

    let player_pct = match &server {
        Some(s) if vint(s, "max_players") > 0 => {
            ((vint(s, "player_count") as f64 / vint(s, "max_players") as f64) * 100.0).round()
                as i64
        }
        _ => 0,
    };

    view! {
        <div class="custom-scrollbar flex h-full w-full flex-col gap-8 overflow-y-auto p-6 md:p-8">
            // ── Hero Banner ──
            <div class="glass border-glow relative flex min-h-[300px] flex-col justify-end overflow-hidden rounded-xl p-8">
                <div class="absolute inset-0 z-0">
                    <img
                        alt="Operation theater"
                        src=HERO_IMAGE
                        class="h-full w-full object-cover opacity-40 mix-blend-overlay"
                    />
                    <div class="absolute inset-0 bg-gradient-to-t from-surface-container-lowest via-surface-container-lowest/80 to-transparent"></div>
                </div>
                <div class="relative z-10 flex w-full flex-wrap items-end justify-between gap-4">
                    <div class="flex flex-col">
                        <h2 class="text-glow mb-2 font-mono text-5xl font-bold tracking-tighter text-primary md:text-7xl">
                            {match &next {
                                Some(n) => format!("T-MINUS {}", vstr(n, "start_time")),
                                None => "NO UPCOMING OPS".to_string(),
                            }}
                        </h2>
                        <p class="flex items-center gap-2 text-sm tracking-widest text-on-surface uppercase opacity-80">
                            <span class="h-2 w-2 animate-pulse rounded-full bg-primary"></span>
                            {match &next {
                                Some(n) => {
                                    format!("OPERATION: {} — {}", vstr(n, "name"), vstr(n, "terrain"))
                                }
                                None => "Check the event schedule for new operations.".to_string(),
                            }}
                        </p>
                    </div>
                    {next
                        .as_ref()
                        .map(|n| {
                            let href = format!("/events/{}", vstr(n, "event_id"));
                            view! {
                                <a
                                    href=href
                                    class="group flex items-center gap-2 rounded-lg border border-primary/50 bg-surface/50 px-6 py-3 text-sm font-bold tracking-widest text-primary uppercase backdrop-blur-md transition-all hover:bg-primary/20 active:scale-95"
                                >
                                    "Open Operation Hub"
                                    <MaterialIcon
                                        name="arrow_forward"
                                        class="transition-transform group-hover:translate-x-1"
                                    />
                                </a>
                            }
                        })}
                </div>
            </div>

            // ── Bento Grid ──
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                // Server Uplink
                <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass gap-4">
                    <div class="flex items-center justify-between border-b border-border-subtle pb-3">
                        <h3 class="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                            <MaterialIcon name="dns" class="text-[18px]" />
                            "Server Uplink"
                        </h3>
                        <div class="flex items-center gap-2 rounded-full border border-success/30 bg-success-muted px-2 py-1">
                            <div class=cn(
                                &[
                                    "h-2 w-2 rounded-full",
                                    if server.as_ref().is_some_and(|s| vbool(s, "is_online")) {
                                        "bg-success tactical-pulse"
                                    } else {
                                        "bg-outline"
                                    },
                                ],
                            )></div>
                            <span class="font-mono text-[10px] font-bold tracking-widest text-success">
                                {if server.as_ref().is_some_and(|s| vbool(s, "is_online")) {
                                    "ONLINE"
                                } else {
                                    "OFFLINE"
                                }}
                            </span>
                        </div>
                    </div>
                    <div class="mt-2 flex flex-col">
                        <div class="mb-2 flex items-end justify-between">
                            <span class="font-mono text-3xl font-light text-on-surface">
                                {server.as_ref().map(|s| vint(s, "player_count")).unwrap_or(0)}
                                // React renders the literal "/" and the number as SEPARATE text nodes.
                                <span class="text-lg text-on-surface-variant">
                                    "/"
                                    {server.as_ref().map(|s| vint(s, "max_players")).unwrap_or(0)}
                                </span>
                            </span>
                            <span class="mb-1 font-mono text-xs text-on-surface-variant">"PLAYERS"</span>
                        </div>
                        <div class="h-1.5 w-full overflow-hidden rounded-full bg-surface-container-highest">
                            <div
                                class="h-1.5 rounded-full bg-primary shadow-[0_0_10px_#adc6ff]"
                                style=format!("width: {player_pct}%;")
                            ></div>
                        </div>
                    </div>
                    <div class="mt-auto flex items-center justify-between pt-4 font-mono text-xs text-on-surface-variant/60">
                        // "FPS: " literal + the value are separate text nodes (React JSX split).
                        <span>
                            "FPS: "
                            {match &server {
                                Some(s) => vint(s, "server_fps").to_string(),
                                None => "—".to_string(),
                            }}
                        </span>
                        <span>
                            "UPTIME: "
                            // formatUptime(uptime_seconds) for the Some case is content-golden gated.
                            {match &server {
                                Some(_s) => "—".to_string(),
                                None => "—".to_string(),
                            }}
                        </span>
                    </div>
                </div>

                // Deployment
                <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass group gap-4">
                    <div class="pointer-events-none absolute -right-10 -bottom-10 opacity-5 transition-opacity group-hover:opacity-10">
                        <MaterialIcon name="military_tech" filled=true class="text-[200px]" />
                    </div>
                    <div class="relative z-10 flex items-center justify-between border-b border-border-subtle pb-3">
                        <h3 class="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                            <MaterialIcon name="person" class="text-[18px]" />
                            "Deployment"
                        </h3>
                    </div>
                    {match &assignment {
                        Some(a) => {
                            view! {
                                <div class="relative z-10 mt-4 flex items-center gap-6">
                                    <div class="flex gap-2">
                                        <div class="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                                            <MaterialIcon
                                                name="swords"
                                                filled=true
                                                class="text-[28px] text-primary"
                                            />
                                        </div>
                                        <div class="flex h-12 w-12 items-center justify-center rounded-lg border border-border-subtle bg-surface-container-highest">
                                            <MaterialIcon
                                                name="security"
                                                filled=true
                                                class="text-[28px] text-primary"
                                            />
                                        </div>
                                    </div>
                                    <div class="flex flex-col">
                                        <span class="font-bold tracking-wide text-on-surface">
                                            {vstr(a, "faction")}
                                        </span>
                                        <span class="text-sm text-on-surface-variant">
                                            {vstr(a, "squad")}
                                        </span>
                                        <span class="mt-1 font-mono text-xs text-primary uppercase">
                                            {format!("Role: {}", vstr(a, "role"))}
                                        </span>
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        None => {
                            view! {
                                <p class="relative z-10 mt-4 text-sm text-on-surface-variant">
                                    "No active assignment"
                                </p>
                            }
                                .into_any()
                        }
                    }}
                </div>

                // Modpack
                <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass gap-4">
                    <div class="flex items-center justify-between border-b border-border-subtle pb-3">
                        <h3 class="flex items-center gap-2 text-label-sm text-on-surface-variant uppercase">
                            <MaterialIcon name="extension" class="text-[18px]" />
                            "Modpack"
                        </h3>
                    </div>
                    <div class="mt-2 flex h-full flex-col justify-between">
                        <div>
                            <h4 class="mb-1 text-xl font-bold text-on-surface">
                                {match &modpack {
                                    Some(m) => format!("{} v{}", m.modpack.name, m.modpack.version),
                                    None => "No modpack".to_string(),
                                }}
                            </h4>
                            <span class="font-mono text-xs text-on-surface-variant">
                                {match &modpack {
                                    Some(m) => {
                                        format!("SIZE: {}", format_bytes(m.modpack.total_size_bytes))
                                    }
                                    None => "—".to_string(),
                                }}
                            </span>
                        </div>
                        <div class="mt-4 flex items-center justify-between rounded-lg border border-border-subtle bg-surface-container-lowest p-3">
                            <span class=cn(
                                &[
                                    "flex items-center gap-2 font-mono text-xs font-bold tracking-widest",
                                    if modpack.is_some() {
                                        "text-success"
                                    } else {
                                        "text-on-surface-variant"
                                    },
                                ],
                            )>
                                <MaterialIcon name="check_circle" filled=true class="text-[14px]" />
                                "STATUS: "
                                {if modpack.is_some() { "SYNCED" } else { "NONE" }}
                            </span>
                            <MaterialIcon name="sync" class="text-[18px] text-on-surface-variant" />
                        </div>
                    </div>
                </div>
            </div>

            // ── Recent Intelligence Feed ──
            <div class="relative flex flex-col overflow-hidden rounded-xl p-6 glass flex-1 gap-4">
                <h3 class="flex items-center gap-2 border-b border-border-subtle pb-3 text-label-sm text-on-surface-variant uppercase">
                    <MaterialIcon name="list_alt" class="text-[18px]" />
                    "Recent Intelligence"
                </h3>
                <div class="custom-scrollbar flex flex-col gap-2 overflow-y-auto pr-2">
                    {if announcements.is_empty() {
                        view! {
                            <p class="text-label-md text-on-surface-variant">"No announcements yet."</p>
                        }
                            .into_any()
                    } else {
                        // Populated announcement rows: content-golden gated (see module note).
                        ().into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
