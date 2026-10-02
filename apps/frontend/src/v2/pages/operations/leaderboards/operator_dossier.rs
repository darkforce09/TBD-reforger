//! The slide-over operator dossier: one operator's full stat card.
//!
//! **Role:** fetches the clicked operator's statistics and renders the header — avatar, name and
//! rank — above a grid of stat tiles.
//! **Position:** inside the sheet the ladders page mounts, over the board.
//! **Signals & state:** owns the stats resource, keyed on the operator id it was opened with.
//! **Invariants:** the response is read untyped, so this page adds no shape to the shared DTOs.
//! The command win rate arrives as a fraction and is scaled; the attendance rate arrives already
//! multiplied out and is not. The fetch is a browser-only path and resolves to `None` in a
//! native build.
#![allow(dead_code)]

use super::board_table::avatar;
use super::page::{v_f64, v_i64, win_rate_pct, Row};
use leptos::prelude::*;
use serde_json::Value;

/// The stat card the dossier button and the roster rows open.
///
/// The response carries the operator's statistics alongside a total and an attendance rate, and
/// is read untyped: giving it a DTO is a change to the shared types and their goldens.
#[component]
pub(super) fn OperatorDossier(row: Row) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let id = StoredValue::new(row.discord_id.clone());
    let stats = LocalResource::new(move || {
        let id = id.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/users/{id}/stats");
                crate::v2::core::api::client::api_get::<Value>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<Value>
            }
        }
    });
    let header_glyph = avatar(
        &row.avatar_url,
        &row.username,
        "h-14 w-14 shrink-0 rounded-full object-cover",
    );
    let name = row.username.clone();
    let rank = row.rank;
    view! {
        <div class="flex items-center gap-4">
            {header_glyph}
            <div class="min-w-0">
                <p class="truncate text-headline-sm text-on-surface">{name}</p>
                <p class="font-mono text-label-sm text-on-surface-variant">
                    {format!("RANK #{rank}")}
                </p>
            </div>
        </div>
        <Suspense fallback=move || {
            view! { <p class="mt-6 text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                stats
                    .get()
                    .map(|opt| match opt {
                        Some(body) => dossier_stats(&body).into_any(),
                        None => {
                            view! {
                                <p class="mt-6 text-error">"Failed to load this operator's record."</p>
                            }
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn stat_tile(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-white/5 bg-black/20 px-4 py-3">
            <p class="text-label-sm text-on-surface-variant">{label}</p>
            <p class="mt-1 font-mono text-lg font-semibold text-on-surface">{value}</p>
        </div>
    }
}

/// The grid of stat tiles, read off the fetched stat card.
fn dossier_stats(body: &Value) -> impl IntoView {
    let s = body.get("stats").cloned().unwrap_or(Value::Null);
    let tiles = vec![
        stat_tile("Kills", v_i64(&s, "kills").to_string()),
        stat_tile("Deaths", v_i64(&s, "deaths").to_string()),
        stat_tile("K/D Ratio", format!("{:.2}", v_f64(&s, "kd_ratio"))),
        stat_tile("Team Kills", v_i64(&s, "team_kills").to_string()),
        stat_tile("Longest Kill", format!("{}m", v_i64(&s, "longest_kill_m"))),
        stat_tile(
            "Vehicles Destroyed",
            v_i64(&s, "vehicles_destroyed").to_string(),
        ),
        stat_tile("Missions Played", v_i64(&s, "missions_played").to_string()),
        stat_tile("Command Wins", v_i64(&s, "command_wins").to_string()),
        stat_tile(
            "Command Win Rate",
            win_rate_pct(v_f64(&s, "command_win_rate")),
        ),
        stat_tile(
            "Total Operations",
            v_i64(body, "total_operations").to_string(),
        ),
        // Already a percentage, where the command win rate is a fraction.
        stat_tile(
            "Attendance",
            format!("{:.0}%", v_f64(body, "attendance_rate")),
        ),
    ];
    view! { <div class="mt-6 grid grid-cols-2 gap-3">{tiles}</div> }
}
