use website_map_engine::streaming::loaders::manifest::parse_manifest_binary;

use super::*;
use crate::browser_testing::server::repo_root;

/// The committed everon export: 1623 prefabs, 36 land-cover regions, 1,216,066 instances.
/// (`map-engine-core`'s `store.rs` census pin says the same three numbers.) Re-pin
/// deliberately if the export changes — a silently shrinking corpus is how a parity test
/// stops proving anything.
const EVERON_PREFABS: usize = 1623;
const EVERON_REGIONS: usize = 36;
const EVERON_INSTANCES: u64 = 1_216_066;

fn everon_dir() -> PathBuf {
    repo_root().join("packages/map-assets/everon")
}

fn tmp_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("t935-11-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("objects")).expect("tempdir");
    d
}

/// A terrain directory holding copies of everon's three catalogue JSONs and nothing else.
fn staged_everon(tag: &str) -> PathBuf {
    let d = tmp_dir(tag);
    for rel in [PREFABS_GZ, TYPE_INVENTORY_JSON, FOREST_REGIONS_GZ] {
        std::fs::copy(everon_dir().join(rel), d.join(rel))
            .unwrap_or_else(|e| panic!("copy {rel}: {e}"));
    }
    d
}

/// THE SLICE'S EMIT PIN. Run the emitter over the committed everon catalogue JSONs and prove
/// all three archives land, are not JSON and not gzip, and read back through the shipped
/// loader entry points as the very structures the JSON path yields.
#[test]
fn everon_catalog_archives_emit_and_read_back_as_their_json() {
    let dir = staged_everon("emit");
    assert!(!dir.join(PREFAB_CATALOG_RKYV).exists(), "precondition");

    let written = emit_catalog_archives(&dir).expect("emit");
    assert_eq!(written.len(), 3, "three archives: {written:?}");
    for (p, n) in &written {
        assert_eq!(
            std::fs::metadata(p).expect("stat").len() as usize,
            *n,
            "{} size",
            p.display()
        );
        assert!(*n > 0, "{} is empty", p.display());
        let bytes = std::fs::read(p).expect("read");
        assert!(
            bytes_to_json(&bytes).is_err(),
            "{} parses as JSON — the rkyv route did not run",
            p.display()
        );
        assert_ne!(&bytes[..2], &[0x1f, 0x8b], "{} is gzip", p.display());
    }
    assert_eq!(written[0].0, dir.join(PREFAB_CATALOG_RKYV));
    assert_eq!(written[1].0, dir.join(TYPE_INVENTORY_RKYV));
    assert_eq!(written[2].0, dir.join(FOREST_REGIONS_RKYV));

    // The catalogue: same rows, in the same ORDER, and the file is exactly the value the
    // builder produced.
    //
    // The order half is not decoration. `by_id` below is a hash map, so it is blind to a
    // reordered catalogue — measured: a `rows_from_archive` that rotates the row vector by one
    // leaves every assertion on `by_id` passing. The reader side of that is pinned in
    // `map-engine-core`'s `everon_catalogue_archive_equals_the_json_rows`; what is pinned HERE
    // is the writer side, JSON order → archive order → file bytes.
    let json_rows = narrow_prefab_rows(&read_doc(&dir, PREFABS_GZ).expect("prefabs json"));
    assert_eq!(json_rows.len(), EVERON_PREFABS);
    let rebuilt = build_prefab_catalog_archive(&dir).expect("rebuild");
    assert_eq!(rebuilt.prefabs.len(), json_rows.len());
    for (i, (a, j)) in rebuilt.prefabs.iter().zip(json_rows.iter()).enumerate() {
        assert_eq!(
            f64::from(a.prefab_id),
            j.prefab_id,
            "row {i}: the archive is not in prefabs.json.gz order"
        );
    }
    assert_eq!(
        std::fs::read(dir.join(PREFAB_CATALOG_RKYV)).expect("read"),
        to_bytes(&rebuilt).expect("serialise").as_slice(),
        "the file on disk is not the catalogue the builder produced"
    );

    let cat = catalog_from_bytes(
        &std::fs::read(dir.join(PREFAB_CATALOG_RKYV)).expect("read"),
        "everon",
    )
    .expect("catalogue");
    assert_eq!(cat.by_id.len(), EVERON_PREFABS);
    assert_eq!(cat.terrain_id, "everon");
    assert_eq!(cat.total_instances, EVERON_INSTANCES);
    for row in &json_rows {
        let entry = cat
            .by_id
            .get(&row.prefab_id.to_bits())
            .unwrap_or_else(|| panic!("prefab {} missing from the archive", row.prefab_id));
        assert_eq!(entry.row.kind, row.kind, "prefab {}", row.prefab_id);
        assert_eq!(entry.row.class, row.class, "prefab {}", row.prefab_id);
        assert_eq!(entry.row.label, row.label, "prefab {}", row.prefab_id);
    }

    // The regions: same set, same rings.
    let json_regions =
        parse_regions_payload(&read_doc(&dir, FOREST_REGIONS_GZ).expect("regions json"));
    assert_eq!(json_regions.len(), EVERON_REGIONS);
    let back = regions_from_bytes(&std::fs::read(dir.join(FOREST_REGIONS_RKYV)).expect("read"))
        .expect("regions");
    assert_eq!(back.len(), EVERON_REGIONS);
    let mut vertices = 0usize;
    for (a, j) in back.iter().zip(json_regions.iter()) {
        assert_eq!(a.id, j.id);
        assert_eq!(a.kind, j.kind);
        assert_eq!(a.tree_count, j.tree_count, "region {}", j.id);
        assert_eq!(a.polygon.len(), j.polygon.len(), "region {}", j.id);
        for (ra, rj) in a.polygon.iter().zip(j.polygon.iter()) {
            assert_eq!(ra.len(), rj.len(), "region {} ring length", j.id);
            vertices += ra.len();
        }
    }
    assert!(
        vertices >= 2_000,
        "only {vertices} ring vertices — the corpus is empty or LFS-pointered, so this test \
         proved nothing"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The census is written into two files from one value, and they agree. Without this the
/// standalone file could drift from the copy inside the catalogue and nothing would notice —
/// `TypeInventory` carries no `schema_version` of its own to catch it later.
#[test]
fn standalone_census_matches_the_catalogue() {
    let dir = staged_everon("census");
    emit_catalog_archives(&dir).expect("emit");
    let standalone =
        inventory_from_bytes(&std::fs::read(dir.join(TYPE_INVENTORY_RKYV)).expect("read"))
            .expect("standalone census");
    let embedded = build_prefab_catalog_archive(&dir)
        .expect("catalogue")
        .type_inventory;
    assert_eq!(standalone, embedded);
    assert_eq!(standalone.unique_prefabs as usize, EVERON_PREFABS);
    assert_eq!(standalone.total_instances, EVERON_INSTANCES);
    assert_eq!(
        standalone.by_kind.iter().map(|k| k.instances).sum::<u64>(),
        EVERON_INSTANCES,
        "the per-kind census must add up to the declared total"
    );
    // T-946.19 — the ORDER, on the emitter side too. A sum is permutation-blind, and
    // `by_kind` is an order contract (`INSTANCE_KINDS`, `road` last): the wave-241 verifier
    // showed `by_kind[1..8]` could be shuffled with every test in both crates still green.
    assert_eq!(
        standalone
            .by_kind
            .iter()
            .map(|k| k.kind.as_str())
            .collect::<Vec<_>>(),
        crate::world_export_pipeline::INSTANCE_KINDS.to_vec(),
        "the emitted census must keep `INSTANCE_KINDS` order"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// An empty catalogue is refused rather than written over a committed archive (T-537's rule,
/// applied to the binary lane) — and nothing is written when it is refused.
#[test]
fn an_empty_catalogue_is_refused() {
    let dir = tmp_dir("empty");
    std::fs::write(dir.join(PREFABS_GZ), br#"{"terrainId":"x","prefabs":[]}"#).expect("write");
    std::fs::write(
        dir.join(TYPE_INVENTORY_JSON),
        br#"{"terrainId":"x","censusStatus":"partial","levels":{"uniquePrefabs":0,"totalInstances":0},"byKind":{}}"#,
    )
    .expect("write");
    let msg = format!(
        "{:#}",
        emit_catalog_archives(&dir).expect_err("must refuse")
    );
    assert!(msg.contains("refusing empty write (prefabs-rkyv)"), "{msg}");
    assert!(!dir.join(PREFAB_CATALOG_RKYV).exists(), "nothing written");

    std::fs::write(dir.join(FOREST_REGIONS_GZ), br#"{"regions":[]}"#).expect("write");
    let msg = format!(
        "{:#}",
        build_forest_regions_archive(&dir).expect_err("must refuse")
    );
    assert!(
        msg.contains("refusing empty write (forest-regions-rkyv)"),
        "{msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The two catalogue documents have to describe the same export. A stale census beside a
/// fresh catalogue is exactly the failure a count-only check would miss when the counts
/// happen to match.
#[test]
fn a_census_from_another_export_is_refused() {
    let dir = staged_everon("stale");
    let mut inv: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join(TYPE_INVENTORY_JSON)).expect("read"))
            .expect("parse");
    inv["terrainId"] = serde_json::json!("arland");
    std::fs::write(dir.join(TYPE_INVENTORY_JSON), inv.to_string()).expect("write");
    let msg = format!(
        "{:#}",
        build_prefab_catalog_archive(&dir).expect_err("must refuse")
    );
    assert!(
        msg.contains("one of the two catalogue files is stale"),
        "{msg}"
    );

    inv["terrainId"] = serde_json::json!("everon");
    inv["levels"]["uniquePrefabs"] = serde_json::json!(EVERON_PREFABS - 1);
    std::fs::write(dir.join(TYPE_INVENTORY_JSON), inv.to_string()).expect("write");
    let msg = format!(
        "{:#}",
        build_prefab_catalog_archive(&dir).expect_err("must refuse")
    );
    assert!(
        msg.contains("does not describe the catalogue it ships with"),
        "{msg}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The hop nothing else covers: the paths this emitter **writes** have to be the paths the
/// manifest **names**, or `world_host`'s archive branches fetch a URL nothing ever wrote — a
/// 404 that falls silently back to JSON on every boot, which is indistinguishable from "the
/// binary lane is switched off".
///
/// This is also the only place both halves are visible: the emitter constants live in
/// `tbd-tools`, the manifest parser in `map-engine-core`.
///
/// The second half is T-935.13: the committed everon manifest names the same paths this
/// emitter writes. If they drift, this fails here rather than by the editor fetching the
/// wrong files.
#[test]
fn the_manifest_block_names_the_paths_this_emitter_writes() {
    let flipped = serde_json::json!({
        "objects": {
            "prefabsPath": "objects/prefabs.json.gz",
            "chunksPath": "objects/chunks",
            "binary": {
                "schemaVersion": "1.0.0",
                "container": "TBDC", "containerVersion": 1,
                "pod": "ObjectInstancePod", "podBytes": 32,
                "chunks": "objects/chunks/{cx}_{cy}.bin",
                "prefabs": "objects/prefabs.rkyv",
                "roads": "roads/road_network.rkyv",
                "regions": "objects/forest-regions.rkyv",
                "typeInventory": "objects/type-inventory.rkyv"
            }
        }
    });
    let block = parse_manifest_binary(&flipped)
        .objects
        .expect("objects.binary");
    assert_eq!(block.prefabs, PREFAB_CATALOG_RKYV);
    assert_eq!(block.regions, FOREST_REGIONS_RKYV);
    assert_eq!(block.type_inventory, TYPE_INVENTORY_RKYV);
    assert_eq!(block.roads, super::super::roads_emit::ROAD_NETWORK_RKYV);
    assert!(
        !block.prefabs.is_empty() && !block.regions.is_empty(),
        "world_host reads an empty path as \"not named\" — these must be non-empty"
    );

    let everon: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(everon_dir().join("manifest.json")).expect("everon manifest"),
    )
    .expect("parse everon manifest");
    let live = parse_manifest_binary(&everon)
        .objects
        .expect("T-935.13 writes objects.binary");
    assert_eq!(live.prefabs, PREFAB_CATALOG_RKYV);
    assert_eq!(live.regions, FOREST_REGIONS_RKYV);
    assert_eq!(live.type_inventory, TYPE_INVENTORY_RKYV);
    assert_eq!(live.roads, super::super::roads_emit::ROAD_NETWORK_RKYV);
}

/// A terrain with no `forest-regions.json.gz` (a non-density `--phase`) emits the two
/// catalogue archives and reports two rows — it does not invent an empty regions file, and it
/// does not fail.
#[test]
fn a_terrain_without_regions_emits_only_the_catalogue() {
    let dir = tmp_dir("no-regions");
    for rel in [PREFABS_GZ, TYPE_INVENTORY_JSON] {
        std::fs::copy(everon_dir().join(rel), dir.join(rel)).expect("copy");
    }
    let written = emit_catalog_archives(&dir).expect("emit");
    assert_eq!(written.len(), 2, "{written:?}");
    assert!(!dir.join(FOREST_REGIONS_RKYV).exists());
    let _ = std::fs::remove_dir_all(&dir);
}
