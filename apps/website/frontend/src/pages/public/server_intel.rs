//! Server Intel (/server-intel) — ported from pages/ServerIntel.tsx. `<AuthGate>` → `/servers`
//! Resource → the frosted command panel over the satellite backdrop.
//!
//! T-159.25: the FULL populated panel — default-server pick (`is_active` else first), the live
//! telemetry grid fed by the **SSE stream** (`sse.rs`, the useServerTelemetry port; stream frames
//! override the row's cached `status`), copy-address to clipboard, LAUNCH stub toast, theater +
//! environment columns, Recent Intelligence shell. Empty DB keeps the byte-verified
//! "No servers configured." golden. Server rows stay `Value`-read (the row shape carries more than
//! the page renders); the SSE frame is the typed `ServerStatusDto`.
//!
//! **T-359 / T-385 — theater readout.** T-359 removed a phantom `terrain` read that always fell
//! through to "Theater Unknown" because `/servers` had no such key. T-385 landed the
//! `matches.terrain` LEFT JOIN on the route and the golden; the panel reads `terrain` again and
//! only labels a theater when the key carries a non-empty string (no permanent placeholder).
#![allow(dead_code)]
use crate::core::datefmt::format_uptime;
use crate::core::dto::{DataEnvelope, ServerStatusDto};
use crate::core::ui::{cn, AuthGate, MaterialIcon};
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

