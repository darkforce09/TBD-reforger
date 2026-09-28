//! `cargo xtask map tile-index --terrain <terrain>`: the map tile index of a terrain's
//! cartographic tile pyramid.
//!
//! **Role:** lists every tile file of the pyramid under the terrain's `tiles.map.path` and writes
//! that list as `index.json` next to the pyramid, where `/map-assets/<terrain>/tiles/map/index.json`
//! serves it to the page that downloads the offline pack.
//! **Position:** reads `assets_v2/terrains/<terrain>/manifest.json` and the pyramid directory it
//! names; the API's and the gate server's `/map-assets` mounts serve the written file as a static
//! file; the offline service worker crate's `MapTileIndex` reads it.
//! **Signals & state:** none; one directory walk and one file write per run.
//! **Invariants:** tiles are sorted by (z, x, y) and each appears once; a tile outside the
//! manifest's zoom range or its level's `2^z` grid refuses the run instead of being written; a
//! missing or empty pyramid refuses with exit 2 and writes nothing, so an index never stands in
//! for tiles that are not on disk. The index is local build output and is gitignored with the
//! pyramid.
//! @contract map-tile-index.schema.json#

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use developer_tools::repository_layout::terrain_dir;
use serde::{Deserialize, Serialize};

use crate::core::repository_root::find_repo_root;

/// The file name of the index inside the pyramid directory.
pub(crate) const TILE_INDEX_FILE_NAME: &str = "index.json";

/// The `schemaVersion` this writer produces.
pub(crate) const TILE_INDEX_SCHEMA_VERSION: u32 = 1;

/// The map tile index document (`map-tile-index.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MapTileIndex {
    pub(crate) schema_version: u32,
    pub(crate) terrain_id: String,
    pub(crate) tiles: Vec<MapTileIndexEntry>,
}

/// One tile file: zoom, column, row and size in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct MapTileIndexEntry {
    pub(crate) z: u32,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) bytes: u64,
}

/// The pyramid a manifest declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MapPyramid {
    pub(crate) terrain_id: String,
    pub(crate) directory: PathBuf,
    pub(crate) tile_extension: String,
    pub(crate) min_zoom: u32,
    pub(crate) max_zoom: u32,
}

/// Why no index is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TileIndexError {
    /// The manifest is missing or not JSON of the expected shape.
    ManifestUnreadable(String),
    /// The map `urlTemplate` has no `{y}.<extension>` file name.
    UrlTemplateWithoutExtension(String),
    /// The pyramid directory does not exist.
    PyramidMissing(PathBuf),
    /// The pyramid directory holds no tile file.
    PyramidEmpty(PathBuf),
    /// A tile file lies outside the zoom range or its level's grid.
    TileOutsidePyramid { z: u32, x: u32, y: u32 },
}

impl TileIndexError {
    /// The process exit code: 2 when the pyramid is absent, 1 otherwise.
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::PyramidMissing(_) | Self::PyramidEmpty(_) => 2,
            _ => 1,
        }
    }
}

impl std::fmt::Display for TileIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestUnreadable(reason) => write!(f, "terrain manifest unreadable: {reason}"),
            Self::UrlTemplateWithoutExtension(template) => {
                write!(
                    f,
                    "map urlTemplate {template:?} names no `{{y}}.<extension>` file"
                )
            }
            Self::PyramidMissing(path) => {
                write!(
                    f,
                    "tile pyramid missing: {} (build the map tiles first)",
                    path.display()
                )
            }
            Self::PyramidEmpty(path) => write!(f, "tile pyramid holds no tile: {}", path.display()),
            Self::TileOutsidePyramid { z, x, y } => {
                write!(
                    f,
                    "tile {z}/{x}/{y} lies outside the manifest's zoom range or grid"
                )
            }
        }
    }
}

/// The parsed command line.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TileIndexArgs {
    /// `--terrain <terrain>`.
    Terrain(String),
    /// No terrain, or `--terrain` without a value.
    Usage,
    /// An argument the command does not take.
    Unknown(String),
}

/// Parses `--terrain <terrain>`.
pub(crate) fn parse_args(args: &[String]) -> TileIndexArgs {
    let mut terrain = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--terrain" => match iter.next() {
                Some(value) if !value.starts_with("--") => terrain = Some(value.clone()),
                _ => return TileIndexArgs::Usage,
            },
            other => return TileIndexArgs::Unknown(other.to_owned()),
        }
    }
    terrain.map_or(TileIndexArgs::Usage, TileIndexArgs::Terrain)
}

/// Entry for `cargo xtask map tile-index …`.
pub fn run(args: &[String]) -> Result<u8> {
    run_with_root(&find_repo_root()?, args)
}

