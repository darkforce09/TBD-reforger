//! The shape of the load fixture: ten open events titled `[Load fixture] 01` to
//! `[Load fixture] 10`, each attaching one mission with a two-faction ORBAT of eight squads of
//! eight slots.
//!
//! **Role:** the fixture's fixed values (title prefix, event count, ORBAT, start offset, place
//! cap) and the builders that turn them into validated event creations and an attachment
//! template.
//!
//! **Position:** `fixture_seeding.rs` builds its writes from [`fixture_event_plans`] and
//! [`fixture_template`]; `fixture_cleaning.rs` finds the events again by
//! [`FIXTURE_TITLE_PREFIX`]. The load workload maps account k of the population to event
//! k mod 10 and to slot k div 10 of that event.
//!
//! **Signals & state:** none; constants and pure builders.
//!
//! **Invariants:** every event seats [`FIXTURE_SLOTS_PER_EVENT`] = 2 × 8 × 8 = 128 slots, more than
//! the 110 accounts of the 1,100-account load population each event receives, and caps its places
//! at that slot count; slot s of an event is faction s div 64, squad (s mod 64) div 8 + 1 and seat
//! index s mod 8, which is also the order of `(faction, squad, slot_index)` because the faction
//! and squad names sort as their numbers do; titles sort as their numbers do; the first event
//! starts [`FIXTURE_START_OFFSET_DAYS`] days after the seeding minute and each later one an hour
//! after the one before, so every event is still open for registration throughout a load run.

use api_foundation::error_handling::api_error::ApiError;
use api_operations::services::event_authoring::event_creation::{
    EventCreation, EventCreationRequest,
};
use api_operations::services::event_authoring::mission_attachment::AttachmentTemplate;
use chrono::{DateTime, TimeDelta, Utc};
use mission_model::orbat::{OrbatSlotTemplate, OrbatSquadTemplate};

/// The title every fixture event starts with; the cleaner finds the fixture by it.
pub(crate) const FIXTURE_TITLE_PREFIX: &str = "[Load fixture]";
/// The number of fixture events.
pub(crate) const FIXTURE_EVENT_COUNT: u32 = 10;
/// The factions of the fixture ORBAT, in slot order.
pub(crate) const FIXTURE_FACTIONS: [&str; 2] = ["Load A", "Load B"];
/// The squads of each faction.
pub(crate) const FIXTURE_SQUADS_PER_FACTION: u32 = 8;
/// The slots of each squad.
pub(crate) const FIXTURE_SLOTS_PER_SQUAD: u32 = 8;
/// The slots of each fixture event, and its place cap: 2 × 8 × 8.
pub(crate) const FIXTURE_SLOTS_PER_EVENT: u32 =
    FIXTURE_FACTIONS.len() as u32 * FIXTURE_SQUADS_PER_FACTION * FIXTURE_SLOTS_PER_SQUAD;
/// The accounts of the load population the fixture seats; account k registers on event k mod 10
/// and slot k div 10.
pub(crate) const LOAD_POPULATION_ACCOUNTS: u32 = 1_100;
/// The accounts of the population each event receives, rounded up.
pub(crate) const POPULATION_SHARE_PER_EVENT: u32 =
    LOAD_POPULATION_ACCOUNTS.div_ceil(FIXTURE_EVENT_COUNT);
/// How many days after the seeding minute the first fixture event starts.
pub(crate) const FIXTURE_START_OFFSET_DAYS: i64 = 14;
/// The hours between the starts of consecutive fixture events.
const FIXTURE_START_SPACING_HOURS: i64 = 1;
/// The status the fixture events are created in: registration open.
const FIXTURE_STATUS: &str = "open";
/// The briefing of every fixture event.
const FIXTURE_BRIEFING: &str = "Synthetic event of the staging load verification; \
     `staging-fixtures clean-load-fixture-events` removes it.";

// The highest slot the account mapping reaches, slot (1,100 - 1) div 10 = 109, exists.
const _: () = assert!(POPULATION_SHARE_PER_EVENT <= FIXTURE_SLOTS_PER_EVENT);

/// One fixture event to create.
pub(crate) struct FixtureEventPlan {
    /// The number in the title, 1 to [`FIXTURE_EVENT_COUNT`].
    pub(crate) number: u32,
    /// The event, validated by the same rules as `POST /api/v1/events`.
    pub(crate) creation: EventCreation,
}

/// The title of fixture event `number`: two digits, so titles sort as their numbers do.
pub(crate) fn fixture_event_title(number: u32) -> String {
    format!("{FIXTURE_TITLE_PREFIX} {number:02}")
}

/// The fixture events of a seeding at `now`: open, capped at their slot count, the first starting
/// [`FIXTURE_START_OFFSET_DAYS`] days after `now`'s minute and each later one an hour after the
/// one before.
pub(crate) fn fixture_event_plans(now: DateTime<Utc>) -> Result<Vec<FixtureEventPlan>, ApiError> {
    let minute = DateTime::from_timestamp(now.timestamp().div_euclid(60) * 60, 0)
        .ok_or_else(|| ApiError::internal("the database clock is out of range"))?;
    let first_start = minute + TimeDelta::days(FIXTURE_START_OFFSET_DAYS);
    (1..=FIXTURE_EVENT_COUNT)
        .map(|number| {
            let start_time =
                first_start + TimeDelta::hours(FIXTURE_START_SPACING_HOURS * i64::from(number - 1));
            let creation = EventCreation::new(EventCreationRequest {
                start_time: Some(start_time),
                name_override: fixture_event_title(number),
                briefing: FIXTURE_BRIEFING.to_owned(),
                max_slots: i64::from(FIXTURE_SLOTS_PER_EVENT),
                status: FIXTURE_STATUS.to_owned(),
                ..EventCreationRequest::default()
            })?;
            Ok(FixtureEventPlan { number, creation })
        })
        .collect()
}

/// The fixture ORBAT in slot order: each faction's squads `<faction> Squad 1` to
/// `<faction> Squad 8`, each squad's seats `Rifleman 1` to `Rifleman 8` at slot indexes 0 to 7.
/// Squad names carry their faction because `orbat_slots` is unique on
/// `(event_mission_id, squad, slot_index)`.
pub(crate) fn fixture_orbat() -> Vec<OrbatSquadTemplate> {
    FIXTURE_FACTIONS
        .iter()
        .flat_map(|faction| {
            (1..=FIXTURE_SQUADS_PER_FACTION).map(move |squad| OrbatSquadTemplate {
                faction: (*faction).to_owned(),
                callsign: format!("{faction} {squad}"),
                squad: format!("{faction} Squad {squad}"),
                slots: (1..=FIXTURE_SLOTS_PER_SQUAD)
                    .map(|seat| OrbatSlotTemplate {
                        role: format!("Rifleman {seat}"),
                        loadout: String::new(),
                        tag: String::new(),
                    })
                    .collect(),
            })
        })
        .collect()
}

/// The fixture ORBAT as the attachment service accepts it, through the same checks as a
/// requested `orbat`.
pub(crate) fn fixture_template() -> Result<AttachmentTemplate, ApiError> {
    AttachmentTemplate::requested(fixture_orbat())
}
