//! Role: prefab tests.
//! Position: `streaming/loaders/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::buildings::prefab::inventory_to_archive;
use crate::environment::buildings::prefab::row_to_archive;
use crate::formats::archives::codec::to_bytes;
use crate::formats::archives::prefabs::PrefabCatalogArchive;
use crate::formats::archives::prefabs::TypeInventory;
use crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::streaming::loaders::prefab::*;
use crate::streaming::scheduler::state::WorldResidency;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;
use std::path::PathBuf;

const EVERON_PREFABS: usize = 1623;

const FIXTURE_CHUNK: &str = "2_12";

fn map_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/map-assets")
}

fn everon() -> PathBuf {
    map_assets().join("everon")
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bytes).unwrap();
    enc.finish().unwrap()
}

fn everon_prefabs_gz() -> Vec<u8> {
    std::fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs.json.gz")
}

fn everon_prefabs_json() -> Value {
    bytes_to_json(&everon_prefabs_gz()).expect("prefabs decode")
}

#[allow(clippy::cast_possible_truncation)]
fn f32_narrow(v: f64) -> f64 {
    f64::from(v as f32)
}

fn everon_prefabs_json_f32() -> Value {
    let mut raw = everon_prefabs_json();
    let narrow_at = |v: &mut Value, path: &[&str]| {
        let mut cur = v;
        for key in path {
            match cur.get_mut(*key) {
                Some(next) => cur = next,
                None => return,
            }
        }
        if let Some(n) = cur.as_f64() {
            *cur = Value::from(f32_narrow(n));
        }
    };
    let rows = raw
        .get_mut("prefabs")
        .and_then(Value::as_array_mut)
        .expect("prefabs array");
    for row in rows.iter_mut() {
        for path in [
            &["spatial", "halfExtentsM", "x"][..],
            &["spatial", "halfExtentsM", "y"][..],
            &["spatial", "halfExtentsM", "z"][..],
            &["spatial", "heightM"][..],
            &["render", "baseSizePx"][..],
            &["render", "importanceZoom"][..],
        ] {
            narrow_at(row, path);
        }
    }
    raw
}

fn everon_archive_bytes() -> Vec<u8> {
    let rows = narrow_prefab_rows(&everon_prefabs_json());
    let inventory_doc = std::fs::read_to_string(everon().join("objects/type-inventory.json"))
        .expect("type-inventory.json");
    let archive = PrefabCatalogArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        prefabs: rows
            .iter()
            .enumerate()
            .map(|(i, r)| row_to_archive(i, r).expect("row → archive"))
            .collect(),
        type_inventory: inventory_to_archive(
            &serde_json::from_str(&inventory_doc).expect("inventory parse"),
        )
        .expect("inventory → archive"),
    };
    to_bytes(&archive).expect("serialise").to_vec()
}

