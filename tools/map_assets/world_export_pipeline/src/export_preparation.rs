//! **Role:** The export-lane auxiliaries: artifact validation (map-export-validate),
//! the type census (map-census), the spike gates (`verify-spike-k1`,
//! `census-spike`, `verify-spike-ops-log`), the export-profile copy, and
//! the DEM repack. Every stage keeps its stdout shape and exit codes.
//!
//! **Position:** the `copy-export-profile`, `raw-u16-dem-png`, `sap-catalog`, `census`, `spike-*`
//! and `validate-exports` subcommands dispatch here.
//! **Signals & state:** none at the module root.
//! **Invariants:** every check returns its verdict as an exit code.

use std::collections::{HashMap, HashSet};
use std::io::BufRead as _;
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt as _};
use serde_json::{Map, Value, json};
use terrain_elevation::raw as dem_raw;
use world_file_formats::containers::tbde::TbdeHeader;

use super::chunk_partitioner::CHUNK_SIZE_M;
use super::classify::{Classifier, Rules, load_rules};
use super::mathematical_verification::{SchemaSet, gunzip_json};
use crate::forest_contours as forest;
use crate::polygon_geometry::cell_of;
use crate::vegetation_density as density;
use ::repository_root::find_repository_root;

/* ─────────────────────────── verify-spike-k1 ─────────────────────────── */

/* ─────────────────────────── census-spike ─────────────────────────── */

/// A local kind array that misses `vehicle` strands rows; `census_spike` does
/// `by_kind.get_mut(kind).unwrap_or_else(|| panic!("kind {kind}"))`, so one wreck prefab inside
/// the spike region panicked the census. Single source now.
const ALL_KINDS: [&str; 9] = prefab_catalog::instance_kinds::INSTANCE_KINDS;

/* ─────────────────────────── verify-spike-ops-log ─────────────────────────── */

/* ─────────────────────────── census-types (map-census) ─────────────────────────── */

/* ─────────────────────────── validate-export-artifacts (map-export-validate) ─────────────────────────── */

/* ─────────────────────────── copy-world-export-profile ─────────────────────────── */

/* ─────────────────────────── DEM repack ─────────────────────────── */

/// The plugin's fixed V4 encoding range (`TBD_MapExportDEM.c` `DEFAULT_HMIN`/`DEFAULT_HMAX`), used
/// when a meta file predates the `heightRange*` keys. Everon's shipped manifest carries exactly
/// these, so the `.dem` a re-export writes stays comparable with the committed PNG.
const DEM_DEFAULT_MIN_M: f64 = -204.78;
const DEM_DEFAULT_MAX_M: f64 = 375.53;

/* ─────────────────────────── export-terrain phase gate ─────────────────────────── */

/* ─────────────────────────── SAP aerial cell index ─────────────────────────── */

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

use time_source::iso_from_system_time;
