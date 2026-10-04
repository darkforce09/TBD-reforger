//! The totals row under the server uplink card's fleet list.
//!
//! **Role:** renders the fleet totals: servers online of those configured, players against the
//! online capacity, and the summed telemetry backlog and drops.
//! **Position:** the footer of [`super::server_uplink::server_uplink`].
//! **Signals & state:** none — the totals arrive owned.
//! **Invariants:** every figure is the backend's total as sent; nothing is re-added from the
//! rows. Dropped telemetry above zero renders in the error tone, because a dropped entry is data
//! the platform never receives.

#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::FleetTotalsDto;
#[cfg(target_arch = "wasm32")]
use frontend_ui::cn;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The totals row for `totals`.
#[cfg(target_arch = "wasm32")]
pub(super) fn fleet_totals(totals: FleetTotalsDto) -> impl IntoView {
    let any_dropped = totals.telemetry_dropped_total > 0;

    view! {
        <div class="mt-auto grid grid-cols-2 gap-x-4 gap-y-1 border-t border-border-subtle pt-3 font-mono text-xs text-on-surface-variant/60">
            <span class="whitespace-nowrap">
                "ONLINE: "
                <span class="text-on-surface">
                    {format!("{}/{}", totals.online, totals.configured)}
                </span>
            </span>
            <span class="whitespace-nowrap text-right">
                "PLAYERS: "
                <span class="text-on-surface">
                    {format!("{}/{}", totals.players, totals.max_players)}
                </span>
            </span>
            <span class="whitespace-nowrap" title="Telemetry entries waiting in the game servers' outbound queues">
                "QUEUE BACKLOG: "
                <span class="text-on-surface">{totals.telemetry_backlog}</span>
            </span>
            <span class="whitespace-nowrap text-right">
                "DROPPED: "
                <span class=cn(
                    &[if any_dropped { "text-error" } else { "text-on-surface" }],
                )>{totals.telemetry_dropped_total}</span>
            </span>
        </div>
    }
}
