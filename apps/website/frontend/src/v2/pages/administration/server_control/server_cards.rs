//! The server picker and the selected server's card: its identity, its telemetry, its fleet command
//! console and its mission deployments.
//!
//! **Role:** the master list of configured servers, the header with the credential and launch
//! controls, the three telemetry columns under it with the small readings they format, and the two
//! sections that act on the server — fleet commands and mission deployments.
//! **Position:** the two panes of the server control screen.
//! **Signals & state:** the list writes `selected_id`. The card creates the state of the server it
//! shows — its command console, its deployments panel and its credential sheet — so switching
//! servers never shows one server's commands, deployments, credentials or a secret just issued for
//! it under another's name.
//! **Invariants:** the card shows only what `GET /servers` carries: a server with no telemetry reads
//! as zeros and dashes rather than as an offline server that happens to have numbers, and a server
//! between matches reads its terrain as a dash. Launching the game cannot be done from a browser, so
//! the launch control says that instead of pretending. The kick form is offered the runtime session
//! that confirmed the server's newest confirmed deployment — the one session id the web reads.

use super::fleet_commands::{command_history, command_requests, CommandConsole};
use super::machine_credentials::{credential_sheet, CredentialPanel};
use super::mission_deployments::{deployment_list, deployment_request, DeploymentPanel};
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

/// The theatre the current match runs on, capitalised, or a dash between matches.
pub(super) fn terrain_reading(server: &ServerRowDto) -> String {
    match server.terrain.as_deref().filter(|t| !t.is_empty()) {
        Some(terrain) => {
            let mut chars = terrain.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        }
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

/// The selected server's card: identity, telemetry, the command console and the deployments.
pub(super) fn server_detail(s: ServerRowDto) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let toasts = crate::v2::core::ui::toast::use_toasts();
    let name = s.name.clone();
    let endpoint = format_endpoint(&s.ip, s.port);
    let status = s.status.clone();
    let mod_label = modpack_label(s.required_modpack.as_ref());
    let terrain = terrain_reading(&s);

    let (players, max_players, uptime, fps) = match &status {
        Some(st) => (
            st.player_count,
            st.max_players,
            format_uptime(st.uptime_seconds),
            format!("{:.1} Hz", st.server_fps),
        ),
        None => (0, 0, "—".to_string(), "—".to_string()),
    };
    let mission = status
        .as_ref()
        .and_then(|st: &ServerStatusDto| st.current_match_id.clone())
        .unwrap_or_else(|| "—".to_string());

    let on_launch = move |_| {
        #[cfg(target_arch = "wasm32")]
        toasts.message("Launch requires the Reforger client");
    };

    let credentials = CredentialPanel::new(store, s.id.clone(), name.clone());
    let console = CommandConsole::new(store, toasts, s.id.clone());
    let deployments = DeploymentPanel::new(store, toasts, s.id.clone());
    let suggested_session = Signal::derive(move || deployments.latest_confirmed_session());

    view! {
        <div class="flex min-h-full min-w-0 flex-1 flex-col">
            <header class="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
                <div class="min-w-0">
                    <h2 class="truncate text-headline-lg text-on-surface">{name.clone()}</h2>
                    <div class="mt-2 inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                        <MaterialIcon name="lan" class="text-[16px] text-on-surface-variant" />
                        <span class="font-mono text-code-md text-on-surface">{endpoint}</span>
                    </div>
                </div>
                <div class="flex items-center gap-2">
                    <button
                        type="button"
                        data-testid="server-control-credentials"
                        on:click=move |_| credentials.open_sheet()
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        <MaterialIcon name="key" class="text-[18px]" />
                        "Credentials"
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
            <div class="grid gap-6 p-6 2xl:grid-cols-2">
                <section class="space-y-4" data-testid="server-control-fleet-commands">
                    {section_heading("terminal", "Fleet commands")}
                    {command_requests(console, name, suggested_session)}
                    {command_history(console)}
                </section>
                <section class="space-y-4" data-testid="server-control-deployments">
                    {section_heading("rocket_launch", "Mission deployments")}
                    {deployment_request(deployments)}
                    {deployment_list(deployments)}
                </section>
            </div>
            {credential_sheet(credentials)}
        </div>
    }
}

/// A section's heading: its glyph and its name.
fn section_heading(icon: &'static str, title: &'static str) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <MaterialIcon name=icon class="text-[18px] text-on-surface-variant" />
            <h3 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">{title}</h3>
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
