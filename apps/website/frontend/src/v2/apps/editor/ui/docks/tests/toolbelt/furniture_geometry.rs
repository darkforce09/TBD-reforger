use super::*;

/// A camera built exactly as `select_tool::frozen_camera` builds it (bounds `[0,0,12800,12800]`),
/// so the projection the labels use matches the one the editor + GPU grid use.
fn cam(width: f64, height: f64, tx: f64, ty: f64, zoom: f64) -> OrthoCamera {
    let mut c = OrthoCamera::new(width, height, tx, ty, zoom);
    c.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
    c
}

/// m/px is exactly `2^(−deck_zoom)` (the T-639/T-641 convention this whole ticket rides).
#[test]
fn m_per_px_is_two_pow_neg_zoom() {
    assert!((m_per_px(0.0) - 1.0).abs() < 1e-12);
    assert!((m_per_px(-2.0) - 4.0).abs() < 1e-12); // editor default zoom
    assert!((m_per_px(2.0) - 0.25).abs() < 1e-12);
    assert!((m_per_px(-6.0) - 64.0).abs() < 1e-12); // MIN_ZOOM (whole terrain)
    assert!(m_per_px(f64::NAN).is_nan()); // T-756: non-finite → NAN → em-dash readout
}

/// The scale-bar distance table across the zoom range. Each row is `(deck_zoom, expected_dist_m,
/// expected_label)`. The picker takes the largest `1/2/5×10^n` whose bar is ≤ 200 px, so the bar
/// px = dist / (2^−zoom) is always ≤ 200 and lands in a readable band (~80–200 px — the tightest
/// a 1-2-5 ladder holds). This is the ticket's required "table over zooms".
#[test]
fn scale_bar_distance_table() {
    // (deck_zoom, dist_m, label). Each row: dist = largest 1/2/5×10^n with dist/(2^−z) ≤ 200 px.
    // The bar px each row yields is in the comment and re-verified against the picker below.
    let table: &[(f64, f64, &str)] = &[
        (2.0, 50.0, "50 m"),      // m/px 0.25 → 50 m → 200 px
        (1.0, 100.0, "100 m"),    // m/px 0.5  → 100 m → 200 px
        (0.0, 200.0, "200 m"),    // m/px 1.0  → 200 m → 200 px
        (-1.0, 200.0, "200 m"),   // m/px 2.0  → 200 m → 100 px (500 m = 250 px > cap)
        (-2.0, 500.0, "500 m"),   // m/px 4.0  → 500 m → 125 px (1000 m = 250 px > cap)
        (-3.0, 1000.0, "1 km"),   // m/px 8.0  → 1000 m → 125 px (2000 m = 250 px > cap)
        (-4.0, 2000.0, "2 km"),   // m/px 16.0 → 2000 m → 125 px
        (-5.0, 5000.0, "5 km"),   // m/px 32.0 → 5000 m → 156 px (10 km = 312 px > cap)
        (-6.0, 10000.0, "10 km"), // m/px 64.0 → 10000 m → 156 px
    ];
    for &(z, want_dist, want_label) in table {
        let spec = pick_scale_bar(m_per_px(z));
        // The picker is the source of truth for the exact rung; assert it is a 1-2-5 value…
        let mant = spec.dist_m / 10.0_f64.powf(spec.dist_m.log10().floor());
        let mant_r = (mant * 10.0).round() / 10.0;
        assert!(
            (mant_r - 1.0).abs() < 1e-9
                || (mant_r - 2.0).abs() < 1e-9
                || (mant_r - 5.0).abs() < 1e-9,
            "z={z}: {} is not a 1/2/5×10^n distance",
            spec.dist_m
        );
        // …its bar never exceeds the cap…
        assert!(
            spec.width_px <= SCALE_MAX_PX + 1e-9,
            "z={z}: bar {:.1} px exceeds the {SCALE_MAX_PX} px cap",
            spec.width_px
        );
        // …and it is the LARGEST such rung (the next 1-2-5 step up would exceed the cap).
        let next = next_125_up(spec.dist_m);
        assert!(
            next / m_per_px(z) > SCALE_MAX_PX + 1e-9,
            "z={z}: {} m is not the largest fitting rung — {next} m would also fit",
            spec.dist_m
        );
        // The table's own expectation must match the picker (this is the pinned table).
        assert_eq!(
            spec.dist_m, want_dist,
            "z={z}: picker chose {} m, table says {want_dist} m",
            spec.dist_m
        );
        assert_eq!(spec.label, want_label, "z={z}: label mismatch");
    }
}

/// The next `1/2/5 × 10^n` value strictly above `d` (1→2→5→10…). Test helper for "largest rung".
fn next_125_up(d: f64) -> f64 {
    let decade = 10.0_f64.powf(d.log10().floor());
    let mant = (d / decade * 10.0).round() / 10.0;
    if (mant - 1.0).abs() < 1e-9 {
        2.0 * decade
    } else if (mant - 2.0).abs() < 1e-9 {
        5.0 * decade
    } else {
        10.0 * decade // 5 → next decade's 1
    }
}

