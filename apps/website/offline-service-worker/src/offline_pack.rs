//! The terrain offline pack: every `/map-assets` file the map needs with no network.
//!
//! **Role:** turns a served terrain manifest and its map tile index into the list of files the
//! page downloads into the map-asset cache — the manifest, the elevation model, the unified
//! satellite mosaic and every cartographic map tile — and says whether that list is complete.
//! **Position:** the page fetches `manifest.json` and `tiles/map/index.json` of a terrain
//! ([`manifest_url`], [`tile_index_url`]), calls [`terrain_pack`] and downloads each
//! [`PackEntry`]; the worker then serves those entries from the cache.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a missing, empty or partial tile index makes the pack
//! [`PackCompleteness::Incomplete`], never complete on a smaller set; satellite XYZ tiles are
//! never listed; a malformed manifest or index is an [`OfflinePackError`], never an empty pack.
//! @contract terrain-manifest.schema.json#/properties/tiles

use serde::{Deserialize, Serialize};

/// The `schemaVersion` of the map tile index this crate reads.
pub const MAP_TILE_INDEX_SCHEMA_VERSION: u32 = 1;

/// The map tile index: every cartographic tile file of one terrain, written next to the pyramid
/// as `tiles/map/index.json`.
/// @contract map-tile-index.schema.json#
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapTileIndex {
    /// Always [`MAP_TILE_INDEX_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// The terrain the tiles belong to; equals the manifest's `terrainId`.
    pub terrain_id: String,
    /// Every tile file of the pyramid.
    pub tiles: Vec<MapTileIndexEntry>,
}

/// One tile file of the map tile index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapTileIndexEntry {
    /// Zoom level.
    pub z: u32,
    /// Column, `0 <= x < 2^z`.
    pub x: u32,
    /// Row in the XYZ disk layout, `0 <= y < 2^z`.
    pub y: u32,
    /// File size in bytes.
    pub bytes: u64,
}

/// What one pack entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackEntryKind {
    /// The terrain manifest, `manifest.json`.
    TerrainManifest,
    /// The 16-bit elevation model image.
    ElevationModel,
    /// The unified satellite mosaic (`.tbd-sat`), read by `Range` requests.
    SatelliteMosaic,
    /// One cartographic map tile.
    MapTile,
}

/// One file of the pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEntry {
    /// The same-origin path the file is fetched and cached under.
    pub url: String,
    /// What the file is.
    pub kind: PackEntryKind,
    /// The size the manifest or index declares, when it declares one.
    pub expected_bytes: Option<u64>,
}

/// Why a pack is not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompleteReason {
    /// The server has no `tiles/map/index.json` for the terrain.
    TileIndexMissing,
    /// The tile index lists no tile.
    TileIndexEmpty,
    /// The tile index lists no tile at these zoom levels of the manifest's range.
    ZoomLevelsMissing(Vec<u32>),
}

/// Whether the pack lists every file the terrain needs offline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackCompleteness {
    /// Every file is listed.
    Complete,
    /// Some files cannot be listed; the page reports the pack incomplete and never ready.
    Incomplete(IncompleteReason),
}

/// The offline pack of one terrain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflinePack {
    /// The terrain's manifest `terrainId`.
    pub terrain_id: String,
    /// Every file to download, the manifest first.
    pub entries: Vec<PackEntry>,
    /// Whether `entries` is the whole pack.
    pub completeness: PackCompleteness,
}

impl OfflinePack {
    /// Sum of every declared entry size; entries without a declared size count zero.
    pub fn declared_bytes(&self) -> u64 {
        self.entries
            .iter()
            .filter_map(|entry| entry.expected_bytes)
            .sum()
    }

    /// Whether the pack lists every file.
    pub fn is_complete(&self) -> bool {
        self.completeness == PackCompleteness::Complete
    }
}

/// Why a pack cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflinePackError {
    /// The manifest is not JSON of the expected shape.
    ManifestUnreadable(String),
    /// The tile index is not JSON of the expected shape.
    TileIndexUnreadable(String),
    /// The manifest lacks a section the pack needs, named by its JSON path.
    ManifestSectionMissing(&'static str),
    /// The manifest's `terrainId` is not a lower-case slug.
    InvalidTerrainId(String),
    /// The manifest declares a stub elevation model (`widthPx` or `heightPx` of 0).
    StubElevationModel,
    /// The map `urlTemplate` lacks one of `{z}`, `{x}`, `{y}`.
    MapUrlTemplateInvalid(String),
    /// The tile index belongs to another terrain.
    TerrainMismatch {
        /// The manifest's `terrainId`.
        manifest: String,
        /// The index's `terrainId`.
        index: String,
    },
    /// The tile index has another `schemaVersion`.
    UnsupportedTileIndexVersion(u32),
    /// A tile lies outside the manifest's zoom range or its level's `2^z` grid.
    TileOutsidePyramid(MapTileIndexEntry),
}

/// The served path of a terrain's manifest under `map_assets_root` (such as `/map-assets`).
pub fn manifest_url(map_assets_root: &str, terrain_id: &str) -> String {
    format!(
        "{}/{terrain_id}/manifest.json",
        map_assets_root.trim_end_matches('/')
    )
}

/// The served path of a terrain's map tile index under `map_assets_root`.
pub fn tile_index_url(map_assets_root: &str, terrain_id: &str) -> String {
    format!(
        "{}/{terrain_id}/tiles/map/index.json",
        map_assets_root.trim_end_matches('/')
    )
}

