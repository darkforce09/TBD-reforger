//! The Participants section: why each participant is, or is not, admitted.
//!
//! **Role:** renders one row per participant — whether the account is available and a verified TBD
//! member, the place it holds and from which pool, each current reservation with the policy that
//! decides it, the grants that admit it and whether current and last-verified facts admit it, and
//! the evidence behind those facts: Discord guild observations and managed-roster entries with who
//! added them.
//! **Position:** the fourth section of the access panel.
//! **Signals & state:** reads the panel's participant evidence and its missions; owns nothing.
//! **Invariants:** grant numbers are shown one-based, as the policy editor numbers them, although
//! the wire carries zero-based indices. Only the last verified facts release a reservation, so a
//! reservation current facts no longer admit is shown as standing, not as lost. Every line is pure
//! and native-tested; the view only lays them out.

use super::groups::group_form::authorship_line;
use super::state::{AccessPanel, Loadable, MissionSeats};
use crate::v2::core::api::dto::{
    EventGroupView, GuildEvidenceView, ParticipantAccessExplanation, RegistrationDecision,
};
use crate::v2::core::utils::utc_timestamp::utc_label;
use leptos::prelude::*;

/// Which grants admit a reservation, one-based: `Admitted by grants 1 and 3`.
pub(super) fn grants_line(admitting: &[i64]) -> String {
    let numbers: Vec<String> = admitting.iter().map(|i| (i + 1).to_string()).collect();
    match numbers.as_slice() {
        [] => "No grant admits it now".to_string(),
        [one] => format!("Admitted by grant {one}"),
        [rest @ .., last] => format!("Admitted by grants {} and {last}", rest.join(", ")),
    }
}

/// Whether current and last-verified facts admit a reservation, in words.
pub(super) fn authority_line(current: bool, last_verified: bool) -> &'static str {
    match (current, last_verified) {
        (true, true) => "Current and last-verified facts admit it",
        (true, false) => "Current facts admit it; the last verified facts did not yet",
        (false, true) => {
            "Current facts do not admit it, but the last verified facts do, so it stands until a \
             verification confirms the loss"
        }
        (false, false) => "Neither current nor last-verified facts admit it",
    }
}

/// One guild observation, in words.
pub(super) fn guild_line(guild: &GuildEvidenceView) -> String {
    let verified = match guild.verified_at.as_deref() {
        Some(at) => format!("verified {}", utc_label(at)),
        None => "never verified".to_string(),
    };
    let override_until = guild
        .override_until
        .as_deref()
        .map(|until| format!(", override until {}", utc_label(until)))
        .unwrap_or_default();
    let freshness = if guild.current {
        "current"
    } else {
        "not current"
    };
    format!(
        "Guild {}: {}, {verified}{override_until} ({freshness})",
        guild.guild_id, guild.membership_status
    )
}

/// The seat a reservation holds, as `Mission — Faction / Squad, 3. Rifleman`, or its seatless state.
pub(super) fn reservation_line(
    decision: &RegistrationDecision,
    missions: &[MissionSeats],
) -> String {
    let mission = missions
        .iter()
        .find(|m| m.event_mission_id == decision.event_mission_id);
    let title = mission.map(|m| m.title.as_str()).unwrap_or("A mission");
    let seat = match decision.slot_id.as_deref() {
        None => "no seat".to_string(),
        Some(slot_id) => mission
            .and_then(|m| {
                m.squads.iter().find_map(|squad| {
                    squad
                        .slots
                        .iter()
                        .find(|slot| slot.id == slot_id)
                        .map(|slot| {
                            format!(
                                "{} / {}, {}. {}",
                                squad.faction, squad.squad, slot.number, slot.role
                            )
                        })
                })
            })
            .unwrap_or_else(|| format!("seat {slot_id}")),
    };
    format!(
        "{title} — {} ({seat}); decided by the {} policy",
        decision.reservation_state.replace('_', " "),
        decision.policy_source
    )
}