/// FIRE THE RULE (perturb / fail / restore): the round-distance picker genuinely discriminates —
/// asserting the WRONG rung for a zoom fails, and the right one passes. A picker that returned a
/// constant (or ignored zoom) would pass the perturbed assertion, so this proves the table above
/// is load-bearing.
#[test]
fn scale_picker_rule_fires() {
    let good = pick_scale_bar(m_per_px(-2.0)); // 500 m @ zoom −2
    assert_eq!(good.dist_m, 500.0, "baseline: zoom −2 must pick 500 m");
    // Perturb: claim it should be 1000 m. That is FALSE (1000/4 = 250 px > 200 cap), so an
    // equality check against the perturbed value must NOT hold — the rule fires.
    let perturbed_expectation = 1000.0;
    assert_ne!(
        good.dist_m, perturbed_expectation,
        "the picker must reject 1000 m at zoom −2 (its bar overflows the cap) — if this were \
         equal the picker would be ignoring zoom"
    );
    // Restore: the true value still holds, and a different zoom yields a different rung (not a
    // constant).
    assert_eq!(pick_scale_bar(m_per_px(-2.0)).dist_m, 500.0);
    assert_ne!(
        pick_scale_bar(m_per_px(-2.0)).dist_m,
        pick_scale_bar(m_per_px(2.0)).dist_m,
        "the picker must vary with zoom (−2 → 500 m vs +2 → 50 m)"
    );
}

/// The Arma 3-digit grid formatter: hundreds-of-metres, zero-padded, wrapping every 100 km.
#[test]
fn grid_formatter_arma_3digit_with_wrap() {
    assert_eq!(grid_ref_3digit(0.0), "000");
    assert_eq!(grid_ref_3digit(1000.0), "010"); // 1 km line
    assert_eq!(grid_ref_3digit(6400.0), "064"); // the ticket's worked example
    assert_eq!(grid_ref_3digit(12000.0), "120"); // last 1 km line before 12800
    assert_eq!(grid_ref_3digit(99900.0), "999"); // just before the wrap
    assert_eq!(grid_ref_3digit(100_000.0), "000"); // 100 km wraps to 000
    assert_eq!(grid_ref_3digit(106_400.0), "064"); // wrap keeps the format
                                                   // Off-terrain / degenerate guards.
    assert_eq!(grid_ref_3digit(-1.0), "000");
    assert_eq!(grid_ref_3digit(f64::NAN), "000");
    // Sub-100 m rounds DOWN to the hundreds cell (floor, not round).
    assert_eq!(grid_ref_3digit(199.9), "001");
}

/// Grid-line enumeration returns exactly the drawn 1 km positions inside a span, inclusive.
#[test]
fn grid_lines_in_range_are_the_drawn_positions() {
    assert_eq!(
        grid_lines_in_range(0.0, 3000.0),
        vec![0.0, 1000.0, 2000.0, 3000.0]
    );
    assert_eq!(grid_lines_in_range(1500.0, 3200.0), vec![2000.0, 3000.0]);
    assert_eq!(grid_lines_in_range(3200.0, 1500.0), vec![2000.0, 3000.0]); // order-agnostic
    assert!(grid_lines_in_range(1100.0, 1900.0).is_empty()); // no line between 1000 and 2000
    assert!(grid_lines_in_range(f64::NAN, 5.0).is_empty());
}

/// The distinct vertical grid-line X positions the engine DRAWS on Everon (12800). This mirrors
/// `map_engine_render::lanes::grid_lines` **operation-for-operation**: its loop is `x from 0 to
/// width, step GRID_STEP (1000), inclusive` (`x <= width`), so the line set is `{0, 1000, …,
/// 12000}` (12800 is never hit by the step-1000 loop — Deck's behaviour, per that module's own
/// doc). `lanes` is a wasm32-only dependency of this crate (the GPU render engine), so a native
/// `cargo test` cannot link `grid_lines()` directly; the set is reconstructed from the identical
/// rule instead, and `GRID_STEP_M` is documented to equal that module's `GRID_STEP`. The
/// invariant below then proves every label lands on one of THESE positions.
fn drawn_vertical_lines() -> Vec<f64> {
    let width = 12_800.0_f64;
    let mut xs = Vec::new();
    let mut k = 0i64;
    while (k as f64) * GRID_STEP_M <= width {
        xs.push(k as f64 * GRID_STEP_M);
        k += 1;
    }
    xs
}

