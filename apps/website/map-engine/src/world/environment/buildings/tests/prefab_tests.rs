//! Role: prefab tests.
//! Position: `world/environment/buildings/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::buildings::prefab::*;
use serde_json::json;

#[test]
fn narrow_keeps_wellformed_rows() {
    let raw = json!({
        "prefabs": [
            { "prefabId": 0, "kind": "tree", "class": "conifer",
              "spatial": { "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 }, "heightM": 12 },
              "render": { "iconKey": "tree-conifer", "baseSizePx": 18, "defaultColor": "#2d5a27" } },
            { "prefabId": "bad", "kind": "tree" },
            { "prefabId": 5 },
            { "prefabId": 9, "kind": "building" }
        ]
    });
    let rows = narrow_prefab_rows(&raw);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].prefab_id, 0.0);
    assert_eq!(rows[0].half_x, Some(1.2));
    assert_eq!(rows[0].height_m, Some(12.0));
    assert_eq!(rows[0].icon_key.as_deref(), Some("tree-conifer"));
    assert_eq!(rows[1].class, "unknown");
}

#[test]
fn build_maps_codes_and_oversized() {
    let rows = vec![
        PrefabRow {
            prefab_id: 0.0,
            kind: "tree".into(),
            class: "conifer".into(),
            ..Default::default()
        },
        PrefabRow {
            prefab_id: 9.0,
            kind: "building".into(),
            class: "residential".into(),
            half_x: Some(70.0),
            half_y: Some(3.0),
            ..Default::default()
        },
        PrefabRow {
            prefab_id: 3.0,
            kind: "misc".into(),
            class: "x".into(),
            ..Default::default()
        },
    ];
    let (by_id, oversized) = build_prefab_maps(rows);
    assert!(oversized);
    assert_eq!(by_id.get(&0.0_f64.to_bits()).unwrap().code, 1);
    assert_eq!(by_id.get(&9.0_f64.to_bits()).unwrap().code, 0);
    assert_eq!(by_id.get(&3.0_f64.to_bits()).unwrap().code, NO_CLASS);
}

#[test]
fn oversized_only_when_classified() {
    let rows = vec![PrefabRow {
        prefab_id: 1.0,
        kind: "misc".into(),
        class: "x".into(),
        half_x: Some(70.0),
        ..Default::default()
    }];
    let (_by_id, oversized) = build_prefab_maps(rows);
    assert!(!oversized);
}

use crate::io::archives::codec::to_bytes;
use crate::streaming::loaders::store::bytes_to_json;
use std::path::PathBuf;

const EVERON_PREFABS: usize = 1623;
const EVERON_INSTANCES: u64 = 1_216_066;

fn everon() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/map-assets/everon")
}

fn everon_json_rows() -> Vec<PrefabRow> {
    let raw = std::fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs.json.gz");
    narrow_prefab_rows(&bytes_to_json(&raw).expect("prefabs decode"))
}

fn everon_inventory() -> TypeInventory {
    let raw = std::fs::read_to_string(everon().join("objects/type-inventory.json"))
        .expect("type-inventory.json");
    inventory_to_archive(&serde_json::from_str(&raw).expect("inventory parse"))
        .expect("inventory → archive")
}

