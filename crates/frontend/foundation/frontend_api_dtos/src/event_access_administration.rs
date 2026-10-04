//! An operation's access configuration as its administrators see it, and every change to it.
//!
//! **Role:** the access policies of the operation, its squads and its slots; the event groups with
//! their provenance and roster; the three reservation pools and how much of each is used; the
//! evidence behind each participant's eligibility; and the request bodies every change sends.
//! **Position:** deserialised from the access administration routes and handed to the event
//! manager's access panel, which also serialises the request bodies.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** grants are alternatives — any one admits — and the conditions of one grant must
//! all hold. An empty grant list admits nobody, while an absent squad or slot policy inherits: a
//! slot follows its squad's policy, and a squad the operation's. Conditions and group sources are
//! tagged by `kind` and mirror the backend's enums exactly, unknown fields refused: the panel sends
//! them back, so a shape this build does not know must fail to load rather than be silently
//! rewritten by the next save. Every change names the `access_revision` it was prepared against.
//! Provenance keys and a participant's missing allocation cross the wire as explicit nulls, so none
//! of them is skipped when serialising.

use super::identifiers::{
    DiscordGuildId, DiscordUserId, EventGroupId, EventId, EventMissionId, EventRegistrationId,
    OrbatSlotId,
};
use serde::{Deserialize, Serialize};

/* ───────────────────────── policies ───────────────────────── */

/// Who an operation, squad or slot admits: any one of its grants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessPolicy {
    /// The alternatives, at most 32. None at all admits nobody.
    pub grants: Vec<EventAccessGrant>,
}

/// One alternative of a policy: it admits an account only when every condition holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessGrant {
    /// One to sixteen conditions, all required.
    pub conditions: Vec<EventAccessCondition>,
}

/// One fact about an account that a grant can require.
///
/// The field-less kinds are empty struct variants rather than unit variants: only a struct variant
/// refuses an unknown field, and a unit variant would silently drop one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventAccessCondition {
    /// Any signed-in account.
    Authenticated {},
    /// A verified member of the community's own Discord guild.
    TbdMember {},
    /// A holder of `role_id` in the Discord guild `guild_id` — the community's guild or a partner
    /// guild one of the operation's groups names.
    DiscordRole {
        /// The Discord guild's id.
        guild_id: String,
        /// The Discord role's id.
        role_id: String,
    },
    /// A member of one of this operation's event groups.
    EventGroup {
        /// The event group's id.
        group_id: String,
    },
    /// One named account.
    NamedAccount {
        /// The account's Discord id.
        discord_id: String,
    },
}

/// A squad's explicit policy. A squad without one follows the operation's policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// A slot's explicit policy. A slot without one follows its squad's policy, or the operation's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/* ───────────────────────── groups ───────────────────────── */

/// Where an event group's members come from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventGroupSource {
    /// A roster administrators maintain; every entry records who added it.
    ManagedRoster {},
    /// Members of the partner guild `guild_id` holding every one of `required_role_ids` — guild
    /// membership alone when the list is empty — as bot-verified Discord observations report them.
    /// Membership is never self-asserted, so such a group has no roster.
    PartnerGuild {
        /// The partner guild's Discord id.
        guild_id: String,
        /// The Discord role ids a member must hold, every one of them.
        required_role_ids: Vec<String>,
    },
}

/// Who created a group: exactly one of an account (`created_by`) and a system process
/// (`system_origin`) is set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthorshipProvenance {
    /// Manager account that authored the record; absent for a system-authored record.
    pub created_by: Option<String>,
    /// Named system transition that authored the record; absent for a manager-authored one.
    pub system_origin: Option<String>,
    /// When the record was created, as an RFC 3339 UTC timestamp.
    pub created_at: String,
}

/// One member of a managed roster, and who put them there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RosterEntryView {
    /// Discord account of the roster member.
    pub discord_id: DiscordUserId,
    /// The member's platform username; empty when the account has none.
    pub username: String,
    /// The account that added the entry; null when a system process did.
    pub added_by: Option<String>,
    /// Named system transition that added the member; absent when a manager added them.
    pub system_origin: Option<String>,
    /// When the member joined the roster, as an RFC 3339 UTC timestamp.
    pub added_at: String,
}

/// One event group, with its provenance and — for a managed roster — its members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupView {
    /// Group identifier.
    pub id: EventGroupId,
    /// Display name of the group.
    pub name: String,
    /// Membership source: a managed roster or a partner Discord guild.
    pub source: EventGroupSource,
    /// Who or what created the group, and when.
    pub provenance: AuthorshipProvenance,
    /// Empty for a partner-guild group, whose membership comes from verified Discord data.
    pub roster: Vec<RosterEntryView>,
}

/* ───────────────────────── pools ───────────────────────── */

/// One reservation pool's configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaPool {
    /// The pool's limit: null leaves it uncapped and zero closes it. Always sent, as an explicit
    /// null when uncapped — the backend refuses a pool whose limit is absent rather than reading
    /// absence as "uncapped".
    pub seats: Option<i64>,
    /// When the pool starts granting places, as an RFC 3339 UTC instant.
    pub opens_at: String,
}

/// The three pools. Members draw from `member` and everyone else from `guest`; either overflows
/// to `open` once that pool opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotas {
    /// The pool verified TBD members draw from first.
    pub member: ReservationQuotaPool,
    /// The pool other admitted accounts draw from first.
    pub guest: ReservationQuotaPool,
    /// The fallback pool any admitted account may draw from.
    pub open: ReservationQuotaPool,
}

