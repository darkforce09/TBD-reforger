//! Role: towns tests.
//! Position: `world/environment/locations/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::to_bytes;
use crate::world::environment::locations::towns::*;

fn archive_of(towns: Vec<TownLabel>, heights: Vec<HeightLabelWire>) -> rkyv::util::AlignedVec {
    to_bytes(&MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        towns,
        height_labels: heights,
        road_names: Vec::new(),
    })
    .expect("serialise")
}

#[test]
fn parse_sample_row() {
    let json = r#"[
          {"id":"everon-morton","name":"Morton","x":5135.24,"y":4011.78,"importance":0.7,"kind":"village"}
        ]"#;
    let rows = parse_locations_json(json).expect("parse");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Morton");
    let specs = locations_to_label_specs(&rows);
    assert_eq!(specs[0].text, "Morton");
}

#[test]
fn parse_height_labels_reads_the_exporter_shape() {
    let json = r#"[
          {"x":5014.1,"y":8474.51,"value_m":113,"kind":"peak","name":"Center North Hill 01"},
          {"x":1.5,"y":2.5,"value_m":90,"kind":"peak"}
        ]"#;
    let rows = parse_height_labels_json(json).expect("parse");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].value_m, 113);
    assert_eq!(rows[0].name.as_deref(), Some("Center North Hill 01"));
    assert_eq!(rows[0].kind, HeightLabelKind::Peak);
    assert!(rows[1].name.is_none());
}

#[test]
fn parse_height_labels_rejects_a_row_without_coordinates() {
    let err = parse_height_labels_json(r#"[{"x":1.0,"value_m":90}]"#).expect_err("must reject");
    assert!(err.contains("row 0 missing x/y/value_m"), "{err}");
    let err = parse_height_labels_json("{}").expect_err("must reject");
    assert!(err.contains("not an array"), "{err}");
}

#[test]
fn towns_round_trip_through_the_archive() {
    let json = r#"[
          {"id":"everon-entre-deux","name":"Entre Deux","x":5135.24,"y":4011.78,"importance":0.7,"kind":"village"},
          {"id":"everon-saint-philippe","name":"Saint-Philippe é","x":1280.0,"y":6400.5,"importance":0.78}
        ]"#;
    let rows = parse_locations_json(json).expect("parse");
    assert!(
        rows[1].kind.is_none(),
        "fixture must exercise the None kind"
    );
    let bytes = archive_of(towns_to_archive(&rows), Vec::new());
    let back = towns_from_archive(access_checked::<MapLabelsArchive>(&bytes).expect("access"));
    assert_eq!(back.len(), rows.len());
    for (a, b) in back.iter().zip(&rows) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.x, f64::from(b.x as f32), "{}", b.name);
        assert_eq!(a.y, f64::from(b.y as f32), "{}", b.name);
        assert_eq!(a.importance, f64::from(b.importance as f32));
        assert!(a.id.is_empty(), "the wire type carries no id");
    }
    assert_eq!(
        locations_to_label_specs(&back),
        locations_to_label_specs(&rows),
        "the packed label specs must be identical from either source"
    );
}

#[test]
fn height_labels_round_trip_through_the_archive() {
    let json = r#"[
          {"x":5014.1,"y":8474.51,"value_m":113,"kind":"peak","name":"Center North Hill 01"},
          {"x":3854.08,"y":4344.24,"value_m":163,"kind":"peak"}
        ]"#;
    let rows = parse_height_labels_json(json).expect("parse");
    let bytes = archive_of(Vec::new(), height_labels_to_archive(&rows));
    let back =
        height_labels_from_archive(access_checked::<MapLabelsArchive>(&bytes).expect("access"));
    assert_eq!(back.len(), rows.len());
    for (a, b) in back.iter().zip(&rows) {
        assert_eq!(a.value_m, b.value_m);
        assert_eq!(a.x, f64::from(b.x as f32));
        assert_eq!(a.y, f64::from(b.y as f32));
        assert_eq!(a.kind, HeightLabelKind::Peak);
        assert!(a.name.is_none(), "the wire type carries no name");
    }
}

#[test]
fn empty_lanes_survive_the_round_trip() {
    let bytes = archive_of(Vec::new(), Vec::new());
    let a = access_checked::<MapLabelsArchive>(&bytes).expect("access");
    assert!(towns_from_archive(a).is_empty());
    assert!(height_labels_from_archive(a).is_empty());
}

use crate::io::archives::labels::RoadNameLabel;

fn three_lane_file() -> Vec<u8> {
    to_bytes(&MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        towns: vec![TownLabel {
            name: "Montignac".into(),
            position: [6400.0, 6400.5],
            importance: 0.875,
            kind: "town".into(),
        }],
        height_labels: vec![HeightLabelWire {
            position: [1024.0, 2048.0],
            elevation_m: 375.0,
        }],
        road_names: vec![RoadNameLabel {
            name: "Main Highway".into(),
            position: [10.5, -20.25],
            angle_deg: 12.5,
            road_class: 1,
        }],
    })
    .expect("serialise")
    .to_vec()
}

#[test]
fn a_file_buffer_reads_all_three_lanes() {
    let file = three_lane_file();
    let labels = map_labels_from_bytes(&file).expect("read");
    assert_eq!(labels.towns.len(), 1);
    assert_eq!(labels.towns[0].name, "Montignac");
    assert_eq!(labels.towns[0].y, 6400.5);
    assert_eq!(labels.height_labels.len(), 1);
    assert_eq!(labels.height_labels[0].value_m, 375);
    assert_eq!(labels.road_names.len(), 1);
    assert_eq!(labels.road_names[0].name, "Main Highway");
    assert_eq!(labels.road_names[0].road_class, "highway_paved");
}

#[test]
fn the_read_survives_every_offset_the_allocator_can_hand_it() {
    let file = three_lane_file();
    for skew in 0..MAP_LABELS_ALIGN {
        let mut shifted = vec![0u8; skew];
        shifted.extend_from_slice(&file);
        let labels = map_labels_from_bytes(&shifted[skew..]).expect("read");
        assert_eq!(labels.towns.len(), 1, "skew {skew}");
        assert_eq!(labels.road_names.len(), 1, "skew {skew}");
    }
}

#[test]
fn a_truncated_file_is_an_error_not_a_wild_read() {
    let file = three_lane_file();
    for cut in [0, 1, file.len() / 2, file.len() - 1] {
        let err = map_labels_from_bytes(&file[..cut]).expect_err("must reject");
        assert!(
            matches!(err, BinaryError::Archive { .. }),
            "cut {cut}: {err}"
        );
    }
}

#[test]
fn a_future_schema_version_is_refused() {
    let file = to_bytes(&MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION + 1,
        towns: Vec::new(),
        height_labels: Vec::new(),
        road_names: Vec::new(),
    })
    .expect("serialise")
    .to_vec();
    let err = map_labels_from_bytes(&file).expect_err("must refuse");
    assert!(
        matches!(
            err,
            BinaryError::UnsupportedVersion {
                expected: ARCHIVE_SCHEMA_VERSION,
                ..
            }
        ),
        "{err}"
    );
}
