//! The server uplink card: how busy the primary game server is right now.
//!
//! **Role:** renders the online indicator, the player count with its fill bar, and the two
//! footer readouts for frame rate and uptime.
//! **Position:** the first cell of the dashboard's three-column card grid.
//! **Signals & state:** none — the status arrives owned and is read once.
//! **Invariants:** with no status row the card renders its offline shape: zero players, an
//! empty bar and an em dash in both footer readouts. The fill bar is a whole percentage of the
//! player cap, and a cap of zero yields an empty bar rather than a division by zero. Frame rate
//! prints as the wire sent it, so a whole number shows no decimal part.
#![allow(dead_code)]

use crate::v2::core::api::dto::ServerStatusDto;
use crate::v2::core::ui::{cn, MaterialIcon};
use crate::v2::core::utils::datefmt::format_uptime;
use leptos::prelude::*;

/// The uplink card for `server`, the status row the payload carried.
pub(super) fn server_uplink(server: Option<ServerStatusDto>) -> impl IntoView {
    let player_pct = match &server {
        Some(s) if s.max_players > 0 => {
            ((s.player_count as f64 / s.max_players as f64) * 100.0).round() as i64
        }
        _ => 0,
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
                            if server.as_ref().is_some_and(|s| s.is_online) {
                                "bg-success tactical-pulse"
                            } else {
                                "bg-outline"
                            },
                        ],
                    )></div>
                    <span class="font-mono text-[10px] font-bold tracking-widest text-success">
                        {if server.as_ref().is_some_and(|s| s.is_online) {
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
                        {server.as_ref().map(|s| s.player_count).unwrap_or(0)}
                        <span class="text-lg text-on-surface-variant">
                            "/"
                            {server.as_ref().map(|s| s.max_players).unwrap_or(0)}
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
                <span>
                    "FPS: "
                    {match &server {
                        Some(s) => format!("{}", s.server_fps),
                        None => "—".to_string(),
                    }}
                </span>
                <span>
                    "UPTIME: "
                    {match &server {
                        Some(s) => format_uptime(s.uptime_seconds),
                        None => "—".to_string(),
                    }}
                </span>
            </div>
        </div>
    }
}
