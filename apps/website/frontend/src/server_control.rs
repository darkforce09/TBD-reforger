//! Server Control (/admin/server) — ported from pages/utility.tsx `ServerControlPage`. `<AdminGate>`
//! → a transparent `SplitPane` over the topo/frosted encasing: a managed-server list (master) + the
//! selected server's command surface (header actions + telemetry grid + RCON console). Fully
//! MOCK-driven (client `MOCK_SERVERS`) until multi-server management is wired.
//!
//! **Gate scope (this slice):** the default render — `MOCK_SERVERS[0]` ("TBD Main Server") selected —
//! is byte-exact-verified (deterministic mock). Server switching + the RCON console send / quick
//! actions (local echo) are behavior (T-interaction) — a follow-up.
#![allow(dead_code)]
use crate::ui::{cn, AdminGate, MaterialIcon};
use leptos::prelude::*;

struct Server {
    id: &'static str,
    name: &'static str,
    ip: &'static str,
    status: &'static str,
    players: i64,
    max_players: i64,
    uptime: &'static str,
    terrain: &'static str,
    mission: &'static str,
    fps: i64,
    mod_config: &'static str,
    log: &'static [&'static str],
}

/// (dot class, label, pulse) per status — mirrors STATUS_META.
fn status_meta(status: &str) -> (&'static str, &'static str, bool) {
    match status {
        "online" => ("bg-success", "Online", true),
        "starting" => ("bg-tactical-yellow", "Starting", true),
        _ => ("bg-outline", "Offline", false),
    }
}

const MOCK_SERVERS: &[Server] = &[
    Server {
        id: "main",
        name: "TBD Main Server",
        ip: "198.51.100.24:2001",
        status: "online",
        players: 55,
        max_players: 64,
        uptime: "3d 14h 22m",
        terrain: "Everon",
        mission: "Operation Iron Veil",
        fps: 44,
        mod_config: "Core Modern v2.4.1",
        log: &[
            "[12:00:01] Server initialized on Everon",
            "[12:00:02] Loading modpack: Core Modern v2.4.1 (6 mods)",
            "[12:00:05] RCON listener bound to 0.0.0.0:19999",
            "[12:01:10] Player \"Reaper\" connected (76561198000000001)",
            "[12:03:42] Player \"Hawk\" connected (76561198000000002)",
            "[12:07:15] Mission \"Operation Iron Veil\" started",
        ],
    },
    Server {
        id: "training",
        name: "TBD Training Ground",
        ip: "198.51.100.24:2011",
        status: "online",
        players: 8,
        max_players: 32,
        uptime: "1d 02h 09m",
        terrain: "Arland",
        mission: "Live-Fire Range",
        fps: 60,
        mod_config: "Training Pack v1.0",
        log: &[],
    },
    Server {
        id: "event-01",
        name: "Event Server 01",
        ip: "198.51.100.25:2021",
        status: "starting",
        players: 0,
        max_players: 64,
        uptime: "00h 00m",
        terrain: "Everon",
        mission: "—",
        fps: 0,
        mod_config: "Event Pack v0.9",
        log: &[],
    },
];

const QUICK_ACTIONS: &[(&str, &str)] = &[
    ("Change Map", "map"),
    ("Swap Modpack", "extension"),
    ("Global Broadcast", "campaign"),
    ("Force Restart", "restart_alt"),
];

#[component]
pub fn ServerControlPage() -> impl IntoView {
    // selectedId defaults to MOCK_SERVERS[0] → "main".
    let selected = &MOCK_SERVERS[0];
    view! {
        <AdminGate>
            <div class="relative h-full w-full overflow-hidden">
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                    <crate::split_pane::SplitPane
                        transparent=true
                        master_width="17rem"
                        master_header=master_header().into_any()
                        master=server_list(selected.id).into_any()
                        detail=server_detail(selected).into_any()
                    />
                </div>
            </div>
        </AdminGate>
    }
}

fn master_header() -> impl IntoView {
    view! {
        <h1 class="w-full text-label-md font-semibold tracking-wide text-on-surface uppercase">
            "Servers"
            <span class="ml-2 font-mono text-code-md text-outline">{MOCK_SERVERS.len() as i64}</span>
        </h1>
    }
}

fn server_list(selected_id: &'static str) -> impl IntoView {
    MOCK_SERVERS
        .iter()
        .map(move |s| {
            let (dot, label, pulse) = status_meta(s.status);
            let active = s.id == selected_id;
            let btn = cn(&[
                "flex items-center gap-3 rounded-lg border-l-4 px-3 py-3 text-left transition-all duration-200",
                if active {
                    "border-primary bg-primary/15"
                } else {
                    "border-transparent hover:bg-white/[0.03]"
                },
            ]);
            let ping = cn(&["absolute inline-flex h-full w-full animate-ping rounded-full opacity-60", dot]);
            let solid = cn(&["relative inline-flex size-2.5 rounded-full", dot]);
            // name span cn(): text-label-md dropped by twMerge vs trailing text-{color}.
            let name_class = if active {
                "block truncate font-medium text-on-surface"
            } else {
                "block truncate font-medium text-on-surface-variant"
            };
            view! {
                <button type="button" class=btn>
                    <span class="relative flex size-2.5 shrink-0">
                        {pulse.then(|| view! { <span class=ping.clone()></span> })}
                        <span class=solid></span>
                    </span>
                    <span class="min-w-0 flex-1">
                        <span class=name_class>{s.name}</span>
                        <span class="block font-mono text-code-md text-outline">{label}</span>
                    </span>
                </button>
            }
        })
        .collect_view()
}

