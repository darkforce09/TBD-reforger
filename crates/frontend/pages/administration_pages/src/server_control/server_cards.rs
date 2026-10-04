//! The server picker and the selected server's card: its identity, its telemetry, its fleet command
//! console and its mission deployments.
//!
//! **Role:** the master list of configured servers, the header with the server's address and id
//! and the edit, credential and launch controls, the small readings the card formats, and the two
//! sections that act on the server — fleet commands and mission deployments. The telemetry band
//! under the header is [`super::server_card_telemetry::telemetry_columns`].
//! **Position:** the two panes of the server control screen.
//! **Signals & state:** the list writes the registry's `selected_id`. The card creates the state of
//! the server it shows — its command console, its deployments panel and its credential sheet — so
//! switching servers never shows one server's commands, deployments, credentials or a secret just
//! issued for it under another's name; it reads its row from the [`ServerRegistry`], so an edit,
//! a deactivation or a reactivation shows at once and keeps that state.
//! **Invariants:** the card shows only what `GET /servers` and the registry's writes carry: a
//! server between matches reads its terrain as a dash, and an inactive server (outside the
//! configured fleet) carries an "Inactive" badge in the list and in the card header. Launching the
//! game cannot be done from a browser, so the launch control says that instead of pretending. The
//! kick form is offered the runtime session that confirmed the server's newest confirmed
//! deployment — the one session id the web reads.

#[cfg(target_arch = "wasm32")]
use super::fleet_commands::{CommandConsole, command_history, command_requests};
#[cfg(target_arch = "wasm32")]
use super::machine_credentials::{CredentialPanel, credential_sheet};
#[cfg(target_arch = "wasm32")]
use super::mission_deployments::{DeploymentPanel, deployment_list, deployment_request};
#[cfg(target_arch = "wasm32")]
use super::server_card_telemetry::telemetry_columns;
#[cfg(target_arch = "wasm32")]
use super::server_registry::ServerRegistry;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ModpackDto;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::ServerRowDto;
#[cfg(target_arch = "wasm32")]
use frontend_ui::badge_class;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, cn};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The badge an inactive server carries; an active server carries none.
#[cfg(target_arch = "wasm32")]
fn inactive_badge(is_active: bool) -> Option<impl IntoView> {
    (!is_active).then(|| {
        view! {
            <span class=badge_class("neutral") data-testid="server-control-inactive">
                "Inactive"
            </span>
        }
    })
}

/// An address and port as one `ip:port` string.
#[cfg(target_arch = "wasm32")]
pub(super) fn format_endpoint(ip: &str, port: i64) -> String {
    format!("{ip}:{port}")
}

/// The status word a server's telemetry maps to.
#[cfg(target_arch = "wasm32")]
pub(super) fn status_label(online: bool) -> &'static str {
    if online { "online" } else { "offline" }
}

/// The dot colour, label and whether-to-pulse for a status word.
///
/// Anything unrecognised reads as offline, which is the safe direction: a status this table has not
/// heard of is not evidence that the server is up.
#[cfg(target_arch = "wasm32")]
pub(super) fn status_meta(status: &str) -> (&'static str, &'static str, bool) {
    match status {
        "online" => ("bg-success", "Online", true),
        "starting" => ("bg-tactical-yellow", "Starting", true),
        _ => ("bg-outline", "Offline", false),
    }
}

/// A modpack as `name vversion`, or a dash when the server requires none.
#[cfg(target_arch = "wasm32")]
pub(super) fn modpack_label(mp: Option<&ModpackDto>) -> String {
    match mp {
        Some(m) => format!("{} v{}", m.modpack.name, m.modpack.version),
        None => "—".to_string(),
    }
}

/// The theatre the current match runs on, capitalised, or a dash between matches.
#[cfg(any(target_arch = "wasm32", test))]
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

