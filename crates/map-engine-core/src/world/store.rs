//! `WorldStore` — the manifest + prefab table + roads + regions + the last parsed chunk. The
//! loaders gunzip (`.json.gz`, sniffing the `0x1f 0x8b` magic like `bytesToJson`) and dispatch
//! to the pure parsers. W2 keeps one `last_chunk` live at a time (parse-one / read / next); the
//! wasm shim exposes its columns zero-copy.

use std::collections::HashMap;
use std::io::Read;

use serde_json::Value;

use super::airfield::compute_airfield_bbox;
use super::chunk::{WorldChunk, parse_chunk};
use super::chunk_math::Bbox;
use super::manifest::{ObjectsManifest, parse_objects_manifest};
use super::prefab::{PrefabEntry, build_prefab_maps, narrow_prefab_rows};
use super::regions::{LandCoverRegion, parse_regions_payload};
use super::roads::{RoadSegment, parse_roads_payload, road_network_from_bytes};

/// A world-parse failure (gunzip, JSON, a binary archive, or a manifest missing the object-export
/// paths).
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("world: gzip inflate failed: {0}")]
    Gzip(String),
    #[error("world: json parse failed: {0}")]
    Json(String),
    #[error("world: manifest missing objects/prefabsPath/chunksPath")]
    Manifest,
    /// T-935.6 — a Tier-2 rkyv archive failed to load. The rendered [`BinaryError`] is kept as a
    /// `String` for the same reason `BinaryError::Archive` keeps rkyv's: the public shape of this
    /// enum stays free of the binary module's types, so a `world`-without-`binary` build (should
    /// one ever exist) would not change it.
    ///
    /// [`BinaryError`]: super::binary::BinaryError
    #[error("world: binary archive failed to load: {0}")]
    Archive(String),
    /// A zero-length payload. Named rather than folded into [`WorldError::Json`] because the
    /// format sniff cannot even *classify* an empty buffer, and "json parse failed: EOF while
    /// parsing a value" is a misleading thing to say about a file that was never fetched.
    #[error("world: empty payload — no format to sniff")]
    EmptyPayload,
}

/// Gunzip-or-plain JSON parse (mirror of `bytesToJson`). Static `.json.gz` files are served raw,
/// so sniff the gzip magic; otherwise parse as UTF-8 JSON. `serde_json` is built with
/// `float_roundtrip` (see `Cargo.toml`) so floats parse correctly-rounded like `JSON.parse`.
/// Shared with [`super::residency`] (the W3 multi-chunk ingest path uses the identical decode).
/// Gunzip-or-plain JSON parse (shared host/debug surface for the Leptos map-asset loader).
pub fn bytes_to_json(bytes: &[u8]) -> Result<Value, WorldError> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut inflated = Vec::new();
        decoder
            .read_to_end(&mut inflated)
            .map_err(|e| WorldError::Gzip(e.to_string()))?;
        serde_json::from_slice(&inflated).map_err(|e| WorldError::Json(e.to_string()))
    } else {
        serde_json::from_slice(bytes).map_err(|e| WorldError::Json(e.to_string()))
    }
}

/// Manifest + prefab table + roads + regions + the last parsed chunk.
#[derive(Default)]
pub struct WorldStore {
    pub manifest: Option<ObjectsManifest>,
    pub prefab_by_id: HashMap<u64, PrefabEntry>,
    pub has_oversized: bool,
    pub roads: Vec<RoadSegment>,
    pub regions: Vec<LandCoverRegion>,
    pub last_chunk: Option<WorldChunk>,
    pub chunks_loaded: usize,
}

