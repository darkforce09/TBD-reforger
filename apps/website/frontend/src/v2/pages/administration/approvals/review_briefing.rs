//! The briefing and settings of the mission under review, read off the mission itself.
//!
//! **Role:** the mission's own briefing and its four settings tiles — maximum players, game mode,
//! weather and time of day — with the labels they are shown under.
//! **Position:** inside the approvals drawer, under the header.
//! **Signals & state:** none; the mission detail is handed in already read.
//! **Invariants:** the words under the header belong to the mission being reviewed; a mission that
//! supplied no briefing says so rather than borrowing someone else's words, and a detail that could
//! not be read says where to review it instead of showing an empty surface.

use crate::v2::core::api::dto::MissionDetail;
use leptos::prelude::*;

/// A game mode's wire value as the label the review surface shows.
pub(super) fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// A snake_case wire enum as a readable label, or a dash when it is empty.
pub(super) fn enum_label(v: &str) -> String {
    if v.is_empty() {
        return "—".into();
    }
    let spaced = v.replace('_', " ");
    let mut c = spaced.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The briefing and the settings tiles, or the line that says the mission could not be read.
pub(super) fn briefing_and_settings(detail: Option<MissionDetail>) -> impl IntoView {
    let Some(m) = detail else {
        return view! {
            <p class="text-body-md text-error">
                "Could not load this mission's briefing — review it in the Mission Library before deciding."
            </p>
        }
        .into_any();
    };
    let briefing = m.briefing.clone().unwrap_or_default();
    let body = if briefing.trim().is_empty() {
        view! {
            <p class="text-body-md text-outline italic">"The author submitted no briefing."</p>
        }
        .into_any()
    } else {
        view! {
            <p class="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
                {briefing}
            </p>
        }
        .into_any()
    };
    view! {
        {body}
        <div class="mt-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
            {stat_tile("Max Players", m.max_players.to_string())}
            {stat_tile("Game Mode", game_mode_label(&m.game_mode).to_string())}
            {stat_tile("Weather", enum_label(&m.weather))}
            {stat_tile("Time of Day", m.time_of_day.clone())}
        </div>
    }
    .into_any()
}

/// One of the four setting tiles under the briefing.
fn stat_tile(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
            <p class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </p>
            <p class="mt-1 truncate text-headline-sm text-on-surface">{value}</p>
        </div>
    }
}