/// One selectable row per configured server, with a status dot and its label.
#[cfg(target_arch = "wasm32")]
pub(super) fn server_list(
    servers: &[ServerRowDto],
    selected_id: RwSignal<String>,
) -> impl IntoView + use<> {
    servers
        .iter()
        .map(|s| {
            let id = s.id.clone();
            let id_click = id.clone();
            let name = s.name.clone();
            let is_active = s.is_active;
            let online = s.status.as_ref().is_some_and(|st| st.is_online);
            let status = status_label(online);
            view! {
                {move || {
                    let (dot, label, pulse) = status_meta(status);
                    let active = id == selected_id.get().as_str();
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
                            on:click=move |_| selected_id.set(id_click.to_string())
                        >
                            <span class="relative flex size-2.5 shrink-0">
                                {pulse.then(|| view! { <span class=ping.clone()></span> })}
                                <span class=solid></span>
                            </span>
                            <span class="min-w-0 flex-1">
                                <span class=name_class>{name.clone()}</span>
                                <span class="block font-mono text-code-md text-outline">{label}</span>
                            </span>
                            {inactive_badge(is_active)}
                        </button>
                    }
                }}
            }
        })
        .collect_view()
}

/// The selected server's card: identity, telemetry, the command console and the deployments.
///
/// Built once per selected server. Its row is read from the registry, so a write that changes it
/// updates the header and the telemetry band without rebuilding the console, the deployments panel
/// or the credential sheet.
#[cfg(target_arch = "wasm32")]
pub(super) fn server_detail(registry: ServerRegistry, id: String) -> impl IntoView {
    let store = registry.store;
    let toasts = registry.toasts;
    let row = Memo::new({
        let id = id.clone();
        move |_| registry.row(&id)
    });
    let name = Signal::derive(move || {
        row.with(|s| s.as_ref().map(|s| s.name.clone()))
            .unwrap_or_default()
    });
    let is_active = Signal::derive(move || row.with(|s| s.as_ref().is_some_and(|s| s.is_active)));
    let endpoint = move || {
        row.with(|s| s.as_ref().map(|s| format_endpoint(&s.ip, s.port)))
            .unwrap_or_default()
    };

    let on_launch = move |_| {
        toasts.message("Launch requires the Reforger client");
    };

    let credentials = CredentialPanel::new(store, id.clone(), name);
    let console = CommandConsole::new(store, toasts, id.clone());
    let deployments = DeploymentPanel::new(store, toasts, id.clone());
    let suggested_session = Signal::derive(move || deployments.latest_confirmed_session());
    let shown_id = id.clone();
    let edit_id = StoredValue::new(id);

    view! {
        <div class="flex min-h-full min-w-0 flex-1 flex-col">
            <header class="flex flex-wrap items-center justify-between gap-4 border-b border-white/5 p-6 pb-6">
                <div class="min-w-0">
                    <div class="flex min-w-0 items-center gap-3">
                        <h2 class="truncate text-headline-lg text-on-surface">{move || name.get()}</h2>
                        {move || inactive_badge(is_active.get())}
                    </div>
                    <div class="mt-2 flex flex-wrap items-center gap-2">
                        <div class="inline-flex items-center gap-2 rounded-full bg-white/5 px-3 py-1">
                            <MaterialIcon name="lan" class="text-[16px] text-on-surface-variant" />
                            <span class="font-mono text-code-md text-on-surface">{endpoint}</span>
                        </div>
                        <span
                            class="select-all font-mono text-code-md text-outline"
                            title="Server id"
                            data-testid="server-control-id"
                        >
                            {shown_id}
                        </span>
                    </div>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                    <button
                        type="button"
                        data-testid="server-control-edit"
                        on:click=move |_| registry.open_to_edit(edit_id.get_value())
                        class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
                    >
                        <MaterialIcon name="edit" class="text-[18px]" />
                        "Edit"
                    </button>
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
            {move || row.with(|s| s.as_ref().map(telemetry_columns))}
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
#[cfg(target_arch = "wasm32")]
fn section_heading(icon: &'static str, title: &'static str) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <MaterialIcon name=icon class="text-[18px] text-on-surface-variant" />
            <h3 class="text-label-md font-semibold tracking-wide text-on-surface uppercase">{title}</h3>
        </div>
    }
}
