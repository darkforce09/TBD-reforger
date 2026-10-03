//! Unit tests of the road name pipeline: placement, declutter, the polyline geometry, the
//! `road-names.json` parser, the class codes and the labels archive's road lane.
//!
//! **Role:** pins the road name rules on synthetic segments.
//! **Position:** mounted by [`crate::route_placement`]; reads [`crate::route_geometry`],
//! [`crate::route_labels`], `road_network`'s segments and class codec and `world_file_formats`'
//! labels archive.
//! **Signals & state:** none.
//! **Invariants:** the cases keep their assertions and fixtures.

use road_network::network::RoadSegment;
use road_network::road_class::{ROAD_CLASSES, road_class_code, road_class_name};
use serde_json::json;
use world_file_formats::archives::codec::access_checked;
use world_file_formats::archives::codec::to_bytes;
use world_file_formats::archives::labels::MapLabelsArchive;

use super::*;
use crate::route_geometry::{placement_fractions, road_declutter_min_dist_m, upright_angle_deg};
use crate::route_labels::{
    RoadNameEntry, RoadNamesFile, build_road_label_draw_set_from_archive, parse_road_names_json,
    road_names_from_archive, road_names_to_archive,
};

fn seg(id: &str, cls: &str, points: Vec<[f64; 2]>) -> RoadSegment {
    RoadSegment {
        id: id.into(),
        road_class: cls.into(),
        points,
        width_m: 4.0,
    }
}

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

#[test]
fn declutter_dist_scales_with_zoom() {
    let d0 = road_declutter_min_dist_m(0.0);
    let d1 = road_declutter_min_dist_m(1.0);
    assert!((d0 - 60.0).abs() < 1e-6);
    assert!((d1 - 30.0).abs() < 1e-6);
}

#[test]
fn upright_flips_past_90() {
    let down = upright_angle_deg([0.0, -1.0]);
    assert!(down.abs() <= 90.0);
    let left = upright_angle_deg([-1.0, 0.0]);
    assert!(left.abs() <= 90.0);
}

#[test]
fn long_segment_gets_three_fractions() {
    let fr = placement_fractions(4000.0);
    assert_eq!(fr, vec![0.25, 0.5, 0.75]);
    assert_eq!(placement_fractions(1000.0), vec![0.5]);
}

#[test]
fn placement_within_perp_tol() {
    let points = vec![[0.0, 0.0], [1000.0, 0.0]];
    let s = seg("r1", "highway_paved", points.clone());
    let names = RoadNamesFile {
        schema_version: "1".into(),
        terrain_id: "everon".into(),
        roads: vec![RoadNameEntry {
            id: "t".into(),
            name: "Test Highway".into(),
            segment_ids: vec!["r1".into()],
            min_deck_zoom: None,
        }],
    };
    let drawn = build_road_label_draw_set(&names, std::slice::from_ref(&s), 0.0);
    assert_eq!(drawn.len(), 1);
    assert!(road_placement_geometry_holds(&drawn, &[s]));
}

#[test]
fn close_labels_drop_lower_priority() {
    let a = RoadLabelPlacement {
        name: "A".into(),
        x: 0.0,
        y: 0.0,
        angle_deg: 0.0,
        priority: 400,
        segment_id: "a".into(),
        road_class: "highway_paved".into(),
        arc_frac: 0.5,
    };
    let b = RoadLabelPlacement {
        name: "B".into(),
        x: 10.0,
        y: 0.0,
        angle_deg: 0.0,
        priority: 100,
        segment_id: "b".into(),
        road_class: "track".into(),
        arc_frac: 0.5,
    };
    let out = declutter_road_labels(&[b, a], 0.0);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "A");
    assert!(road_declutter_invariant_holds(&out, 0.0));
}

