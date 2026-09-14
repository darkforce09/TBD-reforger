//! The board itself: the three-place podium and the roster rows under it.
//!
//! **Role:** renders the board for a fetched category — the podium for the top three and a roster
//! row for everyone after them — along with the per-category statistic pair, the podium tier
//! styling, and the avatar glyph every row shares.
//! **Position:** the body of the ladders page, below the category tabs and the search field.
//! **Signals & state:** none — the board is built from the rows handed to it, and a click calls
//! the opener the page passed down.
//! **Invariants:** the two empty states are distinct: a search that matched nobody reads
//! differently from a ladder with nothing in it yet. The avatar cell emits an `<img src>` only
//! for an `http(s)` URL and falls back to initials otherwise, so a stored `javascript:` or
//! `data:` value never reaches the attribute.
#![allow(dead_code)]

use super::page::{win_rate_pct, Row};
use crate::v2::core::auth::url_guard;
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;

/// The `(primary, secondary, accent)` triple a row is shown with, for the given category.
pub(super) fn stat_for(r: &Row, category: &str) -> (String, String, &'static str) {
    match category {
        "command_win" => (
            win_rate_pct(r.command_win_rate),
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

/// The podium styling for a rank — avatar size, ring, badge, score and flex order. Ranks beyond
/// the third take the third's treatment; only the top three are ever passed here.
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

/// The first letters of the first two words, or a question mark — the fallback glyph.
pub(super) fn initials(name: &str) -> String {
    let mut out = String::new();
    for word in name.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    if out.is_empty() {
        "?".to_string()
    } else {
        out
    }
}

/// The operator's avatar image, or their initials when the row carries none or carries a value
/// that is not an `http(s)` URL.
///
/// The handler sends an empty string for a missing avatar, which would otherwise render a broken
/// image on every such row. The scheme check is here rather than only at the writer because this
/// sink sees every value whatever door it came in by.
pub(super) fn avatar(url: &str, username: &str, class: &str) -> impl IntoView {
    match avatar_img_src(url) {
        None => {
            let c = cn(&[
                "flex items-center justify-center bg-gradient-to-br from-primary/40 to-tertiary/30 font-semibold text-on-surface",
                class,
            ]);
            view! { <span class=c>{initials(username)}</span> }.into_any()
        }
        Some(src) => {
            view! { <img src=src.to_string() alt="" class=class.to_string() /> }.into_any()
        }
    }
}

/// Whether the avatar cell would emit an `<img src>` for `url`.
///
/// Lifted out of the cell so the sink can be tested without rendering: the crate renders in a
/// browser and cannot render to a string natively.
pub(super) fn avatar_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}

/// The podium and roster, or the matching empty state.
///
/// `searchless` separates "your query matched nobody" from "the ladder has nothing in it yet".
pub(super) fn board_body(
    rows: Vec<Row>,
    category: &'static str,
    searchless: bool,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoView {
    if rows.is_empty() {
        let msg = if searchless {
            "No ranked operators yet — telemetry has not reported any tracked matches."
        } else {
            "No operators match your search."
        };
        return view! { <p class="mt-8 text-on-surface-variant">{msg}</p> }.into_any();
    }
    let podium: Vec<Row> = rows.iter().take(3).cloned().collect();
    let rest: Vec<Row> = rows.iter().skip(3).cloned().collect();
    view! {
        <>
            <div class="flex flex-row items-end justify-center gap-8 pt-16 pb-12 md:gap-16">
                {podium.iter().map(|p| podium_place(p, category, open)).collect_view()}
            </div>
            {(!rest.is_empty())
                .then(|| {
                    view! {
                        <div class="mt-2 flex flex-col gap-0.5 border-t border-white/5 pt-4">
                            {rest.iter().map(|r| roster_row(r, category, open)).collect_view()}
                        </div>
                    }
                })}
        </>
    }
    .into_any()
}

/// One podium place: the avatar with its rank badge, the name, and the category statistics —
/// plus the dossier button, on the first place only.
fn podium_place(r: &Row, category: &str, open: impl Fn(Row) + Copy + 'static) -> impl IntoView {
    let (avatar_size, ring, badge, score, order) = tier(r.rank);
    let (primary, secondary, _accent) = stat_for(r, category);
    let outer = cn(&["flex flex-col items-center", order]);
    let img_class = cn(&["rounded-xl border-2 object-cover", avatar_size, ring]);
    let badge_class = cn(&[
        "absolute -bottom-3 left-1/2 -translate-x-1/2 rounded-full px-3 py-0.5 text-xs font-bold",
        badge,
    ]);
    let score_class = cn(&["mt-1 font-bold drop-shadow-md", score]);
    let is_first = r.rank == 1;
    let glyph = avatar(&r.avatar_url, &r.username, &img_class);
    let dossier_row = r.clone();
    view! {
        <div class=outer>
            <div class="relative">
                {glyph} <span class=badge_class>"#"{r.rank}</span>
            </div>
            <p class="mt-6 text-label-md font-semibold text-on-surface">{r.username.clone()}</p>
            <p class=score_class>{primary}</p>
            <span class="text-label-sm text-on-surface-variant">{secondary}</span>
            {is_first
                .then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| open(dossier_row.clone())
                            class="mt-3 font-mono text-[11px] tracking-widest text-tactical-yellow/80 transition-colors hover:text-tactical-yellow"
                        >
                            "[ VIEW DOSSIER ]"
                        </button>
                    }
                })}
        </div>
    }
}

/// One roster row below the podium: rank, avatar, name and the category statistics. The whole
/// row opens the operator's dossier.
fn roster_row(r: &Row, category: &str, open: impl Fn(Row) + Copy + 'static) -> impl IntoView {
    let (primary, secondary, accent) = stat_for(r, category);
    let primary_class = cn(&["w-16 text-right font-mono font-semibold", accent]);
    let glyph = avatar(
        &r.avatar_url,
        &r.username,
        "h-8 w-8 shrink-0 rounded-full object-cover",
    );
    // The row looks clickable — a pointer cursor and a trailing chevron — so it opens the same
    // dossier the podium button does.
    let dossier_row = r.clone();
    view! {
        <div
            class="group flex cursor-pointer items-center gap-4 rounded-lg px-2 py-3 transition-colors hover:bg-white/5"
            on:click=move |_| open(dossier_row.clone())
        >
            <span class="w-8 shrink-0 font-mono text-sm text-on-surface-variant">
                {format!("{:02}", r.rank)}
            </span>
            {glyph}
            <span class="flex-1 truncate text-label-md font-medium text-on-surface">
                {r.username.clone()}
            </span>
            <span class="hidden text-sm text-on-surface-variant sm:inline">{secondary}</span>
            <span class=primary_class>{primary}</span>
            <MaterialIcon name="chevron_right" class="text-on-surface-variant group-hover:text-white" />
        </div>
    }
}
