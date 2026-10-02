//! Role: Module boundary for environment/locations/routes/tests.
//! Position: `world/environment/locations/routes/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::roads::network::RoadSegment;

use super::*;

use serde_json::json;

fn seg(id: &str, cls: &str, points: Vec<[f64; 2]>) -> RoadSegment {
    RoadSegment {
        id: id.into(),
        road_class: cls.into(),
        points,
        width_m: 4.0,
    }
}

use crate::io::archives::labels::MapLabelsArchive;

use crate::io::archives::codec::access_checked;

use crate::io::archives::codec::to_bytes;

fn class_codes_round_trip_fixture() -> (RoadNamesFile, Vec<RoadSegment>) {
    let segments = vec![
        seg("hw", "highway_paved", vec![[0.0, 0.0], [4000.0, 0.0]]),
        seg("sec", "road_paved", vec![[0.0, 900.0], [4000.0, 900.0]]),
        seg(
            "pinned",
            "road_paved",
            vec![[0.0, 1800.0], [4000.0, 1800.0]],
        ),
    ];
    let names = RoadNamesFile {
        schema_version: "1.0.0".into(),
        terrain_id: "fixture".into(),
        roads: vec![
            RoadNameEntry {
                id: "a".into(),
                name: "Main Highway".into(),
                segment_ids: vec!["hw".into()],
                min_deck_zoom: None,
            },
            RoadNameEntry {
                id: "b".into(),
                name: "Secondary Road".into(),
                segment_ids: vec!["sec".into()],
                min_deck_zoom: None,
            },
            RoadNameEntry {
                id: "c".into(),
                name: "Pinned Road".into(),
                segment_ids: vec!["pinned".into()],
                min_deck_zoom: Some(0.0),
            },
        ],
    };
    (names, segments)
}

mod cases_1;
