//! The map label exporters: `locations.json` from staged raw JSONL, and `height-labels.json`
//! computed directly from the terrain elevation model through
//! `place_names::peaks`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use place_names::peaks::HeightLabel;
use place_names::peaks::HeightLabelKind;
use place_names::peaks::PEAK_MIN_VALUE_M;
use place_names::peaks::declutter_height_labels;
use place_names::peaks::find_peaks;
use serde_json::{Map, Value, json};
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::png::decode_png_to_meters;
use terrain_elevation::sampling::sample_elevation_from_meters_cache;

use crate::world_export_pipeline::json_number_formatting::{js_math_round, js_num};
use ::repository_layout::find_repository_root;

/* ─────────────────────────── locations export ─────────────────────────── */

const SUBFEATURE_WORDS: [&str; 5] = ["sawmill", "sawmil", "farm", "quarry", "mine"];
const LOCALITY_IMPORTANCE: f64 = 0.4;
const N_MIN: usize = 10;
const REQUIRED_EVERON_TOWNS: [&str; 7] = [
    "Morton",
    "Gorey",
    "Raccoon Rock",
    "Saint Philippe",
    "Levie",
    "Montignac",
    "Kermovan",
];

/* ─────────────────────────── height-labels export (native restore) ─────────────────────────── */

#[cfg(test)]
#[path = "tests/map_labels/map_labels_tests.rs"]
mod map_labels_tests;

#[path = "map_labels/importance_by_name.rs"]
mod importance_by_name;
pub use importance_by_name::export_locations;
pub use importance_by_name::export_locations_from_jsonl;
pub use importance_by_name::verify_locations_gates;

#[path = "map_labels/export_height_labels.rs"]
mod export_height_labels;
pub use export_height_labels::export_height_labels;
