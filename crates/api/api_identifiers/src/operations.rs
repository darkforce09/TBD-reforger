//! The identifiers of the operations domain: events, their missions, ORBAT slots, registrations
//! and the records that hang off them.
//!
//! **Role:** declares one typed id per operations table key, plus the game runtime's player-life
//! key and a submitted, unparsed event id.
//! **Position:** declared here, below every API crate; the operations domain owns the tables,
//! and the missions, telemetry, membership and administration code that names an event or a slot
//! takes the same type.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner UUID, text or
//! integer (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those
//! of the bare value.

newtype_ids::uuid_id! {
    sqlx,
    /// An event, the scheduled session record: the key of `events`.
    pub struct EventId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One mission attached to an event: the key of `event_missions`.
    pub struct EventMissionId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One slot of an event mission's ORBAT: the key of `orbat_slots`.
    pub struct OrbatSlotId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A leader's hold on a whole squad of an event mission: the key of `orbat_reservations`.
    pub struct OrbatReservationId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A member's registration for an event mission: the key of `event_registrations`.
    pub struct EventRegistrationId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A named group of members an event's access policy admits: the key of `event_groups`.
    pub struct EventGroupId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A member's hold on one of an event's quota places: the key of
    /// `event_participant_allocations`.
    pub struct EventParticipantAllocationId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A member's leave of absence: the key of `leave_requests`.
    pub struct LeaveRequestId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A player's live occupancy of an ORBAT slot during a runtime session: the key of
    /// `live_slot_occupancies`.
    pub struct LiveSlotOccupancyId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A stored fire mission: the key of `fire_missions`.
    pub struct FireMissionId;
}

newtype_ids::string_id! {
    sqlx,
    /// A player's life in a running game, chosen by the game runtime: it identifies the retries
    /// of one deployment request.
    pub struct PlayerLifeId;
}

newtype_ids::string_id! {
    /// An event id as a client submitted it, unparsed: the handler that reads it parses it into
    /// an [`EventId`] and answers its own refusal when it is blank or not one.
    pub struct SubmittedEventId;
}
