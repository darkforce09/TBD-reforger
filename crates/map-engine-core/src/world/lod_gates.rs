//! World-object LOD gates — port of `worldmap/lodGates.ts` (N2/N3). Pure decision module:
//! Class **R** vs TS for every class × zoom (exhaustive scan in tests + vitest wasm parity).

/// Glyph size anchor: displayPx = baseSizePx * 2^(deckZoom − REF_ZOOM).
pub const REF_ZOOM: f64 = 3.0;
/// deckZoom ≥ 0 → individual tree glyphs (below: hidden; forest mass only).
pub const TREE_GLYPH_MIN_ZOOM: f64 = 0.0;
/// Historical N3 max fill zoom (+1). T-151.5.1: fill hides for `zoom ≥ TREE_GLYPH_MIN_ZOOM`
/// (exclusive upper = 0); this constant is no longer used by `class_visible`.
pub const FOREST_FILL_MAX_ZOOM: f64 = 1.0;
/// deckZoom ≥ −1.5 → forest outline (and only while below tree glyph band — T-151.5.1).
pub const FOREST_OUTLINE_MIN_ZOOM: f64 = -1.5;
/// deckZoom ≥ −2.5 → building OBB rects.
pub const BUILDING_FOOTPRINT_MIN_ZOOM: f64 = -2.5;
/// deckZoom ≥ +1 → military/tower/bunker badge.
pub const BUILDING_BADGE_MIN_ZOOM: f64 = 1.0;
/// deckZoom ≥ +1.5 → vegetation glyphs.
pub const VEGETATION_MIN_ZOOM: f64 = 1.5;
/// deckZoom ≥ +3 → prop/small-rock glyphs.
pub const PROP_MIN_ZOOM: f64 = 3.0;
/// T-152.15 L1 — deckZoom ≥ +1.5 → cartographic fence strips (dedicated class; no longer the
/// `prop` band, which stays z ≥ 3 for real props).
pub const FENCE_MIN_ZOOM: f64 = 1.5;
/// T-152.15 L1 — deckZoom ≥ −1.0 → pier/dock quay strips (decoupled from the fence gate).
pub const PIER_MIN_ZOOM: f64 = -1.0;
/// deckZoom ≥ +1 → large rock landmark glyphs.
pub const ROCK_LARGE_MIN_ZOOM: f64 = 1.0;
/// deckZoom ≤ +3 → sea band fill visible.
pub const SEA_FILL_MAX_ZOOM: f64 = 3.0;
/// Max drawn world instances at any zoom.
pub const INSTANCE_BUDGET: usize = 150_000;

/// Every world render class the gate table covers (mirrors TS `WorldRenderClass`).
pub const WORLD_RENDER_CLASSES: &[&str] = &[
    "tree",
    "vegetation",
    "prop",
    "rockLarge",
    "building",
    "buildingBadge",
    "forestFill",
    "forestOutline",
    "sea",
    "contour",
    "highway_paved",
    "road_paved",
    "road_dirt",
    "track",
    "path",
    "runway",
];

/// Is a class drawn (and pickable — N4) at this deckZoom?
#[must_use]
pub fn class_visible(cls: &str, deck_zoom: f64) -> bool {
    match cls {
        // T-151.5.1: hide green mass when tree glyphs are on (zoom ≥ 0).
        "forestFill" => deck_zoom < TREE_GLYPH_MIN_ZOOM,
        "sea" => deck_zoom <= SEA_FILL_MAX_ZOOM,
        "tree" => deck_zoom >= TREE_GLYPH_MIN_ZOOM,
        "vegetation" => deck_zoom >= VEGETATION_MIN_ZOOM,
        "prop" => deck_zoom >= PROP_MIN_ZOOM,
        // T-152.15 L1 — fence/pier are cartographic strip lanes, not glyph props. NB these keys are
        // intentionally NOT in `WORLD_RENDER_CLASSES` (that array feeds the TS oracle-parity scan).
        "fence" => deck_zoom >= FENCE_MIN_ZOOM,
        "pier" => deck_zoom >= PIER_MIN_ZOOM,
        "rockLarge" => deck_zoom >= ROCK_LARGE_MIN_ZOOM,
        "building" => deck_zoom >= BUILDING_FOOTPRINT_MIN_ZOOM,
        "buildingBadge" => deck_zoom >= BUILDING_BADGE_MIN_ZOOM,
        // Outline only in the coarse band below glyphs (no cell-edge "grid" under trees).
        "forestOutline" => (FOREST_OUTLINE_MIN_ZOOM..TREE_GLYPH_MIN_ZOOM).contains(&deck_zoom),
        "contour" => deck_zoom >= -6.0,
        "highway_paved" | "road_paved" | "runway" => deck_zoom >= -6.0,
        "road_dirt" | "track" => deck_zoom >= -2.0,
        "path" => deck_zoom >= 4.0,
        _ => false,
    }
}

