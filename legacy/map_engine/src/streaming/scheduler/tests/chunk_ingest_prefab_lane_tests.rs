//! The chunk-ingest prefab lanes: the residency the rkyv prefab archive builds equals the one its
//! gzip JSON twin builds.
//!
//! **Role:** unit tests of `WorldResidency::load_prefabs` and the chunk ingest over the committed
//! Everon export.
//! **Position:** mounted from `crate::streaming::scheduler::chunk_ingest`; reads the prefab
//! fixtures of `prefab_catalog::test_fixtures` and `assets/glyphs/`.
//! **Signals & state:** none; every case builds its own residencies.
//! **Invariants:** both lanes compose the same buildings, strips, glyphs and statistics.

use crate::streaming::scheduler::state::WorldResidency;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use prefab_catalog::prefab_tables::tables_from_bytes;
use prefab_catalog::test_fixtures::EVERON_PREFABS;
use prefab_catalog::test_fixtures::everon;
use prefab_catalog::test_fixtures::everon_archive_bytes;
use prefab_catalog::test_fixtures::everon_prefabs_json_f32;
use prefab_catalog::test_fixtures::gzip;
use prefab_catalog::world_payload::bytes_to_json;
use serde_json::Value;
use std::path::PathBuf;

const FIXTURE_CHUNK: &str = "2_12";

/// Glyphs are shared by every terrain, so they sit beside the terrain tree rather than inside one.
fn glyph_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/glyphs")
}

fn glyph_keys() -> Vec<String> {
    let raw =
        std::fs::read_to_string(glyph_assets().join("manifest.json")).expect("glyphs manifest");
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

    let ordered_json =
        narrow_prefab_rows(&everon_prefabs_json_f32()).expect("Everon ids are whole u32s");
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
