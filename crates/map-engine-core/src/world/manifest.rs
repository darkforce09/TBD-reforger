//! Terrain-manifest `objects` block + chunk-index cells — ports of the `doLoadManifest`
//! `ObjectsBlock` read (`worldObjectsCore.ts:487`) and `narrowCells` (`:361`).
//!
//! T-935.1 adds the **binary blocks** (spec §5). Each is optional and each means one thing: *this
//! asset also exists in its binary form, at this path*. A block that is absent means the JSON/PNG
//! path is the only one, which is why every one of them is `Option` and why
//! [`parse_objects_manifest`] behaves byte-for-byte as before on a manifest that has none — the
//! committed everon manifest is exactly such a manifest today, and
//! `everon_manifest_parses_unchanged` pins that. T-935.13 is the only slice that writes these
//! blocks into `packages/map-assets/everon/manifest.json`.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::binary::chunk_container::CONTAINER_VERSION;
use super::binary::pod::{POD_BYTES, POD_NAME};

/// Default chunk edge in meters when the manifest omits `chunkSizeM` (`DEFAULT_CHUNK_SIZE_M`).
pub const DEFAULT_CHUNK_SIZE_M: f64 = 512.0;

/// The `container` name a `TBDC` `objects.binary` block declares.
pub const TBDC_CONTAINER: &str = "TBDC";

/// `tiles.satellite.unified.encoding` for the rkyv-indexed satellite container (spec §3.5). The
/// shipped value is `tbd-sat-v1`; the flip is T-935.10/.13.
pub const SAT_UNIFIED_ENCODING_V2: &str = "tbd-sat-v2";

/// The manifest `objects` fields this parser consumes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectsManifest {
    pub prefabs_path: String,
    pub chunks_path: String,
    pub chunk_size_m: f64,
    pub roads_path: Option<String>,
    pub density_path: Option<String>,
    pub regions_path: Option<String>,
    pub instance_count: Option<f64>,
    pub prefab_count: Option<f64>,
    /// T-935 `objects.binary` (spec §5). `None` on every manifest shipped before T-935.13.
    pub binary: Option<ObjectsBinaryBlock>,
}

/// `objects.binary` — the chunk container and the four Tier-2 archives that replace the
/// `objects/*.json.gz` set.
///
/// `pod` / `pod_bytes` are carried so a loader can refuse a manifest describing a row shape it does
/// not implement instead of reading every chunk at the wrong stride; see
/// [`ObjectsBinaryBlock::matches_this_build`].
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ObjectsBinaryBlock {
    pub schema_version: String,
    /// `"TBDC"`.
    pub container: String,
    pub container_version: u16,
    /// `"ObjectInstancePod"`.
    pub pod: String,
    pub pod_bytes: u32,
    /// Chunk path template, `objects/chunks/{cx}_{cy}.bin`.
    pub chunks: String,
    pub prefabs: String,
    pub roads: String,
    pub regions: String,
    pub type_inventory: String,
}

impl ObjectsBinaryBlock {
    /// Does this block describe the container and row shape **this build** actually implements?
    ///
    /// This is the whole point of putting `pod`/`podBytes`/`containerVersion` on the wire: the
    /// alternative to checking them is reading a 24-byte-row file at a 32-byte stride and drawing
    /// a map made of garbage that never once errors.
    #[must_use]
    pub fn matches_this_build(&self) -> bool {
        self.container == TBDC_CONTAINER
            && self.container_version == CONTAINER_VERSION
            && self.pod == POD_NAME
            && self.pod_bytes as usize == POD_BYTES
    }
}

/// `dem.raw` — the `TBDE` twin of the 16-bit DEM PNG.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DemRawBlock {
    /// `dem/elevation.dem`.
    pub path: String,
    /// `tbde-v1`.
    pub encoding: String,
}

/// Top-level `labels` — the rkyv label archive that replaces `locations.json` + the road-name JSON.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LabelsBlock {
    /// `locations/map_labels.rkyv`.
    pub path: String,
    /// `rkyv-map-labels-v1`.
    pub encoding: String,
}

/// Top-level `water` — vectors (rkyv) and the bathymetry raster (`TBDB`).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WaterBlock {
    /// `water/water_vectors.rkyv`.
    pub vectors: String,
    /// `water/bathymetry.tbd-bath`.
    pub bathymetry: String,
    /// `tbdb-v1`.
    pub encoding: String,
}

