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
use ::repository_layout::find_repository_root;
use enfusion_pak::PakVfs;
use time_source::iso_from_system_time;
use world_export_pipeline::enfusion_texture_decoder;

pub(crate) const HW: usize = 4;
pub(crate) const ANCHOR: usize = HW + 1;
pub(crate) const FILL_FLOOR: f64 = 0.25;
pub(crate) const REL_FLOOR: f64 = 0.05;
pub(crate) const STEP_CAP: f64 = 6.0;
pub(crate) const DETAIL_MIN: f64 = 1.0;
pub(crate) const FLAT_EPS: f64 = 0.15;
pub(crate) const CELL_PX: usize = 256;
pub(crate) const GRID: usize = 50;
pub(crate) const ORTHO_PX: usize = GRID * CELL_PX;
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

mod sap_dir;
pub(crate) use sap_dir::analyze_seams;
use sap_dir::fmt2;
use sap_dir::metric_json;
use sap_dir::sap_dir;
pub(crate) use sap_dir::summarize;
pub(crate) use sap_dir::verify_sap_seams;

mod analyze_sap_seams;
pub(crate) use analyze_sap_seams::analyze_sap_seams;
pub(crate) use analyze_sap_seams::bridge_seams;
pub(crate) use analyze_sap_seams::verify_sap_ortho;

mod stitch_sap_ortho;
pub(crate) use stitch_sap_ortho::blend_sap_seams_cli;
pub(crate) use stitch_sap_ortho::stitch_sap_ortho;
