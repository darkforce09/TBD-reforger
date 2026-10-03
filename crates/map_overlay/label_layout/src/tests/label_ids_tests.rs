//! Cases of the label ids: each serialises exactly as its inner value, and the committed Everon
//! location rows keep their `id` bytes through a typed round trip.

use crate::importance::LocationLabel;
use crate::label_ids::{LabelId, LocationId};

const EVERON_LOCATIONS: &str = include_str!("../../../../../assets/terrains/everon/locations.json");

#[test]
fn a_label_id_serialises_as_its_integer() {
    assert_eq!(serde_json::to_string(&LabelId::new(7)).unwrap(), "7");
    let back: LabelId = serde_json::from_str("7").unwrap();
    assert_eq!(back, LabelId::new(7));
}

#[test]
fn a_location_id_serialises_as_its_string() {
    let id = LocationId::new("everon-airport");
    assert_eq!(
        serde_json::to_string(&id).unwrap(),
        serde_json::to_string("everon-airport").unwrap()
    );
}

#[test]
fn the_committed_location_rows_keep_their_id_bytes() {
    let raw: Vec<serde_json::Value> = serde_json::from_str(EVERON_LOCATIONS).unwrap();
    let typed: Vec<LocationLabel> = serde_json::from_str(EVERON_LOCATIONS).unwrap();
    assert_eq!(raw.len(), typed.len());
    assert!(!typed.is_empty(), "the committed location data has rows");
    for (raw_row, row) in raw.iter().zip(&typed) {
        let raw_id = raw_row["id"].as_str().expect("every row has a string id");
        assert_eq!(
            serde_json::to_string(&row.id).unwrap(),
            serde_json::to_string(raw_id).unwrap()
        );
    }
    let again: Vec<LocationLabel> =
        serde_json::from_str(&serde_json::to_string(&typed).unwrap()).unwrap();
    assert_eq!(again, typed);
}