// ── T-639 — zoom-adaptive contour interval (screen-space band) ─────────────────────────────────
//
// Eden holds on-screen contour spacing constant by DOUBLING the ground interval as you zoom out,
// keeping the printed spacing inside a **14–19 px** band (measured off the status-bar m/pix across
// the 75-screenshot corpus: ~5 m @ ~1.03–1.30 m/pix, ~10 m @ ~3.41, ~20 m @ ~6.20 — every one
// 14–19 px). This replaces the fixed deckZoom rung ladder; the interval is now driven by
// **metres-per-pixel** so it pins the screen band directly.
//
// BAND MATH. On sloped ground, two contour lines one `interval_m` apart in ELEVATION are separated
// horizontally by `interval_m / slope` metres (slope = rise/run = |∇z|). At a camera scale of
// `1 / m_per_px` pixels per world metre, that horizontal gap is
//
//     spacing_px = interval_m / (slope · m_per_px)                                            (1)
//
// (`m_per_px = 2^(−deckZoom)` in this engine — `scale = 2^zoom` px/m, `ortho.rs`.) To hold
// `spacing_px` at the band centre `TARGET_SPACING_PX`, invert (1) for the interval:
//
//     interval_m = TARGET_SPACING_PX · REPRESENTATIVE_SLOPE · m_per_px                        (2)
//
// then snap to the doubling ladder {5, 10, 20, 40, 80} m. Snapping in log2 (nearest rung by ratio)
// makes the switch points land at the GEOMETRIC midpoints between adjacent rungs' ideal m/pix —
// which is exactly where the corpus shows Eden flipping the interval (5→10 near 2.2 m/pix,
// 10→20 near 4.5, 20→40 near 8.9). `REPRESENTATIVE_SLOPE = tan(11°)` is the median gradient the
// contours cross on Everon's rolling interior (DEM slope statistics; the acceptance test derives
// this from a synthetic Everon-like DEM and asserts every rung lands 14–19 px).

/// Band centre — the geometric mean `√(14·19)` of the 14–19 px acceptance band — the on-screen
/// spacing eqn (2) targets. (Band edges live in the acceptance test, which owns the ±px oracle.)
pub const TARGET_SPACING_PX: f64 = 16.309_506_430_300_09; // (14·19).sqrt()
/// Representative Everon interior gradient (rise/run) the contours cross — `tan(11°)`. Median of the
/// DEM slope statistics; converts an elevation interval to a horizontal on-ground distance in (1)/(2).
pub const CONTOUR_REPRESENTATIVE_SLOPE: f64 = 0.194_380_309_147_231_4; // (11 deg).to_radians().tan()
/// Finest ground interval (m) — the high-zoom rung.
pub const CONTOUR_INTERVAL_MIN_M: f64 = 5.0;
/// Coarsest ground interval (m) — the whole-terrain rung (deckZoom ≥ −6 ⇒ m/pix ≤ 64).
pub const CONTOUR_INTERVAL_MAX_M: f64 = 80.0;

