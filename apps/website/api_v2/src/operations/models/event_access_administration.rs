//! Manager-facing access administration: policies, groups with provenance, quotas, and the
//! evidence behind each participant's eligibility. Every change names the access revision it
//! was prepared against, so concurrent manager edits cannot silently overwrite each other.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event_access_policy::EventAccessPolicy;
use super::event_group::EventGroupSource;
use super::participant_allocation::ParticipantAllocationKind;
use super::reservation_quota::ReservationQuotas;
use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};
use crate::operations::models::RegistrationState;
use crate::operations::services::event_access::evaluation::PolicySource;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessPolicyChange {
    pub expected_access_revision: i64,
    pub policy: EventAccessPolicy,
}

/// Precondition for changes without a body, carried as a query parameter.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessRevisionPrecondition {
    pub expected_access_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroupCreation {
    pub expected_access_revision: i64,
    pub name: String,
    pub source: EventGroupSource,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroupChange {
    pub expected_access_revision: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub source: Option<EventGroupSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaChange {
    pub expected_access_revision: i64,
    pub reservation_quotas: ReservationQuotas,
}

#[derive(Debug, Clone, Serialize)]
pub struct SquadAccessPolicy {
    pub event_mission_id: Uuid,
    pub faction: String,
    pub squad: String,
    pub policy: EventAccessPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlotAccessPolicy {
    pub slot_id: Uuid,
    pub event_mission_id: Uuid,
    pub faction: String,
    pub squad: String,
    pub slot_index: i64,
    pub policy: EventAccessPolicy,
}

/// Exactly one of `created_by` and `system_origin` is present.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorshipProvenance {
    pub created_by: Option<String>,
    pub system_origin: Option<String>,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterEntryView {
    pub discord_id: String,
    pub username: String,
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    #[serde(with = "rfc3339_utc")]
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventGroupView {
    pub id: Uuid,
    pub name: String,
    pub source: EventGroupSource,
    pub provenance: AuthorshipProvenance,
    /// Managed rosters list their members; partner-guild groups rely on verified Discord data.
    pub roster: Vec<RosterEntryView>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct QuotaUsageView {
    pub member: u64,
    pub guest: u64,
    pub open: u64,
    pub legacy_unclassified: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventAccessAdministration {
    pub event_id: Uuid,
    pub access_revision: i64,
    pub max_slots: i64,
    pub event_policy: EventAccessPolicy,
    pub squad_policies: Vec<SquadAccessPolicy>,
    pub slot_policies: Vec<SlotAccessPolicy>,
    pub groups: Vec<EventGroupView>,
    pub reservation_quotas: ReservationQuotas,
    pub quota_usage: QuotaUsageView,
}

/// The access view after a change, with the reservations the change released or promoted.
#[derive(Debug, Clone, Serialize)]
pub struct AccessChangeOutcome {
    pub access: EventAccessAdministration,
    pub released_registrations: Vec<Uuid>,
    pub promoted_registrations: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildEvidenceView {
    pub guild_id: String,
    pub membership_status: String,
    #[serde(with = "rfc3339_utc_opt")]
    pub verified_at: Option<DateTime<Utc>>,
    #[serde(with = "rfc3339_utc_opt")]
    pub override_until: Option<DateTime<Utc>>,
    /// Fresh, inside the grace period, or extended by an audited override.
    pub current: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RosterEvidenceView {
    pub group_id: Uuid,
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    #[serde(with = "rfc3339_utc")]
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParticipantAllocationView {
    pub quota_kind: ParticipantAllocationKind,
    #[serde(with = "rfc3339_utc")]
    pub acquired_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrationDecision {
    pub registration_id: Uuid,
    pub event_mission_id: Uuid,
    pub reservation_state: RegistrationState,
    pub slot_id: Option<Uuid>,
    #[serde(with = "rfc3339_utc")]
    pub queue_entered_at: DateTime<Utc>,
    pub policy_source: PolicySource,
    /// Indices of the effective policy's grants whose conditions all hold under current authority.
    pub admitting_grants: Vec<usize>,
    pub current_authority_admits: bool,
    /// Last verified facts decide confirmed loss; stale data never releases a reservation.
    pub last_verified_admits: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParticipantAccessExplanation {
    pub discord_id: String,
    pub username: String,
    pub available: bool,
    pub tbd_member: bool,
    pub guilds: Vec<GuildEvidenceView>,
    pub roster_groups: Vec<RosterEvidenceView>,
    pub allocation: Option<ParticipantAllocationView>,
    pub registrations: Vec<RegistrationDecision>,
}
