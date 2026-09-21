//! The SAP ortho lane: seam metrics,
//! `verify-sap-seams` / `analyze-sap-seams` / `verify-sap-ortho`, the stitcher
//! the stitch (2500-cell EDDS decode → north-up canvas), and the seam bridge
//! (both the in-canvas op and the CLI fallback).
//!
//! The magick shell-outs are native ops here (decode/encode via image/png, stddev/HSL/
//! threshold/resize in `img`). Numeric gate thresholds are unchanged; the two derived
//! readouts that ImageMagick computed (global stddev, orientation AE ratio) are recomputed
//! natively — same formulas, huge gate margins (floor 0.02 vs observed ~0.1; ORIENT_MAX 0.2
//! vs match ~0.08), so verdicts are stable even where the resampler differs in the last dp.

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::image_operations::{self, Rgb8};
use crate::browser_testing::server::repo_root;
use crate::enfusion_pak::PakVfs;
use crate::timestamp_formatting::iso_from_system_time;
use crate::world_export_pipeline::enfusion_texture_decoder;

pub const HW: usize = 4;
pub const ANCHOR: usize = HW + 1;
pub const FILL_FLOOR: f64 = 0.25;
pub const REL_FLOOR: f64 = 0.05;
pub const STEP_CAP: f64 = 6.0;
pub const DETAIL_MIN: f64 = 1.0;
pub const FLAT_EPS: f64 = 0.15;
pub const CELL_PX: usize = 256;
pub const GRID: usize = 50;
pub const ORTHO_PX: usize = GRID * CELL_PX;
const MIN_STDDEV: f64 = 0.02;
const ORIENT_MAX: f64 = 0.2;

/* ─────────────────────────── seam metrics ─────────────────────────── */

pub struct SeamMetric {
    pub axis: char,
    pub k: usize,
    pub c: usize,
    pub band_min_grad: f64,
    pub interior_grad: f64,
    pub apron_left: usize,
    pub apron_right: usize,
    pub anchor_safe: bool,
    pub step_delta_rgb: f64,
    pub evaluated: bool,
}

pub struct ControlMetric {
    pub axis: char,
    pub at: usize,
    pub band_min_grad: f64,
}

pub struct SeamAnalysis {
    pub vertical: Vec<SeamMetric>,
    pub horizontal: Vec<SeamMetric>,
    pub controls: Vec<ControlMetric>,
}

pub struct SeamSummary<'a> {
    pub seam_count: usize,
    pub evaluated_count: usize,
    pub worst_evaluated: Option<&'a SeamMetric>,
    pub mean_band_min_grad_eval: Option<f64>,
    pub max_step_delta: f64,
    pub absolute_floor_met: usize,
    pub worst_apron: usize,
    pub worst_ratio: Option<f64>,
    pub fill_failures: Vec<&'a SeamMetric>,
    pub step_failures: Vec<&'a SeamMetric>,
    pub anchor_unsafe: Vec<&'a SeamMetric>,
}

/* ─────────────────────────── verify-sap-seams ─────────────────────────── */

/* ─────────────────────────── analyze-sap-seams ─────────────────────────── */

/* ─────────────────────────── verify-sap-ortho ─────────────────────────── */

/* ─────────────────────────── seam bridge + stitch ─────────────────────────── */

#[path = "aerial_orthophoto/sap_dir.rs"]
mod sap_dir;
pub use sap_dir::analyze_seams;
use sap_dir::fmt2;
use sap_dir::metric_json;
use sap_dir::sap_dir;
pub use sap_dir::summarize;
pub use sap_dir::verify_sap_seams;

#[path = "aerial_orthophoto/analyze_sap_seams.rs"]
mod analyze_sap_seams;
pub use analyze_sap_seams::analyze_sap_seams;
pub use analyze_sap_seams::bridge_seams;
pub use analyze_sap_seams::verify_sap_ortho;

#[path = "aerial_orthophoto/stitch_sap_ortho.rs"]
mod stitch_sap_ortho;
pub use stitch_sap_ortho::blend_sap_seams_cli;
pub use stitch_sap_ortho::stitch_sap_ortho;
