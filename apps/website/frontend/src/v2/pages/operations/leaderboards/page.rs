//! The global ladders: five orderings of the same operator table, searched and ranked server-side.
//!
//! **Role:** owns the category and search controls, keys the board fetch on both, parses the wire
//! rows, and mounts the slide-over operator dossier.
//! **Position:** the `/leaderboards` route, rendered inside the navigation frame behind the
//! sign-in gate.
//! **Signals & state:** owns the category and query signals, the board resource keyed on them,
//! and the two signals behind the dossier — the clicked row and the sheet's open flag. Reads the
//! session store from context for the fetch.
//! **Invariants:** the controls sit outside the transition, so the search field keeps focus
//! across the refetch every keystroke fires, and the resolved board stays on screen while the
//! next one is in flight. Ordering and filtering are the server's, so the ranking is correct
//! across the whole table rather than only within the page of rows served. The fetch is a
//! browser-only path and resolves to `None` in a native build.
#![allow(dead_code)]

use super::board_table::board_body;
use super::operator_dossier::OperatorDossier;
use crate::v2::core::api::dto::Leaderboard;
use crate::v2::core::ui::{PageHeader, Sheet};
use leptos::prelude::*;
use serde_json::Value;

/// One ranked operator, as rendered.
///
/// Mirrors the wire row for the fields this page shows; the dossier reads the full stat card
/// straight off the per-user stats endpoint.
#[derive(Clone)]
pub(super) struct Row {
    pub(super) rank: i64,
    pub(super) discord_id: String,
    pub(super) username: String,
    pub(super) avatar_url: String,
    pub(super) kills: i64,
    pub(super) kd_ratio: f64,
    pub(super) team_kills: i64,
    pub(super) command_win_rate: f64,
    pub(super) missions_played: i64,
    pub(super) longest_kill_m: i64,
}

/// The five ladders, as `(label, category)` — the category is what the request sends.
const LEADERBOARD_TABS: [(&str, &str); 5] = [
    ("K/D Ratio", "kd"),
    ("Command Win Rate", "command_win"),
    ("Missions Played", "missions"),
    ("Longest Kill", "longest_kill"),
    ("Wall of Shame", "team_kills"),
];

/// The string at `key`, or an empty string when it is absent or not a string.
fn v_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The integer at `key`, or zero when it is absent or not an integer.
pub(super) fn v_i64(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// The float at `key`, or zero when it is absent or not a float.
pub(super) fn v_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// One wire row, read into a [`Row`].
///
/// The handler ranks server-side and the board already arrives in category order, so the rank is
/// taken from the payload. The positional fallback only covers a row that arrived without one,
/// which would otherwise render every operator as rank zero.
pub(super) fn parse_row(v: &Value, index: usize) -> Row {
    let rank = v
        .get("rank")
        .and_then(Value::as_i64)
        .filter(|&r| r > 0)
        .unwrap_or(index as i64 + 1);
    Row {
        rank,
        discord_id: v_str(v, "discord_id"),
        username: v_str(v, "username"),
        avatar_url: v_str(v, "avatar_url"),
        kills: v_i64(v, "kills"),
        kd_ratio: v_f64(v, "kd_ratio"),
        team_kills: v_i64(v, "team_kills"),
        command_win_rate: v_f64(v, "command_win_rate"),
        missions_played: v_i64(v, "missions_played"),
        longest_kill_m: v_i64(v, "longest_kill_m"),
    }
}

/// A command win rate, as a percentage.
///
/// The wire carries this one as a fraction between zero and one — it is computed as wins over
/// games, rounded — so it is scaled here. Not to be confused with the attendance rate, which the
/// telemetry recompute already multiplies out and which the dossier therefore prints unscaled.
pub(super) fn win_rate_pct(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

/// The `/leaderboards` route: the ladders behind the sign-in gate.
#[component]
pub fn LeaderboardsPage() -> impl IntoView {
    view! {
        <crate::v2::core::ui::AuthGate>
            <LeaderboardsInner />
        </crate::v2::core::ui::AuthGate>
    }
}

/// The signed-in half of the page: the controls, the board fetch, and the dossier sheet.
#[component]
fn LeaderboardsInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let category = RwSignal::new("kd");
    let query = RwSignal::new(String::new());
    // The slide-over dossier: the clicked row, for its header, and the sheet's open flag.
    let sheet_open = RwSignal::new(false);
    let selected = RwSignal::new(None::<Row>);

    // Keyed on both controls: a tab click re-orders on the server and a keystroke re-filters
    // there. Neither is a local re-sort, so the ranking stays correct across the whole table
    // rather than only within the page of rows that was served.
    let board = LocalResource::new(move || {
        let cat = category.get();
        let q = query.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let mut path = format!("/leaderboards?category={cat}");
                let trimmed = q.trim();
                if !trimmed.is_empty() {
                    path.push_str("&q=");
                    path.push_str(
                        &js_sys::encode_uri_component(trimmed)
                            .as_string()
                            .unwrap_or_default(),
                    );
                }
                crate::v2::core::api::client::api_get::<Leaderboard>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, cat, q);
                None::<Leaderboard>
            }
        }
    });

    let open_dossier = move |row: Row| {
        selected.set(Some(row));
        sheet_open.set(true);
    };

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
                                let cat = *cat;
                                // The class merge drops the base text size against the trailing
                                // text colour, so it is left out here.
                                let class = move || {
                                    if category.get() == cat {
                                        "rounded-full px-6 py-1.5 transition-colors bg-white/10 text-white shadow-sm"
                                    } else {
                                        "rounded-full px-6 py-1.5 transition-colors text-on-surface-variant hover:bg-white/5 hover:text-on-surface"
                                    }
                                };
                                view! {
                                    <button
                                        type="button"
                                        on:click=move |_| category.set(cat)
                                        class=class
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <input
                        type="search"
                        placeholder="Search operators..."
                        // The empty `value` attribute is the resting state; the live value is
                        // the property below it.
                        value=""
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                        class="w-full max-w-xs rounded-full border border-white/10 bg-black/20 px-4 py-2 text-sm text-on-surface placeholder:text-on-surface-variant"
                    />
                </div>
                // A transition rather than a suspense: a refetch keeps the resolved board on
                // screen instead of collapsing the page to a loading line on every keystroke.
                <Transition fallback=move || {
                    view! { <p class="mt-8 text-on-surface-variant">"Loading…"</p> }
                }>
                    {move || {
                        board
                            .get()
                            .map(|opt| match opt {
                                Some(board) => {
                                    let rows: Vec<Row> = board
                                        .data
                                        .iter()
                                        .enumerate()
                                        .map(|(i, v)| parse_row(v, i))
                                        .collect();
                                    board_body(
                                            rows,
                                            category.get(),
                                            query.get().trim().is_empty(),
                                            open_dossier,
                                        )
                                        .into_any()
                                }
                                None => {
                                    view! {
                                        <p class="mt-8 text-error">"Failed to load the leaderboard."</p>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Transition>
            </div>
        </div>

        // The operator dossier slides over the page; it is not a route of its own.
        <Sheet open=sheet_open title="Operator Dossier">
            {move || { selected.get().map(|row| view! { <OperatorDossier row=row /> }) }}
        </Sheet>
    }
}
