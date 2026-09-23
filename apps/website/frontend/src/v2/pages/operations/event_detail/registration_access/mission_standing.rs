//! The viewer's standing on one mission: what they hold, where they wait, and what they may do.
//!
//! **Role:** gathers everything a mission card and its slotting controls need to know about the
//! viewer beyond the order of battle — the reservation state, the waiting position, a released
//! signup's reason and time, whether any seat admits them, and their place outlook — and decides
//! from it which of register, join the waiting list and withdraw are offered. Renders the notices
//! that explain that standing on the mission card.
//! **Position:** built from the operation dossier by the hub body for each mission card, and by the
//! standalone slotting page for its one mission; handed down to the slotting selector.
//! **Signals & state:** none; plain data and pure decisions, plus one stateless view.
//! **Invariants:** the offers are exclusive in the way the backend is. Registering for a seat needs
//! a place to be had now; joining the waiting list is offered only when a seat of the mission admits
//! the viewer and there is no place or free seat to take; a viewer already seated or waiting is
//! offered neither, while one registered without a seat — who holds the place already — may take a
//! free seat. A mission the operation does not list — a stale link — knows nothing about the viewer
//! and leaves every decision to the backend.

use super::super::slotting_selector::can_register_reservation;
use super::place_outlook::{holds_place, place_outlook, PlaceOutlook};
use super::refusal_notices::pool_name;
use crate::v2::core::api::dto::{EventHub, EventMissionDossier};
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::datefmt::format_local_datetime;
use crate::v2::core::utils::utc_timestamp::utc_label;
use leptos::prelude::*;

/// The viewer's standing on one mission of an operation.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionStanding {
    /// The viewer's current reservation state on the mission, if they have signed up.
    pub(crate) reservation_state: Option<String>,
    /// The viewer's signup holds a seat of this mission.
    pub(crate) holds_seat: bool,
    /// One-based position in the mission's waiting queue, while waitlisted.
    pub(crate) waiting_position: Option<i64>,
    /// Why a released signup was released.
    pub(crate) release_reason: Option<String>,
    /// When the signup was first released, as an RFC 3339 UTC instant.
    pub(crate) released_at: Option<String>,
    /// Some seat of the mission admits the viewer now.
    pub(crate) eligible: bool,
    /// A Discord verification the operation's policies rely on is pending for the viewer.
    pub(crate) verification_pending: bool,
    /// Every seat the viewer can see on the mission is taken.
    pub(crate) mission_full: bool,
    /// The viewer's prospect of a place in the operation.
    pub(crate) outlook: PlaceOutlook,
}

impl MissionStanding {
    /// The standing of the viewer on `mission`, one of the missions of `hub`.
    pub(crate) fn of(hub: &EventHub, mission: &EventMissionDossier) -> Self {
        Self {
            reservation_state: mission.my_reservation_state.clone(),
            holds_seat: mission.my_slot_id.is_some(),
            waiting_position: mission.my_waiting_position,
            release_reason: mission.my_release_reason.clone(),
            released_at: mission.my_withdrawn_at.clone(),
            eligible: mission.viewer_eligible,
            verification_pending: hub.viewer_access.membership_verification_pending,
            mission_full: mission.total > 0 && mission.filled >= mission.total,
            outlook: place_outlook(hub),
        }
    }

    /// The standing on a mission the operation does not list: nothing is known about the viewer,
    /// so no offer is withheld and the backend decides.
    pub(crate) fn unlisted() -> Self {
        Self {
            reservation_state: None,
            holds_seat: false,
            waiting_position: None,
            release_reason: None,
            released_at: None,
            eligible: true,
            verification_pending: false,
            mission_full: false,
            outlook: PlaceOutlook::Available,
        }
    }

    /// Whether claiming a seat is offered: the viewer may sign up, or is registered without a
    /// seat, and a place can be had now.
    pub(crate) fn seat_claim_offered(&self) -> bool {
        let state = self.reservation_state.as_deref();
        let registered_without_seat = holds_place(state) && !self.holds_seat;
        (can_register_reservation(state) || registered_without_seat)
            && matches!(self.outlook, PlaceOutlook::Held | PlaceOutlook::Available)
            && !self.mission_full
    }

