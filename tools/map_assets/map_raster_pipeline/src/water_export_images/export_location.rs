//! Where a Workbench water export's files sit, and where its images go.
//!
//! **Role:** [`locate_water_export`] finds the water folder under the export folder, picks its
//! metadata file and the first present candidate of each grid and vector file
//! ([`WaterExportFiles`]); [`output_prefix`] names the images; [`create_output_directory`] makes
//! the folder they are written to.
//! **Position:** the first step of the lane's `run` (`water_export_images.rs`); its paths feed
//! the grid decoder and the vector rasterizer.
//! **Signals & state:** none held; reads the file system's existence answers only, and
//! [`create_output_directory`] creates folders.
//! **Invariants:** candidates are tried in one fixed order and the first that exists wins; the
//! water folder is the first candidate folder holding a metadata file or the mask grid, and the
//! export folder itself when none does; an export with no metadata file is refused, naming the
//! folder searched; an inland metadata file (`inland_water_meta.json`,
//! `TBD_InlandWaterExport_meta.json`) gives the `-inland-water` prefix, every other the `-water`
//! prefix.

use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt, bail};

/// The folders under the export folder that may hold the water files, after the export folder
/// itself; `{terrain}` stands for the terrain name.
const WATER_FOLDER_CANDIDATES: [&str; 5] = [
    "{terrain}/terrain/water",
    "{terrain}/water",
    "terrain/water",
    "water",
    "$tbd_framework:worlds/water",
];

/// The files whose presence marks a folder as the water folder.
const WATER_FOLDER_MARKERS: [&str; 5] = [
    "water_meta.json",
    "inland_water_meta.json",
    "bathymetry_mask.txt",
    "TBD_WaterExport_meta.json",
    "TBD_InlandWaterExport_meta.json",
];

/// The metadata files in the order they are preferred, each with whether it describes an inland
/// export.
const METADATA_CANDIDATES: [(&str, bool); 4] = [
    ("water_meta.json", false),
    ("inland_water_meta.json", true),
    ("TBD_WaterExport_meta.json", false),
    ("TBD_InlandWaterExport_meta.json", true),
];

/// The water class mask grid's file names, preferred first.
const MASK_GRID_CANDIDATES: [&str; 3] = [
    "bathymetry_mask.txt",
    "TBD_WaterExport_mask.txt",
    "TBD_InlandWaterExport_mask.txt",
];

/// The depth grid's file names, preferred first.
const DEPTH_GRID_CANDIDATES: [&str; 3] = [
    "bathymetry_depth.txt",
    "TBD_WaterExport_depth.txt",
    "TBD_InlandWaterExport_depth.txt",
];

/// The river vector file names, preferred first.
const RIVER_VECTOR_CANDIDATES: [&str; 3] = [
    "rivers.json",
    "TBD_InlandWaterExport_vectors.json",
    "TBD_WaterExport_vectors.json",
];

/// The lake vector file names, preferred first.
const LAKE_VECTOR_CANDIDATES: [&str; 2] = ["lakes.json", "TBD_WaterExport_vectors.json"];

/// The pond vector file names, preferred first.
const POND_VECTOR_CANDIDATES: [&str; 1] = ["ponds.json"];

/// The files of one Workbench water export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WaterExportFiles {
    /// The folder the files were found in.
    pub(super) water_dir: PathBuf,
    /// The metadata JSON (grid size, world size, resolution).
    pub(super) meta_path: PathBuf,
    /// Whether the metadata file is an inland export's.
    pub(super) is_inland: bool,
    /// The ASCII water class mask grid, when present.
    pub(super) mask_path: Option<PathBuf>,
    /// The ASCII depth grid in decimetres, when present.
    pub(super) depth_path: Option<PathBuf>,
    /// The river vectors, when present.
    pub(super) rivers_path: Option<PathBuf>,
    /// The lake vectors, when present.
    pub(super) lakes_path: Option<PathBuf>,
    /// The pond vectors, when present.
    pub(super) ponds_path: Option<PathBuf>,
}

/// The files of the water export under `export_dir` for `terrain`; see the module invariants for
/// the search order. Refuses an export whose water folder holds no metadata file.
pub(super) fn locate_water_export(export_dir: &Path, terrain: &str) -> Result<WaterExportFiles> {
    let water_dir = water_folder(export_dir, terrain);
    let Some((meta_path, is_inland)) = METADATA_CANDIDATES
        .iter()
        .map(|(name, is_inland)| (water_dir.join(name), *is_inland))
        .find(|(path, _)| path.exists())
    else {
        bail!(
            "missing the water metadata file (water_meta.json or inland_water_meta.json) in {}",
            water_dir.display()
        );
    };
    Ok(WaterExportFiles {
        mask_path: first_present(&water_dir, &MASK_GRID_CANDIDATES),
        depth_path: first_present(&water_dir, &DEPTH_GRID_CANDIDATES),
        rivers_path: first_present(&water_dir, &RIVER_VECTOR_CANDIDATES),
        lakes_path: first_present(&water_dir, &LAKE_VECTOR_CANDIDATES),
        ponds_path: first_present(&water_dir, &POND_VECTOR_CANDIDATES),
        meta_path,
        is_inland,
        water_dir,
    })
}

/// The first candidate folder (the export folder, then [`WATER_FOLDER_CANDIDATES`]) that exists
/// and holds one of [`WATER_FOLDER_MARKERS`]; the export folder when none does.
fn water_folder(export_dir: &Path, terrain: &str) -> PathBuf {
    std::iter::once(export_dir.to_path_buf())
        .chain(
            WATER_FOLDER_CANDIDATES
                .iter()
                .map(|relative| export_dir.join(relative.replace("{terrain}", terrain))),
        )
        .find(|folder| {
            folder.exists()
                && WATER_FOLDER_MARKERS
                    .iter()
                    .any(|marker| folder.join(marker).exists())
        })
        .unwrap_or_else(|| export_dir.to_path_buf())
}

/// The first of `names` under `folder` that exists.
fn first_present(folder: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| folder.join(name))
        .find(|path| path.exists())
}

/// The file name prefix of the images: `<terrain>-inland-water` for an inland export,
/// `<terrain>-water` otherwise.
pub(super) fn output_prefix(terrain: &str, is_inland: bool) -> String {
    if is_inland {
        format!("{terrain}-inland-water")
    } else {
        format!("{terrain}-water")
    }
}

/// Creates and returns the image folder: `out_dir` when given, otherwise `images` under the water
/// folder.
pub(super) fn create_output_directory(out_dir: Option<&Path>, water_dir: &Path) -> Result<PathBuf> {
    let folder = out_dir.map_or_else(|| water_dir.join("images"), Path::to_path_buf);
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("create the image folder {}", folder.display()))?;
    Ok(folder)
}

#[cfg(test)]
#[path = "../tests/water_export_images_location_tests.rs"]
mod tests;
