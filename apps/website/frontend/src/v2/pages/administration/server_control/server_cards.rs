//! The server picker and the selected server's card: its identity, its controls and its telemetry.
//!
//! **Role:** the master list of configured servers, the header with the restart, stop and launch
//! controls, and the three telemetry columns under it, plus the small readings they format.
//! **Position:** the two panes of the server control screen, above the console.
//! **Signals & state:** the list writes `selected_id`; the card reads `busy` to disable its
//! controls while a request is out, and passes `console_log` and `command` on to the console.
//! **Invariants:** the card shows only what `GET /servers` actually carries. Terrain and the active
//! mission are not on that payload, so they read as a dash rather than as invented values. Stop has
//! no endpoint of any kind and is disabled with copy that says so; launch cannot be done from a
//! browser and says that instead of pretending. A server with no telemetry reads as zeros and
//! dashes rather than as an offline server that happens to have numbers.
#![allow(dead_code)]

use super::rcon::{post_rcon, rcon_body_restart};
use super::rcon_console::rcon_console;
use crate::v2::core::api::dto::{ModpackDto, ServerRowDto, ServerStatusDto};
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// Seconds as `Nd HHh MMm`, dropping the day part when there is none.
pub(super) fn format_uptime(seconds: i64) -> String {
    let d = seconds / 86_400;
    let h = (seconds % 86_400) / 3600;
    let m = (seconds % 3600) / 60;
    if d > 0 {
        format!("{d}d {h:02}h {m:02}m")
    } else {
        format!("{h:02}h {m:02}m")
    }
}

/// An address and port as one `ip:port` string.
pub(super) fn format_endpoint(ip: &str, port: i64) -> String {
    format!("{ip}:{port}")
}

/// The status word a server's telemetry maps to.
pub(super) fn status_label(online: bool) -> &'static str {
    if online {
        "online"
    } else {
        "offline"
    }
}

/// The dot colour, label and whether-to-pulse for a status word.
///
/// Anything unrecognised reads as offline, which is the safe direction: a status this table has not
/// heard of is not evidence that the server is up.
pub(super) fn status_meta(status: &str) -> (&'static str, &'static str, bool) {
    match status {
        "online" => ("bg-success", "Online", true),
        "starting" => ("bg-tactical-yellow", "Starting", true),
        _ => ("bg-outline", "Offline", false),
    }
}

/// A modpack as `name vversion`, or a dash when the server requires none.
pub(super) fn modpack_label(mp: Option<&ModpackDto>) -> String {
    match mp {
        Some(m) => format!("{} v{}", m.modpack.name, m.modpack.version),
        None => "—".to_string(),
    }
}

/// The server the screen opens on: the active one, else the first, else none.
pub(super) fn pick_default_id(servers: &[ServerRowDto]) -> Option<String> {
    servers
        .iter()
        .find(|s| s.is_active)
        .or_else(|| servers.first())
        .map(|s| s.id.clone())
}

/// One selectable row per configured server, with a status dot and its label.
pub(super) fn server_list(
    servers: &[ServerRowDto],
    selected_id: RwSignal<String>,
) -> impl IntoView {
    servers
        .iter()
        .cloned()
        .map(|s| {
            let id = s.id.clone();
            let id_click = id.clone();
            let name = s.name.clone();
            let online = s.status.as_ref().is_some_and(|st| st.is_online);
            let status = status_label(online);
            view! {
                {move || {
                    let (dot, label, pulse) = status_meta(status);
                    let active = selected_id.get() == id;
                    let btn = cn(&[
                        "flex items-center gap-3 rounded-lg border-l-4 px-3 py-3 text-left transition-all duration-200",
                        if active {
                            "border-primary bg-primary/15"
                        } else {
                            "border-transparent hover:bg-white/[0.03]"
                        },
                    ]);
                    let ping = cn(&[
                        "absolute inline-flex h-full w-full animate-ping rounded-full opacity-60",
                        dot,
                    ]);
                    let solid = cn(&["relative inline-flex size-2.5 rounded-full", dot]);
                    let name_class = if active {
                        "block truncate font-medium text-on-surface"
                    } else {
                        "block truncate font-medium text-on-surface-variant"
                    };
                    let id_click = id_click.clone();
                    let name = name.clone();
                    view! {
                        <button
                            type="button"
                            class=btn
                            on:click=move |_| selected_id.set(id_click.clone())
                        >
                            <span class="relative flex size-2.5 shrink-0">
                                {pulse.then(|| view! { <span class=ping.clone()></span> })}
                                <span class=solid></span>
                            </span>
                            <span class="min-w-0 flex-1">
                                <span class=name_class>{name.clone()}</span>
                                <span class="block font-mono text-code-md text-outline">{label}</span>
                            </span>
                        </button>
                    }
                }}
            }
        })
        .collect_view()
}