/// The FPS the panel calls "Optimal", from the React original's `status.server_fps >= 30`.
///
/// **Not the same number as the backend's alert floor, and not a mistake.** `handlers/telemetry.rs`
/// raises its "FPS dropped below 20" audit at `LOW_FPS_THRESHOLD = 20.0`; this is a cosmetic label
/// on a live panel. Unifying them would either start calling a 25-FPS server "Optimal" or start
/// implying an alert fired when none did.
///
/// `f64` since T-306 — the wire is `numeric(5,1)`, so an `i64` threshold no longer type-checks
/// against the field, which is how this constant came to be named at all.
const FPS_OPTIMAL_FLOOR: f64 = 30.0;

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
    let store = expect_context::<crate::core::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::core::client::api_get::<DataEnvelope<Value>>(store, "/servers")
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
    // T-287 — abort the SSE fetch on route-leave. `AbortController` is `!Send`, so cleanup is the
    // zero-capture `abort_server_status_stream` that reaches the thread_local in `sse.rs` (same
    // pattern as T-189's unload guard). Must register on the *page* owner, not inside the Suspense
    // reactive fragment — a fragment re-run would abort a still-mounted stream.
    #[cfg(target_arch = "wasm32")]
    on_cleanup(crate::core::sse::abort_server_status_stream);

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
                                    crate::core::sse::stream_server_status(
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
                style=format!("background-image: url(\"{COMMAND_MAP_IMAGE}\");")
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
    //
    // T-306 — this was `.and_then(|v| serde_json::from_value(..).ok())`, and the `.ok()` absorbed
    // two completely different events as if they were one:
    //
    //   * `"status": null` (or absent) — a registered server with no telemetry row yet. Entirely
    //     expected; the third row of the committed `/servers` golden is exactly this. Warning here
    //     would cry wolf on every render of a healthy staging box.
    //   * a **present** status object the DTO cannot read — a wire/DTO contract breach, which is
    //     what a `server_fps` of `58.7` against an `i64` field was for a month.
    //
    // So the two are split rather than blanket-logged: absent stays silent, present-but-unparseable
    // is audited through the same deduped channel as a rejected SSE frame. Still best-effort — the
    // panel renders with the live stream's frames and its own `—` fallbacks either way.
    let row_status: Option<ServerStatusDto> = match s.get("status") {
        None | Some(Value::Null) => None,
        Some(v) => match serde_json::from_value::<ServerStatusDto>(v.clone()) {
            Ok(dto) => Some(dto),
            Err(e) => {
                let payload: String = v
                    .to_string()
                    .chars()
                    .take(crate::core::dto::PAYLOAD_AUDIT_CHARS)
                    .collect();
                crate::core::dto::audit_rejected_frame(
                    "server_intel cached row status",
                    &e.to_string(),
                    &payload,
                );
                None
            }
        },
    };
    let row_status = StoredValue::new(row_status);
    let live = move || live_sig.get().or_else(|| row_status.get_value());
    let modpack = s.get("required_modpack").cloned().filter(|m| !m.is_null());
    // T-385 — theater name from the join-sourced `terrain` key. Absent/null/empty means no live
    // match theater (honest absence). Do not invent a permanent placeholder label when the key is
    // empty — that was the T-359 defect.
    let terrain_name = {
        let t = v_str(&s, "terrain");
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    };
    let theater_alt = terrain_name
        .as_ref()
        .map(|n| format!("{n} terrain"))
        .unwrap_or_else(|| "Theater of operations".into());

    // T-773 — copy the connect address through the ONE clipboard path in the crate.
    //
    // This used to be `let _ = win.navigator().clipboard().write_text(…)` followed unconditionally
    // by `toasts.success("Server address copied")`. The promise was DROPPED. `writeText` rejects on
    // an insecure context (plain http on a non-localhost host — exactly how staging is reached), on
    // an unfocused document, and on a denied permission; in every one of those the rejection went
    // in the bin and the operator was told the copy worked. They then paste whatever was on the
    // clipboard before into a connect dialog and blame the server.
    //
    // `mission_commands::write_clipboard` (T-698) already resolves `navigator.clipboard` through
    // Reflect and refuses with a readable message when it is absent, awaits the promise, and toasts
    // success on the RESOLVE arm only. It is reused verbatim rather than re-derived here: a second
    // clipboard vocabulary is how the two drift apart and one of them starts lying again.
    let copy_address = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::mission_commands::write_clipboard(
            copy_text.get_value(),
            "Server address copied".to_string(),
            crate::core::toast::use_toasts(),
        );
    };
    let launch_stub = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::core::toast::use_toasts().success("Launch requires the Reforger client");
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
                                if live().map(|l| l.server_fps >= FPS_OPTIMAL_FLOOR).unwrap_or(false)
                                {
                                    "text-tactical-yellow"
                                } else {
                                    "text-error"
                                }
                            }>
                                {move || {
                                    live()
                                        .map(|l| {
                                            let opt = if l.server_fps >= FPS_OPTIMAL_FLOOR {
                                                "Optimal"
                                            } else {
                                                "Low"
                                            };
                                            // `{}` on an f64 prints `58.7` for a fractional value and
                                            // `30` for a whole one — the same text React's
                                            // `${status.server_fps}` produced from a JS number. Do not
                                            // reach for `{:.1}`: that would print `30.0` where the
                                            // byte-verified original prints `30`.
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
                                alt=theater_alt.clone()
                                src=THEATER_IMAGE
                                class="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                            />
                            <div class="absolute inset-0 bg-gradient-to-t from-surface-container-highest/90 via-surface-container-highest/20 to-transparent"></div>
                            <div class="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                                <div>
                                    {terrain_name.clone().map(|name| {
                                        view! {
                                            <span class="block text-label-md text-on-surface">
                                                {name}
                                            </span>
                                        }
                                    })}
                                    <span class=move || {
                                        if terrain_name.is_some() {
                                            "mt-0.5 block text-label-sm text-primary".to_string()
                                        } else {
                                            "block text-label-sm text-primary".to_string()
                                        }
                                    }>
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

#[cfg(test)]
mod t385 {
    /// Class-R flip of T-359's assert-absent tripwire: the committed `/servers` golden **must**
    /// carry `terrain` from the match join. Primary (current_match_id set) pins `"everon"`;
    /// unmatched rows pin explicit JSON `null` (same encoding as `status`).
    ///
    /// Removing the key, or softening it to Option+skip ahead of a live value, makes this RED —
    /// which is the whole point.
    #[test]
    fn servers_golden_carries_terrain_from_match_join() {
        const GOLDEN: &str = include_str!("../../../tests/fixtures/api/GET__servers.json");
        let v: serde_json::Value = serde_json::from_str(GOLDEN).expect("golden parses");
        let rows = v["data"].as_array().expect("golden has a `data` array");
        assert!(!rows.is_empty(), "golden must carry rows to assert against");

        let primary = &rows[0];
        assert_eq!(
            primary.get("terrain").and_then(|t| t.as_str()),
            Some("everon"),
            "primary server (current_match_id → matches.terrain) must carry terrain \"everon\""
        );

        for (i, r) in rows.iter().enumerate() {
            assert!(
                r.as_object().expect("row object").contains_key("terrain"),
                "GET /servers row {i} must carry an explicit `terrain` key (string or null) — \
                 T-385 Class-R. Do not drop it or hide it behind skip_serializing_if."
            );
        }

        // Unmatched rows: explicit null, not absent.
        assert!(
            rows[1].get("terrain").is_some_and(|t| t.is_null()),
            "secondary (no current_match_id) must serialize terrain as null"
        );
        assert!(
            rows[2].get("terrain").is_some_and(|t| t.is_null()),
            "staging (no status) must serialize terrain as null"
        );
    }

    /// Panel source must read `terrain` again — deleting the readout without removing the route
    /// field recreates the "wire proves nothing about the page" hole T-359 documented.
    #[test]
    fn server_panel_reads_terrain_key() {
        const SRC: &str = include_str!("server_intel.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests");
        let panel = production
            .split("fn server_panel")
            .nth(1)
            .expect("server_panel fn")
            .split("\nfn ")
            .next()
            .expect("panel body");
        assert!(
            panel.contains("v_str(&s, \"terrain\")"),
            "server_panel must read the terrain key restored by T-385"
        );
        assert!(
            !panel.contains("Theater Unknown"),
            "do not restore the T-359 permanent placeholder inside server_panel"
        );
    }
}

#[cfg(test)]
mod t773 {
    /// **T-773 — the Copy button may not report a copy it never confirmed.**
    ///
    /// The shipped defect was three lines: `let _ = win.navigator().clipboard().write_text(…)`
    /// with the promise dropped, then an unconditional `toasts.success("Server address copied")`.
    /// `writeText` rejects on an insecure context, an unfocused document and a denied permission,
    /// so on staging over plain http the toast said "copied" while the clipboard was untouched.
    ///
    /// This is a **source** pin rather than a behavioural one, deliberately and with its limits
    /// stated: the thing under test is a `navigator.clipboard` promise, which does not exist in a
    /// native `cargo test` process at all, and granting a headless browser clipboard permission
    /// would test the browser rather than the button. What can be pinned without a browser is
    /// *which path the button takes* — and since [`crate::mission_commands::write_clipboard`]'s
    /// await-then-report contract is pinned in turn by
    /// `class_r_write_clipboard_toasts_only_on_the_resolve_arm`, the two together say: this button
    /// reaches the one helper, and that helper only claims success after the promise resolved.
    ///
    /// It reads through `class_r_scrub::live_code`, which cuts this test module before scanning —
    /// a bare `include_str!` would match the needles in these very assertions and stay green with
    /// the production code deleted.
    #[test]
    fn class_r_copy_address_routes_through_the_awaited_clipboard_helper() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("server_intel.rs");
        let production = live_code(SRC);
        let body = only_body(&production, "let copy_address = move |_|");

        assert!(
            body.contains("crate::mission_commands::write_clipboard("),
            "the Copy button must copy through the one awaited clipboard helper; got:\n{body}"
        );
        // The two halves of the original defect, each forbidden on its own so that re-introducing
        // either — a raw write, or a success toast the panel decides for itself — is RED.
        assert!(
            !body.contains("write_text"),
            "no raw navigator.clipboard.writeText in the panel — its promise is what got dropped; \
             got:\n{body}"
        );
        assert!(
            !body.contains(".clipboard()"),
            "do not reach for navigator.clipboard here; write_clipboard resolves it and refuses \
             readably when it is absent; got:\n{body}"
        );
        assert!(
            !body.contains("success("),
            "the panel must not toast success itself — only write_clipboard's resolve arm may; \
             got:\n{body}"
        );
    }
}
