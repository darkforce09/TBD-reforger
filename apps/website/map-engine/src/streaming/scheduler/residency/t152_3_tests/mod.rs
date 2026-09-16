//! Role: Module boundary for streaming/scheduler/residency/t152_3_tests.
//! Position: `streaming/scheduler/residency/t152_3_tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::overlay::lod::class_visible;

use crate::world::environment::classify::class_code;

use crate::streaming::buffers::revision::norm;

use super::*;

use crate::overlay::symbology::labels::glyph_math::BUILDING_CLASSES;

use crate::overlay::symbology::labels::glyph_math::badge_icon_key;

use crate::overlay::symbology::labels::glyph_math::building_icon_key;

use crate::overlay::symbology::labels::glyph_math::landmark_glyph_icon_key;

use std::collections::{HashMap, HashSet};

use std::fs;

use std::path::PathBuf;

const FIXTURE_CHUNK: &str = "2_12";

const N_MIN_BUILDING_GLYPH_LOOKUP: usize = 15;

fn map_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/map-assets")
}

fn glyph_keys_from_manifest() -> Vec<String> {
    let raw = std::fs::read_to_string(map_assets().join("glyphs/manifest.json"))
        .expect("glyphs manifest");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let mut keys: Vec<String> = v["glyphs"]
        .as_object()
        .expect("glyphs object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

fn load_everon_residency() -> WorldResidency {
    let everon = map_assets().join("everon");
    let mut r = WorldResidency::new();
    r.load_manifest_json(
        &std::fs::read_to_string(everon.join("manifest.json")).expect("everon manifest"),
    )
    .unwrap();
    r.load_prefabs_gz(
        &std::fs::read(everon.join("objects/prefabs.json.gz")).expect("everon prefabs"),
    )
    .unwrap();
    r.load_chunk_index_json(
        &std::fs::read_to_string(everon.join("objects/chunks/manifest.json")).expect("chunk index"),
    )
    .unwrap();
    r.set_glyph_key_map(&glyph_keys_from_manifest());
    r
}

fn building_class_by_prefab_u16() -> HashMap<u16, String> {
    let everon = map_assets().join("everon");
    let raw = crate::streaming::loaders::store::bytes_to_json(
        &std::fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    let mut out = HashMap::new();
    for row in narrow_prefab_rows(&raw) {
        if row.kind != "building" {
            continue;
        }
        let pid = row.prefab_id;
        if (0.0..65536.0).contains(&pid) && pid.fract() == 0.0 {
            out.insert(pid as u16, row.class);
        }
    }
    out
}

fn building_importance_by_prefab_u16() -> HashMap<u16, f64> {
    let everon = map_assets().join("everon");
    let raw = crate::streaming::loaders::store::bytes_to_json(
        &std::fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    let mut out = HashMap::new();
    for row in narrow_prefab_rows(&raw) {
        if row.kind != "building" {
            continue;
        }
        let pid = row.prefab_id;
        if !(0.0..65536.0).contains(&pid) || pid.fract() != 0.0 {
            continue;
        }
        if let Some(iz) = row.importance_zoom {
            out.insert(pid as u16, iz);
        }
    }
    out
}

fn drive_fixture_chunk(r: &mut WorldResidency, chunk_id: &str, z: f64) {
    let mut parts = chunk_id.split('_');
    let cx: f64 = parts.next().unwrap().parse().unwrap();
    let cy: f64 = parts.next().unwrap().parse().unwrap();
    let min_x = cx * 512.0;
    let min_y = cy * 512.0;
    let missing = r.set_viewport(min_x, min_y, min_x + 512.0, min_y + 512.0, z);
    let chunk_path = map_assets()
        .join("everon/objects/chunks")
        .join(format!("{chunk_id}.json.gz"));
    for id in &missing {
        let bytes = std::fs::read(&chunk_path).unwrap_or_else(|_| {
            std::fs::read(
                map_assets()
                    .join("everon/objects/chunks")
                    .join(format!("{id}.json.gz")),
            )
            .unwrap()
        });
        r.ingest_chunk_gz(id, &bytes).unwrap();
    }
    if !missing.is_empty() {
        r.end_apply_frame(0.0);
    }
}

fn oracle_badge_count(
    r: &WorldResidency,
    prefab_class: &HashMap<u16, String>,
    prefab_importance: &HashMap<u16, f64>,
    atlas_keys: &HashSet<String>,
    z: f64,
) -> usize {
    let badge_gate = class_visible("buildingBadge", z);
    let building_code = class_code("building");
    let mut n = 0usize;
    for id in &r.draw_ids {
        let Some(chunk) = r.chunks.get(id) else {
            continue;
        };
        let Some(rows) = chunk.rows_by_class.get(&building_code) else {
            continue;
        };
        for &row in rows {
            let row = row as usize;
            let u16k = chunk.prefab_idx[row];
            let Some(cls) = prefab_class.get(&u16k) else {
                continue;
            };
            let importance_ok = prefab_importance.get(&u16k).is_some_and(|iz| z >= *iz);
            if !badge_gate && !importance_ok {
                continue;
            }
            let Some(key) = landmark_glyph_icon_key(cls) else {
                continue;
            };
            if atlas_keys.contains(key) {
                n += 1;
            }
        }
    }
    n
}

fn oracle_landmark_glyph_count_for_chunk(
    r: &WorldResidency,
    chunk_id: &str,
    prefab_class: &HashMap<u16, String>,
    atlas_keys: &HashSet<String>,
    z: f64,
) -> usize {
    if !class_visible("buildingBadge", z) {
        return 0;
    }
    let Some(chunk) = r.chunks.get(chunk_id) else {
        return 0;
    };
    let building_code = class_code("building");
    let Some(rows) = chunk.rows_by_class.get(&building_code) else {
        return 0;
    };
    let landmark = ["lighthouse", "castle", "bridge"];
    let mut n = 0usize;
    for &row in rows {
        let row = row as usize;
        let Some(cls) = prefab_class.get(&chunk.prefab_idx[row]) else {
            continue;
        };
        if !landmark.contains(&cls.as_str()) {
            continue;
        }
        let Some(key) = landmark_glyph_icon_key(cls) else {
            continue;
        };
        if atlas_keys.contains(key) {
            n += 1;
        }
    }
    n
}

fn badge_glyph_indices(buf: &[u8]) -> Vec<u16> {
    let stride = crate::overlay::symbology::labels::glyph_math::ICON_INSTANCE_STRIDE;
    buf.chunks(stride)
        .map(|chunk| u16::from_le_bytes(chunk[14..16].try_into().unwrap()))
        .collect()
}

fn world_glyphs_atlas_keys() -> HashSet<String> {
    let raw = std::fs::read_to_string(map_assets().join("glyphs/atlas/world-glyphs.json"))
        .expect("world-glyphs.json");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["icons"]
        .as_object()
        .expect("icons object")
        .keys()
        .cloned()
        .collect()
}

const EVERON_TERRAIN_M: f64 = 12800.0;

const PIER_CENSUS: u32 = 2299;

const BRIDGE_CENSUS: usize = 144;

fn drive_full_island(r: &mut WorldResidency, z: f64) {
    let missing = r.set_viewport(0.0, 0.0, EVERON_TERRAIN_M, EVERON_TERRAIN_M, z);
    let dir = map_assets().join("everon/objects/chunks");
    for id in &missing {
        match std::fs::read(dir.join(format!("{id}.json.gz"))) {
            Ok(bytes) => {
                r.ingest_chunk_gz(id, &bytes).unwrap();
            }
            Err(_) => r.note_undelivered(id),
        }
    }
    r.end_apply_frame(0.0);
}

fn island_bridge_count(r: &WorldResidency) -> usize {
    let building_code = class_code("building");
    let mut ids = r.pinned_ids.clone();
    ids.sort();
    let mut n = 0usize;
    for id in &ids {
        let Some(chunk) = r.chunks.get(id) else {
            continue;
        };
        let Some(rows) = chunk.rows_by_class.get(&building_code) else {
            continue;
        };
        for &row in rows {
            let row = row as usize;
            if let Some(info) = r.building_by_u16.get(&chunk.prefab_idx[row])
                && info.building_class == "bridge"
            {
                n += 1;
            }
        }
    }
    n
}

fn fill_instances_with_color(fill: &[f32], rgba: [u8; 4]) -> usize {
    let want = norm(rgba);
    fill.chunks_exact(10)
        .filter(|inst| {
            (inst[6] - want[0]).abs() < 1e-6
                && (inst[7] - want[1]).abs() < 1e-6
                && (inst[8] - want[2]).abs() < 1e-6
                && (inst[9] - want[3]).abs() < 1e-6
        })
        .count()
}

mod cases_1;