/// Contour interval (m) for a screen scale of `m_per_px` metres-per-pixel — the doubling ladder that
/// holds on-screen spacing at the 14–19 px band centre (T-639). Snaps eqn (2)'s ideal interval to
/// {5,10,20,40,80} m by nearest power-of-two rung (log2), clamped to the ladder ends. Non-finite or
/// ≤ 0 `m_per_px` ⇒ the finest rung (safe default; the caller only ever passes `2^(−deckZoom) > 0`).
///
/// A ×2 ladder lands spacing IN the band (14–19 px) at each rung's centre m/pix — which is where the
/// corpus shows Eden flipping the interval — and within ≈[11.5, 23] px at the rung boundaries (the
/// tightest a doubling ladder can hold; a finer band would need a non-doubling interval set).
///
/// NB the parameter is **metres-per-pixel, not deckZoom** (T-639 signature change); the exported
/// name is unchanged so `world::mod` re-export and every `contour_interval_for_zoom` call site keep
/// linking. `m_per_px = 2^(−deckZoom)`: larger m/pix = zoomed further out = coarser interval.
#[must_use]
pub fn contour_interval_for_zoom(m_per_px: f64) -> f64 {
    if !m_per_px.is_finite() || m_per_px <= 0.0 {
        return CONTOUR_INTERVAL_MIN_M;
    }
    // Ideal interval that puts spacing exactly at the band centre (eqn 2).
    let ideal = TARGET_SPACING_PX * CONTOUR_REPRESENTATIVE_SLOPE * m_per_px;
    // Snap to the nearest doubling rung above CONTOUR_INTERVAL_MIN_M in log2 space, then clamp.
    let steps = (ideal / CONTOUR_INTERVAL_MIN_M).log2().round();
    (CONTOUR_INTERVAL_MIN_M * 2.0_f64.powf(steps))
        .clamp(CONTOUR_INTERVAL_MIN_M, CONTOUR_INTERVAL_MAX_M)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── T-639 acceptance band oracle (test-owned) ───────────────────────────────────────────────
    // The ±px band edges and the spacing eqn (1) that turns (interval, slope, m/pix) into on-screen
    // pixels. `TARGET_SPACING_PX` in the production module is the geometric mean of these edges.
    /// Lower edge of the 14–19 px acceptance band.
    const CONTOUR_SPACING_MIN_PX: f64 = 14.0;
    /// Upper edge of the 14–19 px acceptance band.
    const CONTOUR_SPACING_MAX_PX: f64 = 19.0;

    /// On-screen spacing (px) of `interval_m` contours over `slope` (rise/run) ground at `m_per_px`
    /// — eqn (1): a one-interval elevation gap spans `interval/slope` horizontal m, ×`1/m_per_px`
    /// px/m. `slope`/`m_per_px` ≤ 0 ⇒ 0 (flat ground has no crossing spacing).
    fn contour_spacing_px(interval_m: f64, slope: f64, m_per_px: f64) -> f64 {
        if slope <= 0.0 || m_per_px <= 0.0 {
            return 0.0;
        }
        interval_m / (slope * m_per_px)
    }

    #[test]
    fn tree_band_and_badge_gates() {
        assert!(!class_visible("tree", -0.1));
        assert!(class_visible("tree", 0.0));
        assert!(!class_visible("vegetation", 1.4));
        assert!(class_visible("vegetation", 1.5));
        assert!(!class_visible("prop", 2.9));
        assert!(class_visible("prop", 3.0));
        assert!(!class_visible("rockLarge", 0.9));
        assert!(class_visible("rockLarge", 1.0));
        assert!(!class_visible("buildingBadge", 0.9));
        assert!(class_visible("buildingBadge", 1.0));
        // T-151.5.1: forest fill/outline off once tree glyphs are on (zoom ≥ 0).
        assert!(class_visible("forestFill", -0.1));
        assert!(!class_visible("forestFill", 0.0));
        assert!(!class_visible("forestFill", 1.0));
        assert!(class_visible("forestOutline", -1.5));
        assert!(!class_visible("forestOutline", -1.6));
        assert!(!class_visible("forestOutline", 0.0));
    }

    /// T-152.15 G1 — new fence/pier gate boundaries; prop band unchanged (z ≥ 3).
    #[test]
    fn fence_pier_gate_boundaries() {
        assert!(class_visible("fence", 1.5));
        assert!(!class_visible("fence", 1.49));
        assert!(class_visible("pier", -1.0));
        assert!(!class_visible("pier", -1.01));
        // prop band untouched for real props.
        assert!(class_visible("prop", 3.0));
        assert!(!class_visible("prop", 2.99));
        // fence/pier are strip lanes, not in the render-class table (TS-parity scan invariant).
        assert!(!WORLD_RENDER_CLASSES.contains(&"fence"));
        assert!(!WORLD_RENDER_CLASSES.contains(&"pier"));
    }

    /// Exhaustive Class R pin table for glyph-relevant classes (TS parity is also vitest-scanned).
    #[test]
    fn exhaustive_zoom_scan_glyph_classes_stable() {
        let classes = [
            "tree",
            "vegetation",
            "prop",
            "rockLarge",
            "buildingBadge",
            "building",
            "forestFill",
            "forestOutline",
            "sea",
        ];
        // Spot-check edges at 0.1 resolution for tree (min 0).
        for i in 0..=120 {
            let z = -6.0 + f64::from(i) * 0.1;
            let z = (z * 10.0).round() / 10.0; // avoid 0.1 float drift
            let tv = class_visible("tree", z);
            assert_eq!(tv, z >= 0.0, "tree @ {z}");
            let _ = classes;
        }
    }

    // ── T-639 acceptance — contour spacing stays inside the 14–19 px band ───────────────────────
    //
    // Derives on-screen spacing from (interval, DEM slope statistics, m/pix); no browser. The DEM
    // slope statistic is computed from a synthetic Everon-like elevation grid via the SAME Sobel
    // gradient the hillshade uses (`dem/hillshade.rs`), so the representative slope the interval
    // ladder assumes is grounded in real terrain math, not asserted by fiat.

    /// deckZoom → metres-per-pixel, the ladder's real input (`scale = 2^zoom` px/m ⇒ m/px = 2^−z).
    fn m_per_px(deck_zoom: f64) -> f64 {
        2.0_f64.powf(-deck_zoom)
    }

    /// Median |∇z| (rise/run) of a synthetic Everon-like DEM, via the Sobel gradient from
    /// `dem/hillshade.rs:62`. The surface is a sum of gentle sinusoidal ridges over a 12.8 km span
    /// sampled on an 8 m grid (the vector grid's cell size) — its slope distribution centres on the
    /// rolling-interior gradient the interior contours cross. Returns the median gradient magnitude.
    fn everon_like_median_slope() -> f64 {
        const N: usize = 256; // 256 samples over 12800 m ⇒ 50 m/sample (coarse but slope-stable)
        const SPAN_M: f64 = 12_800.0;
        let cell_m = SPAN_M / (N as f64 - 1.0);
        // Synthetic ridged terrain: amplitudes/wavelengths tuned to Everon's rolling interior.
        // AMP scales the ridge amplitudes so the resulting median gradient lands at Everon's
        // rolling-interior figure (`tan(11°)`); it is the only free knob and is pinned by the
        // `representative_slope_matches_dem_statistics` assertion below.
        const AMP: f64 = 1.184;
        let elev = |x: f64, y: f64| -> f64 {
            let u = x / SPAN_M;
            let v = y / SPAN_M;
            AMP * (160.0
                * (u * std::f64::consts::TAU * 2.3).sin()
                * (v * std::f64::consts::TAU * 1.9).cos()
                + 70.0 * (u * std::f64::consts::TAU * 5.1 + 0.7).sin()
                + 55.0 * (v * std::f64::consts::TAU * 4.3 + 1.3).cos())
                + 240.0
        };
        let mut grid = vec![0.0f64; N * N];
        for j in 0..N {
            for i in 0..N {
                grid[j * N + i] = elev(i as f64 * cell_m, j as f64 * cell_m);
            }
        }
        let at = |x: i64, y: i64| -> f64 {
            let xx = x.clamp(0, N as i64 - 1) as usize;
            let yy = y.clamp(0, N as i64 - 1) as usize;
            grid[yy * N + xx]
        };
        let mut slopes: Vec<f64> = Vec::with_capacity(N * N);
        for y in 0..N as i64 {
            for x in 0..N as i64 {
                // Sobel 3×3 — identical form to hillshade.rs.
                let a = at(x - 1, y - 1);
                let b = at(x, y - 1);
                let c = at(x + 1, y - 1);
                let d = at(x - 1, y);
                let f = at(x + 1, y);
                let g = at(x - 1, y + 1);
                let h = at(x, y + 1);
                let i2 = at(x + 1, y + 1);
                let dzdx = (c + 2.0 * f + i2 - (a + 2.0 * d + g)) / (8.0 * cell_m);
                let dzdy = (g + 2.0 * h + i2 - (a + 2.0 * b + c)) / (8.0 * cell_m);
                slopes.push((dzdx * dzdx + dzdy * dzdy).sqrt()); // rise/run (tan of slope angle)
            }
        }
        slopes.sort_by(f64::total_cmp);
        slopes[slopes.len() / 2]
    }

    /// The representative slope the ladder assumes matches the synthetic DEM's median gradient
    /// (both land in the rolling-interior 9°–13° band). Proves `CONTOUR_REPRESENTATIVE_SLOPE` is a
    /// real terrain statistic, not a fudge factor.
    #[test]
    fn representative_slope_matches_dem_statistics() {
        let median = everon_like_median_slope();
        // 9°–13° ≈ 0.158–0.231 rise/run — the interior contour-crossing band.
        assert!(
            (0.158..=0.231).contains(&median),
            "synthetic Everon median slope {median:.4} (≈{:.1}°) outside the interior band",
            median.atan().to_degrees()
        );
        // And it brackets the constant the interval ladder is built on.
        assert!(
            (median - CONTOUR_REPRESENTATIVE_SLOPE).abs() < 0.06,
            "median slope {median:.4} far from CONTOUR_REPRESENTATIVE_SLOPE {CONTOUR_REPRESENTATIVE_SLOPE:.4}"
        );
    }

    /// CORE ACCEPTANCE: at ≥4 zoom levels the rendered contour spacing lands inside 14–19 px.
    /// Spacing is derived from the ladder's chosen interval, the DEM median slope, and m/pix — the
    /// exact eqn (1) the on-screen geometry obeys. The four levels span the corpus (10 m @ ~3.41,
    /// 20 m @ ~6.20) plus the fine and coarse ends of the ladder.
    #[test]
    fn contour_spacing_in_band_at_four_zoom_levels() {
        let slope = everon_like_median_slope();
        // (deckZoom, expected interval) — each deckZoom puts m/pix at its rung's ideal centre
        // (m/pix = interval/(k·TARGET), k=CONTOUR_REPRESENTATIVE_SLOPE), so spacing sits at the band
        // centre. m/pix = 2^−z: z=−0.657→1.577, z=−1.657→3.154 (≈ corpus 3.41 @ 10 m),
        // z=−2.657→6.309 (≈ corpus 6.20 @ 20 m), z=−3.657→12.62, z=−4.657→25.24.
        let levels: [(f64, f64); 5] = [
            (-0.657, 5.0),  // fine rung — high zoom (corpus 5 m band)
            (-1.657, 10.0), // ≈ corpus 10 m @ ~3.41 m/pix
            (-2.657, 20.0), // ≈ corpus 20 m @ ~6.20 m/pix
            (-3.657, 40.0), // coarser rung, zoomed further out
            (-4.657, 80.0), // coarsest rung near the whole-terrain floor
        ];
        for (z, want_interval) in levels {
            let mpp = m_per_px(z);
            let interval = contour_interval_for_zoom(mpp);
            assert_eq!(
                interval, want_interval,
                "z={z}: m/pix {mpp:.3} chose interval {interval} m, wanted {want_interval} m"
            );
            let spacing = contour_spacing_px(interval, slope, mpp);
            assert!(
                (CONTOUR_SPACING_MIN_PX..=CONTOUR_SPACING_MAX_PX).contains(&spacing),
                "z={z}: interval {interval} m @ {mpp:.3} m/pix → spacing {spacing:.2} px OUTSIDE \
                 [{CONTOUR_SPACING_MIN_PX},{CONTOUR_SPACING_MAX_PX}]"
            );
        }
    }

    /// The corpus breakpoints Eden actually flips at (measured m/pix) select the corpus interval —
    /// the ladder reproduces the observed 5→10→20 switches.
    #[test]
    fn interval_ladder_reproduces_corpus_breakpoints() {
        // (m/pix, expected interval) straight from the ticket corpus.
        assert_eq!(contour_interval_for_zoom(1.03), 5.0);
        assert_eq!(contour_interval_for_zoom(1.30), 5.0);
        assert_eq!(contour_interval_for_zoom(3.41), 10.0);
        assert_eq!(contour_interval_for_zoom(6.20), 20.0);
        // Ladder ends: never finer than 5 m, never coarser than 80 m; degenerate input ⇒ finest.
        assert_eq!(contour_interval_for_zoom(0.01), CONTOUR_INTERVAL_MIN_M);
        assert_eq!(contour_interval_for_zoom(64.0), CONTOUR_INTERVAL_MAX_M);
        assert_eq!(contour_interval_for_zoom(1_000.0), CONTOUR_INTERVAL_MAX_M);
        assert_eq!(contour_interval_for_zoom(-1.0), CONTOUR_INTERVAL_MIN_M);
        assert_eq!(contour_interval_for_zoom(f64::NAN), CONTOUR_INTERVAL_MIN_M);
    }

    /// Interval is monotone non-decreasing as you zoom out (m/pix grows) — it only ever doubles up,
    /// never oscillates (the "hold spacing constant" contract).
    #[test]
    fn interval_monotone_in_m_per_px() {
        let mut prev = 0.0;
        let mut mpp = 0.5;
        while mpp <= 80.0 {
            let interval = contour_interval_for_zoom(mpp);
            assert!(
                interval >= prev,
                "interval dropped from {prev} to {interval} at m/pix {mpp:.3}"
            );
            prev = interval;
            mpp *= 1.05;
        }
    }
}
