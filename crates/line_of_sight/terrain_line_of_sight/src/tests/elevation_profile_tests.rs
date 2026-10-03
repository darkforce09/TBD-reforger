//! **Role:** Unit tests of the ground elevation profile along a segment: monotone distances, both
//! endpoints, the step, the sampler's elevations in order and the samples dropped off coverage.
//! **Position:** Test-only child of the module it exercises,
//! [`crate::elevation_profile::sample_segment`] over a synthetic manifest and over
//! [`terrain_elevation::sampling::sample_elevation_meters`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** a profile keeps both endpoints, never exceeds the step between samples and never
//! invents a height for a point the sampler cannot answer.

use crate::elevation_profile::sample_segment;
use terrain_elevation::manifest::DemManifest;
use terrain_elevation::sampling::sample_elevation_meters;

fn synth(span: f64, w: usize, h: usize) -> DemManifest {
    DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: span,
        max_y: span,
        width_px: w,
        height_px: h,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 100.0,
    }
}

#[test]
fn segment_distances_are_monotone_and_end_exact() {
    let m = synth(1000.0, 100, 100);

    let prof = sample_segment(&m, (0.0, 0.0), (300.0, 400.0), 8.0, |_, _| Some(0.0));
    assert!(prof.len() >= 2, "a real segment yields ≥2 samples");

    for w in prof.windows(2) {
        assert!(
            w[1].dist_m > w[0].dist_m,
            "distances must strictly increase: {} then {}",
            w[0].dist_m,
            w[1].dist_m
        );
    }

    assert!((prof.first().unwrap().dist_m - 0.0).abs() < 1e-9);
    assert!(
        (prof.last().unwrap().dist_m - 500.0).abs() < 1e-9,
        "last dist must equal the segment length (500 m), got {}",
        prof.last().unwrap().dist_m
    );
}

#[test]
fn segment_includes_both_endpoints_and_respects_step() {
    let m = synth(1000.0, 100, 100);

    let prof = sample_segment(&m, (10.0, 10.0), (110.0, 10.0), 8.0, |_, _| Some(0.0));
    assert_eq!(prof.len(), 14, "ceil(100/8)=13 intervals → 14 samples");
    assert!(
        (prof[0].dist_m).abs() < 1e-9,
        "first sample at the observer end (0 m)"
    );
    assert!(
        (prof.last().unwrap().dist_m - 100.0).abs() < 1e-9,
        "last sample at the target end (100 m)"
    );

    for w in prof.windows(2) {
        assert!(
            w[1].dist_m - w[0].dist_m <= 8.0 + 1e-9,
            "gap {} exceeds the 8 m step",
            w[1].dist_m - w[0].dist_m
        );
    }
}

#[test]
fn segment_reads_sampler_elevations_in_order() {
    let m = synth(1000.0, 100, 100);

    let prof = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 25.0, |x, _| Some(x));

    let elevs: Vec<f64> = prof.iter().map(|s| s.elev_m).collect();
    assert_eq!(elevs, vec![0.0, 25.0, 50.0, 75.0, 100.0]);

    for s in &prof {
        assert!((s.dist_m - s.elev_m).abs() < 1e-9);
    }
}

#[test]
fn segment_drops_off_coverage_samples() {
    let m = synth(100.0, 50, 50);

    let prof = sample_segment(&m, (50.0, 50.0), (200.0, 50.0), 10.0, |_, _| Some(7.0));
    assert!(
        !prof.is_empty(),
        "the in-coverage head must still yield samples"
    );

    for s in &prof {
        let x = 50.0 + s.dist_m;
        assert!(
            x <= 100.0 + 1e-9,
            "sample at world-x {x} is outside coverage"
        );
        assert_eq!(s.elev_m, 7.0);
    }

    let none_prof = sample_segment(&m, (10.0, 10.0), (90.0, 10.0), 10.0, |_, _| None);
    assert!(
        none_prof.is_empty(),
        "None sampler yields no samples, never 0 m"
    );

    let gone = sample_segment(&m, (150.0, 10.0), (300.0, 10.0), 10.0, |_, _| Some(1.0));
    assert!(gone.is_empty(), "a segment outside coverage yields nothing");
}

#[test]
fn segment_degenerate_length_and_step() {
    let m = synth(1000.0, 100, 100);

    let point = sample_segment(&m, (42.0, 42.0), (42.0, 42.0), 8.0, |_, _| Some(3.0));
    assert_eq!(point.len(), 1);
    assert!((point[0].dist_m).abs() < 1e-9);
    assert_eq!(point[0].elev_m, 3.0);

    let off = sample_segment(&m, (2000.0, 2000.0), (2000.0, 2000.0), 8.0, |_, _| {
        Some(3.0)
    });
    assert!(off.is_empty());

    let two = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 0.0, |_, _| Some(1.0));
    assert_eq!(
        two.len(),
        2,
        "a 0 step degenerates to one interval (both ends)"
    );
    assert!((two[0].dist_m).abs() < 1e-9 && (two[1].dist_m - 100.0).abs() < 1e-9);
}

#[test]
fn segment_composes_with_the_raster_point_sampler() {
    let raster: [u16; 4] = [0, 100, 200, 300];
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 100.0,
        max_y: 100.0,
        width_px: 2,
        height_px: 2,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 300.0,
    };

    let prof = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 50.0, |x, y| {
        sample_elevation_meters(x, y, &m, &raster, 2, 2)
    });

    assert_eq!(prof.len(), 3);

    assert!((prof[0].dist_m).abs() < 1e-9);
    assert!((prof[2].dist_m - 100.0).abs() < 1e-9);
    assert!(
        prof[0].elev_m <= prof[1].elev_m && prof[1].elev_m <= prof[2].elev_m,
        "metres increase along the raster's increasing top row"
    );
}
