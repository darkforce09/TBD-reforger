//! Server Intel (/server-intel) — ported from pages/ServerIntel.tsx. `<AuthGate>` → `/servers`
//! Resource → the frosted command panel over the satellite backdrop.
//!
//! T-159.25: the FULL populated panel — default-server pick (`is_active` else first), the live
//! telemetry grid fed by the **SSE stream** (`sse.rs`, the useServerTelemetry port; stream frames
//! override the row's cached `status`), copy-address to clipboard, LAUNCH stub toast, theater +
//! environment columns, Recent Intelligence shell. Empty DB keeps the byte-verified
//! "No servers configured." golden. Server rows stay `Value`-read (the row shape carries more than
//! the page renders); the SSE frame is the typed `ServerStatusDto`.
#![allow(dead_code)]
use crate::dto::{DataEnvelope, ServerStatusDto};
use crate::ui::{cn, AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

const THEATER_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuBJhklFaKKJXQ3-uOGwrugGr_URw1Dq_3Jslvkc3lEtT4ObLWKv52ipE-EQWEm3QF4HeoY5vA8NcYt_e87d76A14Z48tuHODNidNphecUVm_Zy7NLBRexvt9uUcFOBLTk3RbiSAetUEMYX2BmQMPU-BU-HvmweLf1P4-jc1CjC0jDdMMR-fzb5BVtNID-Ak1iW3MuGzWiO4LfZ4WIPy8Ijk3kcsqRFXVroQ_rZSJ8yw4se-gszeDoVOc8Vp9HL5qLcEAtnI4pFEC4I";
const COMMAND_MAP_IMAGE: &str = "https://lh3.googleusercontent.com/aida-public/AB6AXuBqY9NRsaLKSRk7V0g9XrVkysuxuTRsc8FcMfq76JZujkDPkAAihMyRIw6mOuvFI4tTOwRDvDEhOe-p2Coym8zpmONJeueKLL379Yzecw64o3wzqJMRZdGCA7iBbwrno1hge-AU7AZNCE4XVo9q6IXTH5A2NRf3IToSchzAuj5JUT-Y81VVXfb-Ic4CrnLbV_So9xy2vBIxVHrwDztZ-YuY78DL-Jb5qsgNACRmxHXgRYRrsCxsCJnHBrgj-DD3LUVa31rIo4Arzrc";

fn v_str<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or_default()
}
fn v_i64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or_default()
}
fn v_bool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or_default()
}

