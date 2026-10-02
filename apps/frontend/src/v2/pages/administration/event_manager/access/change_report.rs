//! What an access change did, and why one was refused, in words an administrator reads.
//!
//! **Role:** names the reservations a change released or promoted by the participant and mission
//! they belong to, recognises a stale-revision refusal, and words every change refusal.
//! **Position:** read by the access panel's state after every change, and by its report banner.
//! **Signals & state:** none; pure over the answer and the participants read before the change.
//! **Invariants:** a released reservation no longer appears in the participants read after the
//! change, so reservations are named from the list read before it — both released and promoted
//! reservations were current then. An id the list does not know is shown as the id itself rather
//! than dropped, so the count the operator sees is always the count the backend reported.

use super::state::MissionSeats;
use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::ParticipantAccessExplanation;

/// What the panel says about the last change until the next one: what it did, or why it was
/// refused.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PanelNotice {
    Changed(ChangeReport),
    Refused(String),
}

/// What the last change did.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChangeReport {
    /// What was changed, for the banner's heading.
    pub(super) change: String,
    /// The reservations the change released.
    pub(super) released: Vec<String>,
    /// The waiting participants the change seated.
    pub(super) promoted: Vec<String>,
}

/// Whether a refusal says the access settings changed after the panel read them.
pub(super) fn is_revision_conflict(refusal: &ApiRefusal) -> bool {
    refusal.status == 409 && refusal.code() == Some("ACCESS_REVISION_CONFLICT")
}

/// The sentence a refused change is reported with.
pub(super) fn change_refusal_sentence(refusal: &ApiRefusal) -> String {
    if is_revision_conflict(refusal) {
        "Another administrator changed this operation's access settings after you loaded them, so \
         your change was not applied. The latest settings are shown now; apply your change again \
         if it still stands."
            .to_string()
    } else {
        refusal.message_or("The change was refused")
    }
}

/// Each registration id as `participant — mission`, from the participants read before the change.
pub(super) fn describe_registrations(
    ids: &[String],
    participants: &[ParticipantAccessExplanation],
    missions: &[MissionSeats],
) -> Vec<String> {
    ids.iter()
        .map(|id| {
            participants
                .iter()
                .find_map(|participant| {
                    participant
                        .registrations
                        .iter()
                        .find(|registration| &registration.registration_id == id)
                        .map(|registration| {
                            let mission = missions
                                .iter()
                                .find(|m| m.event_mission_id == registration.event_mission_id)
                                .map(|m| m.title.as_str())
                                .unwrap_or("a mission");
                            format!("{} — {mission}", participant.username)
                        })
                })
                .unwrap_or_else(|| format!("Registration {id}"))
        })
        .collect()
}
