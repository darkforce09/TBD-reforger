//! Unit tests of the prefab tables: the JSON and archive lanes agree, and the sniff routes each form.

use crate::InvalidPrefabId;
use crate::prefab_rows::row_to_archive;
use crate::prefab_tables::*;
use crate::test_fixtures::everon_archive_bytes;
use crate::test_fixtures::everon_prefabs_json;
use crate::test_fixtures::everon_prefabs_json_f32;
use crate::test_fixtures::gzip;
use world_file_formats::archives::codec::to_bytes;
use world_file_formats::archives::prefabs::PrefabCatalogArchive;
use world_file_formats::archives::prefabs::TypeInventory;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;

#[test]
fn the_f32_projection_is_not_a_no_op_on_everon() {
    let raw = tables_from_json(&everon_prefabs_json()).expect("json lane");
    let narrowed = tables_from_json(&everon_prefabs_json_f32()).expect("json lane");
    let differing = raw
        .building_by_u16
        .iter()
        .filter(|(k, v)| narrowed.building_by_u16.get(k) != Some(v))
        .count();
    assert!(
        differing > 0,
        "no building footprint changed under f32 — the everon export is now f32-exact and the \
             parity pin's projection needs re-stating rather than silently narrowing nothing"
    );
    assert_ne!(raw.by_id, narrowed.by_id, "row table under f32");
}

#[test]
fn tables_agree_on_a_real_zero_half_extent() {
    let doc = serde_json::json!({ "prefabs": [
        { "prefabId": 4, "kind": "building", "class": "civic", "label": "Zero",
          "spatial": { "halfExtentsM": { "x": 0, "y": 0, "z": 0 }, "heightM": 0 },
          "render": { "iconKey": "building-civic", "baseSizePx": 18, "importanceZoom": -4 } },
        { "prefabId": 5, "kind": "prop", "class": "fence",
          "spatial": { "halfExtentsM": { "x": 0, "y": 0 } } },
        { "prefabId": 6, "kind": "building", "class": "hut" },
        { "prefabId": 7, "kind": "water", "class": "pier",
          "spatial": { "halfExtentsM": { "x": 10, "y": 1.5 } } },
        { "prefabId": 70000, "kind": "building", "class": "outside-u16",
          "spatial": { "halfExtentsM": { "x": 3, "y": 3 } } }
    ]});
    let rows = narrow_prefab_rows(&doc).expect("whole u32 ids");
    assert_eq!(
        rows[0].half_x,
        Some(0.0),
        "the fixture must carry a real 0.0"
    );
    assert_eq!(rows[2].half_x, None, "…and a genuinely absent one");

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
            unique_prefabs: 5,
            total_instances: 0,
            by_kind: Vec::new(),
        },
    };
    let bytes = to_bytes(&archive).expect("serialise");

    let from_json = tables_from_json(&doc).expect("json lane");
    let from_rkyv = tables_from_bytes(&bytes, "probe").expect("archive lane");
    assert_eq!(from_rkyv, from_json, "zero is a value, not an absence");

    assert_eq!(from_rkyv.by_id[&PrefabId::new(4)].row.half_x, Some(0.0));
    assert_eq!(from_rkyv.by_id[&PrefabId::new(6)].row.half_x, None);

    let zero = &from_rkyv.building_by_u16[&4];
    assert_eq!((zero.half_x, zero.half_y), (2.0, 2.0));
    assert_eq!(from_rkyv.building_by_u16[&6].half_x, 2.0);
    assert_eq!(
        from_rkyv.fence_by_u16[&5],
        FencePrefabInfo {
            half_x: 1.0,
            half_y: 0.25
        }
    );
    assert_eq!(from_rkyv.building_by_u16[&7].building_class, "pier");
    assert_eq!(
        from_rkyv.building_by_u16.len(),
        3,
        "70000 is outside the u16 lookup domain"
    );
    assert_eq!(from_rkyv.min_importance_zoom, Some(-4.0));
    assert_eq!(from_rkyv.importance_breakpoints, vec![-4.0]);
}

