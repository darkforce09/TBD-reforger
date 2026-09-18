//! T-165.8 — the export-lane auxiliaries: `validate-export-artifacts.mjs` (map-export-validate),
//! `census-types.mjs` (map-census), the T-090.3.0 spike gates (`verify-spike-k1`,
//! `census-spike`, `verify-spike-ops-log`), `copy-world-export-profile.mjs`, and
//! `raw-u16-to-dem-png.mjs` (T-091.0 DEM repack). Ports preserve stdout shapes + exit codes.

use std::collections::{HashMap, HashSet};
use std::io::BufRead as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use website_map_engine::io::containers::tbde::TbdeHeader;
use website_map_engine::world::terrain::dem::raw as dem_raw;

use super::chunk_partitioner::CHUNK_SIZE_M;
use super::classify::{Classifier, Rules, load_rules};
use super::mathematical_verification::{SchemaSet, gunzip_json};
use crate::browser_testing::server::repo_root;
use crate::world_export_pipeline::forest_contours as forest;
use crate::world_export_pipeline::polygon_geometry::cell_of;
use crate::world_export_pipeline::vegetation_density as density;

/* ─────────────────────────── verify-spike-k1 ─────────────────────────── */

/* ─────────────────────────── census-spike ─────────────────────────── */

/// T-278 — was a local 8-kind array missing T-244's `vehicle`; `census_spike` does
/// `by_kind.get_mut(kind).unwrap_or_else(|| panic!("kind {kind}"))`, so one wreck prefab inside
/// the spike region panicked the census. Single source now.
const ALL_KINDS: [&str; 9] = super::INSTANCE_KINDS;

/* ─────────────────────────── verify-spike-ops-log ─────────────────────────── */

/* ─────────────────────────── census-types (map-census) ─────────────────────────── */

/* ─────────────────────────── validate-export-artifacts (map-export-validate) ─────────────────────────── */

/* ─────────────────────────── copy-world-export-profile ─────────────────────────── */

/* ─────────────────────────── raw-u16-to-dem-png (T-091.0) ─────────────────────────── */

/// The plugin's fixed V4 encoding range (`TBD_MapExportDEM.c` `DEFAULT_HMIN`/`DEFAULT_HMAX`), used
/// when a meta file predates the `heightRange*` keys. Everon's shipped manifest carries exactly
/// these, so the `.dem` a re-export writes stays comparable with the committed PNG.
const DEM_DEFAULT_MIN_M: f64 = -204.78;
const DEM_DEFAULT_MAX_M: f64 = 375.53;

/* ─────────────────────────── export-terrain phase gate ─────────────────────────── */

/* ─────────────────────────── catalog-sap-cells (T-090.1.2) ─────────────────────────── */

#[cfg(test)]
#[path = "tests/export_preparation/elevation_dem_tests.rs"]
mod elevation_dem_tests;

#[path = "export_preparation/object_census.rs"]
mod object_census;
pub use object_census::census_spike;
pub use object_census::census_types;
use object_census::classified_rows;
use object_census::is_finite;
pub use object_census::verify_spike_k1;

#[path = "export_preparation/export_validation/operations_log.rs"]
mod export_validation_operations_log;
pub use export_validation_operations_log::verify_spike_ops_log;

#[path = "export_preparation/export_validation/artifact_integrity.rs"]
mod export_validation_artifact_integrity;
pub use export_validation_artifact_integrity::validate_export_artifacts;

#[path = "export_preparation/export_profile.rs"]
mod export_profile;
pub use export_profile::copy_world_export_profile;

#[path = "export_preparation/dem_elevation.rs"]
mod dem_elevation;
pub use dem_elevation::raw_u16_to_dem_png;
pub use dem_elevation::write_elevation_dem;

#[path = "export_preparation/aerial_cell_catalog.rs"]
mod aerial_cell_catalog;
pub use aerial_cell_catalog::catalog_sap_cells;

use crate::timestamp_formatting::iso_from_system_time;
