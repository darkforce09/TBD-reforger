//! Unit tests of the road export reader.
//!
//! **Role:** pins how [`super::load_road_export_meta`] and [`super::load_road_layers`] read
//! present, falsy, missing and malformed values, 2-D and 3-D points, and malformed files.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `road_export_images/road_layer_loading.rs`.
//! **Signals & state:** each case writes its files into its own folder under the system temporary
//! folder and removes it when done.
//! **Invariants:** a malformed file never fails a read; it yields the defaults or an empty layer.

use std::path::{Path, PathBuf};

use super::{load_road_export_meta, load_road_layers};

/// A temporary road export folder, removed on drop.
struct ExportFolder(PathBuf);

impl ExportFolder {
    fn new(case: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-road-layer-loading-{case}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&path).expect("create the export folder");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).expect("write an export file");
    }
}

impl Drop for ExportFolder {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn missing_meta_gives_the_default_world_and_no_junctions() {
    let folder = ExportFolder::new("missing-meta");
    let meta = load_road_export_meta(folder.path(), 12800.0);
    assert_eq!(meta.world_size_m, 12800.0);
    assert!(meta.junctions.is_empty());
}

#[test]
fn malformed_meta_gives_the_defaults() {
    let folder = ExportFolder::new("malformed-meta");
    folder.write("roads_meta.json", "{\"worldSizeM\": 640,");
    let meta = load_road_export_meta(folder.path(), 12800.0);
    assert_eq!(meta.world_size_m, 12800.0);
    assert!(meta.junctions.is_empty());
}

#[test]
fn falsy_world_size_keeps_the_default_and_non_array_junctions_count_as_none() {
    let folder = ExportFolder::new("falsy-world");
    folder.write(
        "roads_meta.json",
        r#"{"worldSizeM":0,"junctions":{"pos":[1,2]}}"#,
    );
    let meta = load_road_export_meta(folder.path(), 12800.0);
    assert_eq!(meta.world_size_m, 12800.0);
    assert!(meta.junctions.is_empty());
}

#[test]
fn non_numeric_world_size_is_nan() {
    let folder = ExportFolder::new("text-world");
    folder.write("roads_meta.json", r#"{"worldSizeM":"wide"}"#);
    assert!(
        load_road_export_meta(folder.path(), 12800.0)
            .world_size_m
            .is_nan()
    );
}

#[test]
fn junction_degree_falls_back_to_connected_segments_then_two() {
    let folder = ExportFolder::new("junctions");
    folder.write(
        "roads_meta.json",
        r#"{"worldSizeM":640,"junctions":[
            {"pos":[100,0,200],"degree":3},
            {"pos":[300,0,320],"connectedSegments":[1,2,3,4]},
            {"pos":[50,600],"degree":0},
            {"pos":[7]},
            {"degree":5},
            {"pos":[1,2],"connectedSegments":{"a":1}},
            {"pos":[null,true],"degree":true}
        ]}"#,
    );
    let meta = load_road_export_meta(folder.path(), 12800.0);
    assert_eq!(meta.world_size_m, 640.0);
    let expected_positions = [
        [100.0, 200.0],
        [300.0, 320.0],
        [50.0, 600.0],
        [1.0, 2.0],
        [0.0, 1.0],
    ];
    let positions: Vec<[f64; 2]> = meta
        .junctions
        .iter()
        .map(|junction| junction.world_position)
        .collect();
    assert_eq!(positions, expected_positions);
    let degrees: Vec<f64> = meta
        .junctions
        .iter()
        .map(|junction| junction.degree)
        .collect();
    assert_eq!(&degrees[..3], &[3.0, 4.0, 2.0]);
    assert!(degrees[3].is_nan());
    assert_eq!(degrees[4], 1.0);
}

#[test]
fn layers_come_back_in_file_table_order_with_missing_files_empty() {
    let folder = ExportFolder::new("layer-order");
    folder.write(
        "tracks.json",
        r#"{"segments":[{"points":[[1,0,2],[3,0,4]]}]}"#,
    );
    let layers = load_road_layers(folder.path());
    let stems: Vec<&str> = layers.iter().map(|layer| layer.file_stem).collect();
    assert_eq!(
        stems,
        [
            "highways",
            "roads_paved",
            "roads_dirt",
            "tracks",
            "paths",
            "runways"
        ]
    );
    assert_eq!(layers[3].road_class, "track");
    assert_eq!(layers[3].listed_segment_count, 1);
    assert_eq!(layers[3].segments[0].world_points, [[1.0, 2.0], [3.0, 4.0]]);
    for (index, layer) in layers.iter().enumerate() {
        if index != 3 {
            assert_eq!(layer.listed_segment_count, 0);
            assert!(layer.segments.is_empty());
        }
    }
}

#[test]
fn segments_read_widths_and_points_by_the_loose_rules() {
    let folder = ExportFolder::new("segments");
    folder.write(
        "highways.json",
        r#"{"totalLengthM":12.5,"segments":[
            {"widthM":8,"points":[[20,0,20],[320,0,330]]},
            {"widthM":0,"points":[[40,600],[600,40]]},
            {"widthM":"wide","points":[[1,2],"corner",[3]]},
            {"points":[[5,0,5]]},
            {"points":"none"},
            7
        ]}"#,
    );
    let layer = &load_road_layers(folder.path())[0];
    assert_eq!(layer.listed_segment_count, 6);
    assert_eq!(layer.segments.len(), 3);
    assert_eq!(layer.segments[0].width_m, Some(8.0));
    assert_eq!(
        layer.segments[0].world_points,
        [[20.0, 20.0], [320.0, 330.0]]
    );
    assert_eq!(layer.segments[1].width_m, None);
    assert_eq!(
        layer.segments[1].world_points,
        [[40.0, 600.0], [600.0, 40.0]]
    );
    let loose = &layer.segments[2];
    assert!(loose.width_m.is_some_and(f64::is_nan));
    assert_eq!(loose.world_points[0], [1.0, 2.0]);
    assert!(
        loose.world_points[1]
            .iter()
            .all(|coordinate| coordinate.is_nan())
    );
    assert_eq!(loose.world_points[2][0], 3.0);
    assert!(loose.world_points[2][1].is_nan());
}

#[test]
fn a_listed_segment_that_cannot_be_drawn_still_counts() {
    let folder = ExportFolder::new("single-point");
    folder.write("paths.json", r#"{"segments":[{"points":[[5,0,5]]}]}"#);
    let layer = &load_road_layers(folder.path())[4];
    assert_eq!(layer.listed_segment_count, 1);
    assert!(layer.segments.is_empty());
}

#[test]
fn malformed_layer_files_give_empty_layers() {
    let folder = ExportFolder::new("malformed-layers");
    folder.write("highways.json", "{\"segments\": [");
    folder.write(
        "roads_paved.json",
        r#"{"totalLengthM":"long","segments":[{"points":[[1,2],[3,4]]}]}"#,
    );
    folder.write(
        "roads_dirt.json",
        r#"{"segments":{"points":[[1,2],[3,4]]}}"#,
    );
    folder.write("tracks.json", "null");
    let layers = load_road_layers(folder.path());
    for layer in &layers[..4] {
        assert_eq!(layer.listed_segment_count, 0, "{}", layer.file_stem);
        assert!(layer.segments.is_empty(), "{}", layer.file_stem);
    }
}
