//! Global Leaderboards (/leaderboards) — ported from pages/operations.tsx `LeaderboardsPage`.
//! `<AuthGate>` → `/leaderboards` Resource → until telemetry serves ranked rows the page renders a
//! deterministic client MOCK_LEADERBOARD (re-sorted per category, re-ranked) as a 3-tier podium +
//! roster. Header + segmented category control + search input are always on.
//!
//! **Gate scope (this slice):** the empty-DB `/leaderboards` golden ({category:"kd", data:[]}) → the
//! real list is empty → the MOCK fallback renders on the default K/D tab (podium Reaper/Wraith/Havoc
//! + roster Cobra…Bandit) — byte-exact-verified. The populated (real-rows) render + tab/search
//! interactivity are follow-ups (a content golden + T-interaction); the API row type stays
//! `serde_json::Value` until then.
#![allow(dead_code)]
use crate::dto::Leaderboard;
use crate::ui::{cn, MaterialIcon, PageHeader};
use leptos::prelude::*;

#[derive(Clone)]
struct Row {
    rank: i64,
    discord_id: &'static str,
    username: &'static str,
    avatar_url: &'static str,
    kills: i64,
    deaths: i64,
    kd_ratio: f64,
    team_kills: i64,
    command_win_rate: f64,
    missions_played: i64,
    longest_kill_m: i64,
}

const LEADERBOARD_TABS: [(&str, &str); 5] = [
    ("K/D Ratio", "kd"),
    ("Command Win Rate", "command_win"),
    ("Missions Played", "missions"),
    ("Longest Kill", "longest_kill"),
    ("Wall of Shame", "team_kills"),
];

const fn row(
    rank: i64,
    discord_id: &'static str,
    username: &'static str,
    avatar_url: &'static str,
    kills: i64,
    deaths: i64,
    kd_ratio: f64,
    team_kills: i64,
    command_win_rate: f64,
    missions_played: i64,
    longest_kill_m: i64,
) -> Row {
    Row {
        rank,
        discord_id,
        username,
        avatar_url,
        kills,
        deaths,
        kd_ratio,
        team_kills,
        command_win_rate,
        missions_played,
        longest_kill_m,
    }
}

// Mock leaderboard data (used until the backend serves ranked rows) — byte-identical to operations.tsx.
const MOCK: [Row; 8] = [
    row(
        1,
        "mock-1",
        "Reaper",
        "https://cdn.discordapp.com/embed/avatars/0.png",
        1842,
        311,
        5.92,
        1,
        81.0,
        154,
        912,
    ),
    row(
        2,
        "mock-2",
        "Wraith",
        "https://cdn.discordapp.com/embed/avatars/1.png",
        1610,
        354,
        4.55,
        3,
        74.0,
        138,
        1043,
    ),
    row(
        3,
        "mock-3",
        "Havoc",
        "https://cdn.discordapp.com/embed/avatars/2.png",
        1455,
        402,
        3.62,
        0,
        69.0,
        171,
        720,
    ),
    row(
        4,
        "mock-4",
        "Cobra",
        "https://cdn.discordapp.com/embed/avatars/3.png",
        1245,
        388,
        3.21,
        5,
        66.0,
        129,
        655,
    ),
    row(
        5,
        "mock-5",
        "Specter",
        "https://cdn.discordapp.com/embed/avatars/4.png",
        1130,
        410,
        2.76,
        2,
        61.0,
        147,
        588,
    ),
    row(
        6,
        "mock-6",
        "Viper",
        "https://cdn.discordapp.com/embed/avatars/0.png",
        998,
        421,
        2.37,
        8,
        57.0,
        112,
        503,
    ),
    row(
        7,
        "mock-7",
        "Ghost",
        "https://cdn.discordapp.com/embed/avatars/1.png",
        874,
        399,
        2.19,
        4,
        54.0,
        133,
        471,
    ),
    row(
        8,
        "mock-8",
        "Bandit",
        "https://cdn.discordapp.com/embed/avatars/2.png",
        765,
        388,
        1.97,
        11,
        49.0,
        121,
        402,
    ),
];