/// The offline pack of the terrain whose served manifest is `manifest_json`, with its map tile
/// index `tile_index_json` (`None` when the server has none).
pub fn terrain_pack(
    map_assets_root: &str,
    manifest_json: &str,
    tile_index_json: Option<&str>,
) -> Result<OfflinePack, OfflinePackError> {
    let manifest: ManifestProjection = serde_json::from_str(manifest_json)
        .map_err(|error| OfflinePackError::ManifestUnreadable(error.to_string()))?;
    let terrain_id = manifest.terrain_id.clone();
    if !is_slug(&terrain_id) {
        return Err(OfflinePackError::InvalidTerrainId(terrain_id));
    }
    let terrain_root = format!("{}/{terrain_id}", map_assets_root.trim_end_matches('/'));
    let dem = manifest
        .dem
        .ok_or(OfflinePackError::ManifestSectionMissing("dem"))?;
    if dem.width_px == 0 || dem.height_px == 0 {
        return Err(OfflinePackError::StubElevationModel);
    }
    let tiles = manifest
        .tiles
        .ok_or(OfflinePackError::ManifestSectionMissing("tiles"))?;
    let mosaic = tiles
        .satellite
        .and_then(|satellite| satellite.unified)
        .ok_or(OfflinePackError::ManifestSectionMissing(
            "tiles.satellite.unified",
        ))?;
    let map_template = tiles
        .map
        .ok_or(OfflinePackError::ManifestSectionMissing("tiles.map"))?
        .url_template;
    if !["{z}", "{x}", "{y}"]
        .iter()
        .all(|placeholder| map_template.contains(placeholder))
    {
        return Err(OfflinePackError::MapUrlTemplateInvalid(map_template));
    }
    let min_zoom = tiles
        .min_zoom
        .ok_or(OfflinePackError::ManifestSectionMissing("tiles.minZoom"))?;
    let max_zoom = tiles
        .max_zoom
        .ok_or(OfflinePackError::ManifestSectionMissing("tiles.maxZoom"))?;

    let mut entries = vec![
        PackEntry {
            url: format!("{terrain_root}/manifest.json"),
            kind: PackEntryKind::TerrainManifest,
            expected_bytes: None,
        },
        PackEntry {
            url: resolve_path(&terrain_root, &dem.path),
            kind: PackEntryKind::ElevationModel,
            expected_bytes: None,
        },
        PackEntry {
            url: resolve_path(&terrain_root, &mosaic.url),
            kind: PackEntryKind::SatelliteMosaic,
            expected_bytes: Some(mosaic.bytes),
        },
    ];
    let Some(index_json) = tile_index_json else {
        return Ok(incomplete(
            terrain_id,
            entries,
            IncompleteReason::TileIndexMissing,
        ));
    };
    let index: MapTileIndex = serde_json::from_str(index_json)
        .map_err(|error| OfflinePackError::TileIndexUnreadable(error.to_string()))?;
    if index.schema_version != MAP_TILE_INDEX_SCHEMA_VERSION {
        return Err(OfflinePackError::UnsupportedTileIndexVersion(
            index.schema_version,
        ));
    }
    if index.terrain_id != terrain_id {
        return Err(OfflinePackError::TerrainMismatch {
            manifest: terrain_id,
            index: index.terrain_id,
        });
    }
    for tile in &index.tiles {
        let inside = (min_zoom..=max_zoom).contains(&tile.z)
            && tile.z < 32
            && u64::from(tile.x) < 1u64 << tile.z
            && u64::from(tile.y) < 1u64 << tile.z;
        if !inside {
            return Err(OfflinePackError::TileOutsidePyramid(*tile));
        }
    }
    if index.tiles.is_empty() {
        return Ok(incomplete(
            terrain_id,
            entries,
            IncompleteReason::TileIndexEmpty,
        ));
    }
    let missing_levels: Vec<u32> = (min_zoom..=max_zoom)
        .filter(|level| !index.tiles.iter().any(|tile| tile.z == *level))
        .collect();
    entries.extend(index.tiles.iter().map(|tile| {
        PackEntry {
            url: resolve_path(
                &terrain_root,
                &map_template
                    .replace("{z}", &tile.z.to_string())
                    .replace("{x}", &tile.x.to_string())
                    .replace("{y}", &tile.y.to_string()),
            ),
            kind: PackEntryKind::MapTile,
            expected_bytes: Some(tile.bytes),
        }
    }));
    if !missing_levels.is_empty() {
        return Ok(incomplete(
            terrain_id,
            entries,
            IncompleteReason::ZoomLevelsMissing(missing_levels),
        ));
    }
    Ok(OfflinePack {
        terrain_id,
        entries,
        completeness: PackCompleteness::Complete,
    })
}

fn incomplete(
    terrain_id: String,
    entries: Vec<PackEntry>,
    reason: IncompleteReason,
) -> OfflinePack {
    OfflinePack {
        terrain_id,
        entries,
        completeness: PackCompleteness::Incomplete(reason),
    }
}

/// An absolute path stays as served; a manifest-relative path hangs off the terrain folder.
fn resolve_path(terrain_root: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("{terrain_root}/{path}")
    }
}

fn is_slug(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

/// The manifest fields the pack reads; every other field is ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestProjection {
    terrain_id: String,
    dem: Option<ElevationModelProjection>,
    tiles: Option<TilesProjection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ElevationModelProjection {
    path: String,
    width_px: u64,
    height_px: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TilesProjection {
    min_zoom: Option<u32>,
    max_zoom: Option<u32>,
    satellite: Option<SatelliteProjection>,
    map: Option<MapPyramidProjection>,
}

#[derive(Deserialize)]
struct SatelliteProjection {
    unified: Option<SatelliteMosaicProjection>,
}

#[derive(Deserialize)]
struct SatelliteMosaicProjection {
    url: String,
    bytes: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapPyramidProjection {
    url_template: String,
}

#[cfg(test)]
#[path = "tests/offline_pack.rs"]
mod tests;
