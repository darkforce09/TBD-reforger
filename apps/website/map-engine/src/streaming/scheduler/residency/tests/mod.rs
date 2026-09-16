//! Role: Module boundary for streaming/scheduler/residency/tests.
//! Position: `streaming/scheduler/residency/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::classify::class_code;

use super::*;

use crate::environment::vegetation::canopy::density_grid_dims;

use crate::environment::vegetation::canopy::density_texel_sum_for_draw_ids;

use crate::environment::vegetation::canopy::pack_density_grid_r32;

use flate2::Compression;

use flate2::write::GzEncoder;

use std::io::Write;

fn gzip(text: &str) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(text.as_bytes()).unwrap();
    enc.finish().unwrap()
}

fn density_grid_of(r: &WorldResidency) -> (Vec<u32>, u32) {
    let (gw, gh) = density_grid_dims(r.terrain.width, r.terrain.height, 512.0);
    (pack_density_grid_r32(&r.chunks, gw, gh), gw)
}

fn setup() -> WorldResidency {
    let mut r = WorldResidency::new();
    r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
    r.load_prefabs_gz(
            br#"{ "prefabs": [ { "prefabId": 9, "kind": "building", "class": "residential", "spatial": { "halfExtentsM": { "x": 5, "y": 5, "z": 4 } } } ] }"#,
        )
        .unwrap();

    let mut cells = String::from("{\"cells\":[");
    for cy in 0..25 {
        for cx in 0..25 {
            if cx != 0 || cy != 0 {
                cells.push(',');
            }
            cells.push_str(&format!(
                "{{\"cx\":{cx},\"cy\":{cy},\"path\":\"objects/chunks/{cx}_{cy}.json.gz\"}}"
            ));
        }
    }
    cells.push_str("]}");
    r.load_chunk_index_json(&cells).unwrap();
    r
}

fn chunk_bytes(id: &str) -> Vec<u8> {
    let mut parts = id.split('_');
    let cx: f64 = parts.next().unwrap().parse().unwrap();
    let cy: f64 = parts.next().unwrap().parse().unwrap();
    let x = cx * 512.0 + 100.5;
    let y = cy * 512.0 + 200.25;
    gzip(&format!("{{\"instances\":[[9,{x},{y},10,45]]}}"))
}

fn drive(r: &mut WorldResidency, bbox: [f64; 4]) {
    let missing = r.set_viewport(bbox[0], bbox[1], bbox[2], bbox[3], -2.0);
    for id in &missing {
        r.ingest_chunk_gz(id, &chunk_bytes(id)).unwrap();
    }
    if !missing.is_empty() {
        r.end_apply_frame(0.0);
    }
}

fn inject_trees(r: &mut WorldResidency, id: &str, n: usize) {
    let c = r.chunks.get_mut(id).unwrap();
    c.count = n as u32;
    c.positions = vec![0.0; n * 2];
    c.prefab_idx = vec![0; n];
    c.rotations = vec![0.0; n];
    c.z = vec![0.0; n];
    c.cls_codes = vec![class_code("tree"); n];
    c.rows_by_class.clear();
    c.rows_by_class
        .insert(class_code("tree"), (0..n as u32).collect());

    r.content_epoch += 1;
}

fn dense_forest_setup() -> WorldResidency {
    let mut r = WorldResidency::new();
    r.load_manifest_json(
            r#"{ "worldBounds": [0,0,12800,12800], "objects": { "prefabsPath": "p", "chunksPath": "c", "chunkSizeM": 512 } }"#,
        )
        .unwrap();
    r.load_prefabs_gz(
            br##"{ "prefabs": [ { "prefabId": 0, "kind": "tree", "class": "conifer", "spatial": { "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 }, "heightM": 12 }, "render": { "iconKey": "tree-conifer", "baseSizePx": 18, "defaultColor": "#2d5a27" } } ] }"##,
        )
        .unwrap();
    let mut cells = String::from("{\"cells\":[");
    for cy in 0..25 {
        for cx in 0..25 {
            if cx != 0 || cy != 0 {
                cells.push(',');
            }
            cells.push_str(&format!(
                "{{\"cx\":{cx},\"cy\":{cy},\"path\":\"objects/chunks/{cx}_{cy}.json.gz\"}}"
            ));
        }
    }
    cells.push_str("]}");
    r.load_chunk_index_json(&cells).unwrap();
    r.set_glyph_key_map(&["tree-conifer".to_string()]);
    r
}

mod cases_1;
