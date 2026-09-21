//! `locations/map_labels.rkyv`: the binary twin of the three cartographic label files.
//!
//! `map labels-rkyv` reads `locations.json`, `height-labels.json` and `road-names.json` and writes
//! one `MapLabelsArchive`. Dual emission, per spec §7 wave 2: the JSON files stay the source of
//! truth and stay on disk, the archive is derived from them, and the SPA only reads it once
//! The terrain manifest carries a `labels` block naming it.
//!
//! # Why the road lane needs a fourth file
//!
//! `road-names.json` is a *curated route list* — name plus segment ids — while the archive's
//! `RoadNameLabel` is a **baked placement**: anchor, angle, class byte. Turning one into the other
//! needs the centreline geometry those segment ids point at, which lives in
//! `objects/roads.json.gz`. That file is read exactly the way the SPA reads it
//! (`bytes_to_json`/`parse_roads_payload` — the same chain behind `WorldStore::load_roads_gz`), so
//! the anchors the emitter bakes are the anchors `place_road_labels` would have produced in the
//! browser. The three JSON files remain the *sources*; roads.json.gz is the geometry they join to.
//!
//! # What is refused rather than approximated
//!
//! * A curated `minDeckZoom` the wire's class byte cannot express — see
//!   `road_names_to_archive`.
//! * `road-names.json` present without `objects/roads.json.gz`: the alternative is an archive with
//!   a silently empty road lane that a loader would trust.
//! * An archive with nothing in any lane (`refuse_empty_write`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use website_map_engine::io::archives::codec::access_checked;
use website_map_engine::io::archives::codec::to_bytes;
use website_map_engine::io::archives::labels::MapLabelsArchive;
use website_map_engine::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use website_map_engine::streaming::loaders::store::bytes_to_json;
use website_map_engine::world::environment::locations::route_labels::parse_road_names_json;
use website_map_engine::world::environment::locations::route_labels::road_names_to_archive;
use website_map_engine::world::environment::locations::towns::height_labels_to_archive;
use website_map_engine::world::environment::locations::towns::parse_height_labels_json;
use website_map_engine::world::environment::locations::towns::parse_locations_json;
use website_map_engine::world::environment::locations::towns::towns_to_archive;
use website_map_engine::world::terrain::roads::network::parse_roads_payload;

use crate::browser_testing::server::repo_root;
use crate::repository_layout;

/// Terrain-relative source paths.
pub const LOCATIONS_JSON: &str = "locations.json";
/// See [`LOCATIONS_JSON`].
pub const HEIGHT_LABELS_JSON: &str = "height-labels.json";
/// See [`LOCATIONS_JSON`].
pub const ROAD_NAMES_JSON: &str = "road-names.json";
/// The centreline geometry the curated road names join to.
pub const ROADS_GZ: &str = "objects/roads.json.gz";
/// Terrain-relative output path (spec §4).
pub const MAP_LABELS_RKYV: &str = "locations/map_labels.rkyv";

fn read_to_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

/// Build the archive for one terrain directory.
///
/// # Errors
/// When a required source is missing or malformed, or when a curated road entry's visibility floor
/// cannot be encoded — see the module docs.
pub fn build_map_labels_archive(terrain_dir: &Path) -> Result<MapLabelsArchive> {
    let locations = parse_locations_json(&read_to_string(&terrain_dir.join(LOCATIONS_JSON))?)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let heights = parse_height_labels_json(&read_to_string(&terrain_dir.join(HEIGHT_LABELS_JSON))?)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let names_path = terrain_dir.join(ROAD_NAMES_JSON);
    let roads_path = terrain_dir.join(ROADS_GZ);
    let road_names = if names_path.exists() {
        if !roads_path.exists() {
            bail!(
                "{} exists but {} does not — the curated names cannot be anchored without the \
                 centreline geometry, and an empty road lane would be trusted by the loader",
                names_path.display(),
                roads_path.display()
            );
        }
        let names = parse_road_names_json(&read_to_string(&names_path)?)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        let raw =
            std::fs::read(&roads_path).with_context(|| format!("read {}", roads_path.display()))?;
        let doc =
            bytes_to_json(&raw).map_err(|e| anyhow::anyhow!("{roads_path:?} decode: {e:?}"))?;
        let segments = parse_roads_payload(&doc);
        if segments.is_empty() {
            bail!("{} decoded to 0 road segments", roads_path.display());
        }
        road_names_to_archive(&names, &segments).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        Vec::new()
    };

    Ok(MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        towns: towns_to_archive(&locations),
        height_labels: height_labels_to_archive(&heights),
        road_names,
    })
}

/// Serialise, **re-read through the validating reader**, then write. Returns the file size.
///
/// The read-back is not ceremony: `access_checked` is the only thing the SPA will ever use on this
/// file, so an archive that cannot survive it is a broken file whether or not `to_bytes` returned
/// `Ok`, and the emitter is the last place that can say so cheaply.
///
/// # Errors
/// When serialisation, validation or the write fails.
pub fn write_map_labels_rkyv(path: &Path, archive: &MapLabelsArchive) -> Result<usize> {
    let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    access_checked::<MapLabelsArchive>(&bytes)
        .map_err(|e| anyhow::anyhow!("emitted archive fails its own validating read: {e}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(bytes.len())
}

/// Resolve the `--terrain` value: a directory path when it names one, else a terrain id under
/// `assets_v2/terrains`.
#[must_use]
pub fn terrain_dir(terrain: &str) -> PathBuf {
    let as_path = PathBuf::from(terrain);
    if as_path.is_dir() {
        return as_path;
    }
    repository_layout::terrain_dir(&repo_root(), terrain)
}

/// `map labels-rkyv --terrain <id|dir>`.
///
/// # Errors
/// When the terrain directory is missing, or any step of the build/write fails.
pub fn emit_map_labels(terrain: &str) -> Result<u8> {
    let dir = terrain_dir(terrain);
    if !dir.is_dir() {
        eprintln!("labels-rkyv: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let archive = build_map_labels_archive(&dir)?;
    super::refuse_empty_write(
        "labels-rkyv",
        archive.towns.is_empty()
            && archive.height_labels.is_empty()
            && archive.road_names.is_empty(),
        "no towns, height labels or road names — refusing to write an empty map_labels.rkyv",
    )?;
    let out = dir.join(MAP_LABELS_RKYV);
    let bytes = write_map_labels_rkyv(&out, &archive)?;
    println!(
        "labels-rkyv: {} towns + {} height labels + {} road names → {} ({bytes} bytes)",
        archive.towns.len(),
        archive.height_labels.len(),
        archive.road_names.len(),
        out.display()
    );
    Ok(0)
}

#[cfg(test)]
#[path = "tests/map_label_archives/tests.rs"]
mod tests;
