//! The label gates: height labels, location labels, town labels, road names and the elevation
//! anchors.
//!
//! **Role:** declares the gate bodies and the names they share: the towns and roads Everon must
//! draw, the name normaliser, the JSON reader, the 16-bit raster decoder and the JavaScript
//! `toFixed(3)` formatter.
//! **Position:** the `cargo xtask schema` height-labels, locations, town-labels, road-names and
//! terrain-alignment commands call the five gates through the CI task catalogue's map asset
//! checks; each gate runs the same `place_names`, `label_layout` and `terrain_elevation` code the
//! map draws with, so a passing gate means the map draws what was checked.
//! **Signals & state:** none; each gate reads the terrain folder afresh.
//! **Invariants:** a gate prints one `PASS`, `FAIL`, `SKIP` or `NOTE` line per check and returns
//! 1 when any check failed; a missing label file is a printed failure, not an error.
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt, bail, refusal};
use serde_json::Value;

use ::repository_layout::{definition_path, terrain_dir};

use place_names::peaks::HeightLabel;
use place_names::peaks::HeightLabelKind;
use place_names::peaks::PEAK_MIN_VALUE_M;
use place_names::peaks::declutter_height_labels;
use place_names::peaks::height_label_min_sep_m;
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::png::decode_png_to_meters;
use terrain_elevation::sampling::sample_elevation_from_meters_cache;

const PEAK_LABEL_MAX: usize = 48;

/* ─────────────────────────── locations (G2–G7) ─────────────────────────── */

/// The towns the Everon location and town-label gates require, matched by normalised name.
pub const REQUIRED_EVERON_TOWNS: [&str; 7] = [
    "Morton",
    "Gorey",
    "Raccoon Rock",
    "Saint Philippe",
    "Levie",
    "Montignac",
    "Kermovan",
];
/// The roads the Everon road-name gate requires among the drawn labels.
pub const MAJOR_EVERON_ROADS: [&str; 6] = [
    "Main Highway",
    "North-South Highway",
    "Coastal Road",
    "Airfield Access",
    "Gorey Road",
    "Morton Road",
];
const N_MIN: usize = 10;

/* ─────────── town labels (native rebuild on core importance_declutter) ─────────── */

/* ─────────── road names (native rebuild on core road_labels) ─────────── */

/* ─────────── terrain alignment (DEM vs GetSurfaceY anchors) ─────────── */

mod height_and_location_labels;
pub use height_and_location_labels::height_labels;
pub use height_and_location_labels::locations;
use height_and_location_labels::normalized_name;
use height_and_location_labels::read_json_file;

mod town_labels;
use town_labels::decode_u16_gray_png;
use town_labels::format_javascript_fixed_three;
pub use town_labels::road_names;
pub use town_labels::town_labels;

mod terrain_alignment;
pub use terrain_alignment::terrain_alignment;