fn category_value(r: &Row, category: &str) -> f64 {
    match category {
        "command_win" => r.command_win_rate,
        "missions" => r.missions_played as f64,
        "longest_kill" => r.longest_kill_m as f64,
        "team_kills" => r.team_kills as f64,
        _ => r.kd_ratio,
    }
}

/// (primary, secondary, accent) per category — mirrors statFor().
fn stat_for(r: &Row, category: &str) -> (String, String, &'static str) {
    match category {
        "command_win" => (
            format!("{:.0}%", r.command_win_rate),
            format!("{} Ops", r.missions_played),
            "text-success",
        ),
        "missions" => (
            format!("{}", r.missions_played),
            format!("{} Kills", r.kills),
            "text-primary",
        ),
        "longest_kill" => (
            format!("{}m", r.longest_kill_m),
            format!("{} Kills", r.kills),
            "text-tactical-yellow",
        ),
        "team_kills" => (
            format!("{}", r.team_kills),
            format!("{} Ops", r.missions_played),
            "text-error-alert",
        ),
        _ => (
            format!("{:.2}", r.kd_ratio),
            format!("{} Kills", r.kills),
            "text-success",
        ),
    }
}

/// Podium tier styling (avatar, ring, badge, score, order) for ranks 1/2/3.
fn tier(
    rank: i64,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match rank {
        1 => (
            "h-32 w-32",
            "border-tactical-yellow shadow-[0_0_50px_rgba(250,204,21,0.5)]",
            "bg-tactical-yellow text-black",
            "text-4xl text-tactical-yellow",
            "order-2",
        ),
        2 => (
            "h-24 w-24",
            "border-slate-300 shadow-[0_0_35px_rgba(203,213,225,0.45)]",
            "bg-slate-300 text-black",
            "text-2xl text-slate-200",
            "order-1",
        ),
        _ => (
            "h-20 w-20",
            "border-orange-400 shadow-[0_0_30px_rgba(251,146,60,0.45)]",
            "bg-orange-400 text-black",
            "text-xl text-orange-300",
            "order-3",
        ),
    }
}

fn ranked(category: &str) -> Vec<Row> {
    // Search filter is empty on load (all rows); sort by category value desc (stable), re-rank 1..n.
    let mut rows: Vec<Row> = MOCK.to_vec();
    rows.sort_by(|a, b| {
        category_value(b, category)
            .partial_cmp(&category_value(a, category))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.into_iter()
        .enumerate()
        .map(|(i, mut r)| {
            r.rank = (i + 1) as i64;
            r
        })
        .collect()
}

#[component]
pub fn LeaderboardsPage() -> impl IntoView {
    view! {
        <crate::ui::AuthGate>
            <LeaderboardsInner />
        </crate::ui::AuthGate>
    }
}

#[component]
fn LeaderboardsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // Fire the query like React (result feeds the real-rows path); the DOM falls back to MOCK while
    // the board is empty. Gate on it so the settled state renders.
    let board = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Leaderboard>(store, "/leaderboards")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Leaderboard>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                board
                    .get()
                    .map(|opt| {
                        let real_empty = opt
                            .as_ref()
                            .map(|b| b.data.is_empty())
                            .unwrap_or(true);
                        // Default tab = kd; populated real rows are content-golden gated.
                        let rows = if real_empty { ranked("kd") } else { Vec::new() };
                        board_view("kd", rows).into_any()
                    })
            }}
        </Suspense>
    }
}

