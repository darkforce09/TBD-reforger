//! Manager-facing access administration: policies, groups with provenance, quotas, and the
//! evidence behind each participant's eligibility. Every change names the access revision it
//! was prepared against, so concurrent manager edits cannot silently overwrite each other.

use api_identifiers::{
    DiscordGuildId, DiscordUserId, EventGroupId, EventId, EventMissionId, EventRegistrationId,
    OrbatSlotId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::event_access_policy::EventAccessPolicy;
use super::event_group::EventGroupSource;
use super::participant_allocation::ParticipantAllocationKind;
use super::reservation_quota::ReservationQuotas;
use crate::models::RegistrationState;
use crate::services::event_access::evaluation::PolicySource;
use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// Manager request replacing the event-wide access policy.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessPolicyChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// The new event-wide policy.
    pub policy: EventAccessPolicy,
}

/// Precondition for changes without a body, carried as a query parameter.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessRevisionPrecondition {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
}

/// Manager request creating an event group.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroupCreation {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// Display name of the new group.
    pub name: String,
    /// Membership source: a managed roster or a partner Discord guild.
    pub source: EventGroupSource,
}

/// Manager request editing an event group; an absent field keeps its stored value.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroupChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// New display name, or absent to keep the current one.
    #[serde(default)]
    pub name: Option<String>,
    /// New membership source, or absent to keep the current one.
    #[serde(default)]
    pub source: Option<EventGroupSource>,
}

/// Manager request replacing the event's reservation pools.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// The new member, guest and open pool limits and opening times.
    pub reservation_quotas: ReservationQuotas,
}

/// An explicit access policy set on one squad of one event mission.
#[derive(Debug, Clone, Serialize)]
pub struct SquadAccessPolicy {
    /// The event mission whose ORBAT holds the squad.
    pub event_mission_id: EventMissionId,
    /// Faction the squad belongs to.
    pub faction: String,
    /// Squad name within its faction.
    pub squad: String,
    /// The squad's policy; it overrides the event policy for the squad's seats.
    pub policy: EventAccessPolicy,
}

/// An explicit access policy set on one ORBAT seat.
#[derive(Debug, Clone, Serialize)]
pub struct SlotAccessPolicy {
    /// The seat the policy applies to.
    pub slot_id: OrbatSlotId,
    /// The event mission whose ORBAT holds the seat.
    pub event_mission_id: EventMissionId,
    /// Faction the seat belongs to.
    pub faction: String,
    /// Squad the seat belongs to.
    pub squad: String,
    /// Position of the seat within its squad.
    pub slot_index: i64,
    /// The seat's policy; it overrides its squad and event policies.
    pub policy: EventAccessPolicy,
}

/// Exactly one of `created_by` and `system_origin` is present.
#[derive(Debug, Clone, Serialize)]
pub struct AuthorshipProvenance {
    /// Manager account that authored the record; absent for a system-authored record.
    pub created_by: Option<String>,
    /// Named system transition that authored the record; absent for a manager-authored one.
    pub system_origin: Option<String>,
    /// When the record was created, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// One member of a managed roster, as the manager view lists it.
#[derive(Debug, Clone, Serialize)]
pub struct RosterEntryView {
    /// Discord account of the roster member.
    pub discord_id: DiscordUserId,
    /// The member's platform username; empty when the account has none.
    pub username: String,
    /// Manager who added the member; absent when a system transition added them.
    pub added_by: Option<String>,
    /// Named system transition that added the member; absent when a manager added them.
    pub system_origin: Option<String>,
    /// When the member joined the roster, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub added_at: DateTime<Utc>,
}

/// One event group with its provenance and roster, as the manager view lists it.
#[derive(Debug, Clone, Serialize)]
pub struct EventGroupView {
    /// Group identifier.
    pub id: EventGroupId,
    /// Display name of the group.
    pub name: String,
    /// Membership source: a managed roster or a partner Discord guild.
    pub source: EventGroupSource,
    /// Who or what created the group, and when.
    pub provenance: AuthorshipProvenance,
    /// Managed rosters list their members; partner-guild groups rely on verified Discord data.
    pub roster: Vec<RosterEntryView>,
}

/// The event's active participant allocations, counted per pool.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct QuotaUsageView {
    /// Allocations drawn from the member pool.
    pub member: u64,
    /// Allocations drawn from the guest pool.
    pub guest: u64,
    /// Allocations drawn from the open pool.
    pub open: u64,
    /// Allocations recorded before pools existed; they count toward `total` and no pool.
    pub legacy_unclassified: u64,
    /// Every active allocation of the event, whatever its kind.
    pub total: u64,
}