fn glyph_keys() -> Vec<String> {
    let raw = std::fs::read_to_string(map_assets().join("glyphs/manifest.json"))
        .expect("glyphs manifest");
    let v: Value = serde_json::from_str(&raw).unwrap();
    let mut keys: Vec<String> = v["glyphs"]
        .as_object()
        .expect("glyphs object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

fn everon_residency(bytes: &[u8]) -> WorldResidency {
    let mut r = WorldResidency::new();
    r.load_manifest_json(
        &std::fs::read_to_string(everon().join("manifest.json")).expect("everon manifest"),
    )
    .expect("manifest");
    r.load_chunk_index_json(
        &std::fs::read_to_string(everon().join("objects/chunks/manifest.json"))
            .expect("chunk index"),
    )
    .expect("chunk index");
    r.set_glyph_key_map(&glyph_keys());
    assert_eq!(
        r.load_prefabs(bytes, "everon").expect("prefab load"),
        EVERON_PREFABS,
        "everon prefab corpus changed; re-pin EVERON_PREFABS deliberately"
    );
    r.set_glyph_toggles(true, true, true);

    let mut parts = FIXTURE_CHUNK.split('_');
    let cx: f64 = parts.next().unwrap().parse().unwrap();
    let cy: f64 = parts.next().unwrap().parse().unwrap();
    let (min_x, min_y) = (cx * 512.0, cy * 512.0);
    let missing = r.set_viewport(min_x, min_y, min_x + 512.0, min_y + 512.0, 3.5);
    assert!(!missing.is_empty(), "the viewport must request chunks");
    for id in &missing {
        let path = everon()
            .join("objects/chunks")
            .join(format!("{id}.json.gz"));
        r.ingest_chunk_gz(
            id,
            &std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}")),
        )
        .expect("ingest");
    }
    r.end_apply_frame(0.0);
    r
}

#[test]
fn everon_archive_lane_builds_the_same_residency_as_the_json_lane() {
    let json_f32 = gzip(
        serde_json::to_string(&everon_prefabs_json_f32())
            .unwrap()
            .as_bytes(),
    );
    let rkyv = everon_archive_bytes();
    assert!(
        bytes_to_json(&rkyv).is_err(),
        "the archive bytes must not parse as JSON, or the rkyv route has not been proven to run"
    );
    assert_ne!(
        &rkyv[..2],
        &[0x1f, 0x8b],
        "the archive must not carry gzip magic"
    );

    let from_json = tables_from_bytes(&json_f32, "everon").expect("json lane");
    let from_rkyv = tables_from_bytes(&rkyv, "everon").expect("archive lane");

    assert_eq!(from_rkyv.by_id.len(), EVERON_PREFABS, "distinct prefab ids");
    assert!(from_rkyv.building_by_u16.len() > 100, "buildings");
    assert!(!from_rkyv.fence_by_u16.is_empty(), "fences");
    assert!(!from_rkyv.importance_breakpoints.is_empty(), "breakpoints");

    assert!(
        !from_rkyv.has_oversized,
        "everon carries no oversized prefab"
    );

    assert_eq!(from_rkyv.by_id, from_json.by_id, "prefab row table");
    assert_eq!(
        from_rkyv.building_by_u16, from_json.building_by_u16,
        "building lookup"
    );
    assert_eq!(
        from_rkyv.fence_by_u16, from_json.fence_by_u16,
        "fence lookup"
    );
    assert_eq!(
        from_rkyv, from_json,
        "every table, including the derived two"
    );

    let ordered_json = narrow_prefab_rows(&everon_prefabs_json_f32());
    assert_eq!(ordered_json.len(), EVERON_PREFABS, "rows in the export");
    assert_eq!(
        from_json.by_id.len(),
        ordered_json.len(),
        "a duplicate prefabId would make file order decide the table — everon has none"
    );

    let a = everon_residency(&rkyv);
    let j = everon_residency(&json_f32);

    assert!(!a.world_building_fill().is_empty(), "fill must compose");
    assert!(
        !a.world_building_outline().is_empty(),
        "outline must compose"
    );
    assert!(!a.world_fence_strips().is_empty(), "strips must compose");
    assert!(a.badge_glyph_count() > 0, "badges must compose");
    assert!(a.tree_glyph_count() > 0, "tree glyphs must compose");
    assert!(a.prop_glyph_count() > 0, "prop glyphs must compose");
    assert_eq!(a.draw_ids(), j.draw_ids(), "draw set");
    assert_eq!(a.world_building_fill(), j.world_building_fill(), "fill");
    assert_eq!(
        a.world_building_outline(),
        j.world_building_outline(),
        "outline"
    );
    assert_eq!(a.world_fence_strips(), j.world_fence_strips(), "strips");
    assert_eq!(
        (
            a.fence_strip_segment_count(),
            a.pier_strip_segment_count(),
            a.bridge_rail_strip_count()
        ),
        (
            j.fence_strip_segment_count(),
            j.pier_strip_segment_count(),
            j.bridge_rail_strip_count()
        ),
        "strip lane counts"
    );
    assert_eq!(a.world_tree_glyphs(), j.world_tree_glyphs(), "tree glyphs");
    assert_eq!(a.world_prop_glyphs(), j.world_prop_glyphs(), "prop glyphs");
    assert_eq!(
        a.world_badge_glyphs(),
        j.world_badge_glyphs(),
        "badge glyphs"
    );
    for group in 0..=2u8 {
        assert_eq!(
            a.glyph_lookup_len_for_group(group),
            j.glyph_lookup_len_for_group(group),
            "glyph table, group {group}"
        );
    }
    assert_eq!(a.stats_json(), j.stats_json(), "residency stats");
}

#[test]
fn the_f32_projection_is_not_a_no_op_on_everon() {
    let raw = tables_from_json(&everon_prefabs_json());
    let narrowed = tables_from_json(&everon_prefabs_json_f32());
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
    let rows = narrow_prefab_rows(&doc);
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

    let from_json = tables_from_json(&doc);
    let from_rkyv = tables_from_bytes(&bytes, "probe").expect("archive lane");
    assert_eq!(from_rkyv, from_json, "zero is a value, not an absence");

    assert_eq!(from_rkyv.by_id[&4.0_f64.to_bits()].row.half_x, Some(0.0));
    assert_eq!(from_rkyv.by_id[&6.0_f64.to_bits()].row.half_x, None);

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
fn the_manifest_can_name_the_archive_the_host_branch_fetches() {
    let named = |b: &crate::streaming::loaders::manifest::ObjectsBinaryBlock| {
        (!b.prefabs.is_empty()).then(|| b.prefabs.clone())
    };
    let declared: Value = serde_json::from_str(
        r#"{ "objects": { "prefabsPath": "objects/prefabs.json.gz",
                              "chunksPath": "objects/chunks",
                              "binary": { "prefabs": "objects/prefabs.rkyv" } } }"#,
    )
    .unwrap();
    let block = crate::streaming::loaders::manifest::parse_manifest_binary(&declared)
        .objects
        .expect("objects.binary");
    assert_eq!(named(&block).as_deref(), Some("objects/prefabs.rkyv"));

    let everon_manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(everon().join("manifest.json")).expect("everon manifest"),
    )
    .unwrap();
    assert_eq!(
        named(
            &crate::streaming::loaders::manifest::parse_manifest_binary(&everon_manifest)
                .objects
                .expect("T-935.13 writes objects.binary")
        )
        .as_deref(),
        Some("objects/prefabs.rkyv")
    );
}

#[test]
fn a_catalogue_with_a_duplicate_prefab_id_is_refused() {
    let doc = serde_json::json!({ "prefabs": [
        { "prefabId": 3, "kind": "building", "class": "hut",
          "spatial": { "halfExtentsM": { "x": 4, "y": 4 } } },
        { "prefabId": 3, "kind": "tree", "class": "conifer",
          "spatial": { "halfExtentsM": { "x": 2, "y": 2 } } }
    ]});
    let rows = narrow_prefab_rows(&doc);

    let json = tables_from_json(&doc);
    assert_eq!(json.by_id[&3.0_f64.to_bits()].row.kind, "tree");
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
