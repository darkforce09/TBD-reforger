//! The server uplink card: the configured fleet, one row per active server, and its totals.
//!
//! **Role:** renders the fleet's online pill, one row per active server (name, online state,
//! players against the cap, frame rate) and the totals row beneath them.
//! **Position:** the first cell of the dashboard's three-column card grid; the totals row is
//! [`super::fleet_totals::fleet_totals`].
//! **Signals & state:** none — the fleet arrives owned and is read once.
//! **Invariants:** rows keep the backend's order (name, then id). A server without a status row
//! renders as offline with an em dash for players and frame rate, the way the backend counts
//! it. Frame rate prints as the wire sent it, so a whole number shows no decimal part. The pill
//! reads the backend's totals and never recounts the rows. An empty fleet renders one line
//! saying so instead of an empty list.

#[cfg(target_arch = "wasm32")]
use super::fleet_totals::fleet_totals;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::{FleetOverviewDto, FleetServerDto};
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, cn};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The uplink card for the dashboard's `fleet`.
#[cfg(target_arch = "wasm32")]
pub(super) fn server_uplink(fleet: FleetOverviewDto) -> impl IntoView {
    let totals = fleet.totals.clone();
    let any_online = totals.online > 0;
    let pill = format!("{}/{} ONLINE", totals.online, totals.configured);
    let rows = if fleet.servers.is_empty() {
        view! {
            <p class="font-mono text-xs text-on-surface-variant">"No servers configured"</p>
        }
        .into_any()
    } else {
        view! {
            <ul class="flex flex-col divide-y divide-border-subtle">
                {fleet.servers.into_iter().map(fleet_server_row).collect_view()}
            </ul>
        }
        .into_any()
    };

    view! {
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
                            if any_online { "bg-success tactical-pulse" } else { "bg-outline" },
                        ],
                    )></div>
                    <span class="font-mono text-[10px] font-bold tracking-widest text-success">
                        {pill}
                    </span>
                </div>
            </div>
            {rows}
            {fleet_totals(totals)}
        </div>
    }
}

/// One server of the fleet: its online dot and name, then players against the cap and the
/// frame rate.
#[cfg(target_arch = "wasm32")]
fn fleet_server_row(server: FleetServerDto) -> impl IntoView {
    let online = server.status.as_ref().is_some_and(|s| s.is_online);
    let players = match &server.status {
        Some(s) if s.is_online => format!("{}/{}", s.player_count, s.max_players),
        _ => "—".to_string(),
    };
    let title = server.name.clone();
    let fps = match &server.status {
        Some(s) if s.is_online => format!("{}", s.server_fps),
        _ => "—".to_string(),
    };

    view! {
        <li class="flex flex-col gap-1 py-2">
            <div class="flex min-w-0 items-center gap-2">
                <div class=cn(
                    &[
                        "h-2 w-2 shrink-0 rounded-full",
                        if online { "bg-success" } else { "bg-outline" },
                    ],
                )></div>
                <span class="min-w-0 flex-1 truncate text-sm text-on-surface" title=title>
                    {server.name}
                </span>
                <span class=cn(
                    &[
                        "font-mono text-[10px] tracking-widest",
                        if online { "text-success" } else { "text-on-surface-variant" },
                    ],
                )>{if online { "ONLINE" } else { "OFFLINE" }}</span>
            </div>
            <div class="flex items-center gap-3 pl-4 font-mono text-xs text-on-surface-variant">
                <span>
                    <span class="text-on-surface">{players}</span>
                    " PLAYERS"
                </span>
                <span>"FPS: " {fps}</span>
            </div>
        </li>
    }
}