/// Top-level `buildings` — the blueprint archive plus the directory its `.bvh` sidecars live in.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildingsBlock {
    /// `prefabs/building_blueprints.rkyv`.
    pub archive: String,
    /// `prefabs/blas`.
    pub blas: String,
}

/// Every binary block a manifest can carry, in one value.
///
/// All-`None` (and `satellite_unified_v2 == false`) is the shipped state and means "use the
/// JSON/PNG paths for everything" — the migration is per-asset, so a half-converted manifest is a
/// legal and expected intermediate state through waves 2-5.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ManifestBinary {
    pub objects: Option<ObjectsBinaryBlock>,
    pub dem_raw: Option<DemRawBlock>,
    pub labels: Option<LabelsBlock>,
    pub water: Option<WaterBlock>,
    pub buildings: Option<BuildingsBlock>,
    /// `tiles.satellite.unified.encoding == "tbd-sat-v2"`.
    pub satellite_unified_v2: bool,
}

impl ManifestBinary {
    /// Nothing binary here — every asset loads from its JSON/PNG path.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A block deserialised from its `Value`, or `None`.
///
/// `None` covers both "absent" and "present but malformed", and that is deliberate: the fallback
/// for a binary block is the JSON path that still works, so a typo in a hand-edited manifest costs
/// a slower load rather than a blank map. Every field is `serde(default)`, so only a wrong *type*
/// can reach the `None` arm here.
fn block<T: DeserializeOwned>(v: Option<&Value>) -> Option<T> {
    serde_json::from_value(v?.clone()).ok()
}

/// `tiles.satellite.unified.encoding`, when the manifest declares one.
#[must_use]
pub fn satellite_unified_encoding(raw: &Value) -> Option<&str> {
    raw.get("tiles")?
        .get("satellite")?
        .get("unified")?
        .get("encoding")?
        .as_str()
}

/// Read every T-935 binary block (spec §5) out of a terrain manifest.
///
/// Never fails: a manifest with no binary blocks — which is every manifest shipped before T-935.13
/// — yields [`ManifestBinary::default`], and [`ManifestBinary::is_empty`] reports it.
#[must_use]
pub fn parse_manifest_binary(raw: &Value) -> ManifestBinary {
    ManifestBinary {
        objects: block(raw.get("objects").and_then(|o| o.get("binary"))),
        dem_raw: block(raw.get("dem").and_then(|d| d.get("raw"))),
        labels: block(raw.get("labels")),
        water: block(raw.get("water")),
        buildings: block(raw.get("buildings")),
        satellite_unified_v2: satellite_unified_encoding(raw) == Some(SAT_UNIFIED_ENCODING_V2),
    }
}

/// One chunk-index cell (mirror of `WorldChunkCell`, `narrowCells`).
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkCell {
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub path: String,
    pub instance_count: Option<f64>,
}

/// Parse the manifest `objects` block. Returns `None` when the block is absent or is missing
/// `prefabsPath`/`chunksPath` (the `doLoadManifest` v2-export gate).
#[must_use]
pub fn parse_objects_manifest(raw: &Value) -> Option<ObjectsManifest> {
    let objects = raw.get("objects")?;
    let string_field = |k: &str| objects.get(k).and_then(Value::as_str).map(str::to_string);
    let prefabs_path = string_field("prefabsPath")?;
    let chunks_path = string_field("chunksPath")?;
    Some(ObjectsManifest {
        prefabs_path,
        chunks_path,
        chunk_size_m: objects
            .get("chunkSizeM")
            .and_then(Value::as_f64)
            .unwrap_or(DEFAULT_CHUNK_SIZE_M),
        roads_path: string_field("roadsPath"),
        density_path: string_field("densityPath"),
        regions_path: string_field("regionsPath"),
        instance_count: objects.get("instanceCount").and_then(Value::as_f64),
        prefab_count: objects.get("prefabCount").and_then(Value::as_f64),
        binary: block(objects.get("binary")),
    })
}

/// `narrowCells(indexRaw)` (`:361`) — the chunk-index `cells[]`. `None` when `cells` is not an
/// array (full-grid sweep mode). Keeps rows with numeric `cx`/`cy` and string `path`.
#[must_use]
pub fn narrow_cells(index_raw: &Value) -> Option<Vec<ChunkCell>> {
    let raw_cells = index_raw.get("cells")?.as_array()?;
    let mut cells = Vec::with_capacity(raw_cells.len());
    for c in raw_cells {
        let (Some(cx), Some(cy), Some(path)) = (
            c.get("cx").and_then(Value::as_f64),
            c.get("cy").and_then(Value::as_f64),
            c.get("path").and_then(Value::as_str),
        ) else {
            continue;
        };
        cells.push(ChunkCell {
            id: format!("{}_{}", cx as i64, cy as i64),
            cx,
            cy,
            path: path.to_string(),
            instance_count: c.get("instanceCount").and_then(Value::as_f64),
        });
    }
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_objects_block_with_defaults() {
        let raw = json!({ "objects": {
            "prefabsPath": "objects/prefabs.json.gz",
            "chunksPath": "objects/chunks",
            "roadsPath": "objects/roads.json.gz",
            "regionsPath": "objects/forest-regions.json.gz",
            "densityPath": "objects/density",
            "prefabCount": 391,
            "instanceCount": 508291
        }});
        let m = parse_objects_manifest(&raw).unwrap();
        assert_eq!(m.chunk_size_m, DEFAULT_CHUNK_SIZE_M); // omitted → 512
        assert_eq!(m.prefab_count, Some(391.0));
        assert_eq!(m.instance_count, Some(508291.0));
        assert_eq!(m.roads_path.as_deref(), Some("objects/roads.json.gz"));
    }

    #[test]
    fn gate_requires_prefabs_and_chunks_paths() {
        assert!(parse_objects_manifest(&json!({ "objects": { "prefabsPath": "p" } })).is_none());
        assert!(parse_objects_manifest(&json!({})).is_none());
    }

    #[test]
    fn narrow_cells_reads_index() {
        let raw = json!({ "cells": [
            { "cx": 10, "cy": 12, "path": "objects/chunks/10_12.json.gz", "instanceCount": 42 },
            { "cx": "x", "cy": 1, "path": "p" }   // non-numeric cx → dropped
        ]});
        let cells = narrow_cells(&raw).unwrap();
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].id, "10_12");
        assert_eq!(cells[0].instance_count, Some(42.0));
        assert!(narrow_cells(&json!({})).is_none()); // no cells → sweep mode
    }

    /* ───────────────────────────── T-935.1 binary blocks ───────────────────────────── */

    /// The committed everon manifest, compiled in. `include_str!` rather than a runtime read so a
    /// moved or deleted manifest is a build failure — a test that can silently not find its input
    /// is the signature defect this program exists to avoid.
    const EVERON_MANIFEST: &str =
        include_str!("../../../../packages/map-assets/everon/manifest.json");

    fn everon() -> Value {
        serde_json::from_str(EVERON_MANIFEST).expect("committed everon manifest is valid JSON")
    }

    /// ACCEPTANCE: the committed everon manifest parses exactly as it did before T-935.1 — every
    /// pre-existing field identical, and every new optional block absent.
    #[test]
    fn everon_manifest_parses_unchanged() {
        let raw = everon();
        let m = parse_objects_manifest(&raw).expect("everon has objects.prefabsPath/chunksPath");
        assert_eq!(m.prefabs_path, "objects/prefabs.json.gz");
        assert_eq!(m.chunks_path, "objects/chunks");
        assert_eq!(m.chunk_size_m, 512.0);
        assert_eq!(m.roads_path.as_deref(), Some("objects/roads.json.gz"));
        assert_eq!(m.density_path.as_deref(), Some("objects/density"));
        assert_eq!(
            m.regions_path.as_deref(),
            Some("objects/forest-regions.json.gz")
        );
        assert_eq!(m.instance_count, Some(1_216_066.0));
        assert_eq!(m.prefab_count, Some(1623.0));
        // The new field, and the whole of "parses exactly as today": no binary block is present.
        assert_eq!(m.binary, None);

        let b = parse_manifest_binary(&raw);
        assert!(b.is_empty(), "everon declares no binary blocks yet: {b:?}");
        assert_eq!(b, ManifestBinary::default());
        // The satellite container IS declared, at v1 — so the flag must read false, not "absent".
        assert_eq!(satellite_unified_encoding(&raw), Some("tbd-sat-v1"));
        assert!(!b.satellite_unified_v2);
    }

    /// The T-935.13 end state: every block present and read.
    #[test]
    fn every_binary_block_is_read_when_present() {
        let raw = json!({
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
            },
            "dem": { "raw": { "path": "dem/elevation.dem", "encoding": "tbde-v1" } },
            "labels": { "path": "locations/map_labels.rkyv", "encoding": "rkyv-map-labels-v1" },
            "water": {
                "vectors": "water/water_vectors.rkyv",
                "bathymetry": "water/bathymetry.tbd-bath",
                "encoding": "tbdb-v1"
            },
            "buildings": { "archive": "prefabs/building_blueprints.rkyv", "blas": "prefabs/blas" },
            "tiles": { "satellite": { "unified": { "encoding": "tbd-sat-v2" } } }
        });

        let b = parse_manifest_binary(&raw);
        assert!(!b.is_empty());
        let objects = b.objects.clone().expect("objects.binary");
        assert_eq!(objects.container, TBDC_CONTAINER);
        assert_eq!(objects.pod_bytes, 32);
        assert_eq!(objects.chunks, "objects/chunks/{cx}_{cy}.bin");
        assert_eq!(objects.type_inventory, "objects/type-inventory.rkyv");
        assert_eq!(b.dem_raw.expect("dem.raw").encoding, "tbde-v1");
        assert_eq!(b.labels.expect("labels").path, "locations/map_labels.rkyv");
        assert_eq!(
            b.water.expect("water").bathymetry,
            "water/bathymetry.tbd-bath"
        );
        assert_eq!(b.buildings.expect("buildings").blas, "prefabs/blas");
        assert!(b.satellite_unified_v2);

        // `objects.binary` also reaches callers through the objects manifest itself.
        let m = parse_objects_manifest(&raw).expect("objects block");
        assert_eq!(m.binary, Some(objects));
    }

    /// `matches_this_build` is the drift check: a manifest describing the operator's original
    /// 24-byte row, or a future container version, must be refused rather than read at the wrong
    /// stride.
    #[test]
    fn pod_shape_mismatch_is_detected() {
        let good = ObjectsBinaryBlock {
            container: TBDC_CONTAINER.to_string(),
            container_version: 1,
            pod: "ObjectInstancePod".to_string(),
            pod_bytes: 32,
            ..ObjectsBinaryBlock::default()
        };
        assert!(good.matches_this_build());

        let mut narrow = good.clone();
        narrow.pod_bytes = 24;
        assert!(
            !narrow.matches_this_build(),
            "24-byte rows are not this POD"
        );

        let mut future = good.clone();
        future.container_version = 2;
        assert!(!future.matches_this_build());

        let mut other = good.clone();
        other.container = "TBDX".to_string();
        assert!(!other.matches_this_build());

        let mut renamed = good;
        renamed.pod = "ObjectInstancePodV2".to_string();
        assert!(!renamed.matches_this_build());
    }

    /// Partial blocks are legal (every field is `serde(default)`) — waves 2-5 ship a
    /// half-converted manifest on purpose — but a field of the wrong *type* falls back to the JSON
    /// path rather than being read as garbage.
    #[test]
    fn partial_blocks_default_and_malformed_blocks_fall_back() {
        let partial = parse_manifest_binary(&json!({
            "dem": { "raw": { "path": "dem/elevation.dem" } }
        }));
        let raw_block = partial
            .dem_raw
            .expect("dem.raw with a missing field still parses");
        assert_eq!(raw_block.path, "dem/elevation.dem");
        assert_eq!(raw_block.encoding, "", "missing field defaults, not errors");
        assert!(partial.objects.is_none());

        let malformed = parse_manifest_binary(&json!({
            "objects": { "binary": { "podBytes": "thirty-two" } },
            "labels": [1, 2, 3]
        }));
        assert!(malformed.objects.is_none(), "wrong type → JSON fallback");
        assert!(malformed.labels.is_none());
        assert!(malformed.is_empty());
    }

    /// A manifest with no `tiles` block at all must not be mistaken for one declaring v2.
    #[test]
    fn satellite_encoding_absent_is_not_v2() {
        assert_eq!(satellite_unified_encoding(&json!({})), None);
        assert_eq!(
            satellite_unified_encoding(&json!({ "tiles": { "satellite": {} } })),
            None
        );
        assert!(!parse_manifest_binary(&json!({})).satellite_unified_v2);
    }
}