/// A participant's place, in words.
pub(super) fn place_line(participant: &ParticipantAccessExplanation) -> String {
    match &participant.allocation {
        Some(place) => format!(
            "Holds a {} place since {}",
            place.quota_kind.replace('_', " "),
            utc_label(&place.acquired_at)
        ),
        None => "Holds no place".to_string(),
    }
}

/// The Participants section.
pub(super) fn participants_table(panel: AccessPanel) -> impl IntoView {
    move || {
        let missions = panel
            .missions
            .with(|m| m.loaded().cloned())
            .unwrap_or_default();
        let groups: Vec<EventGroupView> = panel
            .access
            .with(|a| a.loaded().map(|view| view.groups.clone()))
            .unwrap_or_default();
        match panel.participants.get() {
            Loadable::Loaded(list) if list.is_empty() => view! {
                <p class="text-sm text-on-surface-variant">"Nobody holds a place, a seat or a waiting entry yet."</p>
            }
            .into_any(),
            Loadable::Loaded(list) => {
                let names: std::collections::HashMap<String, String> = list
                    .iter()
                    .map(|p| (p.discord_id.clone(), p.username.clone()))
                    .collect();
                let name_of = move |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_string());
                list.iter()
                    .map(|participant| participant_row(participant, &missions, &groups, &name_of))
                    .collect_view()
                    .into_any()
            }
            Loadable::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
            Loadable::Loading | Loadable::Idle => view! {
                <p class="text-sm text-on-surface-variant">"Loading participants…"</p>
            }
            .into_any(),
        }
    }
}

/// One participant's row.
fn participant_row(
    participant: &ParticipantAccessExplanation,
    missions: &[MissionSeats],
    groups: &[EventGroupView],
    name_of: &dyn Fn(&str) -> String,
) -> impl IntoView {
    let membership = if participant.tbd_member {
        "Verified TBD member"
    } else {
        "Not a verified TBD member"
    };
    let reservations: Vec<(String, String, &'static str)> = participant
        .registrations
        .iter()
        .map(|r| {
            (
                reservation_line(r, missions),
                grants_line(&r.admitting_grants),
                authority_line(r.current_authority_admits, r.last_verified_admits),
            )
        })
        .collect();
    let guilds: Vec<String> = participant.guilds.iter().map(guild_line).collect();
    let rosters: Vec<String> = participant
        .roster_groups
        .iter()
        .map(|entry| {
            let group = groups
                .iter()
                .find(|g| g.id == entry.group_id)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| format!("group {}", entry.group_id));
            format!(
                "On the roster of {group}, added by {}",
                authorship_line(
                    entry.added_by.as_deref(),
                    entry.system_origin.as_deref(),
                    &entry.added_at,
                    name_of,
                )
            )
        })
        .collect();
    view! {
        <article class="rounded-xl border border-white/10 p-3 text-sm" data-testid="participant-evidence">
            <div class="flex flex-wrap items-baseline justify-between gap-2">
                <span class="text-on-surface">
                    {participant.username.clone()}
                    <span class="ml-2 font-mono text-xs text-on-surface-variant">{participant.discord_id.clone()}</span>
                </span>
                <span class="text-xs text-on-surface-variant">{place_line(participant)}</span>
            </div>
            <p class="mt-1 text-xs text-on-surface-variant">
                {membership}
                {(!participant.available).then_some(" · account unavailable (banned or removed)")}
            </p>
            <ul class="mt-2 space-y-1">
                {reservations
                    .into_iter()
                    .map(|(line, grants, authority)| {
                        view! {
                            <li class="rounded-lg bg-white/[0.02] px-2 py-1">
                                <p class="text-on-surface">{line}</p>
                                <p class="text-xs text-on-surface-variant">{grants} " · " {authority}</p>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <ul class="mt-2 space-y-0.5 text-xs text-on-surface-variant">
                {guilds.into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}
                {rosters.into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}
            </ul>
        </article>
    }
}