#[test]
fn cap_at_24() {
    let mut cands = Vec::new();
    for i in 0..40 {
        cands.push(RoadLabelPlacement {
            name: format!("Road {i}"),
            x: f64::from(i) * 100.0,
            y: 0.0,
            angle_deg: 0.0,
            priority: 300,
            segment_id: format!("s{i}").into(),
            road_class: "road_paved".into(),
            arc_frac: 0.5,
        });
    }
    let out = declutter_road_labels(&cands, 0.0);
    assert!(out.len() <= ROAD_NAME_MAX_ON_SCREEN);
}

#[test]
fn parse_road_names_sample() {
    let json = json!({
        "schemaVersion": "1.0.0",
        "terrainId": "everon",
        "roads": [{ "id": "x", "name": "Main Highway", "segmentIds": ["road-everon-0010"] }]
    });
    let file = parse_road_names_json(&json.to_string()).expect("parse");
    assert_eq!(file.roads[0].name, "Main Highway");
}

#[test]
fn class_codes_round_trip_and_reject_the_unknown() {
    for c in ROAD_CLASSES {
        assert_eq!(road_class_name(road_class_code(c)), c);
    }
    assert_eq!(road_class_code("not_a_class"), 0);
    assert_eq!(road_class_name(0), "");
    assert_eq!(road_class_name(200), "");
    assert!(!road_name_visible_for_class(road_class_name(0), 99.0));
}

#[test]
fn archive_road_labels_match_the_json_draw_set_at_every_zoom() {
    let (names, segments) = class_codes_round_trip_fixture();
    let lane = road_names_to_archive(&names, &segments).expect("bake");
    let bytes = to_bytes(&MapLabelsArchive {
        schema_version: 1,
        towns: Vec::new(),
        height_labels: Vec::new(),
        road_names: lane,
    })
    .expect("serialise");
    let candidates =
        road_names_from_archive(access_checked::<MapLabelsArchive>(&bytes).expect("access"));

    let mut saw_pinned_below_one = false;
    for z in [-2.0, -0.5, 0.0, 0.5, 1.0, 2.0, 3.0] {
        let json = build_road_label_draw_set(&names, &segments, z);
        let arch = build_road_label_draw_set_from_archive(&candidates, z);
        assert_eq!(
            arch.len(),
            json.len(),
            "z={z}: {:?} vs {:?}",
            arch.iter().map(|l| &l.name).collect::<Vec<_>>(),
            json.iter().map(|l| &l.name).collect::<Vec<_>>()
        );
        for (a, j) in arch.iter().zip(&json) {
            assert_eq!(a.name, j.name, "z={z}");
            assert_eq!(a.x, f64::from(j.x as f32), "z={z} {}", j.name);
            assert_eq!(a.y, f64::from(j.y as f32), "z={z} {}", j.name);
            assert_eq!(
                a.angle_deg,
                f64::from(j.angle_deg as f32),
                "z={z} {}",
                j.name
            );
        }
        if (0.0..1.0).contains(&z) && json.iter().any(|l| l.name == "Pinned Road") {
            saw_pinned_below_one = true;
        }
    }
    assert!(
        saw_pinned_below_one,
        "fixture must exercise the minDeckZoom override below the class gate"
    );
}

#[test]
fn an_unrepresentable_override_is_refused_rather_than_rounded() {
    let (mut names, segments) = class_codes_round_trip_fixture();
    names.roads[2].min_deck_zoom = Some(2.5);
    let err = road_names_to_archive(&names, &segments)
        .expect_err("must refuse")
        .to_string();
    assert!(err.contains("Pinned Road"), "{err}");
    assert!(err.contains("visibility floor 2.5"), "{err}");
}

#[test]
fn declutter_in_order_is_the_greedy_half_of_declutter() {
    let (names, segments) = class_codes_round_trip_fixture();
    let mut cands = place_road_labels(&names, &segments, 3.0);
    assert!(cands.len() >= 3);
    cands.sort_by(road_declutter_order);
    assert_eq!(
        declutter_road_labels_in_order(&cands, 1.0),
        declutter_road_labels(&cands, 1.0)
    );
}
