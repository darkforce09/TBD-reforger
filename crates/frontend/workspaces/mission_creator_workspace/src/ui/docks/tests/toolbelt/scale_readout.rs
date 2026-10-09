use super::{format_m_per_px, m_per_px, pick_scale_bar};
use camera_math::ortho::state::MAX_ZOOM;
use camera_math::ortho::state::MIN_ZOOM;

/// The readout across the whole zoom clamp, at the real rungs the operator sees. `MIN_ZOOM −6`
/// is whole-Everon (64 m/px), `−2` the editor default (4 m/px), `0` unity, `MAX_ZOOM 6` the
/// close-inspection ceiling (0.0156 m/px). Three significant figures throughout — including
/// below 0.1 m/px, where a fixed 3-decimal format would have dropped to two.
#[test]
fn readout_table_across_the_zoom_clamp() {
    let cases = [
        (MIN_ZOOM, "64.0 m/px"),
        (-4.0, "16.0 m/px"),
        (-2.0, "4.00 m/px"),
        (-1.0, "2.00 m/px"),
        (0.0, "1.00 m/px"),
        (2.0, "0.250 m/px"),
        (4.0, "0.0625 m/px"),
        (MAX_ZOOM, "0.0156 m/px"),
    ];
    for (z, want) in cases {
        let got = format_m_per_px(m_per_px(z));
        assert_eq!(got, want, "zoom {z} must read {want}, got {got}");
    }
    // A degenerate scale reads as the same em-dash "no value" the other cells use — never NaN,
    // never `inf`, on the operator's screen.
    for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(
            format_m_per_px(bad).starts_with('\u{2014}'),
            "degenerate m/px {bad} must render the em-dash cell, not a raw float"
        );
    }
    // T-756 (MINOR-4): a non-finite *zoom* must also hit the em-dash — `m_per_px` used to
    // fabricate 1.0 and print a confident "1.00 m/px".
    for bad_z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let got = format_m_per_px(m_per_px(bad_z));
        assert!(
            got.starts_with('\u{2014}'),
            "non-finite zoom {bad_z} must render the em-dash cell, got {got}"
        );
    }
    // T-756 (MINOR-4): band-top carry must not print four significant figures.
    assert_eq!(format_m_per_px(9.996), "10.0 m/px");
    assert_eq!(format_m_per_px(99.96), "100 m/px");
    assert_eq!(format_m_per_px(0.09996), "0.100 m/px");
}

/// The readout is MONOTONE in zoom: zooming in never prints a larger metres-per-pixel. A
/// formatter that rounded into a non-monotone sequence would make the number lie about the
/// direction of a gesture, which is worse than printing nothing.
#[test]
fn readout_never_goes_backwards_as_you_zoom_in() {
    let mut prev = f64::INFINITY;
    let mut z = MIN_ZOOM;
    while z <= MAX_ZOOM {
        let shown: f64 = format_m_per_px(m_per_px(z))
            .trim_end_matches(" m/px")
            .parse()
            .expect("the readout must be a parseable number plus its unit");
        assert!(
            shown <= prev,
            "zoom {z}: printed {shown} m/px after {prev} — the readout must not increase as \
             you zoom IN"
        );
        prev = shown;
        z += 0.25;
    }
}

/// **Reconciliation with T-667 (wave 106).** One scale, two surfaces: the graphic bar and this
/// number must be the same measurement. Given the same `m_per_px`, the bar's chosen ground
/// distance and the printed number are consistent — the bar is `dist_m / m_per_px` px long,
/// which is exactly what the printed number says it should be.
#[test]
fn the_bar_and_the_number_describe_the_same_scale() {
    let mut z = MIN_ZOOM;
    while z <= MAX_ZOOM {
        let mpp = m_per_px(z);
        let spec = pick_scale_bar(mpp);
        let shown: f64 = format_m_per_px(mpp)
            .trim_end_matches(" m/px")
            .parse()
            .expect("parseable readout");
        // Measuring the drawn bar with the printed scale recovers its labelled distance to
        // within the readout's own display precision (≤ 0.5%).
        let measured = spec.width_px * shown;
        assert!(
            (measured - spec.dist_m).abs() <= spec.dist_m * 0.0051,
            "zoom {z}: a {:.1} px bar read at {shown} m/px measures {measured} m, but is \
             labelled {} m — the two scale surfaces disagree",
            spec.width_px,
            spec.dist_m
        );
        z += 0.125;
    }
}
