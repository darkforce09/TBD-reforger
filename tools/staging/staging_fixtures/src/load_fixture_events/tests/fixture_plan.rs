//! Unit tests for the load fixture's shape: its capacity against the load population's account
//! mapping, the slot order that mapping relies on, and the events a seeding plans.

use std::collections::BTreeSet;

use api_operations::models::EventStatus;
use chrono::{TimeDelta, TimeZone, Utc};

use super::{
    FIXTURE_EVENT_COUNT, FIXTURE_FACTIONS, FIXTURE_SLOTS_PER_EVENT, FIXTURE_START_OFFSET_DAYS,
    LOAD_POPULATION_ACCOUNTS, POPULATION_SHARE_PER_EVENT, fixture_event_plans, fixture_event_title,
    fixture_orbat, fixture_template,
};

/// Account k of the population registers on event k mod 10 and slot k div 10: every target slot
/// exists, and no two accounts share one.
#[test]
fn staging_fixtures_fixture_seats_every_account_of_the_population_once() {
    let template = fixture_template().expect("the fixture ORBAT passes the attachment checks");
    assert_eq!(template.slot_count(), 128);
    assert_eq!(FIXTURE_SLOTS_PER_EVENT, 128);
    assert_eq!(POPULATION_SHARE_PER_EVENT, 110);
    let mut targets = BTreeSet::new();
    for account in 0..LOAD_POPULATION_ACCOUNTS {
        let event = account % FIXTURE_EVENT_COUNT;
        let slot = account / FIXTURE_EVENT_COUNT;
        assert!(
            slot < FIXTURE_SLOTS_PER_EVENT,
            "account {account} lands on slot {slot}"
        );
        assert!(
            targets.insert((event, slot)),
            "account {account} shares a slot"
        );
    }
    assert_eq!(targets.len(), LOAD_POPULATION_ACCOUNTS as usize);
}

/// The ORBAT lists slot s as faction s div 64, squad (s mod 64) div 8 + 1 and seat index s mod 8,
/// and that is the order `(faction, squad, slot_index)` sorts the stored rows into; no two slots
/// share a `(squad, slot_index)` pair, the unique key of `orbat_slots` within an attachment.
#[test]
fn staging_fixtures_fixture_orbat_lists_slots_in_their_sorted_order() {
    let rows: Vec<(String, String, usize)> = fixture_orbat()
        .iter()
        .flat_map(|squad| {
            (0..squad.slots.len())
                .map(move |index| (squad.faction.clone(), squad.squad.clone(), index))
        })
        .collect();
    assert_eq!(rows.len(), 128);
    let mut sorted = rows.clone();
    sorted.sort();
    assert_eq!(rows, sorted, "the listed order is the sorted order");
    let unique_keys: BTreeSet<(&String, usize)> = rows
        .iter()
        .map(|(_, squad, index)| (squad, *index))
        .collect();
    assert_eq!(
        unique_keys.len(),
        rows.len(),
        "a (squad, slot_index) pair repeats"
    );
    for (slot, (faction, squad, index)) in rows.iter().enumerate() {
        assert_eq!(faction, FIXTURE_FACTIONS[slot / 64]);
        assert_eq!(*squad, format!("{faction} Squad {}", slot % 64 / 8 + 1));
        assert_eq!(*index, slot % 8);
    }
}

#[test]
fn staging_fixtures_fixture_events_are_open_future_and_sorted_by_title() {
    let now = Utc.with_ymd_and_hms(2026, 9, 29, 10, 17, 42).unwrap();
    let plans = fixture_event_plans(now).expect("the fixture events pass the event rules");
    assert_eq!(plans.len(), 10);
    let first_start = Utc.with_ymd_and_hms(2026, 9, 29, 10, 17, 0).unwrap()
        + TimeDelta::days(FIXTURE_START_OFFSET_DAYS);
    let titles: Vec<String> = plans
        .iter()
        .map(|plan| plan.creation.name_override().to_owned())
        .collect();
    let mut sorted_titles = titles.clone();
    sorted_titles.sort();
    assert_eq!(titles, sorted_titles);
    for (offset, plan) in plans.iter().enumerate() {
        let number = u32::try_from(offset).unwrap() + 1;
        assert_eq!(plan.number, number);
        assert_eq!(plan.creation.name_override(), fixture_event_title(number));
        assert_eq!(
            plan.creation.start_time(),
            first_start + TimeDelta::hours(i64::try_from(offset).unwrap())
        );
        assert_eq!(plan.creation.status(), EventStatus::Open);
        assert_eq!(plan.creation.max_slots(), 128);
    }
    assert_eq!(fixture_event_title(1), "[Load fixture] 01");
    assert_eq!(fixture_event_title(10), "[Load fixture] 10");
}