fn everon_archive() -> PrefabCatalogArchive {
    let rows = everon_json_rows();
    PrefabCatalogArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        prefabs: rows
            .iter()
            .enumerate()
            .map(|(i, r)| row_to_archive(i, r).expect("row → archive"))
            .collect(),
        type_inventory: everon_inventory(),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn f32_narrow(v: Option<f64>) -> Option<f64> {
    v.map(|n| f64::from(n as f32))
}

#[test]
fn everon_catalogue_archive_equals_the_json_rows() {
    let json = everon_json_rows();
    assert_eq!(
        json.len(),
        EVERON_PREFABS,
        "everon prefab corpus changed; re-pin EVERON_PREFABS deliberately"
    );

    let bytes = to_bytes(&everon_archive()).expect("serialise");
    assert!(
        bytes_to_json(&bytes).is_err(),
        "the archive must not be parseable as JSON, or this test has not proven the rkyv \
             route ran"
    );
    assert_ne!(&bytes[..2], &[0x1f, 0x8b], "must not carry gzip magic");

    let archive = access_checked::<PrefabCatalogArchive>(&bytes).expect("validated access");
    let back = rows_from_archive(archive).expect("rows");
    assert_eq!(back.len(), json.len(), "row count");

    let (mut absent_icon, mut present_icon, mut coloured, mut importance) = (0, 0, 0, 0);
    for (i, (a, j)) in back.iter().zip(json.iter()).enumerate() {
        assert_eq!(a.prefab_id.to_bits(), j.prefab_id.to_bits(), "row {i}: id");
        assert_eq!(a.kind, j.kind, "row {i}: kind");
        assert_eq!(a.class, j.class, "row {i}: class");
        assert_eq!(a.label, j.label, "row {i}: label");
        assert_eq!(a.resource_name, j.resource_name, "row {i}: resourceName");
        assert_eq!(a.icon_key, j.icon_key, "row {i}: iconKey");
        assert_eq!(a.default_color, j.default_color, "row {i}: defaultColor");
        for (name, got, want) in [
            ("halfX", a.half_x, f32_narrow(j.half_x)),
            ("halfY", a.half_y, f32_narrow(j.half_y)),
            ("halfZ", a.half_z, f32_narrow(j.half_z)),
            ("heightM", a.height_m, f32_narrow(j.height_m)),
            ("baseSizePx", a.base_size_px, f32_narrow(j.base_size_px)),
            (
                "importanceZoom",
                a.importance_zoom,
                f32_narrow(j.importance_zoom),
            ),
        ] {
            assert_eq!(
                got.map(f64::to_bits),
                want.map(f64::to_bits),
                "row {i} ({}): {name}",
                j.kind
            );
        }
        if a.icon_key.is_some() {
            present_icon += 1;
        } else {
            absent_icon += 1;
        }
        coloured += usize::from(a.default_color.is_some());
        importance += usize::from(a.importance_zoom.is_some());
    }
    assert!(
        absent_icon > 0 && present_icon > 0,
        "the corpus exercised only one side of the absent-iconKey encoding \
             ({absent_icon} absent / {present_icon} present)"
    );
    assert!(
        coloured > 0 && coloured < back.len(),
        "defaultColor: {coloured} of {} rows — one side of the encoding untested",
        back.len()
    );
    assert!(
        importance > 0 && importance < back.len(),
        "importanceZoom: {importance} of {} rows — one side of the encoding untested",
        back.len()
    );

    let cat = catalog_from_bytes(&bytes, "everon").expect("catalogue");
    let (want_map, want_oversized) = build_prefab_maps(
        json.iter()
            .map(|r| PrefabRow {
                half_x: f32_narrow(r.half_x),
                half_y: f32_narrow(r.half_y),
                half_z: f32_narrow(r.half_z),
                height_m: f32_narrow(r.height_m),
                base_size_px: f32_narrow(r.base_size_px),
                importance_zoom: f32_narrow(r.importance_zoom),
                ..r.clone()
            })
            .collect(),
    );
    assert_eq!(cat.by_id, want_map, "prefab lookup");
    assert_eq!(cat.has_oversized, want_oversized, "has_oversized");
    assert_eq!(cat.terrain_id, "everon");
    assert_eq!(cat.unique_prefabs as usize, EVERON_PREFABS);
    assert_eq!(cat.total_instances, EVERON_INSTANCES);
}

#[test]
fn a_catalogue_for_another_terrain_is_refused() {
    let bytes = to_bytes(&everon_archive()).expect("serialise");
    assert!(catalog_from_bytes(&bytes, "everon").is_ok(), "control");
    let err = catalog_from_bytes(&bytes, "arland").expect_err("must refuse");
    let msg = err.to_string();
    assert!(
        msg.contains("\"everon\"") && msg.contains("\"arland\""),
        "{msg}"
    );
}

#[test]
fn a_census_that_does_not_match_the_row_count_is_refused() {
    let mut archive = everon_archive();
    archive.type_inventory.unique_prefabs -= 1;
    let bytes = to_bytes(&archive).expect("serialise");
    assert!(
        access_checked::<PrefabCatalogArchive>(&bytes).is_ok(),
        "the bytes must still validate, or this proves nothing about the meaning check"
    );
    let msg = catalog_from_bytes(&bytes, "everon")
        .expect_err("must refuse")
        .to_string();
    assert!(msg.contains("stale or half-written"), "{msg}");
}

#[test]
fn wrong_schema_version_is_refused_even_though_the_bytes_validate() {
    let mut archive = everon_archive();
    archive.schema_version = ARCHIVE_SCHEMA_VERSION + 1;
    let bytes = to_bytes(&archive).expect("serialise");
    assert!(access_checked::<PrefabCatalogArchive>(&bytes).is_ok());
    assert!(matches!(
        catalog_from_bytes(&bytes, "everon"),
        Err(BinaryError::UnsupportedVersion { .. })
    ));
}

#[test]
fn a_drifted_class_code_is_refused() {
    let mut archive = everon_archive();
    let row = archive
        .prefabs
        .iter_mut()
        .find(|p| p.kind == "tree")
        .expect("a tree prefab");
    row.class_code = row.class_code.wrapping_add(1);
    let bytes = to_bytes(&archive).expect("serialise");
    assert!(access_checked::<PrefabCatalogArchive>(&bytes).is_ok());
    let msg = catalog_from_bytes(&bytes, "everon")
        .expect_err("must refuse")
        .to_string();
    assert!(msg.contains("RENDER_CLASS_CODES"), "{msg}");
}

#[test]
fn absent_and_present_optionals_round_trip() {
    let rows = vec![
        PrefabRow {
            prefab_id: 7.0,
            kind: "building".into(),
            class: "civic".into(),
            label: Some("Hut".into()),
            resource_name: Some("{ABC}Hut.et".into()),
            half_x: Some(1.5),
            half_y: Some(2.25),
            half_z: Some(0.0),
            height_m: Some(4.0),
            icon_key: Some("hut".into()),
            base_size_px: Some(18.0),
            default_color: Some("#0a1bff".into()),
            importance_zoom: Some(-4.0),
        },
        PrefabRow {
            prefab_id: 8.0,
            kind: "rock".into(),
            class: "boulder".into(),
            ..Default::default()
        },
    ];
    let archive = PrefabCatalogArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        prefabs: rows
            .iter()
            .enumerate()
            .map(|(i, r)| row_to_archive(i, r).expect("encode"))
            .collect(),
        type_inventory: TypeInventory {
            terrain_id: "probe".into(),
            census_status: "partial".into(),
            unique_prefabs: 2,
            total_instances: 0,
            by_kind: Vec::new(),
        },
    };
    let bytes = to_bytes(&archive).expect("serialise");
    let back = rows_from_archive(access_checked::<PrefabCatalogArchive>(&bytes).expect("access"))
        .expect("rows");
    assert_eq!(back, rows, "every optional must survive as itself");
    assert_eq!(back[0].half_z, Some(0.0), "a real 0.0 is not \"absent\"");
    assert!(back[1].label.is_none() && back[1].height_m.is_none());
}