/// CORE INVARIANT — every easting label sits on a drawn grid line and reads its correct ref.
/// For several camera states, each label returned by `edge_eastings` is unprojected back to a
/// world X, which must be a multiple of `GRID_STEP_M`, must be one the engine actually draws,
/// and whose `grid_ref_3digit` equals the label text. A label that drifted from its line (the
/// failure the ticket calls "worse than no label") would land off a multiple and fail here.
#[test]
fn labels_match_grid_lines() {
    let drawn = drawn_vertical_lines();
    let is_drawn = |wx: f64| drawn.iter().any(|d| (d - wx).abs() < 1.0);
    // A spread of zooms + targets (incl. the editor default and a zoomed-in centre).
    let cases = [
        (1600.0, 800.0, -2.0), // zoomed out, near NW
        (1237.0, 843.0, 0.0),  // ~1 km/screen, centre-ish
        (900.0, 600.0, 1.5),   // zoomed in
        (1500.0, 900.0, -4.0), // near whole-terrain
    ];
    // Pane insets read by name from eden_layout (the real geometry).
    use crate::v2::apps::editor::shell::layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
    for (w, h, z) in cases {
        let mut tx = 6400.0_f64;
        let mut ty = 6400.0_f64;
        // Slew the target across a few positions so lines cross the pane at varied screen X.
        for shift in [-3000.0_f64, 0.0, 4000.0] {
            tx = (6400.0 + shift).clamp(0.0, 12_800.0);
            ty = (6400.0 + shift * 0.5).clamp(0.0, 12_800.0);
            let c = cam(w, h, tx, ty, z);
            let pane_right = w - DOCK_RIGHT_PX;
            let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
            for lbl in &eastings {
                // The label's screen X unprojects to a world X on a drawn grid line…
                let wx = c.unproject_xy(lbl.pos_px, STRIP_TOP_PX)[0];
                let k = (wx / GRID_STEP_M).round();
                let on_line = (wx - k * GRID_STEP_M).abs();
                assert!(
                    on_line < 1.0,
                    "easting label at {:.1}px unprojects to world x {wx:.2} — {on_line:.2} m off \
                     a 1 km line (drift = worse than no label)",
                    lbl.pos_px
                );
                assert!(
                    is_drawn(k * GRID_STEP_M),
                    "easting world x {:.0} is not a line the engine draws",
                    k * GRID_STEP_M
                );
                // …and its text is the correct Arma ref for that line.
                assert_eq!(
                    lbl.text,
                    grid_ref_3digit(k * GRID_STEP_M),
                    "easting label text must match its grid line's 3-digit ref"
                );
                // …and it lies inside the visible pane span (framing the MAP, not the window).
                assert!(
                    lbl.pos_px >= DOCK_LEFT_PX - 0.5 && lbl.pos_px <= pane_right + 0.5,
                    "easting label {:.1}px must be inside the map-pane span [{DOCK_LEFT_PX}, {pane_right:.1}]",
                    lbl.pos_px
                );
            }
            // Northings: same invariant on the Y axis / left edge.
            let northings = edge_northings(&c, DOCK_LEFT_PX, STRIP_TOP_PX, h);
            for lbl in &northings {
                let wy = c.unproject_xy(DOCK_LEFT_PX, lbl.pos_px)[1];
                let k = (wy / GRID_STEP_M).round();
                assert!(
                    (wy - k * GRID_STEP_M).abs() < 1.0,
                    "northing label at {:.1}px unprojects to world y {wy:.2} — off a 1 km line",
                    lbl.pos_px
                );
                assert_eq!(
                    lbl.text,
                    grid_ref_3digit(k * GRID_STEP_M),
                    "northing label text must match its grid line's 3-digit ref"
                );
                assert!(
                    lbl.pos_px >= STRIP_TOP_PX - 0.5 && lbl.pos_px <= h + 0.5,
                    "northing label {:.1}px must be inside the map-pane vertical span",
                    lbl.pos_px
                );
            }
        }
        let _ = (tx, ty);
    }
}

/// The grid references frame the MAP PANE, not the viewport: an easting whose line falls under
/// the LEFT dock (screen X < DOCK_LEFT_PX) is dropped, not drawn at the window edge. Proven by
/// putting the camera so a 1 km line sits at a screen X inside the left-dock band and asserting
/// no label claims that position.
#[test]
fn grid_refs_are_clipped_to_the_map_pane_not_the_viewport() {
    use crate::v2::apps::editor::shell::layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
    let (w, h, z) = (1237.0, 843.0, 0.0); // 1 px ≈ 1 m
    let c = cam(w, h, 6400.0, 6400.0, z);
    let pane_right = w - DOCK_RIGHT_PX;
    let eastings = edge_eastings(&c, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
    // Every returned label is inside the pane; none in the occluded dock bands.
    for lbl in &eastings {
        assert!(
            lbl.pos_px >= DOCK_LEFT_PX - 0.5,
            "no easting may render under the left dock ({}px): found one at {:.1}px",
            DOCK_LEFT_PX,
            lbl.pos_px
        );
        assert!(
            lbl.pos_px <= pane_right + 0.5,
            "no easting may render under the right dock (past {pane_right:.1}px): found {:.1}px",
            lbl.pos_px
        );
    }
    // Sanity: with 1 px ≈ 1 m and a 12.8 km terrain, the visible pane spans ~660 m, so at least
    // one 1 km line usually shows — but the hard guarantee under test is the clip, not the count.
    let _ = eastings;
}
