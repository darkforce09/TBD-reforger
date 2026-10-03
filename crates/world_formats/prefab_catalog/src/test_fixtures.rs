//! Everon prefab catalogue fixtures shared by the prefab table tests and the chunk tests.
//!
//! **Role:** builds the committed Everon prefab catalogue in both of its served forms (gzip JSON,
//! narrowed to f32 like the archive, and the rkyv archive) for the tests that compare the lanes.
//! **Position:** compiled for this crate's tests and, through the `test_fixtures` feature, for
//! `world_chunks`' container tests and the map engine's chunk scheduler tests
//! (`chunk_ingest_prefab_lane_tests.rs`, `chunk_ingest_chunk_bin_tests.rs`).
//! **Signals & state:** none; every call reads `assets/terrains/everon/` afresh.
//! **Invariants:** the two forms carry the same rows; `EVERON_PREFABS` pins the size of the
//! committed corpus.

use crate::prefab_rows::inventory_to_archive;
use crate::prefab_rows::narrow_prefab_rows;
use crate::prefab_rows::row_to_archive;
use crate::world_payload::bytes_to_json;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use world_file_formats::archives::codec::to_bytes;
use world_file_formats::archives::prefabs::PrefabCatalogArchive;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;

/// The distinct prefab ids of the committed Everon catalogue.
pub const EVERON_PREFABS: usize = 1623;

fn map_assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets/terrains")
}

/// The committed Everon terrain export.
pub fn everon() -> PathBuf {
    map_assets().join("everon")
}

/// `bytes` gzip-compressed at the default level, the form the JSON lane serves.
pub fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bytes).expect("gzip into memory");
    enc.finish().expect("gzip into memory")
}

fn everon_prefabs_gz() -> Vec<u8> {
    std::fs::read(everon().join("objects/prefabs.json.gz")).expect("prefabs.json.gz")
}

/// The Everon prefab catalogue JSON as served (`objects/prefabs.json.gz`), decoded.
pub fn everon_prefabs_json() -> Value {
    bytes_to_json(&everon_prefabs_gz()).expect("prefabs decode")
}

#[allow(clippy::cast_possible_truncation)]
fn f32_narrow(v: f64) -> f64 {
    f64::from(v as f32)
}

/// The Everon prefab catalogue JSON with every number the archive stores as f32 narrowed to
/// f32, so it compares equal with the archive lane.
pub fn everon_prefabs_json_f32() -> Value {
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

/// The Everon prefab catalogue as `objects/prefabs.rkyv` bytes, built from the committed JSON and
/// type inventory.
pub fn everon_archive_bytes() -> Vec<u8> {
    let rows = narrow_prefab_rows(&everon_prefabs_json()).expect("Everon ids are whole u32s");
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
