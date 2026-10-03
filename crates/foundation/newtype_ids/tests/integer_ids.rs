//! `integer_id!` expanded in a crate of its own: transparent serde, ordering, `Display`,
//! `FromStr`, the conversions, `Copy`, and the declaration with and without attributes passed
//! through.

use std::collections::HashMap;

newtype_ids::integer_id! {
    /// The row number of an event, declared with documentation and an extra derive.
    #[derive(Default)]
    pub struct EventNumber(i64);
}

newtype_ids::integer_id!(struct SlotIndex(u16));

#[test]
fn serialises_exactly_as_its_integer() {
    let event = EventNumber::new(-7);
    let id_json = serde_json::to_string(&event).unwrap();
    assert_eq!(id_json, serde_json::to_string(&-7_i64).unwrap());
    assert_eq!(id_json, "-7");
    let back: EventNumber = serde_json::from_str(&id_json).unwrap();
    assert_eq!(back, event);
    assert!(serde_json::from_str::<EventNumber>("\"7\"").is_err());
}

#[test]
fn the_inner_type_bounds_deserialisation() {
    assert!(serde_json::from_str::<SlotIndex>("65535").is_ok());
    assert!(serde_json::from_str::<SlotIndex>("65536").is_err());
    assert!(serde_json::from_str::<SlotIndex>("-1").is_err());
}

#[test]
fn orders_as_its_integer() {
    let mut events = [
        EventNumber::new(3),
        EventNumber::new(-1),
        EventNumber::new(2),
    ];
    events.sort();
    let values: Vec<i64> = events.iter().map(EventNumber::get).collect();
    assert_eq!(values, [-1, 2, 3]);
    assert!(EventNumber::default() < EventNumber::new(1));
}

#[test]
fn displays_and_parses_as_its_integer() {
    let event: EventNumber = "42".parse().unwrap();
    assert_eq!(event, EventNumber::new(42));
    assert_eq!(event.to_string(), "42");
    assert_eq!(
        format!("{event:>5}|"),
        "   42|",
        "width reaches the integer's `Display`"
    );
    assert_eq!(format!("{event:<5}|"), format!("{:<5}|", 42_i64));
    assert_eq!(
        "4x".parse::<EventNumber>().unwrap_err(),
        "4x".parse::<i64>().unwrap_err()
    );
    assert!("70000".parse::<SlotIndex>().is_err());
}

#[test]
fn converts_both_ways_and_copies() {
    let event = EventNumber::from(9);
    let copy = event;
    assert_eq!(event, copy);
    assert_eq!(i64::from(event), 9);
    assert_eq!(event.get(), 9);
    assert_eq!(event.into_inner(), 9);
    const FIRST: EventNumber = EventNumber::new(1);
    assert_eq!(FIRST.get(), 1);
}

#[test]
fn declares_without_attributes() {
    let slot = SlotIndex::new(4);
    let mut names = HashMap::new();
    names.insert(slot, "Rifleman");
    assert_eq!(names.get(&SlotIndex::new(4)), Some(&"Rifleman"));
    assert_eq!(format!("{slot:?}"), "SlotIndex(4)");
    assert_eq!(serde_json::to_string(&slot).unwrap(), "4");
}