fn server_detail(s: &'static Server) -> impl IntoView {
    view! {
        <div class="flex h-full min-w-0 flex-1 flex-col">
            <header class="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
                <div class="min-w-0">
                    <h2 class="truncate text-headline-lg text-on-surface">{s.name}</h2>
                    <div class="mt-2 inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                        <MaterialIcon name="lan" class="text-[16px] text-on-surface-variant" />
                        <span class="font-mono text-code-md text-on-surface">{s.ip}</span>
                    </div>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        <MaterialIcon name="restart_alt" class="text-[18px]" />
                        "Restart"
                    </button>
                    <button
                        type="button"
                        class="flex items-center gap-1.5 rounded-full border border-error-alert/30 px-4 py-2.5 text-label-md text-error-alert transition hover:bg-error-alert/10"
                    >
                        <MaterialIcon name="stop" class="text-[18px]" />
                        "Stop"
                    </button>
                    <button
                        type="button"
                        class="flex items-center gap-2 rounded-full bg-action px-6 py-2.5 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                    >
                        <MaterialIcon name="rocket_launch" class="text-[18px]" />
                        "LAUNCH & CONNECT"
                    </button>
                </div>
            </header>
            <div class="grid shrink-0 grid-cols-3 divide-x divide-white/10 border-b border-white/5">
                {telemetry_col(
                    "Active Personnel",
                    &format!("{} / {}", s.players, s.max_players),
                    "Uptime",
                    s.uptime,
                )}
                {telemetry_col("Terrain", s.terrain, "Active Mission", s.mission)}
                {telemetry_col(
                    "Server FPS",
                    &format!("{} Hz", s.fps),
                    "Mod Configuration",
                    s.mod_config,
                )}
            </div>
            {rcon_console(s.log)}
        </div>
    }
}

fn telemetry_col(
    primary_label: &str,
    primary_value: &str,
    secondary_label: &str,
    secondary_value: &str,
) -> impl IntoView {
    let (pl, pv, sl, sv) = (
        primary_label.to_string(),
        primary_value.to_string(),
        secondary_label.to_string(),
        secondary_value.to_string(),
    );
    view! {
        <div class="px-6 py-6">
            <p class="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                {pl}
            </p>
            <p class="mt-1 truncate font-mono text-3xl font-bold tracking-tight text-on-surface">
                {pv}
            </p>
            <p class="mt-4 font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                {sl}
            </p>
            <p class="mt-1 truncate text-label-md text-on-surface">{sv}</p>
        </div>
    }
}

fn rcon_console(log: &'static [&'static str]) -> impl IntoView {
    view! {
        <section class="flex min-h-0 flex-1 flex-col bg-surface/40">
            <div class="flex flex-wrap items-center gap-3 border-b border-white/5 bg-surface-container/30 p-4">
                <span class="text-label-sm tracking-wider text-on-surface-variant uppercase">
                    "Quick Actions:"
                </span>
                {QUICK_ACTIONS
                    .iter()
                    .map(|(label, icon)| {
                        view! {
                            <button
                                type="button"
                                class="flex items-center gap-1.5 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-label-sm text-on-surface backdrop-blur-md transition hover:bg-white/10"
                            >
                                <MaterialIcon name=*icon class="text-[16px] text-on-surface-variant" />
                                {*label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="flex min-h-0 flex-1 flex-col p-6">
                <div class="mb-3 flex items-center gap-2">
                    <MaterialIcon name="terminal" class="text-[18px] text-on-surface-variant" />
                    <h3 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">
                        "RCON Console"
                    </h3>
                </div>
                <div class="custom-scrollbar min-h-0 flex-1 overflow-y-auto rounded-xl border border-white/5 bg-black/30 p-4 font-mono text-sm leading-relaxed text-on-surface-variant">
                    {log
                        .iter()
                        .map(|line| {
                            let c = cn(&[
                                "whitespace-pre-wrap",
                                if line.starts_with('$') { "text-primary" } else { "" },
                                if line.contains("RCON:") { "text-success" } else { "" },
                            ]);
                            view! { <p class=c>{*line}</p> }
                        })
                        .collect_view()} <div></div>
                </div>
                <div class="mt-3 flex items-center gap-2 rounded-full border border-white/10 bg-white/5 py-1.5 pr-1.5 pl-5 focus-within:border-primary/40">
                    <span class="font-mono text-sm text-on-surface-variant/60">"$"</span>
                    <input
                        value=""
                        placeholder="Send RCON command…"
                        class="flex-1 bg-transparent font-mono text-sm text-on-surface placeholder:text-on-surface-variant/50 outline-none"
                    />
                    <button
                        type="button"
                        aria-label="Send command"
                        class="flex size-9 items-center justify-center rounded-full bg-primary text-on-primary transition hover:bg-primary/80"
                    >
                        <MaterialIcon name="arrow_upward" class="text-[20px]" />
                    </button>
                </div>
            </div>
        </section>
    }
}
