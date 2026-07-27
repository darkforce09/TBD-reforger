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
//! **T-359 — the `Value`-read row is where this file's one silent defect lived, and the cost is
//! worth naming.** `dto.rs::ServerRowDto` is an exact, R-api-pinned description of a `/servers` row
//! (T-306 typed it and its golden together), and it has no `terrain` field because the backend has
//! no such column. But this page is the *only* reader of `/servers` and it reads
//! `DataEnvelope<Value>`, so `ServerRowDto`'s two references in the whole crate are its own
//! definition and its own test. A DTO that only its own golden reads proves the wire, not the page —
//! the mirror of the rule T-306 wrote for `Value`-typed goldens. The `v_*` helpers then finish the
//! job: each ends in `unwrap_or_default()`, so a key the backend never sends is indistinguishable
//! from one it sends empty, and `terrain` rendered a placeholder for a month rather than failing
//! anything. Adopting `DataEnvelope<ServerRowDto>` here would make that class of bug a compile
//! error; it is not done in this slice because `status` must keep its tolerant two-branch parse
//! below (a typed row makes one unparseable status fail the whole envelope and take the page with
//! it), which needs a change in `dto.rs` — not this file's to make.
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
    // T-287 — abort the SSE fetch on route-leave. `AbortController` is `!Send`, so cleanup is the
    // zero-capture `abort_server_status_stream` that reaches the thread_local in `sse.rs` (same
    // pattern as T-189's unload guard). Must register on the *page* owner, not inside the Suspense
    // reactive fragment — a fragment re-run would abort a still-mounted stream.
    #[cfg(target_arch = "wasm32")]
    on_cleanup(crate::sse::abort_server_status_stream);

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
                    .take(crate::dto::PAYLOAD_AUDIT_CHARS)
                    .collect();
                crate::dto::audit_rejected_frame(
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
    // No theater *name* is rendered here, because `/servers` does not carry one.
    //
    // T-359 — this was `v_str(&s, "terrain")` behind a `"Theater Unknown"` fallback, and the
    // fallback was not the edge case, it was the only case. `servers` has six columns —
    // `id, name, ip, port, required_modpack_id, is_active` (`information_schema`, measured) — and
    // `terrain` is not one of them, so `handlers/servers.rs::server_intel` never had it to serve
    // and no response ever carried the key. `v_str` collapses an absent key to `""`, the
    // `is_empty()` branch turned `""` into product copy, and so every server rendered
    // "Theater Unknown" on every load. That is worse than rendering nothing: a permanent
    // placeholder claims the field exists and went missing, when nothing was ever asked for.
    //
    // The theater is real data — it is just not on this row. `matches.terrain` holds it and
    // `server_statuses.current_match_id` is the key to it; both exist today and the seeded primary
    // joins cleanly (`TBD Primary — Everon` → match `…f000-…0003` → `everon`). What is missing is
    // a **route**: the only `matches` endpoint is `POST /api/v1/ingest/match-results`, so the SPA
    // holds the foreign key with no way to dereference it. The fix is one `LEFT JOIN` in
    // `handlers/servers.rs::server_intel` surfacing `terrain` on `ServerIntelDto`, then the field
    // on `dto.rs::ServerRowDto` and a recaptured `/servers` golden. No migration and no ingest
    // change: the game server already sends terrain, on `MatchInput` (`handlers/telemetry.rs:341`),
    // and storing it on `servers` would be the wrong shape anyway — a server's theater belongs to
    // the match running on it, and rotates with it, while `servers` is static config.
    //
    // Do **not** pre-declare `terrain` on the DTO ahead of that route. An `Option<String>` with
    // `skip_serializing_if` round-trips absent → `None` → absent, so the R-api gate would stay
    // green over a field the backend never sends — the same "the gate asserts nothing" defect
    // T-306 deleted the `#[serde(flatten)] extra` catch-all to prevent. The field arrives together
    // with its golden, or not at all; `mod t359` below is the tripwire for the day it does.

    let copy_address = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::toast::use_toasts();
            if let Some(win) = web_sys::window() {
                let _ = win
                    .navigator()
                    .clipboard()
                    .write_text(&copy_text.get_value());
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
                            // `THEATER_IMAGE` is one hardcoded const shared by every server and
                            // every terrain, so the alt text describes the decoration rather than
                            // naming a theater it cannot know (matching `dashboard.rs`'s
                            // `alt="Operation theater"`). It previously read
                            // "Theater Unknown terrain", repeating the placeholder to screen
                            // readers.
                            <img
                                alt="Theater of operations"
                                src=THEATER_IMAGE
                                class="h-full w-full object-cover transition-transform duration-700 group-hover:scale-105"
                            />
                            <div class="absolute inset-0 bg-gradient-to-t from-surface-container-highest/90 via-surface-container-highest/20 to-transparent"></div>
                            <div class="absolute bottom-3 left-3 right-3 flex items-center justify-between">
                                <div>
                                    // The mission line is the one fact this column actually has,
                                    // and it is honest in both states: a live `current_match_id`
                                    // names the running match, and its absence is a true statement
                                    // ("No Active Mission"), not a stand-in for data that failed to
                                    // arrive. The theater-name span that used to sit above it is
                                    // gone with the phantom `terrain` read; the `mt-0.5` went with
                                    // it, since this span no longer follows anything.
                                    <span class="block text-label-sm text-primary">
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
mod t359 {
    /// The committed `/servers` golden must carry **no** `terrain` key on any row.
    ///
    /// An assert-absent test, deliberately. It pins the measured fact the theater-name removal
    /// rests on, and — more usefully — it is the tripwire for whoever lands the backend join: the
    /// day `handlers/servers.rs::server_intel` surfaces `terrain` and this golden is recaptured,
    /// this test fails and the message says to put the readout back. Without it the next agent
    /// either re-derives the whole investigation, or adds the field to the DTO and leaves the panel
    /// silent — which is how a fix for "renders a placeholder" becomes "renders nothing, forever".
    ///
    /// It reads the same fixture corpus as the R-api gate (`dto.rs` `const FX`), so it moves with
    /// the goldens rather than duplicating a copy of the wire.
    #[test]
    fn servers_golden_carries_no_terrain() {
        const GOLDEN: &str = include_str!("../tests/fixtures/api/GET__servers.json");
        let v: serde_json::Value = serde_json::from_str(GOLDEN).expect("golden parses");
        let rows = v["data"].as_array().expect("golden has a `data` array");
        // An empty corpus would pass the loop below while proving nothing.
        assert!(!rows.is_empty(), "golden must carry rows to assert against");
        for (i, r) in rows.iter().enumerate() {
            assert!(
                r.get("terrain").is_none(),
                "GET /servers row {i} now carries `terrain` — the backend join has landed. Restore \
                 the theater-name readout in `server_panel` (see the T-359 note there), type it \
                 through `dto.rs::ServerRowDto`, and delete this test."
            );
        }
    }
}