fn board_view(category: &'static str, rows: Vec<Row>) -> impl IntoView {
    let podium: Vec<Row> = rows.iter().take(3).cloned().collect();
    let rest: Vec<Row> = rows.iter().skip(3).cloned().collect();
    let empty = rows.is_empty();

    view! {
        <div class="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
            <div class="custom-scrollbar flex h-full w-full flex-col overflow-y-auto bg-surface-glass p-6 backdrop-blur-xl md:p-10">
                <PageHeader
                    title="Global Leaderboards"
                    subtitle="Real-time tactical performance metrics across all active theaters."
                />
                <div class="flex flex-wrap items-center justify-between gap-4">
                    <div class="flex w-max rounded-full border border-white/5 bg-black/20 p-1">
                        {LEADERBOARD_TABS
                            .iter()
                            .map(|(label, cat)| {
                                let active = *cat == category;
                                // cn(): twMerge drops the base `text-label-md` against the trailing
                                // text-{color}, so it's omitted here (unlike plain-string classes).
                                let class = if active {
                                    "rounded-full px-6 py-1.5 transition-colors bg-white/10 text-white shadow-sm"
                                } else {
                                    "rounded-full px-6 py-1.5 transition-colors text-on-surface-variant hover:bg-white/5 hover:text-on-surface"
                                };
                                view! {
                                    <button type="button" class=class>
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <input
                        type="search"
                        placeholder="Search operators..."
                        value=""
                        class="w-full max-w-xs rounded-full border border-white/10 bg-black/20 px-4 py-2 text-sm text-on-surface placeholder:text-on-surface-variant"
                    />
                </div>
                {if empty {
                    view! {
                        <p class="mt-8 text-on-surface-variant">"No operators match your search."</p>
                    }
                        .into_any()
                } else {
                    view! {
                        <>
                            {(!podium.is_empty())
                                .then(|| {
                                    view! {
                                        <div class="flex flex-row items-end justify-center gap-8 pt-16 pb-12 md:gap-16">
                                            {podium
                                                .iter()
                                                .map(|p| podium_place(p, category))
                                                .collect_view()}
                                        </div>
                                    }
                                })}
                            {(!rest.is_empty())
                                .then(|| {
                                    view! {
                                        <div class="mt-2 flex flex-col gap-0.5 border-t border-white/5 pt-4">
                                            {rest
                                                .iter()
                                                .map(|r| roster_row(r, category))
                                                .collect_view()}
                                        </div>
                                    }
                                })}
                        </>
                    }
                        .into_any()
                }}
            </div>
        </div>
    }
}

fn podium_place(r: &Row, category: &str) -> impl IntoView {
    let (avatar, ring, badge, score, order) = tier(r.rank);
    let (primary, secondary, _accent) = stat_for(r, category);
    let outer = cn(&["flex flex-col items-center", order]);
    let img_class = cn(&["rounded-xl border-2 object-cover", avatar, ring]);
    let badge_class = cn(&[
        "absolute -bottom-3 left-1/2 -translate-x-1/2 rounded-full px-3 py-0.5 text-xs font-bold",
        badge,
    ]);
    let score_class = cn(&["mt-1 font-bold drop-shadow-md", score]);
    let is_first = r.rank == 1;
    view! {
        <div class=outer>
            <div class="relative">
                <img src=r.avatar_url alt="" class=img_class />
                <span class=badge_class>"#"{r.rank}</span>
            </div>
            <p class="mt-6 text-label-md font-semibold text-on-surface">{r.username}</p>
            <p class=score_class>{primary}</p>
            <span class="text-label-sm text-on-surface-variant">{secondary}</span>
            {is_first
                .then(|| {
                    view! {
                        <button
                            type="button"
                            class="mt-3 font-mono text-[11px] tracking-widest text-tactical-yellow/80 transition-colors hover:text-tactical-yellow"
                        >
                            "[ VIEW DOSSIER ]"
                        </button>
                    }
                })}
        </div>
    }
}

fn roster_row(r: &Row, category: &str) -> impl IntoView {
    let (primary, secondary, accent) = stat_for(r, category);
    let primary_class = cn(&["w-16 text-right font-mono font-semibold", accent]);
    view! {
        <div class="group flex cursor-pointer items-center gap-4 rounded-lg px-2 py-3 transition-colors hover:bg-white/5">
            <span class="w-8 shrink-0 font-mono text-sm text-on-surface-variant">
                {format!("{:02}", r.rank)}
            </span>
            <img
                src=r.avatar_url
                alt=""
                class="h-8 w-8 shrink-0 rounded-full object-cover"
            />
            <span class="flex-1 truncate text-label-md font-medium text-on-surface">
                {r.username}
            </span>
            <span class="hidden text-sm text-on-surface-variant sm:inline">{secondary}</span>
            <span class=primary_class>{primary}</span>
            <MaterialIcon name="chevron_right" class="text-on-surface-variant group-hover:text-white" />
        </div>
    }
}
