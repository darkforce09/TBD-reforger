//! Role: network tests.
//! Position: `world/terrain/roads/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::roads::network::*;
use serde_json::json;

const QUAD_SOUP: [[f64; 2]; 8] = [
    [-2.0, 0.0],
    [2.0, 0.0],
    [2.0, 10.0],
    [-2.0, 10.0],
    [-2.0, 10.0],
    [2.0, 10.0],
    [2.0, 20.0],
    [-2.0, 20.0],
];

#[test]
fn centerline_midpoints_dedupes_measures() {
    let (path, width) = extract_road_centerline(&QUAD_SOUP).unwrap();
    assert_eq!(path, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
    assert_eq!(width, 4.0);
}

#[test]
fn centerline_drops_odd_trailing_point() {
    let mut pts = QUAD_SOUP.to_vec();
    pts.push([999.0, 999.0]);
    let (path, _) = extract_road_centerline(&pts).unwrap();
    assert_eq!(path, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
}

#[test]
fn centerline_null_when_under_two_midpoints() {
    assert!(extract_road_centerline(&[[-2.0, 0.0], [2.0, 0.0]]).is_none());
    assert!(extract_road_centerline(&[[-2.0, 0.0], [2.0, 0.0], [2.0, 0.0], [-2.0, 0.0]]).is_none());
    assert!(extract_road_centerline(&[]).is_none());
}

#[test]
fn width_is_median_across_cross_edges() {
    let (_p, width) = extract_road_centerline(&[
        [-2.0, 0.0],
        [2.0, 0.0],
        [2.0, 10.0],
        [-2.0, 10.0],
        [-6.0, 20.0],
        [6.0, 20.0],
    ])
    .unwrap();
    assert_eq!(width, 4.0);
}

#[test]
fn parse_payload_narrows_good_segments() {
    let raw = json!({
        "roadSegments": [
            { "id": "r0", "roadClass": "runway", "points": QUAD_SOUP },
            { "id": "r1", "roadClass": "road_dirt", "points": [[0, -1], [0, 1], [10, 1], [10, -1]] }
        ]
    });
    let segs = parse_roads_payload(&raw);
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0].points, vec![[0.0, 0.0], [0.0, 10.0], [0.0, 20.0]]);
    assert_eq!(segs[0].width_m, 4.0);
    assert_eq!(segs[1].points, vec![[0.0, 0.0], [10.0, 0.0]]);
    assert_eq!(segs[1].width_m, 2.0);
}

#[test]
fn parse_payload_drops_malformed() {
    let raw = json!({
        "roadSegments": [
            { "id": "x", "roadClass": "hyperloop", "points": QUAD_SOUP },
            { "id": "y", "roadClass": "track", "points": [[0, 0]] },
            { "id": "z", "roadClass": "track", "points": [[0, 0], [null, 1]] },
            { "roadClass": "track", "points": QUAD_SOUP },
            { "id": "w", "roadClass": "track", "points": [[-2, 0], [2, 0]] }
        ]
    });
    assert_eq!(parse_roads_payload(&raw).len(), 0);
    assert_eq!(parse_roads_payload(&Value::Null).len(), 0);
    assert_eq!(parse_roads_payload(&json!("<html>")).len(), 0);
}

use crate::io::archives::codec::to_bytes;
use crate::io::archives::roads::RoadSegmentArchive;
use crate::world::environment::locations::route_placement::road_class_code;

fn network(segments: Vec<RoadSegmentArchive>) -> RoadNetworkArchive {
    RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments,
    }
}

fn one_segment() -> RoadNetworkArchive {
    network(vec![RoadSegmentArchive {
        id: "road-everon-0007".to_string(),
        road_class: road_class_code("road_dirt"),
        width_m: 2.5,
        centerline: vec![[1.5, -2.25], [10.0, -2.25], [10.0, 40.5]],
    }])
}

#[test]
fn road_network_round_trips_through_the_validating_reader() {
    let bytes = to_bytes(&one_segment()).expect("serialise");
    let segs = road_network_from_bytes(&bytes).expect("read back");
    assert_eq!(segs.len(), 1);
    assert_eq!(segs[0].id, "road-everon-0007");
    assert_eq!(segs[0].road_class, "road_dirt");
    assert_eq!(segs[0].width_m, 2.5);
    assert_eq!(
        segs[0].points,
        vec![[1.5, -2.25], [10.0, -2.25], [10.0, 40.5]]
    );
}

#[test]
fn archive_widens_f32_exactly_and_does_not_pretend_to_be_f64() {
    let bytes = to_bytes(&network(vec![RoadSegmentArchive {
        id: "r".to_string(),
        road_class: road_class_code("track"),
        width_m: 0.1,
        centerline: vec![[0.1, 0.2], [0.3, 0.4]],
    }]))
    .expect("serialise");
    let segs = road_network_from_bytes(&bytes).expect("read back");
    assert_eq!(segs[0].width_m, f64::from(0.1_f32));
    assert_ne!(segs[0].width_m, 0.1_f64, "0.1 is not f32-exact");
    assert_eq!(segs[0].points[0], [f64::from(0.1_f32), f64::from(0.2_f32)]);
}

#[test]
fn unnameable_class_code_is_an_error_not_a_vanished_road() {
    for code in [0u8, 7, 255] {
        let bytes = to_bytes(&network(vec![RoadSegmentArchive {
            id: "r0".to_string(),
            road_class: code,
            width_m: 4.0,
            centerline: vec![[0.0, 0.0], [1.0, 1.0]],
        }]))
        .expect("serialise");
        let err = road_network_from_bytes(&bytes).expect_err("must refuse");
        let msg = err.to_string();
        assert!(msg.contains("cannot name"), "{code}: {msg}");
        assert!(msg.contains("road_class code"), "{code}: {msg}");
    }

    for class in crate::world::environment::locations::route_placement::ROAD_CLASSES {
        let bytes = to_bytes(&network(vec![RoadSegmentArchive {
            id: "r0".to_string(),
            road_class: road_class_code(class),
            width_m: 4.0,
            centerline: vec![[0.0, 0.0], [1.0, 1.0]],
        }]))
        .expect("serialise");
        let segs = road_network_from_bytes(&bytes).unwrap_or_else(|e| panic!("{class}: {e}"));
        assert_eq!(segs[0].road_class, class);
    }
}

#[test]
fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
    let mut archive = one_segment();
    archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
    let bytes = to_bytes(&archive).expect("serialise");
    assert!(
        access_checked::<RoadNetworkArchive>(&bytes).is_ok(),
        "the buffer must be structurally valid, or this test proves nothing"
    );
    assert!(matches!(
        road_network_from_bytes(&bytes),
        Err(BinaryError::UnsupportedVersion {
            what: "RoadNetworkArchive",
            expected: ARCHIVE_SCHEMA_VERSION,
            actual,
        }) if actual == ARCHIVE_SCHEMA_VERSION + 1
    ));
}

#[test]
fn corrupt_buffers_are_refused() {
    let bytes = to_bytes(&one_segment()).expect("serialise");
    assert!(road_network_from_bytes(&[]).is_err(), "empty");
    assert!(
        road_network_from_bytes(&bytes[..bytes.len() - 1]).is_err(),
        "truncated tail"
    );
    assert!(
        road_network_from_bytes(&bytes[1..]).is_err(),
        "truncated head"
    );
    assert!(
        road_network_from_bytes(br#"{"roadSegments":[]}"#).is_err(),
        "JSON must not be read as an archive"
    );
}
