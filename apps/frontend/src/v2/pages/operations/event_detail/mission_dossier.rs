//! One mission inside an operation: its heading, briefing, faction cards and slotting.
//!
//! **Role:** renders a mission dossier card — the mission number and title, the terrain, mode
//! and start time line, the meta badges, the registration state and fill counts, the leader's
//! waiting-list promotion, the notices about the viewer's standing, the briefing, the faction
//! dossier grid, and the slotting selector for that mission — and owns the label and briefing
//! rules the card reads with.
//! **Position:** repeated down the "Mission Dossiers" column of the hub body, below the hero.
//! **Signals & state:** none of its own; the slotting mutation callback is handed down from the
//! route and is what refetches the operation, and the viewer's standing on the mission arrives
//! already built from the operation dossier.
//! **Invariants:** every badge on the header comes out of the dossier. A field the wire does not
//! carry is not rendered at all — neither as a value nor as an empty-state promise that it could
//! one day be authored. The one exception is the briefing, which *is* authorable and therefore
//! gets an explicit affordance when it is blank.
#![allow(dead_code)]

use super::faction_armory::{faction_dossier_card, sort_factions};
use super::registration_access::mission_standing::{standing_notices, MissionStanding};
use super::registration_access::waitlist_promotion::waitlist_promotion_control;
use super::slotting_selector::OrbatSelector;
use crate::v2::core::api::dto::EventMissionDossier;
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

/// The short label a `game_mode` is shown as, or the raw value when it is not one of the three.
fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}
/// A terrain identifier with its first letter capitalised, or an em dash when it is empty.
fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Which modpack the hub chip should show.
///
/// An operation may bind a specific pack; when it does not, the chip falls back to whatever the
/// platform currently calls current. There is no public route that fetches one pack by id, so the
/// bound case is resolved by selecting out of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum HubModpackFetch {
    /// The pack the operation binds, named by id.
    ById(String),
    /// Whatever the platform currently calls the current pack.
    Current,
}

/// Resolve an operation's `modpack_id` into the fetch the chip should make. A blank or
/// whitespace-only id is not a binding.
pub(super) fn hub_modpack_fetch(modpack_id: Option<&str>) -> HubModpackFetch {
    match modpack_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => HubModpackFetch::ById(id.to_string()),
        None => HubModpackFetch::Current,
    }
}

/// One meta badge: a label and the value the dossier carries for it.
fn meta_badge(label: &'static str, value: String) -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-1.5 rounded border border-outline-variant/30 bg-surface-container/60 px-2 py-1 font-mono text-[11px] uppercase tracking-wide">
            <span class="text-on-surface-variant">{label} ":"</span>
            <span class="text-on-surface">{value}</span>
        </span>
    }
}

/// The briefing a mission dossier shows: the authored prose, or the explicit empty-state
/// affordance when there is none.
///
/// A cleared briefing arrives here as an absent key, and a briefing of nothing but whitespace is
/// not authored content either — rendering it verbatim leaves a heading over blank space. Both
/// take the affordance. Nothing here writes: an absent value stays distinguishable from a
/// default, so the reader is told the box is empty rather than having one filled in for them.
pub(super) fn briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => "No briefing provided.".to_string(),
    }
}

/// Every meta badge the dossier header can honestly show, in render order.
///
/// Returning the list rather than inlining its one survivor into the view is what makes "every
/// badge on this row came out of the dossier" something a native test can assert: the view itself
/// needs a DOM and cannot be exercised by `cargo test`.
pub(super) fn meta_badges(m: &EventMissionDossier) -> Vec<(&'static str, String)> {
    vec![("Terrain", terrain_label(&m.terrain))]
}

