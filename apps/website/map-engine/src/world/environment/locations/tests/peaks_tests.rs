//! Role: peaks tests.
//! Position: `world/environment/locations/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::environment::locations::peaks::*;
use crate::world::terrain::dem::manifest::DemManifest;

fn flat_manifest(w: usize, h: usize) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: (w as f64 - 1.0) * 2.0,
        max_y: (h as f64 - 1.0) * 2.0,
        width_px: w,
        height_px: h,
        flip_x: false,
        flip_z: false,
        height_min_m: -10.0,
        height_max_m: 500.0,
    }
}

#[test]
fn synthetic_hill_finds_peak() {
    let w = 21;
    let h = 21;
    let mut m = vec![100.0f32; w * h];
    let cx = 10;
    let cy = 10;
    m[cy * w + cx] = 200.0;
    let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
    assert!(!peaks.is_empty());
    let top = peaks.iter().max_by_key(|p| p.value_m).unwrap();
    assert_eq!(top.value_m, 200);
}

#[test]
fn sea_cells_excluded() {
    let w = 21;
    let h = 21;
    let m = vec![-5.0f32; w * h];
    let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
    assert!(peaks.is_empty());
}

#[test]
fn declutter_respects_sep_and_cap() {
    let labels: Vec<HeightLabel> = (0..60)
        .map(|i| HeightLabel {
            x: f64::from(i * 10),
            y: 0.0,
            value_m: 300 - i,
            kind: HeightLabelKind::Peak,
            name: None,
        })
        .collect();
    let z = 0.0;
    let out = declutter_height_labels(&labels, z);
    assert!(out.len() <= PEAK_LABEL_MAX);
    assert!(declutter_invariant_holds(&out, z));
    assert_eq!(out[0].value_m, 300);
}

#[test]
fn min_sep_scales_with_zoom() {
    assert!((height_label_min_sep_m(0.0) - HEIGHT_LABEL_SCREEN_PITCH_PX).abs() < 1e-9);
    assert!((height_label_min_sep_m(0.0) - 150.0).abs() < 1e-9);
    assert!((height_label_min_sep_m(-1.0) - 300.0).abs() < 1e-9);
    assert!((height_label_min_sep_m(2.0) - 37.5).abs() < 1e-9);

    for &z in &[-2.0_f64, -1.0, 0.0, 1.5, 3.0] {
        assert!((height_label_min_sep_m(z) - height_label_screen_sep_world_m(z)).abs() < 1e-12);
    }
}

#[test]
fn screen_space_density_holds_across_zoom() {
    let field: Vec<HeightLabel> = (0..50)
        .flat_map(|i| {
            (0..50).map(move |j| HeightLabel {
                x: f64::from(i) * 60.0,
                y: f64::from(j) * 60.0,
                value_m: 400 - (i + j),
                kind: HeightLabelKind::Peak,
                name: None,
            })
        })
        .collect();

    let pitch_px = HEIGHT_LABEL_SCREEN_PITCH_PX;
    for &z in &[-2.0_f64, -1.0, 0.0, 1.0] {
        let px_per_m = 2f64.powf(z);
        let kept = declutter_height_labels(&field, z);
        assert!(kept.len() >= 2, "z={z}: need pairs to measure density");

        let mut min_screen = f64::MAX;
        for (a, b) in kept
            .iter()
            .enumerate()
            .flat_map(|(i, a)| kept.iter().skip(i + 1).map(move |b| (a, b)))
        {
            min_screen = min_screen.min(dist_m(a, b) * px_per_m);
        }
        assert!(
            min_screen >= pitch_px - 1e-6,
            "z={z}: min screen sep {min_screen:.1}px < Eden pitch {pitch_px}px"
        );

        let sep_m = height_label_min_sep_m(z);
        assert!(
            (sep_m * px_per_m - pitch_px).abs() < 1e-6,
            "z={z}: sep {sep_m:.1}m ×{px_per_m}px/m ≠ {pitch_px}px screen pitch"
        );

        const VP_W_PX: f64 = 1280.0;
        const VP_H_PX: f64 = 720.0;
        let (vp_w_m, vp_h_m) = (VP_W_PX / px_per_m, VP_H_PX / px_per_m);

        let (x0, y0) = (0.0_f64, 0.0);
        let in_vp = kept
            .iter()
            .filter(|p| p.x >= x0 && p.x <= x0 + vp_w_m && p.y >= y0 && p.y <= y0 + vp_h_m)
            .count();

        let field_span = 49.0 * 60.0;
        let cells = (vp_w_m.min(field_span) / sep_m) * (vp_h_m.min(field_span) / sep_m);
        eprintln!(
            "T-641 density  z={z:>4}  m/px={:>6.3}  sep={sep_m:>6.1}m={:>5.1}px  \
                 kept(total)={:>3}  in 1280×720 VP corner={:>3}  (ideal grid cells≈{cells:.0})",
            1.0 / px_per_m,
            sep_m * px_per_m,
            kept.len(),
            in_vp,
        );
    }

    assert!(
        (height_label_min_sep_m(-2.0) - height_label_min_sep_m(1.0)).abs() > 1.0,
        "sep must vary with zoom (screen-space), not be a fixed world constant"
    );
}