/// The selected server's card: identity, controls, telemetry, and the console below them.
pub(super) fn server_detail(
    s: ServerRowDto,
    console_log: RwSignal<Vec<String>>,
    busy: RwSignal<bool>,
    command: RwSignal<String>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let toasts = crate::v2::core::ui::toast::use_toasts();
    let server_id = s.id.clone();
    let name = s.name.clone();
    let endpoint = format_endpoint(&s.ip, s.port);
    let status = s.status.clone();
    let mod_label = modpack_label(s.required_modpack.as_ref());

    let (players, max_players, uptime, fps) = match &status {
        Some(st) => (
            st.player_count,
            st.max_players,
            format_uptime(st.uptime_seconds),
            format!("{:.1} Hz", st.server_fps),
        ),
        None => (0, 0, "—".to_string(), "—".to_string()),
    };
    // Terrain is not on the server payload at all, so it reads as a dash rather than as an
    // invented map name.
    let terrain = "—".to_string();
    let mission = status
        .as_ref()
        .and_then(|st: &ServerStatusDto| st.current_match_id.clone())
        .unwrap_or_else(|| "—".to_string());

    let restart_id = server_id.clone();
    let on_restart = {
        let console_log = console_log;
        let busy = busy;
        let toasts = toasts;
        move |_| {
            post_rcon(
                store,
                restart_id.clone(),
                rcon_body_restart(),
                "$ restart".into(),
                console_log,
                busy,
                toasts,
            );
        }
    };

    let on_launch = move |_| {
        #[cfg(target_arch = "wasm32")]
        toasts.message("Launch requires the Reforger client");
        #[cfg(not(target_arch = "wasm32"))]
        let _ = toasts;
    };

    view! {
        <div class="flex h-full min-w-0 flex-1 flex-col">
            <header class="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
                <div class="min-w-0">
                    <h2 class="truncate text-headline-lg text-on-surface">{name}</h2>
                    <div class="mt-2 inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                        <MaterialIcon name="lan" class="text-[16px] text-on-surface-variant" />
                        <span class="font-mono text-code-md text-on-surface">{endpoint}</span>
                    </div>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        data-testid="server-control-restart"
                        prop:disabled=move || busy.get()
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5 disabled:opacity-50"
                        on:click=on_restart
                    >
                        <MaterialIcon name="restart_alt" class="text-[18px]" />
                        "Restart"
                    </button>
                    <button
                        type="button"
                        data-testid="server-control-stop"
                        disabled=true
                        title="No Stop HTTP or RCON endpoint — process stop is not wired"
                        class="flex items-center gap-1.5 rounded-full border border-error-alert/30 px-4 py-2.5 text-label-md text-error-alert opacity-50"
                    >
                        <MaterialIcon name="stop" class="text-[18px]" />
                        "Stop"
                    </button>
                    <button
                        type="button"
                        data-testid="server-control-launch"
                        class="flex items-center gap-2 rounded-full bg-action px-6 py-2.5 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                        on:click=on_launch
                    >
                        <MaterialIcon name="rocket_launch" class="text-[18px]" />
                        "LAUNCH & CONNECT"
                    </button>
                </div>
            </header>
            <div class="grid shrink-0 grid-cols-3 divide-x divide-white/10 border-b border-white/5">
                {telemetry_col(
                    "Active Personnel",
                    &format!("{players} / {max_players}"),
                    "Uptime",
                    &uptime,
                )}
                {telemetry_col("Terrain", &terrain, "Active Mission", &mission)}
                {telemetry_col("Server FPS", &fps, "Mod Configuration", &mod_label)}
            </div>
            {rcon_console(
                server_id,
                console_log,
                busy,
                command,
            )}
        </div>
    }
}

/// One telemetry column: a large primary reading, and a smaller secondary one under it.
pub(super) fn telemetry_col(
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
