//! The telemetry band of the selected server's card: personnel, theatre, frame rate and the
//! outbound telemetry queue.
//!
//! **Role:** renders the four telemetry columns under the card header and formats the readings
//! they show — the player count and uptime, the terrain and current match, the frame rate and
//! modpack, and the telemetry queue reading (backlog against capacity, dropped total, oldest
//! entry age and when it was reported).
//! **Position:** called by [`super::server_cards::server_detail`] with the selected server row,
//! and again whenever a registry write changes that row.
//! **Signals & state:** none — each call reads the row it is given once.
//! **Invariants:** the band shows only what `GET /servers` carries: a server with no status reads
//! as zeros and dashes, and a server that never reported a queue reading says "No reading"
//! rather than showing zeros that would claim an empty queue. Dropped telemetry above zero renders
//! in the error tone, because a dropped entry is data the platform never receives.

use super::server_cards::{modpack_label, terrain_reading};
use crate::v2::core::api::dto::{ServerRowDto, TelemetryQueueDto};
use crate::v2::core::ui::cn;
use crate::v2::core::utils::datefmt::format_local_datetime;
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

/// The age of the oldest queued entry: seconds under a minute, minutes and seconds under an
/// hour, and the uptime form beyond that.
pub(super) fn format_queue_age(seconds: i64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format_uptime(seconds)
    }
}

/// A queue's backlog against its capacity, as `backlog / capacity`.
pub(super) fn queue_fill(queue: &TelemetryQueueDto) -> String {
    format!("{} / {}", queue.backlog, queue.capacity)
}

/// The four telemetry columns for `server`.
pub(super) fn telemetry_columns(server: &ServerRowDto) -> impl IntoView {
    let status = server.status.clone();
    let mod_label = modpack_label(server.required_modpack.as_ref());
    let terrain = terrain_reading(server);
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
        .and_then(|st| st.current_match_id.clone())
        .unwrap_or_else(|| "—".to_string());
    let queue = status.and_then(|st| st.telemetry_queue);

    view! {
        <div class="grid shrink-0 grid-cols-4 divide-x divide-white/10 border-b border-white/5">
            {telemetry_col(
                "Active Personnel",
                &format!("{players} / {max_players}"),
                "Uptime",
                &uptime,
            )}
            {telemetry_col("Terrain", &terrain, "Active Mission", &mission)}
            {telemetry_col("Server FPS", &fps, "Mod Configuration", &mod_label)}
            {telemetry_queue_col(queue)}
        </div>
    }
}

/// One telemetry column: a large primary reading, and a smaller secondary one under it.
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
        <div class="min-w-0 px-6 py-6">
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

/// The telemetry queue column: backlog against capacity, then the dropped total, the oldest
/// entry's age and when the reading was reported; "No reading" when the server never sent one.
fn telemetry_queue_col(queue: Option<TelemetryQueueDto>) -> impl IntoView {
    let body = match queue {
        Some(queue) => {
            let dropped_class = cn(&[
                "font-mono text-label-md",
                if queue.dropped_total > 0 { "text-error" } else { "text-on-surface" },
            ]);
            view! {
                <p class="mt-1 truncate font-mono text-3xl font-bold tracking-tight text-on-surface">
                    {queue_fill(&queue)}
                </p>
                <dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
                    {queue_reading_term("Dropped")}
                    <dd class=dropped_class>{queue.dropped_total}</dd>
                    {queue_reading_term("Oldest")}
                    <dd class="font-mono text-label-md text-on-surface">
                        {format_queue_age(queue.oldest_age_seconds)}
                    </dd>
                    {queue_reading_term("Reported")}
                    <dd class="truncate text-label-md text-on-surface">
                        {format_local_datetime(&queue.reported_at)}
                    </dd>
                </dl>
            }
            .into_any()
        }
        None => view! {
            <p class="mt-1 truncate font-mono text-3xl font-bold tracking-tight text-on-surface-variant">
                "No reading"
            </p>
        }
        .into_any(),
    };
    view! {
        <div class="min-w-0 px-6 py-6" data-testid="server-control-telemetry-queue">
            <p class="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
                "Telemetry Queue"
            </p>
            {body}
        </div>
    }
}

/// The label of one queue reading.
fn queue_reading_term(label: &'static str) -> impl IntoView {
    view! {
        <dt class="font-mono text-code-md tracking-wider text-on-surface-variant/70 uppercase">
            {label}
        </dt>
    }
}
