//! The terrain manifest: where a terrain's served map files are and how they are encoded.
//!
//! **Role:** narrows a terrain's `manifest.json` into the blocks the loaders read: the objects
//! block ([`parse_objects_manifest`]), the binary blocks of the chunks, prefabs, roads, regions,
//! elevation, labels, water and buildings ([`parse_manifest_binary`]), and the chunk index cells
//! ([`narrow_cells`]).
//! **Position:** under `world_chunks`; read by the map engine's world store, scheduler, world
//! loader, elevation, label and water loaders and by the developer tools' manifest verification.
//! **Signals & state:** none; pure functions over JSON values.
//! **Invariants:** a missing optional block is `None` or empty, never an error; the objects block
//! needs `prefabsPath` and `chunksPath`; a chunk cell keeps only rows with numeric `cx`, `cy` and
//! a string `path`, its identifier being the chunk grid's `cx_cy`.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::chunk_id::ChunkId;

use world_file_formats::containers::header::CONTAINER_VERSION;
use world_file_formats::pod::instance::POD_BYTES;
use world_file_formats::pod::instance::POD_NAME;

/// Default chunk edge in meters when the manifest omits `chunkSizeM` (`DEFAULT_CHUNK_SIZE_M`).
pub const DEFAULT_CHUNK_SIZE_M: f64 = 512.0;

/// The `container` name a `TBDC` `objects.binary` block declares.
pub const TBDC_CONTAINER: &str = "TBDC";

/// Canonical sat unified encoding v2 value.
pub const SAT_UNIFIED_ENCODING_V2: &str = "tbd-sat-v2";

/// The manifest `objects` fields this parser consumes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectsManifest {
    /// Prefab catalogue file relative to the terrain's asset folder (`objects.prefabsPath`, such
    /// as `objects/prefabs.json.gz`); required.
    pub prefabs_path: String,

    /// Folder of the chunk files and their `manifest.json` index, relative to the terrain's asset
    /// folder (`objects.chunksPath`, such as `objects/chunks`); required.
    pub chunks_path: String,

    /// Edge of one square chunk in metres (`objects.chunkSizeM`); [`DEFAULT_CHUNK_SIZE_M`] when
    /// absent.
    pub chunk_size_m: f64,

    /// Road network JSON file relative to the terrain's asset folder (`objects.roadsPath`); `None`
    /// when absent.
    pub roads_path: Option<String>,

    /// Folder of the per-chunk tree density grids (`objects.densityPath`, such as
    /// `objects/density`); `None` when absent.
    pub density_path: Option<String>,

    /// Land-cover regions file (`objects.regionsPath`, such as `objects/forest-regions.json.gz`);
    /// `None` when absent.
    pub regions_path: Option<String>,

    /// Declared total of placed objects over every chunk (`objects.instanceCount`); `None` when
    /// absent.
    pub instance_count: Option<f64>,

    /// Declared number of prefabs in the catalogue (`objects.prefabCount`); `None` when absent.
    pub prefab_count: Option<f64>,

    /// The `objects.binary` block naming the binary twins of these files; `None` when absent or
    /// malformed.
    pub binary: Option<ObjectsBinaryBlock>,
}

/// `objects.binary` — the chunk container and the four Tier-2 archives that replace the `objects/*.json.gz` set.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ObjectsBinaryBlock {
    /// Version of this block's own layout (`schemaVersion`, such as `1.0.0`); empty when absent.
    pub schema_version: String,

    /// `"TBDC"`.
    pub container: String,

    /// `TBDC` container format version (`containerVersion`); [`Self::matches_this_build`] needs
    /// it to equal [`CONTAINER_VERSION`].
    pub container_version: u16,

    /// `"ObjectInstancePod"`.
    pub pod: String,

    /// Bytes of one object row (`podBytes`); [`Self::matches_this_build`] needs it to equal
    /// [`POD_BYTES`].
    pub pod_bytes: u32,

    /// Chunk path template, `objects/chunks/{cx}_{cy}.bin`.
    pub chunks: String,

    /// Prefab catalogue archive path (`prefabs`, such as `objects/prefabs.rkyv`); empty when
    /// absent, and the loader then reads the JSON catalogue.
    pub prefabs: String,

    /// Road network archive path (`roads`, such as `roads/road_network.rkyv`); empty when absent,
    /// and the loader then reads the JSON roads.
    pub roads: String,

    /// Land-cover regions archive path (`regions`, such as `objects/forest-regions.rkyv`); empty
    /// when absent, and the loader then reads the JSON regions.
    pub regions: String,

    /// Prefab type inventory archive path (`typeInventory`, such as
    /// `objects/type-inventory.rkyv`); empty when absent.
    pub type_inventory: String,
}

impl ObjectsBinaryBlock {
    /// Does this block describe the container and row shape **this build** actually implements?.
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ManifestBinary {
    /// The `objects.binary` block; `None` when absent or not decodable.
    pub objects: Option<ObjectsBinaryBlock>,

    /// The `dem.raw` block, the `TBDE` elevation raster; `None` when absent or not decodable.
    pub dem_raw: Option<DemRawBlock>,

    /// The top-level `labels` block, the map label archive; `None` when absent or not decodable.
    pub labels: Option<LabelsBlock>,

    /// The top-level `water` block, the water vectors and bathymetry raster; `None` when absent
    /// or not decodable.
    pub water: Option<WaterBlock>,

    /// The top-level `buildings` block, the building blueprint archive and its sidecar folder;
    /// `None` when absent or not decodable.
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

/// Never fails: a manifest with no binary blocks yields [`ManifestBinary::default`], and [`ManifestBinary::is_empty`] reports it.
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
    /// The cell's `cx_cy` identifier, spelled from `cx` and `cy`.
    pub id: ChunkId,

    /// Chunk grid column along world X (`cx`): the chunk's west edge divided by the chunk edge.
    pub cx: f64,

    /// Chunk grid row along map Y (`cy`), counted south to north.
    pub cy: f64,

    /// The chunk file the index lists (`path`, such as `objects/chunks/10_12.json.gz`).
    pub path: String,

    /// Objects the chunk holds (`instanceCount`); `None` when absent.
    pub instance_count: Option<f64>,
}

/// Parse the manifest `objects` block. Returns `None` when the block is absent or is missing `prefabsPath`/`chunksPath` (the `doLoadManifest` v2-export gate).
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

/// `narrowCells(indexRaw)` (`:361`) — the chunk-index `cells[]`. `None` when `cells` is not an array (full-grid sweep mode). Keeps rows with numeric `cx`/`cy` and string `path`.
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
            id: ChunkId::of_cell(cx as i64, cy as i64),
            cx,
            cy,
            path: path.to_string(),
            instance_count: c.get("instanceCount").and_then(Value::as_f64),
        });
    }
    Some(cells)
}

#[cfg(test)]
#[path = "tests/terrain_manifest_tests.rs"]
mod tests;
