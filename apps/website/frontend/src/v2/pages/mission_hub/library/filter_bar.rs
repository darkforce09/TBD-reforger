//! The terrain, game-mode and player-count filters above the mission grid.
//!
//! **Role:** the three select controls that narrow the mission list, each writing one query
//! parameter.
//! **Position:** the rest of the library's search and filter toolbar, beside the search box.
//! **Signals & state:** writes the page's `terrain`, `mode` and `players` signals; the list
//! resource re-keys on all three.
//! **Invariants:** the empty option means "no filter" and is what the query builder omits, so a
//! blank value never reaches the request as an empty parameter. The visible labels come from the
//! same formatters the cards use, so a mode reads the same everywhere.

use super::card_grid::{game_mode_label, terrain_label};
use leptos::prelude::*;

/// Shared appearance of the three filter selects.
const SELECT_CLASS: &str = "rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";

/// The three filter selects, bound to the page's filter signals.
pub(super) fn filter_bar(
    terrain: RwSignal<String>,
    mode: RwSignal<String>,
    players: RwSignal<String>,
) -> impl IntoView {
    view! {
                <select
                    prop:value=move || terrain.get()
                    on:change=move |ev| terrain.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Terrains"</option>
                    <option value="everon">{terrain_label("everon")}</option>
                    <option value="arland">{terrain_label("arland")}</option>
                </select>
                <select
                    prop:value=move || mode.get()
                    on:change=move |ev| mode.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Modes"</option>
                    <option value="pve_coop">{game_mode_label("pve_coop")}</option>
                    <option value="pvp">{game_mode_label("pvp")}</option>
                    <option value="zeus">{game_mode_label("zeus")}</option>
                </select>
                <select
                    prop:value=move || players.get()
                    on:change=move |ev| players.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Players"</option>
                    <option value="1-8">"1–8"</option>
                    <option value="9-16">"9–16"</option>
                    <option value="17-32">"17–32"</option>
                    <option value="33-64">"33–64"</option>
                </select>
    }
}