/// Places currently held, by the pool they came from. `legacy_unclassified` places predate the
/// pools: they count toward the operation-wide total and toward no pool.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotaUsageView {
    /// Allocations drawn from the member pool.
    pub member: i64,
    /// Allocations drawn from the guest pool.
    pub guest: i64,
    /// Allocations drawn from the open pool.
    pub open: i64,
    /// Allocations recorded before pools existed; they count toward `total` and no pool.
    pub legacy_unclassified: i64,
    /// Every active allocation of the event, whatever its kind.
    pub total: i64,
}

/* ───────────────────────── the administration view ───────────────────────── */

/// `GET /events/:id/access`: one operation's whole access configuration and its usage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventAccessAdministration {
    /// The event this view describes.
    pub event_id: EventId,
    /// The revision every change must name; each accepted change advances it by one.
    pub access_revision: i64,
    /// The operation-wide participant limit; zero leaves the operation uncapped.
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

/// What every change answers: the access view after it, and the reservations it released or
/// promoted from the waiting list, by registration id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessChangeOutcome {
    /// The access view after the change.
    pub access: EventAccessAdministration,
    /// Reservations the change released because their holder no longer has access.
    pub released_registrations: Vec<String>,
    /// Waiting-list entries the change promoted into the places it freed.
    pub promoted_registrations: Vec<String>,
}

/* ───────────────────────── participant evidence ───────────────────────── */

/// One Discord guild's membership observation for a participant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildEvidenceView {
    /// The guild the observation is for.
    pub guild_id: DiscordGuildId,
    /// `member`, `nonmember` or `unknown`.
    pub membership_status: String,
    /// When the bot last verified the observation; null when it never has.
    pub verified_at: Option<String>,
    /// The end of an audited override that extends the observation; null without one.
    pub override_until: Option<String>,
    /// Fresh, inside the grace period, or extended by an override.
    pub current: bool,
}

/// One managed-roster entry backing a participant's group membership.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RosterEvidenceView {
    /// The roster group.
    pub group_id: EventGroupId,
    /// Manager who added the account; absent when a system transition added it.
    pub added_by: Option<String>,
    /// Named system transition that added the account; absent when a manager added it.
    pub system_origin: Option<String>,
    /// When the account joined the roster, as an RFC 3339 UTC timestamp.
    pub added_at: String,
}

/// The place a participant holds, and which pool it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantAllocationView {
    /// `member`, `guest`, `open` or `legacy_unclassified`.
    pub quota_kind: String,
    /// When the allocation was acquired, as an RFC 3339 UTC timestamp.
    pub acquired_at: String,
}

/// Why one of a participant's current reservations is, or is not, admitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegistrationDecision {
    /// The registration this decision explains.
    pub registration_id: EventRegistrationId,
    /// The event mission the registration belongs to.
    pub event_mission_id: EventMissionId,
    /// `registered`, `waitlisted` or `legacy_unknown`.
    pub reservation_state: String,
    /// The seat held; null for a seatless place or a waiting entry.
    pub slot_id: Option<OrbatSlotId>,
    /// When the registration entered the queue, as an RFC 3339 UTC timestamp.
    pub queue_entered_at: String,
    /// Which policy decides: `event`, `squad` or `slot`.
    pub policy_source: String,
    /// Zero-based indices of the deciding policy's grants whose conditions all hold now.
    pub admitting_grants: Vec<i64>,
    /// Whether current membership authority admits the reservation.
    pub current_authority_admits: bool,
    /// Whether the last verified facts admit it; only these decide a confirmed loss, so stale data
    /// never releases a reservation.
    pub last_verified_admits: bool,
}

/// `GET /events/:id/access/participants`, one element: why a participant is, or is not, eligible.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantAccessExplanation {
    /// Discord account of the participant.
    pub discord_id: DiscordUserId,
    /// The participant's platform username.
    pub username: String,
    /// False when the account is banned or deleted.
    pub available: bool,
    /// The account is a TBD member under current membership authority.
    pub tbd_member: bool,
    /// Membership observations for the guilds this event's policies rely on.
    pub guilds: Vec<GuildEvidenceView>,
    /// Managed roster groups of the event that list the account.
    pub roster_groups: Vec<RosterEvidenceView>,
    /// The place the participant holds; null when they hold none, as a waiting participant does.
    pub allocation: Option<ParticipantAllocationView>,
    /// Every current registration of the account in the event, with its access decision.
    pub registrations: Vec<RegistrationDecision>,
}

/* ───────────────────────── change requests ───────────────────────── */

/// `PUT` body of the operation, squad and slot policy routes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessPolicyChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// The new event-wide policy.
    pub policy: EventAccessPolicy,
}

/// `POST /events/:id/groups` body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupCreation {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// Display name of the new group.
    pub name: String,
    /// Membership source: a managed roster or a partner Discord guild.
    pub source: EventGroupSource,
}

/// `PATCH /events/:id/groups/:groupId` body: an absent field keeps its current value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// New display name, or absent to keep the current one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New membership source, or absent to keep the current one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EventGroupSource>,
}

/// `PUT /events/:id/reservation-quotas` body: all three pools, replaced together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservationQuotaChange {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
    /// The new member, guest and open pool limits and opening times.
    pub reservation_quotas: ReservationQuotas,
}

/// The precondition alone: the body of a roster addition, and the query every removal carries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessRevisionPrecondition {
    /// The `access_revision` the change was prepared against; a stale value refuses it.
    pub expected_access_revision: i64,
}