#[test]
fn density_rule_fires_and_restores() {
    let field: Vec<HeightLabel> = (0..40)
        .map(|i| HeightLabel {
            x: f64::from(i) * 50.0,
            y: 0.0,
            value_m: 300 - i,
            kind: HeightLabelKind::Peak,
            name: None,
        })
        .collect();
    let z = 0.0;

    let with_rule = declutter_height_labels(&field, z).len();

    let keep_all = field.len().min(PEAK_LABEL_MAX);
    assert!(
        with_rule < keep_all,
        "density rule must thin the field: rule kept {with_rule}, keep-all {keep_all}"
    );

    assert!(declutter_invariant_holds(
        &declutter_height_labels(&field, z),
        z
    ));
}

#[test]
fn declutter_is_deterministic() {
    let field: Vec<HeightLabel> = (0..30)
        .map(|i| HeightLabel {
            x: f64::from(i) * 40.0,
            y: f64::from((i * 7) % 13) * 40.0,
            value_m: 250 + (i * 3) % 17,
            kind: HeightLabelKind::Peak,
            name: None,
        })
        .collect();
    let z = -1.0;
    let a = declutter_height_labels(&field, z);
    let b = declutter_height_labels(&field, z);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!((x.x, x.y, x.value_m), (y.x, y.y, y.value_m));
    }

    let top = field.iter().map(|p| p.value_m).max().unwrap();
    assert!(a.iter().any(|p| p.value_m == top), "top peak always kept");
}

#[test]
fn zoom_band_gates_height_labels() {
    const DEFAULT_ZOOM: f64 = -2.0;
    assert_eq!(
        DEFAULT_ZOOM, HEIGHT_LABEL_MIN_ZOOM,
        "default zoom sits on the lower band edge"
    );
    assert!(
        should_draw_height_label(DEFAULT_ZOOM),
        "default zoom -2.0 must show labels (boundary-inclusive gate)"
    );
    assert!(should_draw_height_label(HEIGHT_LABEL_MIN_ZOOM));
    assert!(should_draw_height_label(HEIGHT_LABEL_MAX_ZOOM));
    assert!(should_draw_height_label(0.0));
    assert!(!should_draw_height_label(HEIGHT_LABEL_MIN_ZOOM - 0.01));
    assert!(!should_draw_height_label(HEIGHT_LABEL_MAX_ZOOM + 0.01));

    let labels = vec![HeightLabel {
        x: 0.0,
        y: 0.0,
        value_m: 300,
        kind: HeightLabelKind::Peak,
        name: None,
    }];
    assert!(declutter_height_labels(&labels, -6.0).is_empty());
    assert!(declutter_height_labels(&labels, 4.0).is_empty());
    assert_eq!(
        declutter_height_labels(&labels, DEFAULT_ZOOM).len(),
        1,
        "default zoom -2.0 keeps its labels through the declutter path"
    );
    assert_eq!(declutter_height_labels(&labels, 0.0).len(), 1);
}

#[test]
fn value_floor_drops_sub_80_knolls() {
    let w = 41;
    let h = 41;
    let mut m = vec![30.0f32; w * h];
    m[10 * w + 10] = 90.0;
    m[30 * w + 30] = 55.0;
    let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
    assert_eq!(peaks.len(), 1, "only the ≥80 peak survives the value floor");
    assert_eq!(peaks[0].value_m, 90);
    assert!(peaks.iter().all(|p| p.value_m >= PEAK_MIN_VALUE_M));
    assert!(peaks.iter().all(|p| p.name.is_none()));
}

#[test]
fn named_label_packs_name_and_value() {
    let labels = vec![
        HeightLabel {
            x: 100.0,
            y: 200.0,
            value_m: 372,
            kind: HeightLabelKind::Peak,
            name: Some("Highstone".to_string()),
        },
        HeightLabel {
            x: 300.0,
            y: 400.0,
            value_m: 210,
            kind: HeightLabelKind::Peak,
            name: None,
        },
    ];
    let specs = height_labels_to_specs(&labels);
    assert_eq!(specs[0].text, "Highstone - 372 m");
    assert_eq!(specs[1].text, "210");
}

#[cfg(feature = "world")]
#[test]
fn everon_peaks_max_above_350() {
    use crate::world::terrain::dem::png::decode_png_to_meters;
    use std::path::PathBuf;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/map-assets/everon/dem/everon-dem-16bit.png");
    if !root.exists() {
        return;
    }
    let bytes = std::fs::read(&root).expect("read dem");
    const MIN_M: f64 = -204.78;
    const MAX_M: f64 = 375.53;
    let decoded = decode_png_to_meters(&bytes, MIN_M, MAX_M).expect("decode");
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12800.0,
        max_y: 12800.0,
        width_px: decoded.width as usize,
        height_px: decoded.height as usize,
        flip_x: false,
        flip_z: false,
        height_min_m: MIN_M,
        height_max_m: MAX_M,
    };
    let peaks = find_peaks(&decoded.meters, m.width_px, m.height_px, &m);
    assert!(!peaks.is_empty());
    let max_v = peaks.iter().map(|p| p.value_m).max().unwrap();
    assert!(max_v >= 350, "G6: max peak {max_v} < 350");
    let drawn = declutter_height_labels(&peaks, 0.0);
    assert!(drawn.len() <= PEAK_LABEL_MAX);
    assert!(declutter_invariant_holds(&drawn, 0.0));
}
