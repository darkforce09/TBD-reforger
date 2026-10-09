//! Reading a Workbench road export: its metadata and its six layer files.
//!
//! **Role:** [`load_road_export_meta`] reads `roads_meta.json` (world size, junctions) and
//! [`load_road_layers`] reads `<file stem>.json` of every [`ROAD_EXPORT_LAYER_FILES`] entry into
//! typed segments, applying the export's loose JSON rules once so the drawing sees plain `f64`s.
//! **Position:** called by the lane root `road_export_images::run`; its [`RoadLayer`]s and
//! [`RoadJunction`]s feed `road_image_outputs`.
//! **Signals & state:** none held; each call reads its files and returns owned values. A file that
//! does not parse is reported on stderr and read as empty.
//! **Invariants:** layers come back in [`ROAD_EXPORT_LAYER_FILES`] order, one per entry, a missing
//! or malformed file giving an empty layer; a value counts as present only when it is truthy
//! (`0`, `false`, `""`, `null` and a missing key are not), and a present value turns into a
//! number as arithmetic would: `null` is 0, `false` / `true` are 0 / 1, a missing value, a string,
//! an array and an object are NaN. A NaN coordinate, width or world size draws nothing and never
//! panics. A point carries `[x, y, z]` or `[x, z]`; its plan position is `(x, z)`.

use std::path::Path;

use road_network::prelude::ROAD_EXPORT_LAYER_FILES;
use serde_json::Value;

/// The metadata of a road export: the world it covers and its junctions.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RoadExportMeta {
    /// The side of the square world, in metres.
    pub(super) world_size_m: f64,
    /// The junctions with a plan position, in file order.
    pub(super) junctions: Vec<RoadJunction>,
}

/// One junction of the road network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RoadJunction {
    /// The plan position `(x, z)` in world metres.
    pub(super) world_position: [f64; 2],
    /// How many segments meet here: `degree`, else the length of `connectedSegments`, else 2.
    pub(super) degree: f64,
}

/// One road layer file: its file stem, its road class and the segments it strokes.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RoadLayer {
    /// The file stem, `highways` for `highways.json` and `layer-highways.png`.
    pub(super) file_stem: &'static str,
    /// The road class of [`road_network::prelude::ROAD_CLASSES`] the layer holds.
    pub(super) road_class: &'static str,
    /// How many entries the file's `segments` array lists, drawable or not; a layer image is
    /// written whenever this is at least one.
    pub(super) listed_segment_count: usize,
    /// The drawable segments of the file, in file order, each with at least two points.
    pub(super) segments: Vec<RoadSegment>,
}

/// One road polyline.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RoadSegment {
    /// The segment's own width in metres, `None` when it carries none (or a falsy one) and the
    /// class width applies.
    pub(super) width_m: Option<f64>,
    /// The plan positions `(x, z)` of its points in world metres; at least two.
    pub(super) world_points: Vec<[f64; 2]>,
}

/// Reads `roads_meta.json` under `roads_dir`. A missing file gives `default_world_size_m` and no
/// junctions; a file that does not parse is warned about and gives the same. A truthy
/// `worldSizeM` replaces the default; truthy `junctions` that are not an array count as none.
pub(super) fn load_road_export_meta(roads_dir: &Path, default_world_size_m: f64) -> RoadExportMeta {
    let mut meta = RoadExportMeta {
        world_size_m: default_world_size_m,
        junctions: Vec::new(),
    };
    let meta_path = roads_dir.join("roads_meta.json");
    if !meta_path.exists() {
        return meta;
    }
    let document = match read_json(&meta_path) {
        Ok(document) => document,
        Err(message) => {
            eprintln!("Could not parse roads_meta.json: {message}");
            return meta;
        }
    };
    let world_size = document.get("worldSizeM");
    if is_truthy(world_size) {
        meta.world_size_m = arithmetic_value(world_size);
    }
    let listed_junctions = match document.get("junctions") {
        Some(Value::Array(junctions)) => junctions.as_slice(),
        _ => &[],
    };
    meta.junctions = listed_junctions.iter().filter_map(road_junction).collect();
    println!(
        "Loaded metadata: mapName='{}', worldSize={}m, totalSegments={}, junctions={}",
        display_value(document.get("mapName")),
        meta.world_size_m,
        display_value(document.get("totalSegments")),
        listed_junctions.len()
    );
    meta
}