/// One mission dossier card, numbered by its position in the operation, with the viewer's
/// `standing` on it.
pub(super) fn mission_dossier(
    index: usize,
    m: EventMissionDossier,
    standing: MissionStanding,
    on_change: Callback<()>,
) -> impl IntoView {
    // The class merge helper does not group these two hyphenated background names, so the
    // override is written out already resolved rather than layered over a default.
    let card = "relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 border border-border-subtle bg-surface-container-high";
    // A card is rendered for every faction the mission lists; the armory, keyed by faction,
    // fills in the items.
    let faction_list = sort_factions(if m.factions.is_empty() {
        m.armory_by_faction
            .iter()
            .map(|f| f.faction.clone())
            .collect()
    } else {
        m.factions.clone()
    });
    let badges = meta_badges(&m);
    let armory = m.armory_by_faction;
    let briefing = briefing_text(m.briefing.as_deref());
    let terrain = terrain_label(&m.terrain);
    let mode = game_mode_label(&m.game_mode).to_string();
    let when = format_local_datetime(&m.start_time);
    let my_reservation_state = m.my_reservation_state.clone();
    let my_attendance_state = m.my_attendance_state.clone();
    let notices = standing_notices(&standing);
    let promotion = waitlist_promotion_control(m.event_mission_id.clone(), on_change);
    view! {
        <div class=card>
            <div class="flex flex-wrap items-start justify-between gap-4">
                <div>
                    <span class="text-xs font-semibold uppercase tracking-widest text-on-surface-variant">
                        "Mission " {index}
                    </span>
                    <h3 class="mt-1 text-xl font-semibold">{m.title.clone()}</h3>
                    <p class="mt-1 text-sm text-on-surface-variant">
                        {terrain} " • " {mode} " • " {when}
                    </p>
                    <div class="mt-2 flex flex-wrap gap-2">
                        {badges
                            .into_iter()
                            .map(|(label, value)| meta_badge(label, value))
                            .collect_view()}
                    </div>
                </div>
                <div class="flex flex-col items-end gap-2">
                    {my_reservation_state
                        .clone()
                        .map(|s| {
                            view! {
                                <span class="rounded bg-success-muted px-2 py-0.5 text-xs font-semibold text-success">
                                    {s.to_uppercase()}
                                </span>
                            }
                        })}
                    {my_attendance_state.map(|state| view! { <p class="text-xs text-on-surface-variant">"Attendance: " {state.replace('_', " ")}</p> })}
                    <p class="font-mono text-sm text-on-surface-variant">
                        {m.filled} "/" {m.total} " slots filled"
                    </p>
                    {promotion}
                    <button
                        type="button"
                        disabled
                        title="2D mission planner — coming soon"
                        class="flex cursor-not-allowed items-center gap-2 rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-on-surface-variant opacity-50"
                    >
                        <MaterialIcon name="map" class="text-base" />
                        " Mission Planner"
                    </button>
                </div>
            </div>
            {notices}

            <section class="mt-4">
                <h4 class="mb-2 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                    "Mission Briefing"
                </h4>
                <p class="whitespace-pre-line text-sm leading-relaxed text-on-surface-variant">
                    {briefing}
                </p>
            </section>

            // One card per faction: the uniform frames, and the armory the mission serves.
            {(!faction_list.is_empty())
                .then(|| {
                    view! {
                        <section class="mt-4">
                            <h4 class="mb-2 font-mono text-xs uppercase tracking-widest text-on-surface-variant">
                                "Faction Dossiers"
                            </h4>
                            <div class="grid gap-3 md:grid-cols-2">
                                {faction_list
                                    .iter()
                                    .map(|faction| {
                                        let items = armory
                                            .iter()
                                            .find(|f| &f.faction == faction)
                                            .map(|f| f.items.clone())
                                            .unwrap_or_default();
                                        faction_dossier_card(faction.clone(), items)
                                    })
                                    .collect_view()}
                            </div>
                        </section>
                    }
                })}

            <div class="mt-4">
                <OrbatSelector emid=m.event_mission_id.clone() standing=standing on_change=on_change />
            </div>
        </div>
    }
}