/// lib/format.ts `formatUptime` — HH:MM:SS zero-padded.
fn format_uptime(seconds: i64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// lib/defaultServer.ts `pickDefaultServer` (no VITE_DEFAULT_SERVER_NAME env in Leptos dev):
/// first `is_active` row, else the first row.
fn pick_default(servers: &[Value]) -> Option<Value> {
    servers
        .iter()
        .find(|s| v_bool(s, "is_active"))
        .or_else(|| servers.first())
        .cloned()
}

#[component]
pub fn ServerIntelPage() -> impl IntoView {
    view! {
        <AuthGate>
            <ServerIntelInner />
        </AuthGate>
    }
}

#[component]
fn ServerIntelInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DataEnvelope<Value>>(store, "/servers")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    // useServerTelemetry triple; subscribed once the default server is known.
    let live = RwSignal::new(None::<ServerStatusDto>);
    let connected = RwSignal::new(false);
    let sse_error = RwSignal::new(None::<String>);
    let subscribed = RwSignal::new(false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (connected, sse_error, subscribed);

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                servers
                    .get()
                    .map(|opt| match opt {
                        Some(env) => {
                            let server = pick_default(&env.data);
                            #[cfg(target_arch = "wasm32")]
                            if let Some(s) = &server {
                                let id = v_str(s, "id").to_string();
                                if !id.is_empty() && !subscribed.get_untracked() {
                                    subscribed.set(true);
                                    crate::sse::stream_server_status(
                                        store,
                                        id,
                                        live,
                                        connected,
                                        sse_error,
                                    );
                                }
                            }
                            panel(server, live).into_any()
                        }
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn panel(server: Option<Value>, live_sig: RwSignal<Option<ServerStatusDto>>) -> impl IntoView {
    view! {
        <div class="relative h-full overflow-y-auto">
            <div
                class="absolute inset-0 z-0 bg-cover bg-center"
                style=format!("background-image: url('{COMMAND_MAP_IMAGE}')")
            >
                <div class="absolute inset-0 bg-background/80 backdrop-blur-sm"></div>
                <div class="absolute inset-0 bg-gradient-to-t from-background via-transparent to-transparent"></div>
            </div>

            <div class="relative z-10 flex w-full flex-col">
                {match server {
                    None => {
                        view! { <p class="text-on-surface-variant">"No servers configured."</p> }
                            .into_any()
                    }
                    Some(s) => server_panel(s, live_sig).into_any(),
                }}
            </div>
        </div>
    }
}

fn server_panel(s: Value, live_sig: RwSignal<Option<ServerStatusDto>>) -> impl IntoView {
    let name = v_str(&s, "name").to_string();
    let ip = v_str(&s, "ip").to_string();
    let port = v_i64(&s, "port");
    let connect_address = format!("{ip} : {port}");
    let copy_text = StoredValue::new(format!("{ip}:{port}"));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = copy_text;
    // Cached row status (fallback until the first SSE frame lands).
    let row_status: Option<ServerStatusDto> = s
        .get("status")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let row_status = StoredValue::new(row_status);
    let live = move || live_sig.get().or_else(|| row_status.get_value());
    let modpack = s.get("required_modpack").cloned().filter(|m| !m.is_null());
    let terrain_name = {
        let t = v_str(&s, "terrain");
        if t.is_empty() {
            "Theater Unknown".to_string()
        } else {
            t.to_string()
        }
    };

    let copy_address = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
            if let Some(win) = web_sys::window() {
                let _ = win.navigator().clipboard().write_text(&copy_text.get_value());
                toasts.success("Server address copied");
            }
        }
    };
    let launch_stub = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::toast::use_toasts().success("Launch requires the Reforger client");
    };

    view! {
        <div class="flex w-full flex-col overflow-hidden bg-surface-glass backdrop-blur-xl">
            // Panel Header
            <div class="flex flex-col justify-between gap-6 border-b border-white/5 bg-surface/40 px-8 py-6 md:flex-row md:items-center">
                <div>
                    <div class="mb-2 flex items-center gap-3">
                        <div
                            class=move || {
                                cn(
                                    &[
                                        "pulse-dot h-2.5 w-2.5 rounded-full",
                                        if live().map(|l| l.is_online).unwrap_or(false) {
                                            "bg-success"
                                        } else {
                                            "bg-tactical-yellow"
                                        },
                                    ],
                                )
                            }
                            title=move || {
                                if live().map(|l| l.is_online).unwrap_or(false) {
                                    "Server Online"
                                } else {
                                    "Server Offline"
                                }
                            }
                        ></div>
                        <h2 class="text-headline-md uppercase tracking-wider text-on-surface">
                            {name}
                        </h2>
                    </div>
                    <div class="inline-flex items-center gap-2 rounded-md border border-white/5 bg-surface-container px-3 py-1.5 text-code-md text-on-surface-variant">
                        <MaterialIcon name="dns" class="text-[16px]" />
                        <span>{connect_address}</span>
                        <button
                            type="button"
                            on:click=copy_address
                            aria-label="Copy IP"
                            class="ml-2 transition-colors hover:text-primary"
                        >
                            <MaterialIcon name="content_copy" class="text-[16px]" />
                        </button>
                    </div>
                </div>
                <button
                    type="button"
                    on:click=launch_stub
                    class="flex shrink-0 items-center gap-2 rounded-full border border-secondary-container/50 bg-secondary-container px-6 py-3 text-label-md text-on-secondary-container transition-all duration-300 hover:shadow-[0_0_20px_rgba(5,102,217,0.4)]"
                >
                    <MaterialIcon name="play_arrow" filled=true />
                    "LAUNCH & CONNECT"
                </button>
            </div>

            // Telemetry Grid
            <div class="grid grid-cols-1 gap-8 border-b border-white/5 p-8 md:grid-cols-[1fr_2fr_1fr] md:divide-x md:divide-white/10">
                // Column 1: Performance
                <div class="flex flex-col justify-center space-y-4 md:pr-8">
                    <div>
                        <span class="mb-1 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                            "Active Personnel"
                        </span>
                        <div class="flex items-baseline gap-2">
                            <span class="font-mono text-[30px] font-bold leading-tight text-tertiary-container">
                                {move || live().map(|l| l.player_count).unwrap_or(0)}
                            </span>
                            <span class="font-mono text-[20px] font-semibold text-on-surface-variant">
                                "/ " {move || live().map(|l| l.max_players).unwrap_or(0)}
                            </span>
                        </div>
                    </div>
                    <div class="space-y-2 pt-2">
                        <div class="flex items-center justify-between text-code-md text-on-surface-variant">
                            <span>"Uptime:"</span>
                            <span>
                                {move || {
                                    live()
                                        .map(|l| format_uptime(l.uptime_seconds))
                                        .unwrap_or_else(|| "—".into())
                                }}
                            </span>
                        </div>
                        <div class="flex items-center justify-between text-code-md text-on-surface-variant">
                            <span>"Server FPS:"</span>
                            <span class=move || {
                                if live().map(|l| l.server_fps >= 30).unwrap_or(false) {
                                    "text-tactical-yellow"
                                } else {
                                    "text-error"
                                }
                            }>
                                {move || {
                                    live()
                                        .map(|l| {
                                            let opt = if l.server_fps >= 30 { "Optimal" } else { "Low" };
                                            format!("{} ({opt})", l.server_fps)
                                        })
                                        .unwrap_or_else(|| "—".into())
                                }}
                            </span>
                        </div>
                    </div>
                </div>

                // Column 2: Theater of Operations
                <div class="flex flex-col justify-center md:px-8">
                    <span class="mb-3 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                        "Theater of Operations"
                    </span>
                    <a href="/events" class="block focus:outline-none">
                        <div class="group relative aspect-[21/9] w-full cursor-pointer overflow-hidden rounded-lg border border-white/10 transition-all duration-300 hover:ring-2 hover:ring-primary hover:ring-offset-2 hover:ring-offset-background">
                            <img
                                alt=format!("{terrain_name} terrain")
                                src=THEATER_IMAGE
                                class="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                            />
                            <div class="absolute inset-0 bg-gradient-to-t from-surface-container-highest/90 via-surface-container-highest/20 to-transparent"></div>
                            <div class="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                                <div>
                                    <span class="block text-label-md text-on-surface">
                                        {terrain_name.clone()}
                                    </span>
                                    <span class="mt-0.5 block text-label-sm text-primary">
                                        {move || {
                                            live()
                                                .and_then(|l| l.current_match_id)
                                                .filter(|m| !m.is_empty())
                                                .map(|m| {
                                                    let end = m.len().min(8);
                                                    format!("Match {}", &m[..end])
                                                })
                                                .unwrap_or_else(|| "No Active Mission".into())
                                        }}
                                    </span>
                                </div>
                                <MaterialIcon name="map" class="text-on-surface-variant" />
                            </div>
                        </div>
                    </a>
                </div>

                // Column 3: Environment & Mods
                <div class="flex flex-col justify-center space-y-6 md:pl-8">
                    {env_row(
                        "schedule",
                        "text-primary",
                        "Simulated Time",
                        move || live().and_then(|l| l.ingame_time).unwrap_or_else(|| "—".into()),
                    )}
                    {env_row(
                        "rainy",
                        "text-tertiary-container",
                        "Conditions",
                        move || {
                            live().and_then(|l| l.ingame_weather).unwrap_or_else(|| "—".into())
                        },
                    )}
                    <div class="flex items-center gap-4">
                        <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                            <MaterialIcon name="verified" class="text-tactical-yellow" />
                        </div>
                        <div>
                            <span class="block text-label-sm uppercase text-on-surface-variant">
                                "Mod Configuration"
                            </span>
                            <span class="text-body-md text-on-surface">
                                {match modpack {
                                    Some(mp) => {
                                        let label = format!(
                                            "{} v{}",
                                            v_str(&mp, "name"),
                                            v_str(&mp, "version"),
                                        );
                                        let synced = v_bool(&mp, "is_current");
                                        view! {
                                            {label}
                                            " "
                                            {synced
                                                .then(|| {
                                                    view! {
                                                        <span class="text-[12px] text-on-surface-variant">
                                                            "(Synced)"
                                                        </span>
                                                    }
                                                })}
                                        }
                                            .into_any()
                                    }
                                    None => view! { "No modpack required" }.into_any(),
                                }}
                            </span>
                        </div>
                    </div>
                </div>
            </div>

            // Recent Intelligence
            <div class="bg-surface/20 p-8">
                <span class="mb-4 block text-label-sm uppercase tracking-widest text-on-surface-variant">
                    "Recent Intelligence"
                </span>
                <div class="space-y-3">
                    <div class="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                        <span class="shrink-0 text-primary">"[14:02:00Z]"</span>
                        <span class="text-on-surface">
                            "New hostile movement detected in Sector 4"
                        </span>
                    </div>
                    <div class="flex items-center gap-4 border-b border-white/5 py-2 text-code-md">
                        <span class="shrink-0 text-primary">"[13:45:12Z]"</span>
                        <span class="text-on-surface">"Server Uplink maintenance completed"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}

fn env_row(
    icon: &'static str,
    icon_class: &'static str,
    label: &'static str,
    value: impl Fn() -> String + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-4">
            <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-white/5 bg-surface-container">
                <MaterialIcon name=icon class=icon_class />
            </div>
            <div>
                <span class="block text-label-sm uppercase text-on-surface-variant">{label}</span>
                <span class="text-body-md text-on-surface">{move || value()}</span>
            </div>
        </div>
    }
}
