//! Role: Module boundary for streaming/scheduler/residency/t151_11_3_tests.
//! Position: `streaming/scheduler/residency/t151_11_3_tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

use flate2::Compression;

use flate2::write::GzEncoder;

use std::io::Write;

fn gzip(text: &str) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(text.as_bytes()).unwrap();
    enc.finish().unwrap()
}

fn residency_with_one_building() -> WorldResidency {
    let mut r = WorldResidency::new();
    r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
    r.load_prefabs_gz(
            br#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "residential", "spatial": { "halfExtentsM": { "x": 5, "y": 5, "z": 4 } } } ] }"#,
        )
        .unwrap();
    let missing = r.set_viewport(0.0, 0.0, 600.0, 600.0, -2.0);
    assert!(!missing.is_empty());
    for id in &missing {
        r.ingest_chunk_gz(id, &gzip(r#"{"instances":[[9,100.5,200.25,10,45]]}"#))
            .unwrap();
    }
    r.end_apply_frame(0.0);
    r
}

mod cases_1;
