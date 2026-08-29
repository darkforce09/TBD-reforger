//! Every interpretation tunable in one serde-defaulted struct. The whole point of the
//! dumper/interpreter split is that changing any of these is a `cargo run`, not a Workbench
//! restart — so no per-tunable CLI flags: defaults in code, partial overrides via
//! `--params <file.json>` (unknown keys rejected), reproducible by committing the file.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Params {
    // ── face pairing ────────────────────────────────────────────────────────────────────────
    /// Max distance ahead a forward face may pair with an opposing face (one wall thickness).
    pub max_pair_m: f64,
    /// Opposing face may sit this far BEHIND the forward face (collision jitter slack).
    pub pair_behind_m: f64,
    /// Interval width assumed for an unmatched one-sided face (live: ROW_STEP_M * 0.9).
    pub sliver_m: f64,

    // ── slab detection (ScanVertical port) ──────────────────────────────────────────────────
    /// Histogram bin size for downward entry faces.
    pub slab_bin_m: f64,
    /// Peak support as a fraction of columns that hit anything.
    pub slab_support_frac: f64,
    /// Absolute support floor.
    pub slab_support_min: usize,
    /// Minimum vertical spacing between slabs.
    pub slab_spacing_m: f64,
    /// Slabs above eave + this are roof surfaces, not floors.
    pub slab_above_eave_m: f64,
    /// Floor filter: slab must sit in (min_floor_y, eave - eave_clearance).
    pub min_floor_y: f64,
    pub eave_clearance_m: f64,
    /// Top-surface percentiles for eave / ridge; chimney = spike above ridge by this margin.
    pub eave_pctile: usize,
    pub ridge_pctile: usize,
    pub chimney_margin_m: f64,

    // ── wall geometry (shared) ──────────────────────────────────────────────────────────────
    /// Solid thicker than this is an interior mass, not a wall.
    pub wall_max_thickness_m: f64,
    /// Minimum emitted wall length.
    pub wall_min_len_m: f64,
    /// Isolated solids smaller than this in both dims are collision noise (grid algo).
    pub min_feature_m: f64,

    // ── `segments` algo ─────────────────────────────────────────────────────────────────────
    /// Slice window above the slab: [lo, hi], also capped at band_top - top_margin.
    pub slice_lo_m: f64,
    pub slice_hi_m: f64,
    pub slice_top_margin_m: f64,
    /// Cluster epsilon on interval centers within one scanline.
    pub cluster_eps_m: f64,
    /// A cluster is a wall only if it appears in at least this fraction of slices...
    pub persistence_frac: f64,
    /// ...and its center drifts no more than this from the median across slices.
    pub max_drift_m: f64,
    /// Column-merge along the wall axis: max gap between accepted columns, max lateral offset.
    pub run_gap_m: f64,
    pub run_lateral_m: f64,
    /// Roof-graze veto: an observation is dropped when its slice height sits within this of the
    /// cell's TOP surface and that surface is sloped like a roof plane (slope in [lo, hi] —
    /// not a flat slab, not a vertical face). Slice≈top means the ray grazed the roof itself;
    /// a real attic wall UNDER the roof keeps a clear top-minus-slice margin and survives.
    pub roof_graze_eps_m: f64,
    pub roof_slope_lo: f64,
    pub roof_slope_hi: f64,

    // ── `grid` algo (faithful live port) ────────────────────────────────────────────────────
    /// The live two-height AND probe heights above the slab.
    pub band_low_m: f64,
    pub band_high_m: f64,
    /// Rect merge: cross-overlap tolerance and along-gap maximum.
    pub merge_overlap_m: f64,
    pub merge_gap_m: f64,

    // ── floor plate / rings ─────────────────────────────────────────────────────────────────
    /// Plate probe window around the slab: [slab - below, slab + above].
    pub plate_below_m: f64,
    pub plate_above_m: f64,
    /// Traced boundary rings (outer or hole) under this area are collision noise: dropped from
    /// the POLYGON products only — the painted plate grid stays verbatim, nothing is hidden.
    pub plate_min_ring_area_m2: f64,

    // ── furniture ───────────────────────────────────────────────────────────────────────────
    /// Prop-size ceilings (larger = composition parent, skipped).
    pub furn_max_plan_m: f64,
    pub furn_max_height_m: f64,
    /// Cover classification by height.
    pub furn_full_cover_m: f64,
    pub furn_none_below_m: f64,
    /// Level assignment slack below a band's floor.
    pub furn_level_slack_m: f64,

    // ── band shape ──────────────────────────────────────────────────────────────────────────
    /// Top band height fallback: max(eave, slab + this).
    pub top_band_min_m: f64,
    /// The viewer's slice height marker above each floor.
    pub slice_height_above_floor_m: f64,

    // ── roof heightfield ────────────────────────────────────────────────────────────────────
    /// Coarse grid pitch for the emitted `RoofGrid` (snapped to a multiple of the dump cell).
    /// Farmhouse sweep (phantom 0 at every pitch): 0.2 → 385/400 · 0.3 → 384 · 0.4 → 380 ·
    /// 0.5 → 375; 0.3 takes the knee — 0.2 buys one pair for 2.25× the grid.
    pub roof_cell_m: f64,
    /// Fraction of a coarse cell's fine block that must be covered, else the cell is `None`.
    /// 1.0 = fully covered — the surface never reaches past the true silhouette.
    pub roof_min_coverage: f64,
    /// Top surfaces below `floors[0] + this` are ground clutter (stoops, terraces), not roof.
    pub roof_min_above_floor_m: f64,
    /// Extra 4-neighbor erosion passes on the coarse grid (0 = the coverage rule is enough).
    pub roof_erode_cells: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_pair_m: 0.7,
            pair_behind_m: 0.05,
            sliver_m: 0.09,

            slab_bin_m: 0.1,
            slab_support_frac: 0.15,
            slab_support_min: 4,
            slab_spacing_m: 1.8,
            slab_above_eave_m: 0.5,
            min_floor_y: -0.5,
            eave_clearance_m: 0.5,
            eave_pctile: 20,
            ridge_pctile: 95,
            chimney_margin_m: 0.3,

            wall_max_thickness_m: 0.6,
            wall_min_len_m: 0.5,
            min_feature_m: 0.5,

            slice_lo_m: 0.3,
            slice_hi_m: 1.9,
            slice_top_margin_m: 0.15,
            cluster_eps_m: 0.06,
            persistence_frac: 0.6,
            max_drift_m: 0.08,
            run_gap_m: 0.15,
            run_lateral_m: 0.06,
            roof_graze_eps_m: 0.15,
            roof_slope_lo: 0.25,
            roof_slope_hi: 4.0,

            band_low_m: 0.45,
            band_high_m: 0.80,
            merge_overlap_m: 0.04,
            merge_gap_m: 0.15,

            plate_below_m: 0.35,
            plate_above_m: 0.4,
            plate_min_ring_area_m2: 0.02,

            furn_max_plan_m: 4.0,
            furn_max_height_m: 3.5,
            furn_full_cover_m: 1.6,
            furn_none_below_m: 0.4,
            furn_level_slack_m: 0.3,

            top_band_min_m: 2.0,
            slice_height_above_floor_m: 0.45,

            roof_cell_m: 0.3,
            roof_min_coverage: 1.0,
            roof_min_above_floor_m: 1.2,
            roof_erode_cells: 0,
        }
    }
}

impl Params {
    /// Defaults overlaid with a partial JSON override file.
    pub fn load(path: Option<&std::path::Path>) -> anyhow::Result<Self> {
        match path {
            None => Ok(Self::default()),
            Some(p) => {
                let text = std::fs::read_to_string(p)?;
                Ok(serde_json::from_str(&text)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_live_pipeline_constants() {
        let p = Params::default();
        assert_eq!(p.max_pair_m, 0.7);
        assert_eq!(p.band_low_m, 0.45);
        assert_eq!(p.wall_max_thickness_m, 0.6);
        assert_eq!(p.slab_spacing_m, 1.8);
    }

    #[test]
    fn partial_override_keeps_other_defaults() {
        let p: Params = serde_json::from_str(r#"{"max_drift_m": 0.12}"#).unwrap();
        assert_eq!(p.max_drift_m, 0.12);
        assert_eq!(p.max_pair_m, 0.7);
    }

    #[test]
    fn unknown_key_rejected() {
        assert!(serde_json::from_str::<Params>(r#"{"max_drfit_m": 0.12}"#).is_err());
    }
}