/// The manager's view of one event's access configuration and pool usage.
#[derive(Debug, Clone, Serialize)]
pub struct EventAccessAdministration {
    /// The event this view describes.
    pub event_id: EventId,
    /// Current access revision; a change names it as `expected_access_revision`.
    pub access_revision: i64,
    /// Event-wide participant limit (0 to 256) shared by every pool.
    pub max_slots: i64,
    /// The event-wide access policy.
    pub event_policy: EventAccessPolicy,
    /// Explicit squad policies; a squad without one inherits the event policy.
    pub squad_policies: Vec<SquadAccessPolicy>,
    /// Explicit seat policies; a seat without one inherits its squad or event policy.
    pub slot_policies: Vec<SlotAccessPolicy>,
    /// Every live group of the event, with its roster.
    pub groups: Vec<EventGroupView>,
    /// Configured pool limits and opening times.
    pub reservation_quotas: ReservationQuotas,
    /// Active allocations per pool.
    pub quota_usage: QuotaUsageView,
}

/// The access view after a change, with the reservations the change released or promoted.
#[derive(Debug, Clone, Serialize)]
pub struct AccessChangeOutcome {
    /// The access view after the change.
    pub access: EventAccessAdministration,
    /// Reservations the change released because their holder no longer has access.
    pub released_registrations: Vec<EventRegistrationId>,
    /// Waiting-list entries the change promoted into the places it freed.
    pub promoted_registrations: Vec<EventRegistrationId>,
}

/// One account's recorded membership in one Discord guild the event's policies rely on.
#[derive(Debug, Clone, Serialize)]
pub struct GuildEvidenceView {
    /// The guild the observation is for.
    pub guild_id: DiscordGuildId,
    /// Recorded status: `unknown`, `member` or `nonmember`.
    pub membership_status: String,
    /// When the bot last verified the membership; absent while the status is `unknown`.
    #[serde(with = "rfc3339_utc_opt")]
    pub verified_at: Option<DateTime<Utc>>,
    /// End of an audited override that keeps the observation current; absent without one.
    #[serde(with = "rfc3339_utc_opt")]
    pub override_until: Option<DateTime<Utc>>,
    /// Fresh, inside the grace period, or extended by an audited override.
    pub current: bool,
}

/// One managed roster group of the event that lists the account.
#[derive(Debug, Clone, Serialize)]
pub struct RosterEvidenceView {
    /// The roster group.
    pub group_id: EventGroupId,
    /// Manager who added the account; absent when a system transition added it.
    pub added_by: Option<String>,
    /// Named system transition that added the account; absent when a manager added it.
    pub system_origin: Option<String>,
    /// When the account joined the roster, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub added_at: DateTime<Utc>,
}

/// The pool allocation an account holds in the event.
#[derive(Debug, Clone, Serialize)]
pub struct ParticipantAllocationView {
    /// The pool the allocation was recorded against.
    pub quota_kind: ParticipantAllocationKind,
    /// When the allocation was acquired, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub acquired_at: DateTime<Utc>,
}

/// Whether access policy admits one of the account's registrations, and why.
#[derive(Debug, Clone, Serialize)]
pub struct RegistrationDecision {
    /// The registration this decision explains.
    pub registration_id: EventRegistrationId,
    /// The event mission the registration belongs to.
    pub event_mission_id: EventMissionId,
    /// The registration's reservation state.
    pub reservation_state: RegistrationState,
    /// The seat the registration holds, or `None` for a seatless place or a waiting entry.
    pub slot_id: Option<OrbatSlotId>,
    /// When the registration entered the queue, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub queue_entered_at: DateTime<Utc>,
    /// Level of the effective policy: event, squad or slot.
    pub policy_source: PolicySource,
    /// Indices of the effective policy's grants whose conditions all hold under current authority.
    pub admitting_grants: Vec<usize>,
    /// Policy admits the account under current membership authority.
    pub current_authority_admits: bool,
    /// Last verified facts decide confirmed loss; stale data never releases a reservation.
    pub last_verified_admits: bool,
}

/// The manager's explanation of one participant's eligibility in one event.
#[derive(Debug, Clone, Serialize)]
pub struct ParticipantAccessExplanation {
    /// Discord account of the participant.
    pub discord_id: DiscordUserId,
    /// The participant's platform username.
    pub username: String,
    /// The account is neither banned nor deleted.
    pub available: bool,
    /// The account is a TBD member under current membership authority.
    pub tbd_member: bool,
    /// Membership observations for the guilds this event's policies rely on.
    pub guilds: Vec<GuildEvidenceView>,
    /// Managed roster groups of the event that list the account.
    pub roster_groups: Vec<RosterEvidenceView>,
    /// The account's pool allocation in the event; absent when it holds none.
    pub allocation: Option<ParticipantAllocationView>,
    /// Every current registration of the account in the event, with its access decision.
    pub registrations: Vec<RegistrationDecision>,
}
