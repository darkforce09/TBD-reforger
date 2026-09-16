//! Role: store tests.
//! Position: `streaming/loaders/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::streaming::loaders::store::*;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::Write;

fn gzip(text: &str) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(text.as_bytes()).unwrap();
    enc.finish().unwrap()
}

#[test]
fn gunzip_and_plain_both_parse() {
    let json = r#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "hut" } ] }"#;
    let mut store = WorldStore::new();

    assert_eq!(store.load_prefabs_gz(&gzip(json)).unwrap(), 1);

    assert_eq!(store.load_prefabs_gz(json.as_bytes()).unwrap(), 1);
    assert_eq!(store.prefab_by_id.get(&9.0_f64.to_bits()).unwrap().code, 0);
}

#[test]
fn parse_chunk_gz_uses_prefab_map_and_counts() {
    let mut store = WorldStore::new();
    store
        .load_prefabs_gz(
            br#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "hut" } ] }"#,
        )
        .unwrap();
    let chunk_json = r#"{ "instances": [ [9, 100.0, 200.0, 0, 0], "bad" ] }"#;
    let count = store.parse_chunk_gz("3_4", &gzip(chunk_json)).unwrap();
    assert_eq!(count, 1);
    assert_eq!(store.chunks_loaded, 1);
    let c = store.last_chunk.as_ref().unwrap();
    assert_eq!(c.cls_codes, vec![0u8]);
    assert_eq!(c.positions, vec![100.0_f32, 200.0]);
}

#[test]
fn manifest_gate() {
    let mut store = WorldStore::new();
    assert!(
        store
            .load_manifest_json(
                r#"{ "objects": { "prefabsPath": "p", "chunksPath": "c", "instanceCount": 5 } }"#
            )
            .is_ok()
    );
    assert_eq!(store.instance_count_total(), 5.0);
    assert!(matches!(
        store.load_manifest_json(r#"{ "objects": {} }"#),
        Err(WorldError::Manifest)
    ));
}

use crate::io::archives::codec::to_bytes;
use crate::io::archives::roads::RoadNetworkArchive;
use crate::io::archives::roads::RoadSegmentArchive;
use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use crate::world::environment::locations::route_placement::road_class_code;

fn roads_json() -> &'static str {
    r#"{ "roadSegments": [
            { "id": "a", "roadClass": "track", "points": [[-1,0],[1,0],[1,10],[-1,10]] },
            { "id": "b", "roadClass": "runway", "points": [[-10,0],[10,0],[10,50],[-10,50]] }
        ] }"#
}

fn roads_rkyv() -> Vec<u8> {
    let archive = RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments: vec![
            RoadSegmentArchive {
                id: "a".to_string(),
                road_class: road_class_code("track"),
                width_m: 2.0,
                centerline: vec![[0.0, 0.0], [0.0, 10.0]],
            },
            RoadSegmentArchive {
                id: "b".to_string(),
                road_class: road_class_code("runway"),
                width_m: 20.0,
                centerline: vec![[0.0, 0.0], [0.0, 50.0]],
            },
        ],
    };
    to_bytes(&archive).expect("serialise").to_vec()
}

#[test]
fn load_roads_sniffs_gzip_versus_rkyv() {
    let mut store = WorldStore::new();
    assert_eq!(store.load_roads(&gzip(roads_json())).unwrap(), 2);
    let from_json = store.roads.clone();
    assert_eq!(from_json[0].road_class, "track");

    let rkyv = roads_rkyv();
    assert!(
        bytes_to_json(&rkyv).is_err(),
        "the archive bytes must not be parseable as JSON, or the sniff proves nothing"
    );
    assert_ne!(rkyv[0], 0x1f, "the archive must not carry gzip magic");

    let mut store2 = WorldStore::new();
    assert_eq!(store2.load_roads(&rkyv).unwrap(), 2);
    assert_eq!(store2.roads, from_json, "both routes, one network");
}

#[test]
fn load_roads_refuses_an_empty_payload() {
    let mut store = WorldStore::new();
    assert!(matches!(
        store.load_roads(&[]),
        Err(WorldError::EmptyPayload)
    ));
    assert!(store.roads.is_empty(), "a refusal must not clear or fill");
}

#[test]
fn truncated_payloads_error_rather_than_reaching_the_wrong_parser() {
    let gz = gzip(roads_json());
    let rk = roads_rkyv();
    let mut store = WorldStore::new();
    assert!(
        matches!(
            store.load_roads(&gz[..gz.len() / 2]),
            Err(WorldError::Gzip(_))
        ),
        "tail-truncated gzip keeps its magic and must fail as gzip"
    );
    assert!(
        matches!(store.load_roads(&gz[4..]), Err(WorldError::Archive(_))),
        "head-truncated gzip has no magic and must be refused by the archive reader"
    );
    assert!(
        matches!(
            store.load_roads(&rk[..rk.len() - 8]),
            Err(WorldError::Archive(_))
        ),
        "truncated archive"
    );

    assert!(matches!(
        store.load_roads(&[0x1f]),
        Err(WorldError::Archive(_))
    ));
}

#[test]
fn committed_everon_roads_json_still_loads_through_the_sniff() {
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/map-assets/everon/objects/roads.json.gz");
    let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
    assert_eq!(&bytes[..2], &[0x1f, 0x8b], "committed roads must be gzip");
    let mut sniffed = WorldStore::new();
    let mut direct = WorldStore::new();
    assert_eq!(sniffed.load_roads(&bytes).unwrap(), 887);
    assert_eq!(direct.load_roads_gz(&bytes).unwrap(), 887);
    assert_eq!(sniffed.roads, direct.roads);
}

#[test]
fn full_island_census_matches_pinned_inventory() {
    let everon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/map-assets/everon");
    let objects = everon.join("objects");
    let read = |p: &std::path::Path| std::fs::read(p).unwrap_or_else(|e| panic!("{p:?}: {e}"));

    let mut store = WorldStore::new();
    store
        .load_manifest_json(&String::from_utf8(read(&everon.join("manifest.json"))).unwrap())
        .unwrap();

    assert_eq!(
        store
            .load_prefabs_gz(&read(&objects.join("prefabs.json.gz")))
            .unwrap(),
        1623
    );

    let mut chunk_files: Vec<String> = std::fs::read_dir(objects.join("chunks"))
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.ends_with(".json.gz").then_some(name)
        })
        .collect();
    chunk_files.sort();
    assert_eq!(chunk_files.len(), 315);
    let mut total: u64 = 0;
    for f in &chunk_files {
        let id = f.trim_end_matches(".json.gz");
        total += u64::from(
            store
                .parse_chunk_gz(id, &read(&objects.join("chunks").join(f)))
                .unwrap(),
        );
    }
    assert_eq!(total, 1_216_066);

    assert_eq!(
        store
            .load_roads_gz(&read(&objects.join("roads.json.gz")))
            .unwrap(),
        887
    );
    assert_eq!(
        store
            .load_forest_regions_gz(&read(&objects.join("forest-regions.json.gz")))
            .unwrap(),
        36
    );

    let mut bins: Vec<String> = std::fs::read_dir(objects.join("density"))
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.ends_with(".bin").then_some(name)
        })
        .collect();
    bins.sort();
    assert_eq!(bins.len(), 625);
    for f in bins.iter().take(3) {
        let grid = crate::io::density::tbdd::decode_tbdd(&read(&objects.join("density").join(f)))
            .unwrap_or_else(|e| panic!("{f}: {e}"));
        assert!(grid.cols > 0 && grid.rows > 0, "{f}: empty grid");
    }
}
