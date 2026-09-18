//! T-165.4 — label gates (height/town/road/locations), ported from `scripts/map-assets/`.
//! The height-labels port RESTORES the wasm-era branch natively (declutter G5/G6 + the ASL
//! oracle + G3 completeness) — those gates had retired-skipped when the React wasm pkg died;
//! `map-engine-core::dem` is the same math the wasm wrapped, so the Rust gate runs it directly.
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use website_map_engine::world::environment::locations::peaks::HeightLabel;
use website_map_engine::world::environment::locations::peaks::HeightLabelKind;
use website_map_engine::world::environment::locations::peaks::PEAK_MIN_VALUE_M;
use website_map_engine::world::environment::locations::peaks::declutter_height_labels;
use website_map_engine::world::environment::locations::peaks::height_label_min_sep_m;
use website_map_engine::world::terrain::dem::manifest::DemManifest;
use website_map_engine::world::terrain::dem::png::decode_png_to_meters;
use website_map_engine::world::terrain::dem::sampling::sample_elevation_from_meters_cache;

const PEAK_LABEL_MAX: usize = 48;

/* ─────────────────────────── locations (T-152.6 G2–G7) ─────────────────────────── */

pub const REQUIRED_EVERON_TOWNS: [&str; 7] = [
    "Morton",
    "Gorey",
    "Raccoon Rock",
    "Saint Philippe",
    "Levie",
    "Montignac",
    "Kermovan",
];
pub const MAJOR_EVERON_ROADS: [&str; 6] = [
    "Main Highway",
    "North-South Highway",
    "Coastal Road",
    "Airfield Access",
    "Gorey Road",
    "Morton Road",
];
const N_MIN: usize = 10;

/* ─────────── town labels (T-152.8/.17 — native rebuild on core importance_declutter) ─────────── */

/* ─────────── road names (T-152.9 — native rebuild on core road_labels) ─────────── */

/* ─────────── terrain alignment (T-091.0 — DEM vs GetSurfaceY anchors) ─────────── */

#[path = "labels/read_json.rs"]
mod read_json;
pub use read_json::height_labels;
pub use read_json::locations;
use read_json::norm_name;
use read_json::read_json;

#[path = "labels/town_labels.rs"]
mod town_labels;
use town_labels::decode_u16_gray_png;
use town_labels::js_fixed3;
pub use town_labels::road_names;
pub use town_labels::town_labels;

#[path = "labels/terrain_alignment.rs"]
mod terrain_alignment;
pub use terrain_alignment::terrain_alignment;
