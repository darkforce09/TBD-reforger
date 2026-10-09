//! **Role:** unit tests of [`crate::water_export_images::export_location`]: the water folder
//! search order, the metadata preference with its inland prefix, the grid and vector candidates,
//! and the image folder.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/export_location.rs`.
//! **Signals & state:** each case builds a folder tree under the system temporary folder and
//! removes it.
//! **Invariants:** files hold no content; only their presence is read.

use std::path::{Path, PathBuf};

use super::*;

/// A folder tree under the system temporary folder, removed on drop.
struct TemporaryTree(PathBuf);

impl TemporaryTree {
    fn new(case: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-water-location-{case}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&path).ok();
        std::fs::create_dir_all(&path).expect("create the temporary tree");
        Self(path)
    }

    /// Creates the empty file `relative` (and its folders) and returns its path.
    fn touch(&self, relative: &str) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create the folders");
        std::fs::write(&path, "").expect("create the file");
        path
    }
}

impl Drop for TemporaryTree {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn the_export_folder_itself_wins_when_it_holds_the_files() {
    let tree = TemporaryTree::new("itself");
    tree.touch("water_meta.json");
    tree.touch("water/water_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.water_dir, tree.0);
}

#[test]
fn the_terrain_folders_come_before_the_shared_ones() {
    let tree = TemporaryTree::new("order");
    tree.touch("everon/water/water_meta.json");
    tree.touch("terrain/water/water_meta.json");
    tree.touch("water/water_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.water_dir, tree.0.join("everon/water"));

    tree.touch("everon/terrain/water/bathymetry_mask.txt");
    tree.touch("everon/terrain/water/water_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.water_dir, tree.0.join("everon/terrain/water"));
}

#[test]
fn the_plain_water_folder_comes_before_the_framework_world_folder() {
    let tree = TemporaryTree::new("framework");
    tree.touch("$tbd_framework:worlds/water/water_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.water_dir, tree.0.join("$tbd_framework:worlds/water"));

    tree.touch("water/TBD_WaterExport_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.water_dir, tree.0.join("water"));
    assert_eq!(
        files.meta_path,
        tree.0.join("water/TBD_WaterExport_meta.json")
    );
    assert!(!files.is_inland);
}

#[test]
fn a_folder_with_only_the_mask_grid_is_the_water_folder_but_is_refused_without_metadata() {
    let tree = TemporaryTree::new("mask-only");
    tree.touch("water/bathymetry_mask.txt");
    tree.touch("$tbd_framework:worlds/water/water_meta.json");
    let error = locate_water_export(&tree.0, "everon").expect_err("no metadata");
    let text = error.to_string();
    assert!(text.contains("missing the water metadata file"), "{text}");
    assert!(
        text.contains(&tree.0.join("water").display().to_string()),
        "{text}"
    );
}

#[test]
fn the_sea_metadata_is_preferred_over_the_inland_one() {
    let tree = TemporaryTree::new("sea-first");
    tree.touch("water_meta.json");
    tree.touch("inland_water_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.meta_path, tree.0.join("water_meta.json"));
    assert!(!files.is_inland);
    assert_eq!(output_prefix("everon", files.is_inland), "everon-water");
}

#[test]
fn inland_metadata_gives_the_inland_prefix() {
    let tree = TemporaryTree::new("inland");
    tree.touch("inland_water_meta.json");
    tree.touch("TBD_WaterExport_meta.json");
    let files = locate_water_export(&tree.0, "arland").expect("locate");
    assert_eq!(files.meta_path, tree.0.join("inland_water_meta.json"));
    assert!(files.is_inland);
    assert_eq!(
        output_prefix("arland", files.is_inland),
        "arland-inland-water"
    );
}

#[test]
fn legacy_inland_metadata_is_the_last_choice() {
    let tree = TemporaryTree::new("legacy-inland");
    tree.touch("TBD_InlandWaterExport_meta.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(
        files.meta_path,
        tree.0.join("TBD_InlandWaterExport_meta.json")
    );
    assert!(files.is_inland);
}

#[test]
fn grid_and_vector_candidates_take_the_first_present_name() {
    let tree = TemporaryTree::new("candidates");
    tree.touch("water_meta.json");
    tree.touch("TBD_WaterExport_mask.txt");
    tree.touch("TBD_InlandWaterExport_mask.txt");
    tree.touch("TBD_InlandWaterExport_depth.txt");
    let shared_vectors = tree.touch("TBD_WaterExport_vectors.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(
        files.mask_path,
        Some(tree.0.join("TBD_WaterExport_mask.txt"))
    );
    assert_eq!(
        files.depth_path,
        Some(tree.0.join("TBD_InlandWaterExport_depth.txt"))
    );
    assert_eq!(files.rivers_path, Some(shared_vectors.clone()));
    assert_eq!(files.lakes_path, Some(shared_vectors));
    assert_eq!(files.ponds_path, None);

    let rivers = tree.touch("rivers.json");
    let lakes = tree.touch("lakes.json");
    let ponds = tree.touch("ponds.json");
    let files = locate_water_export(&tree.0, "everon").expect("locate");
    assert_eq!(files.rivers_path, Some(rivers));
    assert_eq!(files.lakes_path, Some(lakes));
    assert_eq!(files.ponds_path, Some(ponds));
}

#[test]
fn the_image_folder_defaults_to_images_under_the_water_folder() {
    let tree = TemporaryTree::new("images");
    let folder = create_output_directory(None, &tree.0).expect("create");
    assert_eq!(folder, tree.0.join("images"));
    assert!(folder.is_dir());

    let chosen = tree.0.join("chosen/nested");
    let folder = create_output_directory(Some(Path::new(&chosen)), &tree.0).expect("create");
    assert_eq!(folder, chosen);
    assert!(folder.is_dir());
}
