//! The map label exporters: `locations.json` from staged raw JSONL, and `height-labels.json`
//! computed directly from the terrain elevation model through
//! `place_names::peaks`.
//!
//! **Role:** the required Everon towns and the locality rules; declares the two exporters.
//! **Position:** a lane of the crate; the command line calls the subcommands it re-exports.
//! **Signals & state:** none; constants.
//! **Invariants:** the label files it writes are the sources `map_label_archives` reads.

use std::path::PathBuf;

use crate::error::{Result, ResultExt};
use place_names::peaks::HeightLabel;
use place_names::peaks::HeightLabelKind;
use place_names::peaks::PEAK_MIN_VALUE_M;
use place_names::peaks::declutter_height_labels;
use place_names::peaks::find_peaks;
use serde_json::{Map, Value, json};
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::png::decode_png_to_meters;
use terrain_elevation::sampling::sample_elevation_from_meters_cache;

use ::repository_layout::find_repository_root;
use world_export_pipeline::json_number_formatting::{js_math_round, js_num};

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

mod importance_by_name;
pub(crate) use importance_by_name::export_locations;

mod export_height_labels;
pub(crate) use export_height_labels::export_height_labels;
