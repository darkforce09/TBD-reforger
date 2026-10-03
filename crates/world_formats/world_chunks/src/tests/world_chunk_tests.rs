//! Unit tests of the chunk JSON decoder over the committed chunk fixtures.

use crate::chunk_id::ChunkId;
use crate::world_chunk::*;
use prefab_catalog::prefab_rows::PrefabRow;
use prefab_catalog::prefab_rows::build_prefab_maps;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use serde_json::{Value, json};
use std::fs;
use world_file_formats::ids::PrefabId;

fn golden(name: &str) -> Value {
    let path = format!(
        "{}/../../../contracts/fixtures/map/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&fs::read(&path).expect("read golden")).expect("parse golden")
}

#[test]
fn parse_chunk_synthetic_exact() {
    let rows = vec![
        PrefabRow {
            prefab_id: PrefabId::new(9),
            kind: "building".into(),
            class: "residential".into(),
            ..Default::default()
        },
        PrefabRow {
            prefab_id: PrefabId::new(14),
            kind: "tree".into(),
            class: "conifer".into(),
            ..Default::default()
        },
        PrefabRow {
            prefab_id: PrefabId::new(18),
            kind: "misc".into(),
            class: "x".into(),
            ..Default::default()
        },
    ];
    let (by_id, _) = build_prefab_maps(rows);

    let raw = json!({ "instances": [
        [9, 512.0, 700.25, 41.3, 90],
        [14, 800.5, 900.0, 55.0, 47.25],
        [18, 1023.999, 640.0, 60.1, 359.5],
        "garbage-row"
    ]});
    let c = parse_chunk(&ChunkId::from("1_1"), &raw, &by_id).unwrap();

    assert_eq!(c.count, 3);
    assert_eq!(c.cx, 1.0);
    assert_eq!(c.cy, 1.0);
    assert_eq!(c.prefab_idx, vec![9u16, 14, 18]);
    assert_eq!(c.cls_codes, vec![0u8, 1, NO_CLASS]);
    assert_eq!(c.rows_by_class.get(&0), Some(&vec![0u32]));
    assert_eq!(c.rows_by_class.get(&1), Some(&vec![1u32]));
    assert_eq!(c.rows_by_class.get(&NO_CLASS), None);

    assert_eq!(c.positions.len(), 6);
    assert_eq!(c.positions[0], 512.0_f64 as f32);
    assert_eq!(c.positions[1], 700.25_f64 as f32);
    assert_eq!(c.positions[4], 1023.999_f64 as f32);
    assert_eq!(c.z[0], 41.3_f64 as f32);
    assert_eq!(c.rotations[2], 359.5_f64 as f32);
}

#[test]
fn parse_chunk_golden_consistent() {
    let (by_id, _) = build_prefab_maps(
        narrow_prefab_rows(&json!({
            "prefabs": golden("map-object-prefabs-sample.json")
        }))
        .expect("whole u32 ids"),
    );
    let chunk_raw = golden("map-object-chunk-sample.json");
    let c = parse_chunk(
        &ChunkId::from("1_1"),
        chunk_raw.get("chunk").unwrap(),
        &by_id,
    )
    .unwrap();

    assert_eq!(c.count, 4);
    assert_eq!(c.prefab_idx, vec![9u16, 14, 14, 18]);
    assert_eq!(c.positions.len(), 8);
    assert_eq!(c.pitch, vec![0.0_f32, 0.0, -3.5, 0.0]);
    assert_eq!(c.roll, vec![0.0_f32, 0.0, 1.25, 0.0]);
    assert_eq!(c.scale, vec![1.0_f32, 1.0, 1.15, 1.0]);

    for (&code, rows) in &c.rows_by_class {
        assert_ne!(code, NO_CLASS);
        for &i in rows {
            assert_eq!(c.cls_codes[i as usize], code);
        }
    }
}

#[test]
fn parse_chunk_v2_columns_default_to_identity_on_v1_rows() {
    let (by_id, _) = build_prefab_maps(vec![PrefabRow {
        prefab_id: PrefabId::new(14),
        kind: "tree".into(),
        class: "conifer".into(),
        ..Default::default()
    }]);
    let raw = json!({ "instances": [
        [14, 800.5, 900.0, 55.0, 47.25],
        [14, 900.0, 950.0, 52.0, 47.25, -3.5, 1.25, 1.15],
        [14, 901.0, 951.0, 52.0, 47.25, 0, 0, 0.5],
    ]});
    let c = parse_chunk(&ChunkId::from("1_1"), &raw, &by_id).unwrap();
    assert_eq!(c.count, 3);
    assert_eq!(
        c.positions,
        vec![800.5_f32, 900.0, 900.0, 950.0, 901.0, 951.0]
    );
    assert_eq!(c.rotations, vec![47.25_f32; 3]);
    assert_eq!(c.pitch, vec![0.0_f32, -3.5, 0.0]);
    assert_eq!(c.roll, vec![0.0_f32, 1.25, 0.0]);
    assert_eq!(c.scale, vec![1.0_f32, 1.15, 0.5]);
    assert_eq!(c.rows_by_class.get(&1), Some(&vec![0u32, 1, 2]));
}

#[test]
fn a_fractional_or_negative_pid_joins_no_prefab() {
    let tree = |prefab_id: u32| PrefabRow {
        prefab_id: PrefabId::new(prefab_id),
        kind: "tree".into(),
        class: "conifer".into(),
        ..Default::default()
    };
    let (by_id, _) = build_prefab_maps(vec![tree(0), tree(14)]);
    let tree_code = by_id[&PrefabId::new(14)].code;
    assert_ne!(tree_code, NO_CLASS, "the joined rows carry a render class");
    let raw: Value = serde_json::from_str(
        r#"{ "instances": [
            [14, 1.0, 1.0, 0, 0],
            [14.5, 2.0, 2.0, 0, 0],
            [-14, 3.0, 3.0, 0, 0],
            [0, 4.0, 4.0, 0, 0],
            [-0.0, 5.0, 5.0, 0, 0]
        ] }"#,
    )
    .expect("chunk JSON");
    let c = parse_chunk(&ChunkId::from("0_0"), &raw, &by_id).unwrap();
    assert_eq!(c.count, 5, "every row is kept, joined or not");
    assert_eq!(
        c.cls_codes,
        vec![tree_code, NO_CLASS, NO_CLASS, tree_code, NO_CLASS],
        "only the exact f64 of a catalogue id joins its prefab"
    );
    assert_eq!(c.rows_by_class.get(&tree_code), Some(&vec![0_u32, 3]));
}
