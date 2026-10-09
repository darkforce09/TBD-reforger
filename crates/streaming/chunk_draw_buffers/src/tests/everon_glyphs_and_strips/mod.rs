//! Role: the Everon glyph and strip cases' fixtures: the Everon residency and the chunk driver.
//! Position: `chunk_draw_buffers::tests::everon_glyphs_and_strips`, compiled only in test builds;
//! drives `crate::world_residency::WorldResidency` over the committed export
//! under `assets/terrains/everon/` and the glyphs under `assets/glyphs/`.
//! Signals & state: none; every case builds its own residency.
//! Invariants: the fixtures read the export directly, never the residency's draw buffers.

use map_draw_lanes::zoom_gates::class_visible;

use prefab_catalog::render_classes::class_code;

use crate::world_residency::WorldResidency;
use prefab_catalog::footprint_lookups::building_prefab_lookup;
use prefab_catalog::footprint_lookups::fence_prefab_lookup;
use prefab_catalog::prefab_rows::narrow_prefab_rows;
use world_chunks::ChunkId;

use label_layout::glyph_math::BUILDING_CLASSES;

use label_layout::glyph_math::badge_icon_key;

use label_layout::glyph_math::building_icon_key;

use label_layout::glyph_math::landmark_glyph_icon_key;

use std::collections::{HashMap, HashSet};

use std::fs;

use std::path::PathBuf;

const FIXTURE_CHUNK: &str = "2_12";

fn map_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/terrains")
}

/// Glyphs are shared by every terrain, so they sit beside the terrain tree rather than inside one.
fn glyph_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/glyphs")
}

fn glyph_keys_from_manifest() -> Vec<String> {
    let raw =
        std::fs::read_to_string(glyph_assets().join("manifest.json")).expect("glyphs manifest");
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
    let raw = prefab_catalog::world_payload::bytes_to_json(
        &std::fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    let mut out = HashMap::new();
    for row in narrow_prefab_rows(&raw).expect("Everon ids are whole u32s") {
        if row.kind != "building" {
            continue;
        }
        if let Ok(pid) = u16::try_from(row.prefab_id.get()) {
            out.insert(pid, row.class);
        }
    }
    out
}

fn building_importance_by_prefab_u16() -> HashMap<u16, f64> {
    let everon = map_assets().join("everon");
    let raw = prefab_catalog::world_payload::bytes_to_json(
        &std::fs::read(everon.join("objects/prefabs.json.gz")).unwrap(),
    )
    .unwrap();
    let mut out = HashMap::new();
    for row in narrow_prefab_rows(&raw).expect("Everon ids are whole u32s") {
        if row.kind != "building" {
            continue;
        }
        let Ok(pid) = u16::try_from(row.prefab_id.get()) else {
            continue;
        };
        if let Some(iz) = row.importance_zoom {
            out.insert(pid, iz);
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
    for id in r.draw_ids() {
        let Some(chunk) = r.chunk(&ChunkId::from(id.as_str())) else {
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

fn world_glyphs_atlas_keys() -> HashSet<String> {
    let raw = std::fs::read_to_string(glyph_assets().join("atlas/world-glyphs.json"))
        .expect("world-glyphs.json");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["icons"]
        .as_object()
        .expect("icons object")
        .keys()
        .cloned()
        .collect()
}

mod cases_1;
