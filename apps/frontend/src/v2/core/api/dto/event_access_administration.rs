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

use serde::{Deserialize, Serialize};

/* ───────────────────────── policies ───────────────────────── */

/// Who an operation, squad or slot admits: any one of its grants.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessPolicy {
    /// The alternatives, at most 32. None at all admits nobody.
    pub grants: Vec<EventAccessGrant>,
}

/// One alternative of a policy: it admits an account only when every condition holds.
#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventAccessCondition {
    /// Any signed-in account.
    Authenticated {},
    /// A verified member of the community's own Discord guild.
    TbdMember {},
    /// A holder of `role_id` in the Discord guild `guild_id` — the community's guild or a partner
    /// guild one of the operation's groups names.
    DiscordRole { guild_id: String, role_id: String },
    /// A member of one of this operation's event groups.
    EventGroup { group_id: String },
    /// One named account.
    NamedAccount { discord_id: String },
}

/// A squad's explicit policy. A squad without one follows the operation's policy.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SquadAccessPolicy {
    pub event_mission_id: String,
    pub faction: String,
    pub squad: String,
    pub policy: EventAccessPolicy,
}

/// A slot's explicit policy. A slot without one follows its squad's policy, or the operation's.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlotAccessPolicy {
    pub slot_id: String,
    pub event_mission_id: String,
    pub faction: String,
    pub squad: String,
    pub slot_index: i64,
    pub policy: EventAccessPolicy,
}

/* ───────────────────────── groups ───────────────────────── */

/// Where an event group's members come from.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventGroupSource {
    /// A roster administrators maintain; every entry records who added it.
    ManagedRoster {},
    /// Members of the partner guild `guild_id` holding every one of `required_role_ids` — guild
    /// membership alone when the list is empty — as bot-verified Discord observations report them.
    /// Membership is never self-asserted, so such a group has no roster.
    PartnerGuild {
        guild_id: String,
        required_role_ids: Vec<String>,
    },
}

/// Who created a group: exactly one of an account (`created_by`) and a system process
/// (`system_origin`) is set.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthorshipProvenance {
    pub created_by: Option<String>,
    pub system_origin: Option<String>,
    pub created_at: String,
}

/// One member of a managed roster, and who put them there.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RosterEntryView {
    pub discord_id: String,
    pub username: String,
    /// The account that added the entry; null when a system process did.
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    pub added_at: String,
}

/// One event group, with its provenance and — for a managed roster — its members.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupView {
    pub id: String,
    pub name: String,
    pub source: EventGroupSource,
    pub provenance: AuthorshipProvenance,
    /// Empty for a partner-guild group, whose membership comes from verified Discord data.
    pub roster: Vec<RosterEntryView>,
}

/* ───────────────────────── pools ───────────────────────── */

/// One reservation pool's configuration.
#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotas {
    pub member: ReservationQuotaPool,
    pub guest: ReservationQuotaPool,
    pub open: ReservationQuotaPool,
}

/// Places currently held, by the pool they came from. `legacy_unclassified` places predate the
/// pools: they count toward the operation-wide total and toward no pool.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotaUsageView {
    pub member: i64,
    pub guest: i64,
    pub open: i64,
    pub legacy_unclassified: i64,
    pub total: i64,
}

/* ───────────────────────── the administration view ───────────────────────── */

/// `GET /events/:id/access`: one operation's whole access configuration and its usage.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventAccessAdministration {
    pub event_id: String,
    /// The revision every change must name; each accepted change advances it by one.
    pub access_revision: i64,
    /// The operation-wide participant limit; zero leaves the operation uncapped.
    pub max_slots: i64,
    pub event_policy: EventAccessPolicy,
    pub squad_policies: Vec<SquadAccessPolicy>,
    pub slot_policies: Vec<SlotAccessPolicy>,
    pub groups: Vec<EventGroupView>,
    pub reservation_quotas: ReservationQuotas,
    pub quota_usage: QuotaUsageView,
}

/// What every change answers: the access view after it, and the reservations it released or
/// promoted from the waiting list, by registration id.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessChangeOutcome {
    pub access: EventAccessAdministration,
    pub released_registrations: Vec<String>,
    pub promoted_registrations: Vec<String>,
}

/* ───────────────────────── participant evidence ───────────────────────── */

/// One Discord guild's membership observation for a participant.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildEvidenceView {
    pub guild_id: String,
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RosterEvidenceView {
    pub group_id: String,
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    pub added_at: String,
}

/// The place a participant holds, and which pool it came from.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantAllocationView {
    /// `member`, `guest`, `open` or `legacy_unclassified`.
    pub quota_kind: String,
    pub acquired_at: String,
}

/// Why one of a participant's current reservations is, or is not, admitted.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegistrationDecision {
    pub registration_id: String,
    pub event_mission_id: String,
    /// `registered`, `waitlisted` or `legacy_unknown`.
    pub reservation_state: String,
    /// The seat held; null for a seatless place or a waiting entry.
    pub slot_id: Option<String>,
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParticipantAccessExplanation {
    pub discord_id: String,
    pub username: String,
    /// False when the account is banned or deleted.
    pub available: bool,
    pub tbd_member: bool,
    pub guilds: Vec<GuildEvidenceView>,
    pub roster_groups: Vec<RosterEvidenceView>,
    /// The place the participant holds; null when they hold none, as a waiting participant does.
    pub allocation: Option<ParticipantAllocationView>,
    pub registrations: Vec<RegistrationDecision>,
}

/* ───────────────────────── change requests ───────────────────────── */

/// `PUT` body of the operation, squad and slot policy routes.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessPolicyChange {
    pub expected_access_revision: i64,
    pub policy: EventAccessPolicy,
}

/// `POST /events/:id/groups` body.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupCreation {
    pub expected_access_revision: i64,
    pub name: String,
    pub source: EventGroupSource,
}

/// `PATCH /events/:id/groups/:groupId` body: an absent field keeps its current value.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventGroupChange {
    pub expected_access_revision: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<EventGroupSource>,
}

/// `PUT /events/:id/reservation-quotas` body: all three pools, replaced together.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservationQuotaChange {
    pub expected_access_revision: i64,
    pub reservation_quotas: ReservationQuotas,
}

/// The precondition alone: the body of a roster addition, and the query every removal carries.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccessRevisionPrecondition {
    pub expected_access_revision: i64,
}