#[test]
fn rows_that_cannot_be_encoded_without_lying_are_refused() {
    let base = PrefabRow {
        prefab_id: 1.0,
        kind: "prop".into(),
        class: "fence".into(),
        ..Default::default()
    };
    let cases: [(&str, PrefabRow); 5] = [
        (
            "not a u32-representable whole number",
            PrefabRow {
                prefab_id: 1.5,
                ..base.clone()
            },
        ),
        (
            "not a u32-representable whole number",
            PrefabRow {
                prefab_id: -1.0,
                ..base.clone()
            },
        ),
        (
            "empty label",
            PrefabRow {
                label: Some(String::new()),
                ..base.clone()
            },
        ),
        (
            "not a lowercase #rrggbb",
            PrefabRow {
                default_color: Some("#2D5A27".into()),
                ..base.clone()
            },
        ),
        (
            "non-finite heightM",
            PrefabRow {
                height_m: Some(f64::NAN),
                ..base.clone()
            },
        ),
    ];
    for (needle, row) in cases {
        let msg = row_to_archive(0, &row)
            .expect_err("must refuse")
            .to_string();
        assert!(msg.contains(needle), "expected {needle:?} in: {msg}");
    }
    row_to_archive(0, &base).expect("the control row must encode");
}

#[test]
fn malformed_buffers_error_rather_than_yielding_rows() {
    let bytes = to_bytes(&everon_archive()).expect("serialise");
    for (what, buf) in [
        ("empty", &[][..]),
        ("truncated tail", &bytes[..bytes.len() - 1]),
        ("shifted head", &bytes[1..]),
        ("json", br#"{"prefabs":[]}"#),
    ] {
        assert!(
            catalog_from_bytes(buf, "everon").is_err(),
            "{what} was accepted"
        );
    }
}

#[test]
fn the_standalone_census_equals_the_one_inside_the_catalogue() {
    let inventory = everon_inventory();
    let bytes = to_bytes(&inventory).expect("serialise");
    assert_eq!(inventory_from_bytes(&bytes).expect("read back"), inventory);
    assert_eq!(inventory.terrain_id, "everon");
    assert_eq!(inventory.unique_prefabs as usize, EVERON_PREFABS);
    assert_eq!(inventory.total_instances, EVERON_INSTANCES);

    assert_eq!(
        inventory
            .by_kind
            .iter()
            .map(|k| k.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "building",
            "tree",
            "vegetation",
            "rock",
            "prop",
            "utility",
            "water",
            "vehicle",
            "road",
        ],
        "`INSTANCE_KINDS` order with `road` last is the census contract"
    );
    assert_eq!(
        inventory.by_kind.iter().map(|k| k.instances).sum::<u64>(),
        EVERON_INSTANCES,
        "the per-kind census must add up to the declared total"
    );
    assert!(inventory_from_bytes(&[]).is_err(), "empty buffer");
}

#[test]
fn an_incomplete_census_is_refused() {
    for doc in [
        json!({ "censusStatus": "partial", "levels": { "uniquePrefabs": 1, "totalInstances": 2 }, "byKind": {} }),
        json!({ "terrainId": "everon", "censusStatus": "partial", "byKind": {} }),
        json!({ "terrainId": "everon", "censusStatus": "partial", "levels": { "uniquePrefabs": 1 }, "byKind": {} }),
        json!({ "terrainId": "everon", "censusStatus": "partial", "levels": { "uniquePrefabs": 1, "totalInstances": 2 } }),
    ] {
        assert!(inventory_to_archive(&doc).is_err(), "accepted: {doc}");
    }
    assert!(
        inventory_to_archive(&json!({
            "terrainId": "everon", "censusStatus": "partial",
            "levels": { "uniquePrefabs": 1, "totalInstances": 2 },
            "byKind": { "tree": { "prefabTypes": 1, "instances": 2 } }
        }))
        .is_ok(),
        "the control document must be accepted"
    );
}