/// [`run`] against an explicit checkout root.
pub fn run_with_root(root: &Path, args: &[String]) -> Result<u8> {
    let terrain = match parse_args(args) {
        TileIndexArgs::Terrain(terrain) => terrain,
        TileIndexArgs::Usage => {
            eprintln!("usage: cargo xtask map tile-index --terrain <terrain>");
            return Ok(1);
        }
        TileIndexArgs::Unknown(arg) => {
            eprintln!("tile-index: unknown arg {arg}");
            return Ok(1);
        }
    };
    let terrain_root = terrain_dir(root, &terrain);
    let outcome = read_pyramid(&terrain_root).and_then(|pyramid| {
        let index = build_index(&pyramid)?;
        Ok((pyramid, index))
    });
    let (pyramid, index) = match outcome {
        Ok(found) => found,
        Err(error) => {
            eprintln!("tile-index: {terrain}: {error}");
            return Ok(error.exit_code());
        }
    };
    let path = pyramid.directory.join(TILE_INDEX_FILE_NAME);
    let mut text = serde_json::to_string(&index).context("serialise the map tile index")?;
    text.push('\n');
    fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    let total_bytes: u64 = index.tiles.iter().map(|tile| tile.bytes).sum();
    println!(
        "tile-index: wrote {} ({} tiles, {total_bytes} bytes, zoom {}..={})",
        path.display(),
        index.tiles.len(),
        pyramid.min_zoom,
        pyramid.max_zoom
    );
    let missing = missing_zoom_levels(&pyramid, &index);
    if !missing.is_empty() {
        eprintln!(
            "tile-index: warning: no tile at zoom levels {missing:?}; the offline pack stays incomplete"
        );
    }
    Ok(0)
}

/// The pyramid the terrain manifest under `terrain_root` declares.
pub(crate) fn read_pyramid(terrain_root: &Path) -> Result<MapPyramid, TileIndexError> {
    let manifest_path = terrain_root.join("manifest.json");
    let text = fs::read_to_string(&manifest_path).map_err(|error| {
        TileIndexError::ManifestUnreadable(format!("{}: {error}", manifest_path.display()))
    })?;
    let manifest: ManifestProjection = serde_json::from_str(&text)
        .map_err(|error| TileIndexError::ManifestUnreadable(error.to_string()))?;
    let template = &manifest.tiles.map.url_template;
    let tile_extension = template
        .rsplit('/')
        .next()
        .and_then(|file_name| file_name.strip_prefix("{y}."))
        .filter(|extension| !extension.is_empty() && !extension.contains(['/', '{', '}']))
        .ok_or_else(|| TileIndexError::UrlTemplateWithoutExtension(template.clone()))?
        .to_owned();
    Ok(MapPyramid {
        terrain_id: manifest.terrain_id,
        directory: terrain_root.join(&manifest.tiles.map.path),
        tile_extension,
        min_zoom: manifest.tiles.min_zoom,
        max_zoom: manifest.tiles.max_zoom,
    })
}

/// Every `<z>/<x>/<y>.<extension>` file of the pyramid, sorted; other files and directories
/// (such as a whole-map preview at the pyramid root) are not tiles and are skipped.
pub(crate) fn build_index(pyramid: &MapPyramid) -> Result<MapTileIndex, TileIndexError> {
    if !pyramid.directory.is_dir() {
        return Err(TileIndexError::PyramidMissing(pyramid.directory.clone()));
    }
    let suffix = format!(".{}", pyramid.tile_extension);
    let mut tiles = Vec::new();
    for (z, zoom_dir) in numbered_children(&pyramid.directory, "", true) {
        for (x, column_dir) in numbered_children(&zoom_dir, "", true) {
            for (y, tile_path) in numbered_children(&column_dir, &suffix, false) {
                let inside = (pyramid.min_zoom..=pyramid.max_zoom).contains(&z)
                    && z < 32
                    && u64::from(x) < 1u64 << z
                    && u64::from(y) < 1u64 << z;
                if !inside {
                    return Err(TileIndexError::TileOutsidePyramid { z, x, y });
                }
                let bytes = fs::metadata(&tile_path).map(|meta| meta.len()).unwrap_or(0);
                tiles.push(MapTileIndexEntry { z, x, y, bytes });
            }
        }
    }
    if tiles.is_empty() {
        return Err(TileIndexError::PyramidEmpty(pyramid.directory.clone()));
    }
    tiles.sort_unstable();
    Ok(MapTileIndex {
        schema_version: TILE_INDEX_SCHEMA_VERSION,
        terrain_id: pyramid.terrain_id.clone(),
        tiles,
    })
}

/// The manifest's zoom levels with no tile in `index`.
pub(crate) fn missing_zoom_levels(pyramid: &MapPyramid, index: &MapTileIndex) -> Vec<u32> {
    (pyramid.min_zoom..=pyramid.max_zoom)
        .filter(|level| !index.tiles.iter().any(|tile| tile.z == *level))
        .collect()
}

/// The children of `dir` whose name is `<number><suffix>`: directories when `want_dirs`, files
/// otherwise. An unreadable directory has no children.
fn numbered_children(dir: &Path, suffix: &str, want_dirs: bool) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() != want_dirs {
                return None;
            }
            let name = entry.file_name().into_string().ok()?;
            let stem = name.strip_suffix(suffix)?;
            if stem.is_empty() || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            Some((stem.parse().ok()?, path))
        })
        .collect()
}

/// The manifest fields the writer reads; every other field is ignored.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestProjection {
    terrain_id: String,
    tiles: TilesProjection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TilesProjection {
    min_zoom: u32,
    max_zoom: u32,
    map: MapPyramidProjection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapPyramidProjection {
    path: String,
    url_template: String,
}

#[cfg(test)]
#[path = "tests/tile_index/tests.rs"]
mod tests;