    /// Whether joining the waiting list is offered: the viewer may sign up, a seat of the mission
    /// admits them, and there is no place or free seat to take now — or a refusal just said so.
    pub(crate) fn waiting_list_offered(&self, refusal_suggests_it: bool) -> bool {
        can_register_reservation(self.reservation_state.as_deref())
            && self.eligible
            && (self.mission_full || self.outlook == PlaceOutlook::Exhausted || refusal_suggests_it)
    }

    /// The footer line about the viewer's own signup, when they have one worth stating.
    /// A reservation recorded before reservation states were tracked (`legacy_unknown`) holds a
    /// place exactly as a registration does.
    pub(crate) fn signup_line(&self) -> Option<String> {
        match self.reservation_state.as_deref()? {
            "registered" | "legacy_unknown" if self.holds_seat => {
                Some("You are registered for this mission.".into())
            }
            "registered" | "legacy_unknown" => Some(
                "You hold a place on this mission without a seat; pick a free seat to take one."
                    .into(),
            ),
            "waitlisted" => Some(match self.waiting_position {
                Some(position) => {
                    format!("You are on the waiting list, position {position}.")
                }
                None => "You are on the waiting list.".to_string(),
            }),
            _ => None,
        }
    }
}

/// Why a signup was released, as a sentence the viewer reads about their own signup.
pub(crate) fn release_reason_sentence(reason: &str) -> String {
    match reason {
        "participant_withdrew" => "You withdrew.".to_string(),
        "mission_removed" => "The mission was removed from the operation.".to_string(),
        "event_cancelled" => "The operation was cancelled.".to_string(),
        "event_deleted" => "The operation was deleted.".to_string(),
        "eligibility_lost" => {
            "Your verified membership no longer meets this mission's access policy.".to_string()
        }
        "access_policy_changed" => {
            "An administrator changed the operation's access settings, and they no longer admit you."
                .to_string()
        }
        "account_unavailable" => "Your account became unavailable.".to_string(),
        "seat_cleared" => "A leader or administrator cleared your seat.".to_string(),
        other => format!("Released: {}.", other.replace('_', " ")),
    }
}

/// The notices on a mission card about the viewer's standing: a waiting position, a released
/// signup with its reason and time, a mission no seat of which admits them, a pending verification
/// and a pool that has not opened yet. Renders nothing when there is nothing to say.
pub(crate) fn standing_notices(standing: &MissionStanding) -> impl IntoView {
    let mut notices: Vec<(&'static str, &'static str, String)> = Vec::new();
    if let (Some("waitlisted"), Some(position)) = (
        standing.reservation_state.as_deref(),
        standing.waiting_position,
    ) {
        notices.push((
            "hourglass_top",
            "text-tactical-yellow",
            format!("You are number {position} on this mission's waiting list."),
        ));
    }
    if let Some(reason) = standing.release_reason.as_deref() {
        let when = standing
            .released_at
            .as_deref()
            .map(|at| {
                format!(
                    " Released {} ({}).",
                    format_local_datetime(at),
                    utc_label(at)
                )
            })
            .unwrap_or_default();
        notices.push((
            "event_busy",
            "text-on-surface-variant",
            format!(
                "Your signup was released. {}{when}",
                release_reason_sentence(reason)
            ),
        ));
    }
    if !standing.eligible {
        notices.push((
            "lock",
            "text-on-surface-variant",
            "No seat in this mission is open to you under its access policies.".to_string(),
        ));
    }
    if standing.verification_pending {
        notices.push((
            "pending",
            "text-tactical-yellow",
            "Your Discord membership is being verified; seats that rely on it open to you once \
             verification completes."
                .to_string(),
        ));
    }
    if let PlaceOutlook::OpensLater {
        quota_kind,
        opens_at,
    } = &standing.outlook
    {
        if can_register_reservation(standing.reservation_state.as_deref()) {
            notices.push((
                "schedule",
                "text-on-surface-variant",
                format!(
                    "{} open {} ({}).",
                    pool_name(Some(quota_kind)),
                    format_local_datetime(opens_at),
                    utc_label(opens_at)
                ),
            ));
        }
    }
    (!notices.is_empty()).then(|| {
        view! {
            <ul class="mt-3 space-y-1.5" data-testid="mission-standing-notices">
                {notices
                    .into_iter()
                    .map(|(icon, tone, text)| {
                        view! {
                            <li class=format!("flex items-start gap-2 text-sm {tone}")>
                                <MaterialIcon name=icon class="text-base" />
                                <span>{text}</span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        }
    })
}