#[test]
fn tables_from_bytes_sniffs_gzip_versus_rkyv() {
    let json_f32 = gzip(
        serde_json::to_string(&everon_prefabs_json_f32())
            .unwrap()
            .as_bytes(),
    );
    assert_eq!(&json_f32[..2], &[0x1f, 0x8b], "the JSON side must be gzip");
    let rkyv = everon_archive_bytes();
    assert_eq!(
        tables_from_bytes(&rkyv, "everon").expect("rkyv"),
        tables_from_bytes(&json_f32, "everon").expect("gz"),
        "both routes, one table"
    );
}

#[test]
fn an_empty_payload_is_refused_before_the_sniff() {
    assert!(matches!(
        tables_from_bytes(&[], "everon"),
        Err(WorldError::EmptyPayload)
    ));
}

#[test]
fn truncated_payloads_error_rather_than_reaching_the_wrong_parser() {
    let gz = gzip(br#"{"prefabs":[{"prefabId":9,"kind":"building","class":"hut"}]}"#);
    let rk = everon_archive_bytes();
    assert!(
        matches!(
            tables_from_bytes(&gz[..gz.len() / 2], "everon"),
            Err(WorldError::Gzip(_))
        ),
        "tail-truncated gzip keeps its magic and must fail as gzip"
    );
    assert!(
        matches!(
            tables_from_bytes(&gz[4..], "everon"),
            Err(WorldError::Archive(_))
        ),
        "head-truncated gzip has no magic and must be refused by the archive reader"
    );
    assert!(matches!(
        tables_from_bytes(&rk[..rk.len() - 8], "everon"),
        Err(WorldError::Archive(_))
    ));

    assert!(matches!(
        tables_from_bytes(&[0x1f], "everon"),
        Err(WorldError::Archive(_))
    ));

    assert!(matches!(
        tables_from_bytes(br#"{"prefabs":[]}"#, "everon"),
        Err(WorldError::Archive(_))
    ));
}

#[test]
fn a_catalogue_for_another_terrain_is_refused_by_the_sniff() {
    let rkyv = everon_archive_bytes();
    assert!(tables_from_bytes(&rkyv, "everon").is_ok(), "control");
    let msg = tables_from_bytes(&rkyv, "arland")
        .expect_err("must refuse")
        .to_string();
    assert!(msg.contains("everon") && msg.contains("arland"), "{msg}");
}

#[test]
fn a_catalogue_with_a_duplicate_prefab_id_is_refused() {
    let doc = serde_json::json!({ "prefabs": [
        { "prefabId": 3, "kind": "building", "class": "hut",
          "spatial": { "halfExtentsM": { "x": 4, "y": 4 } } },
        { "prefabId": 3, "kind": "tree", "class": "conifer",
          "spatial": { "halfExtentsM": { "x": 2, "y": 2 } } }
    ]});
    let rows = narrow_prefab_rows(&doc).expect("whole u32 ids");

    let json = tables_from_json(&doc).expect("json lane");
    assert_eq!(json.by_id[&PrefabId::new(3)].row.kind, "tree");
    assert_eq!(json.building_by_u16[&3].building_class, "hut");

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
    let msg = tables_from_bytes(&bytes, "probe")
        .expect_err("must refuse")
        .to_string();
    assert!(msg.contains("distinct prefabIds"), "{msg}");
}

/// A gzip JSON catalogue carrying a fractional `prefabId` is refused by both the JSON lane and the
/// byte sniffer with the typed error, never narrowed into a table.
#[test]
fn a_json_catalogue_with_a_fractional_prefab_id_is_refused() {
    let doc = serde_json::json!({ "prefabs": [
        { "prefabId": 3.5, "kind": "building", "class": "hut" }
    ]});
    assert_eq!(
        tables_from_json(&doc).expect_err("must refuse"),
        InvalidPrefabId { row: 0, value: 3.5 }
    );
    let bytes = gzip(doc.to_string().as_bytes());
    assert!(matches!(
        tables_from_bytes(&bytes, "probe"),
        Err(WorldError::InvalidPrefabId(InvalidPrefabId { row: 0, .. }))
    ));
}
