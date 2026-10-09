//! The aerial orthophoto lane: the stitch (2500 EDDS cells decoded onto a north-up canvas), the
//! seam bridge (inside the stitch and as `blend-sap-seams`), the seam metrics and the
//! `verify-sap-seams`, `analyze-sap-seams` and `verify-sap-ortho` gates.
//!
//! **Role:** the lane's gate thresholds, grid geometry and seam metric types; declares the stitch,
//! the seam analysis and the seam gates.
//! **Position:** a lane of the crate; the command line calls the subcommands it re-exports.
//! **Signals & state:** none; constants and plain data.
//! **Invariants:** every gate threshold is a named constant here.
//!
//! Decoding, encoding, standard deviation, HSL, threshold and resize are native operations in
//! `image_operations`. The two derived readouts (the global standard deviation and the orientation
//! error ratio) keep wide gate margins (floor 0.02 against an observed ~0.1; `ORIENT_MAX` 0.2
//! against a match of ~0.08), so verdicts hold even where a resampler differs in the last decimal.

use std::path::PathBuf;

use crate::error::{Result, bail};
use serde_json::{Value, json};

use super::image_operations::{self, Rgb8};
use ::repository_root::find_repository_root;
use enfusion_pak::PakVfs;
use time_source::iso_from_system_time;
use world_export_pipeline::enfusion_texture_decoder;

pub(crate) const SEAM_HALF_WIDTH_PIXELS: usize = 4;
pub(crate) const SEAM_ANCHOR_OFFSET_PIXELS: usize = SEAM_HALF_WIDTH_PIXELS + 1;
pub(crate) const FILL_GRADIENT_FLOOR: f64 = 0.25;
pub(crate) const RELATIVE_GRADIENT_FLOOR: f64 = 0.05;
pub(crate) const STEP_DELTA_CAP: f64 = 6.0;
pub(crate) const MINIMUM_INTERIOR_DETAIL: f64 = 1.0;
pub(crate) const FLAT_GRADIENT_EPSILON: f64 = 0.15;
pub(crate) const CELL_PIXELS: usize = 256;
pub(crate) const GRID_CELLS_PER_SIDE: usize = 50;
pub(crate) const ORTHOPHOTO_PIXELS: usize = GRID_CELLS_PER_SIDE * CELL_PIXELS;
const MIN_STDDEV: f64 = 0.02;
const ORIENT_MAX: f64 = 0.2;

/* ─────────────────────────── seam metrics ─────────────────────────── */

pub(crate) struct SeamMetric {
    pub(crate) axis: char,
    pub(crate) k: usize,
    pub(crate) c: usize,
    pub(crate) band_min_grad: f64,
    pub(crate) interior_grad: f64,
    pub(crate) apron_left: usize,
    pub(crate) apron_right: usize,
    pub(crate) anchor_safe: bool,
    pub(crate) step_delta_rgb: f64,
    pub(crate) evaluated: bool,
}

pub(crate) struct ControlMetric {
    pub(crate) axis: char,
    pub(crate) at: usize,
    pub(crate) band_min_grad: f64,
}

pub(crate) struct SeamAnalysis {
    pub(crate) vertical: Vec<SeamMetric>,
    pub(crate) horizontal: Vec<SeamMetric>,
    pub(crate) controls: Vec<ControlMetric>,
}

pub(crate) struct SeamSummary<'a> {
    pub(crate) seam_count: usize,
    pub(crate) evaluated_count: usize,
    pub(crate) worst_evaluated: Option<&'a SeamMetric>,
    pub(crate) mean_band_min_grad_eval: Option<f64>,
    pub(crate) max_step_delta: f64,
    pub(crate) absolute_floor_met: usize,
    pub(crate) worst_apron: usize,
    pub(crate) worst_ratio: Option<f64>,
    pub(crate) fill_failures: Vec<&'a SeamMetric>,
    pub(crate) step_failures: Vec<&'a SeamMetric>,
    pub(crate) anchor_unsafe: Vec<&'a SeamMetric>,
}

/* ─────────────────────────── verify-sap-seams ─────────────────────────── */

/* ─────────────────────────── analyze-sap-seams ─────────────────────────── */

/* ─────────────────────────── verify-sap-ortho ─────────────────────────── */

/* ─────────────────────────── seam bridge + stitch ─────────────────────────── */

mod supertexture_seam_metrics;
pub(crate) use supertexture_seam_metrics::analyze_seams;
use supertexture_seam_metrics::format_two_decimals;
use supertexture_seam_metrics::metric_json;
pub(crate) use supertexture_seam_metrics::summarize;
use supertexture_seam_metrics::supertexture_scratch_dir;
pub(crate) use supertexture_seam_metrics::verify_supertexture_seams;

mod supertexture_seam_analysis;
pub(crate) use supertexture_seam_analysis::analyze_supertexture_seams;
pub(crate) use supertexture_seam_analysis::bridge_seams;
pub(crate) use supertexture_seam_analysis::verify_supertexture_orthophoto;

mod supertexture_orthophoto_stitch;
pub(crate) use supertexture_orthophoto_stitch::blend_supertexture_seams_command_line;
pub(crate) use supertexture_orthophoto_stitch::stitch_supertexture_orthophoto;