impl WorldStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse the terrain manifest's `objects` block.
    ///
    /// # Errors
    /// [`WorldError::Json`] on invalid JSON; [`WorldError::Manifest`] when the object-export
    /// paths are absent.
    pub fn load_manifest_json(&mut self, json: &str) -> Result<(), WorldError> {
        let raw: Value = serde_json::from_str(json).map_err(|e| WorldError::Json(e.to_string()))?;
        self.manifest = Some(parse_objects_manifest(&raw).ok_or(WorldError::Manifest)?);
        Ok(())
    }

    /// Load + narrow `prefabs.json.gz`, building the prefab lookup + `has_oversized`. Returns the
    /// prefab count.
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn load_prefabs_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        let (by_id, has_oversized) = build_prefab_maps(narrow_prefab_rows(&raw));
        self.prefab_by_id = by_id;
        self.has_oversized = has_oversized;
        Ok(self.prefab_by_id.len())
    }

    /// Parse one `objects/chunks/{id}.json.gz` into `last_chunk`. Returns its instance count
    /// (0 when the payload has no `instances` array).
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn parse_chunk_gz(&mut self, id: &str, bytes: &[u8]) -> Result<u32, WorldError> {
        let raw = bytes_to_json(bytes)?;
        let chunk = parse_chunk(id, &raw, &self.prefab_by_id);
        let count = chunk.as_ref().map_or(0, |c| c.count);
        if chunk.is_some() {
            self.chunks_loaded += 1;
        }
        self.last_chunk = chunk;
        Ok(count)
    }

    /// Load + centerline `roads.json.gz`. Returns the kept segment count.
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn load_roads_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        self.roads = parse_roads_payload(&raw);
        Ok(self.roads.len())
    }

    /// T-935.6 — load the road network from **either** source, sniffing which one arrived.
    ///
    /// The caller (the world host, and after T-935.11 the manifest-driven URL switch) hands over
    /// bytes without knowing the format:
    ///
    /// | first bytes | route |
    /// |---|---|
    /// | *none* | [`WorldError::EmptyPayload`] — an empty buffer has no format to sniff |
    /// | `1f 8b` | gzip: [`load_roads_gz`](Self::load_roads_gz), the JSON path, unchanged |
    /// | anything else | [`road_network_from_bytes`], the validating rkyv reader |
    ///
    /// # Why `else` is rkyv rather than "try JSON too"
    ///
    /// Because the two failure modes that matter must land in an *error*, not in the wrong parser.
    /// A file truncated at the head loses the gzip magic and reaches the archive reader, which
    /// rejects it; a file truncated at the tail keeps its magic and reaches the gunzip, which
    /// rejects it; an archive mislabelled `.json.gz` has no magic and is read as what it is. A
    /// fallback chain would instead answer every one of them with the *second* parser's error, and
    /// a JSON payload that happened to survive an rkyv validation pass would be a wild read.
    /// [`load_roads_gz`] is still public and still accepts bare (ungzipped) JSON for the callers
    /// that know they have JSON — this method is for the ones that do not.
    ///
    /// # Errors
    /// [`WorldError::EmptyPayload`] on a zero-length buffer;
    /// [`WorldError::Gzip`]/[`WorldError::Json`] from the JSON route;
    /// [`WorldError::Archive`] from the rkyv route (invalid, truncated, wrong schema version, or a
    /// road class this build cannot name).
    pub fn load_roads(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        if bytes.is_empty() {
            return Err(WorldError::EmptyPayload);
        }
        if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            return self.load_roads_gz(bytes);
        }
        self.roads =
            road_network_from_bytes(bytes).map_err(|e| WorldError::Archive(e.to_string()))?;
        Ok(self.roads.len())
    }

    /// Load + narrow `forest-regions.json.gz`. Returns the kept region count.
    ///
    /// # Errors
    /// [`WorldError::Gzip`]/[`WorldError::Json`] on a bad payload.
    pub fn load_forest_regions_gz(&mut self, bytes: &[u8]) -> Result<usize, WorldError> {
        let raw = bytes_to_json(bytes)?;
        self.regions = parse_regions_payload(&raw);
        Ok(self.regions.len())
    }

    /// Declared total instance count from the manifest (`objects.instanceCount`), if loaded.
    #[must_use]
    pub fn instance_count_total(&self) -> f64 {
        self.manifest
            .as_ref()
            .and_then(|m| m.instance_count)
            .unwrap_or(0.0)
    }

    /// Runway segments only (T-152.5 census / bbox).
    #[must_use]
    pub fn runway_segments(&self) -> Vec<&RoadSegment> {
        self.roads
            .iter()
            .filter(|r| r.road_class == "runway")
            .collect()
    }

    /// NW airfield bbox from runway union + 30 m margin.
    #[must_use]
    pub fn airfield_bbox(&self) -> Option<Bbox> {
        let runways: Vec<&RoadSegment> = self.runway_segments();
        compute_airfield_bbox(&runways.into_iter().cloned().collect::<Vec<RoadSegment>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        // Gzipped path (magic sniff).
        assert_eq!(store.load_prefabs_gz(&gzip(json)).unwrap(), 1);
        // Plain-bytes path (no magic).
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
        assert_eq!(count, 1); // "bad" dropped
        assert_eq!(store.chunks_loaded, 1);
        let c = store.last_chunk.as_ref().unwrap();
        assert_eq!(c.cls_codes, vec![0u8]);
        assert_eq!(c.positions, vec![100.0_f32, 200.0]);
    }

    #[test]
    fn manifest_gate() {
        let mut store = WorldStore::new();
        assert!(store
            .load_manifest_json(r#"{ "objects": { "prefabsPath": "p", "chunksPath": "c", "instanceCount": 5 } }"#)
            .is_ok());
        assert_eq!(store.instance_count_total(), 5.0);
        assert!(matches!(
            store.load_manifest_json(r#"{ "objects": {} }"#),
            Err(WorldError::Manifest)
        ));
    }

    /* ─────────────── T-935.6 — the roads format sniff ─────────────── */

    use super::super::binary::archives::{
        ARCHIVE_SCHEMA_VERSION, RoadNetworkArchive, RoadSegmentArchive,
    };
    use super::super::binary::to_bytes;
    use super::super::road_labels::road_class_code;

    fn roads_json() -> &'static str {
        r#"{ "roadSegments": [
            { "id": "a", "roadClass": "track", "points": [[-1,0],[1,0],[1,10],[-1,10]] },
            { "id": "b", "roadClass": "runway", "points": [[-10,0],[10,0],[10,50],[-10,50]] }
        ] }"#
    }

    /// The same two segments as [`roads_json`], as the emitter would write them.
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

    /// Both formats reach the same in-memory network through one entry point, and — the part that
    /// makes this non-vacuous — the archive bytes are provably NOT readable as JSON, so the
    /// segment count on that side can only have come from the rkyv route.
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

    /// An empty buffer is refused before the two magic bytes are read (there are none to read).
    #[test]
    fn load_roads_refuses_an_empty_payload() {
        let mut store = WorldStore::new();
        assert!(matches!(
            store.load_roads(&[]),
            Err(WorldError::EmptyPayload)
        ));
        assert!(store.roads.is_empty(), "a refusal must not clear or fill");
    }

    /// Truncation on either end lands in an error, never in the other parser. The head-truncated
    /// gzip is the case the sniff could plausibly get wrong: it loses `1f 8b` and is offered to
    /// rkyv, which must reject it rather than validate garbage.
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
        // A one-byte buffer is non-empty but too short to be gzip: it must not index past its end.
        assert!(matches!(
            store.load_roads(&[0x1f]),
            Err(WorldError::Archive(_))
        ));
    }

    /// The JSON route through `load_roads` is byte-for-byte the old `load_roads_gz` behaviour —
    /// this is the "the JSON path still works until cutover" half of the acceptance, pinned on the
    /// COMMITTED everon export rather than a fixture.
    #[test]
    fn committed_everon_roads_json_still_loads_through_the_sniff() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon/objects/roads.json.gz");
        let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        assert_eq!(&bytes[..2], &[0x1f, 0x8b], "committed roads must be gzip");
        let mut sniffed = WorldStore::new();
        let mut direct = WorldStore::new();
        assert_eq!(sniffed.load_roads(&bytes).unwrap(), 887);
        assert_eq!(direct.load_roads_gz(&bytes).unwrap(), 887);
        assert_eq!(sniffed.roads, direct.roads);
    }

    /// T-159.29.2 — the full-Everon census pin, re-homed from the deleted React vitest
    /// (`_wasm/world.parity.test.ts`, T-151.2). The per-column byte-parity half of that suite
    /// compared Rust against the JS `worldObjectsCore` oracle and is retired with it (both sides
    /// now ARE this crate); the census totals are an oracle-independent pin over the committed
    /// `packages/map-assets` export and stay load-bearing: any regression in gunzip/parse/classify
    /// or accidental asset edit breaks an exact number.
    #[test]
    fn full_island_census_matches_pinned_inventory() {
        let everon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon");
        let objects = everon.join("objects");
        let read = |p: &std::path::Path| std::fs::read(p).unwrap_or_else(|e| panic!("{p:?}: {e}"));

        let mut store = WorldStore::new();
        store
            .load_manifest_json(&String::from_utf8(read(&everon.join("manifest.json"))).unwrap())
            .unwrap();
        // 1623 prefabs.
        assert_eq!(
            store
                .load_prefabs_gz(&read(&objects.join("prefabs.json.gz")))
                .unwrap(),
            1623
        );

        // 315 chunk files; ΣInstances = 1,216,066 (T-090.12.1b: the 2026-09-04 Workbench
        // re-export with scale; 1,216,109 before it).
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

        // 887 road segments (roads.json.gz refreshed from the latest roads export at 9ae81694c;
        // was 888) · 36 forest regions.
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

        // 625 TBDD density grids; decode smoke on the first 3 (the decoder's own unit tests cover
        // the format edges — this proves the committed .bin files stay decodable).
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
            let grid = crate::geometry::tbdd::decode_tbdd(&read(&objects.join("density").join(f)))
                .unwrap_or_else(|e| panic!("{f}: {e}"));
            assert!(grid.cols > 0 && grid.rows > 0, "{f}: empty grid");
        }
    }
}