/// Reads the six layer files under `roads_dir`, one [`RoadLayer`] per [`ROAD_EXPORT_LAYER_FILES`]
/// entry in that order. A missing file gives an empty layer; a file that does not parse, or whose
/// `totalLengthM` is truthy but not a number, is warned about and gives an empty layer.
pub(super) fn load_road_layers(roads_dir: &Path) -> Vec<RoadLayer> {
    ROAD_EXPORT_LAYER_FILES
        .iter()
        .map(|&(file_stem, road_class)| {
            let layer_path = roads_dir.join(format!("{file_stem}.json"));
            let (segments, listed_segment_count) = if layer_path.exists() {
                match read_layer_segments(&layer_path) {
                    Ok((segments, listed_count, total_length_m)) => {
                        println!(
                            "Layer [{file_stem}]: {listed_count} continuous routes ({total_length_m:.0} m)"
                        );
                        (segments, listed_count)
                    }
                    Err(message) => {
                        eprintln!(
                            "Warning: Could not parse {}: {message}",
                            layer_path.display()
                        );
                        (Vec::new(), 0)
                    }
                }
            } else {
                (Vec::new(), 0)
            };
            RoadLayer {
                file_stem,
                road_class,
                listed_segment_count,
                segments,
            }
        })
        .collect()
}

/// The drawable segments of one layer file, the number of entries it lists and its
/// `totalLengthM` (0 when falsy), or why the file is unreadable.
fn read_layer_segments(layer_path: &Path) -> Result<(Vec<RoadSegment>, usize, f64), String> {
    let document = read_json(layer_path)?;
    let listed_segments = match document.get("segments") {
        Some(Value::Array(segments)) => segments.as_slice(),
        _ => &[],
    };
    let total_length = document.get("totalLengthM");
    let total_length_m = if is_truthy(total_length) {
        total_length
            .and_then(Value::as_f64)
            .ok_or_else(|| "totalLengthM is not a number".to_string())?
    } else {
        0.0
    };
    let segments = listed_segments.iter().filter_map(road_segment).collect();
    Ok((segments, listed_segments.len(), total_length_m))
}

/// The segment of one `segments` entry, or `None` when its `points` is not an array of at least
/// two entries.
fn road_segment(entry: &Value) -> Option<RoadSegment> {
    let Some(Value::Array(points)) = entry.get("points") else {
        return None;
    };
    if points.len() < 2 {
        return None;
    }
    let width = entry.get("widthM");
    Some(RoadSegment {
        width_m: is_truthy(width).then(|| arithmetic_value(width)),
        world_points: points.iter().map(plan_position).collect(),
    })
}

/// The junction of one `junctions` entry, or `None` when its `pos` is not an array of at least two
/// entries.
fn road_junction(entry: &Value) -> Option<RoadJunction> {
    let Some(Value::Array(position)) = entry.get("pos") else {
        return None;
    };
    if position.len() < 2 {
        return None;
    }
    let degree = entry.get("degree");
    let connected_segments = entry.get("connectedSegments");
    let degree = if is_truthy(degree) {
        arithmetic_value(degree)
    } else if is_truthy(connected_segments) {
        match connected_segments {
            Some(Value::Array(segments)) => segments.len() as f64,
            _ => f64::NAN,
        }
    } else {
        2.0
    };
    Some(RoadJunction {
        world_position: plan_position_of(position),
        degree,
    })
}

/// The plan position `(x, z)` of a point value: `[x, y, z]` or `[x, z]`; NaN for a point that is
/// not an array.
fn plan_position(point: &Value) -> [f64; 2] {
    match point {
        Value::Array(coordinates) => plan_position_of(coordinates),
        _ => [f64::NAN, f64::NAN],
    }
}

/// The plan position `(x, z)` of a coordinate list: `x` is entry 0, `z` is entry 2 when there are
/// at least three entries and entry 1 otherwise.
fn plan_position_of(coordinates: &[Value]) -> [f64; 2] {
    let z_index = if coordinates.len() >= 3 { 2 } else { 1 };
    [
        arithmetic_value(coordinates.first()),
        arithmetic_value(coordinates.get(z_index)),
    ]
}

/// Whether `value` is truthy: present and not `null`, `false`, `0` or `""`.
fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|number| number != 0.0),
        Some(Value::String(text)) => !text.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// The number `value` stands for in arithmetic: a number itself, `null` 0, `false` / `true` 0 / 1,
/// and NaN for a missing value, a string, an array or an object.
fn arithmetic_value(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(f64::NAN),
        Some(Value::Null) => 0.0,
        Some(Value::Bool(flag)) => f64::from(u8::from(*flag)),
        None | Some(Value::String(_) | Value::Array(_) | Value::Object(_)) => f64::NAN,
    }
}

/// A metadata value as the summary line prints it: the JSON text, a string unquoted, `undefined`
/// when missing.
fn display_value(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}

/// The JSON document at `path`, or the read or parse failure as text.
fn read_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../tests/road_export_images_layer_loading_tests.rs"]
mod tests;
