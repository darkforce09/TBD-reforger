//! Role: Domain regression cases.
//! Position: `terrain/dem/sample/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn zero_is_exact_min() {
    assert_eq!(uint16_to_meters(0.0, MIN_M, MAX_M), MIN_M);
}

#[test]
fn full_scale_is_max_within_epsilon() {
    assert!((uint16_to_meters(65535.0, MIN_M, MAX_M) - MAX_M).abs() < 1e-10);
}

#[test]
fn meters_cache_matches_scalar_and_stores_f32() {
    let raster: [u16; 5] = [0, 65535, 12345, 54321, 1];
    let out = meters_cache(&raster, MIN_M, MAX_M);
    assert_eq!(out.len(), raster.len());
    for (i, &u) in raster.iter().enumerate() {
        assert_eq!(out[i], uint16_to_meters(f64::from(u), MIN_M, MAX_M) as f32);
    }
}

#[test]
fn world_to_pixel_endpoints() {
    let m = everon(6400, 6400);
    let a = world_to_pixel(0.0, 0.0, &m);
    assert_eq!((a.px, a.py), (0.0, 0.0));
    let b = world_to_pixel(12800.0, 12800.0, &m);
    assert_eq!((b.px, b.py), (6399.0, 6399.0));
}

#[test]
fn world_to_pixel_axis_flip() {
    let mut m = everon(6400, 6400);
    m.flip_x = true;
    m.flip_z = true;
    let a = world_to_pixel(0.0, 0.0, &m);
    assert_eq!((a.px, a.py), (6399.0, 6399.0));
}

#[test]
fn bilinear_2x2_center_is_mean() {
    let raster: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
    let v = bilinear_sample(&raster, 2, 2, 0.5, 0.5);
    assert!((v - 150.0).abs() < 1e-9);
}

#[test]
fn bilinear_u16_and_f32_agree_when_exact() {
    let u: [u16; 4] = [0, 100, 200, 300];
    let f: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
    for (px, py) in [(0.0, 0.0), (0.25, 0.75), (0.9, 0.1)] {
        assert_eq!(
            bilinear_sample(&u, 2, 2, px, py),
            bilinear_sample(&f, 2, 2, px, py)
        );
    }
}

#[test]
fn sample_elevation_out_of_bounds_is_none() {
    let m = everon(6400, 6400);
    let raster = vec![0u16; 64];
    assert!(sample_elevation_meters(-1.0, 0.0, &m, &raster, 6400, 6400).is_none());
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

#[test]
fn viewshed_radial_coverage_classifies_every_cell() {
    let m = flat_world(4000.0);

    let vs = compute_viewshed(
        &m,
        params(1000.0, 1000.0, Some(50.0), 200.0, 8.0),
        |_, _| Some(50.0),
    );

    assert_eq!(vs.cols, 51, "±radius / cell + 1 columns");
    assert_eq!(vs.rows, 51);
    assert_eq!(vs.cells.len(), vs.cols * vs.rows);

    let (v, h, u) = vs.class_counts();
    assert_eq!(v + h + u, vs.cols * vs.rows, "every cell is classified");
    assert_eq!(u, 0, "fully in-coverage flat disc has no Unknown cells");
    assert!(v > 0, "flat terrain has visible cells");

    let (dv, dh, du) = disc_counts(&vs, 200.0);
    assert_eq!(dh, 0, "flat terrain hides nothing WITHIN the sight radius");
    assert_eq!(du, 0, "in-coverage disc has no Unknown cells");
    assert!(dv > 0);
}

#[test]
fn viewshed_flat_terrain_all_visible() {
    let m = flat_world(4000.0);
    let vs = compute_viewshed(
        &m,
        params(2000.0, 2000.0, Some(100.0), 400.0, 8.0),
        |_, _| Some(100.0),
    );

    let (_, hidden, unknown) = disc_counts(&vs, 400.0);
    assert_eq!(
        hidden, 0,
        "flat terrain: nothing is hidden within the radius"
    );
    assert_eq!(
        unknown, 0,
        "flat terrain fully in coverage: nothing unknown"
    );

    let oc = ((vs.obs_x - vs.min_x) / 8.0).round() as usize;
    let orr = ((vs.obs_y - vs.min_y) / 8.0).round() as usize;
    assert_eq!(
        vs.at(oc, orr),
        Visibility::Visible,
        "observer sees its own cell"
    );
}

#[test]
fn viewshed_ridge_casts_a_shadow() {
    let m = flat_world(4000.0);
    let obs = (1000.0, 1000.0);

    let elev = |x: f64, _y: f64| -> Option<f64> {
        if (x - 1200.0).abs() < 4.0 {
            Some(200.0)
        } else {
            Some(10.0)
        }
    };
    let vs = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), elev);

    let front_c = ((1100.0 - vs.min_x) / 8.0).round() as usize;
    let row = ((obs.1 - vs.min_y) / 8.0).round() as usize;
    assert_eq!(
        vs.at(front_c, row),
        Visibility::Visible,
        "ground between observer and the wall is visible"
    );

    let behind_c = ((1400.0 - vs.min_x) / 8.0).round() as usize;
    assert_eq!(
        vs.at(behind_c, row),
        Visibility::Hidden,
        "ground behind the wall is in dead ground"
    );
}

#[test]
fn viewshed_anchors_eye_at_observer_not_first_covered() {
    let m = flat_world(6000.0);
    let obs = (3000.0, 3000.0);

    let elev = move |x: f64, y: f64| -> Option<f64> {
        let d = ((x - obs.0).powi(2) + (y - obs.1).powi(2)).sqrt();
        if d < 1.0 {
            Some(300.0)
        } else if (16.0..80.0).contains(&d) {
            None
        } else {
            Some(100.0)
        }
    };
    let vs = compute_viewshed(&m, params(obs.0, obs.1, Some(300.0), 400.0, 8.0), elev);

    let far_c = ((3200.0 - vs.min_x) / 8.0).round() as usize;
    let row = ((obs.1 - vs.min_y) / 8.0).round() as usize;
    assert_eq!(
        vs.at(far_c, row),
        Visibility::Visible,
        "wave-109: a high observer's descending line sees lower ground past a coverage hole \
             (eye anchored at the OBSERVER, not the first covered sample)"
    );

    let hole_c = ((obs.0 + 40.0 - vs.min_x) / 8.0).round() as usize;
    assert_eq!(
        vs.at(hole_c, row),
        Visibility::Unknown,
        "an off-coverage cell is Unknown (rendered hidden), never a fabricated Visible"
    );
}

#[test]
fn viewshed_observer_off_coverage_is_all_unknown() {
    let m = flat_world(1000.0);

    let vs = compute_viewshed(&m, params(2000.0, 2000.0, None, 200.0, 8.0), |_, _| {
        Some(50.0)
    });
    let (v, h, u) = vs.class_counts();
    assert_eq!(v, 0, "off-coverage observer: nothing visible");
    assert_eq!(h, 0, "off-coverage observer: nothing hidden");
    assert_eq!(u, vs.cols * vs.rows, "off-coverage observer: all Unknown");
}

#[test]
fn viewshed_ridge_shadow_rule_fires() {
    let m = flat_world(4000.0);
    let obs = (1000.0, 1000.0);
    let behind_c_of = |vs: &Viewshed| ((1400.0 - vs.min_x) / 8.0).round() as usize;
    let row_of = |vs: &Viewshed| ((obs.1 - vs.min_y) / 8.0).round() as usize;

    let walled = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), |x, _| {
        if (x - 1200.0).abs() < 4.0 {
            Some(200.0)
        } else {
            Some(10.0)
        }
    });
    let (bc, br) = (behind_c_of(&walled), row_of(&walled));
    assert_eq!(
        walled.at(bc, br),
        Visibility::Hidden,
        "baseline: the wall shadows the ground behind it"
    );

    assert_ne!(
        walled.at(bc, br),
        Visibility::Visible,
        "the cell behind the wall MUST be hidden — if Visible, the viewshed is ignoring terrain"
    );

    let flat = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), |_, _| {
        Some(10.0)
    });
    assert_eq!(
        flat.at(bc, br),
        Visibility::Visible,
        "removing the wall restores a clear sight to the cell behind where it stood"
    );

    assert_ne!(
        walled.at(bc, br),
        flat.at(bc, br),
        "the viewshed verdict must depend on the terrain"
    );
}

#[test]
fn viewshed_composes_with_the_raster_point_sampler() {
    let raster = [100u16; 16];
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 300.0,
        max_y: 300.0,
        width_px: 4,
        height_px: 4,
        flip_x: false,
        flip_z: false,
        height_min_m: 0.0,
        height_max_m: 300.0,
    };
    let ground = uint16_to_meters(100.0, 0.0, 300.0);
    let vs = compute_viewshed(
        &m,
        params(150.0, 150.0, Some(ground), 100.0, 8.0),
        |x, y| sample_elevation_meters(x, y, &m, &raster, 4, 4),
    );

    let (v, h, _u) = disc_counts(&vs, 100.0);
    assert!(v > 0, "raster-backed flat plain yields visible cells");
    assert_eq!(h, 0, "a flat raster hides nothing within the radius");
}

#[test]
fn viewshed_perf_default_radius_is_reported() {
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12_800.0,
        max_y: 12_800.0,
        width_px: 6400,
        height_px: 6400,
        flip_x: false,
        flip_z: false,
        height_min_m: -204.78,
        height_max_m: 375.53,
    };

    let elev = |x: f64, _y: f64| -> Option<f64> { Some(x * 0.01) };
    let p = params(6400.0, 6400.0, Some(64.0), VIEWSHED_DEFAULT_RADIUS_M, 8.0);
    let t0 = std::time::Instant::now();
    let vs = compute_viewshed(&m, p, elev);
    let dt = t0.elapsed();
    let (v, h, u) = vs.class_counts();
    eprintln!(
        "T-644 viewshed perf: {:.2} ms | {}×{} raster ({} cells) | visible {} hidden {} unknown {}",
        dt.as_secs_f64() * 1000.0,
        vs.cols,
        vs.rows,
        vs.cols * vs.rows,
        v,
        h,
        u
    );

    assert_eq!(v + h + u, vs.cols * vs.rows);
    assert!(
        vs.cols >= 500 && vs.rows >= 500,
        "2000 m / 8 m ≈ 501-cell radius disc"
    );
}

#[test]
fn sliced_viewshed_is_bit_identical_to_the_sync_path() {
    let (m, p, elev) = sliced_fixture();
    let sync = compute_viewshed(&m, p, &elev);

    let (v, h, u) = sync.class_counts();
    assert!(
        v > 0 && h > 0 && u > 0,
        "fixture must produce visible/hidden/unknown: {v}/{h}/{u}"
    );
    for (i, budget) in [0.0, 3.0, 1e9].into_iter().enumerate() {
        let mut job = ViewshedJob::new(&m, p, 7).expect("under the cell cap");
        let batches = drain(&mut job, budget, &elev);
        let sliced = job.raster();
        assert_eq!(
            (sliced.cols, sliced.rows),
            (sync.cols, sync.rows),
            "budget {budget}: dims"
        );
        assert_eq!(
            (sliced.min_x, sliced.min_y, sliced.max_x, sliced.max_y),
            (sync.min_x, sync.min_y, sync.max_x, sync.max_y),
            "budget {budget}: world rect"
        );
        assert_eq!(
            sliced.cells, sync.cells,
            "budget {budget}: every cell must match the synchronous march ({batches} batches)"
        );
        let (rays, total) = job.progress();
        assert_eq!(rays, total, "budget {budget}: every ray marched");
        if i == 0 {
            assert_eq!(batches, total, "a zero budget slices to one ray per batch");
        }
    }
}

#[test]
fn sliced_viewshed_matches_the_off_coverage_opening() {
    let m = flat_world(600.0);
    let p = params(300.0, 300.0, None, 200.0, 8.0);
    let sync = compute_viewshed(&m, p, |_, _| Some(0.0));
    let job = ViewshedJob::new(&m, p, 1).expect("under the cell cap");
    assert!(job.done, "no honest eye — nothing to march");
    assert_eq!(job.raster().cells, sync.cells);
    assert_eq!(
        (job.raster().cols, job.raster().rows),
        (sync.cols, sync.rows)
    );
}

#[test]
fn viewshed_job_cancels_mid_disc() {
    let (m, p, elev) = sliced_fixture();
    let mut job = ViewshedJob::new(&m, p, 42).expect("under the cell cap");
    assert_eq!(job.generation, 42);
    let now = || 0.0f64;
    assert!(job.step(&elev, 0.0, &now), "one ray marched");
    let (before, total) = job.progress();
    assert!(before > 0 && before < total);
    let partial = job.raster().clone();
    job.cancel();
    assert!(job.done);
    assert!(
        !job.step(&elev, 1e9, &now),
        "a cancelled job marches nothing"
    );
    assert_eq!(
        job.progress().0,
        before,
        "cursor frozen at the cancel point"
    );
    assert_eq!(
        job.raster().cells,
        partial.cells,
        "partial raster preserved"
    );

    assert_eq!(partial.cells.len(), partial.cols * partial.rows);
}

#[test]
fn over_cap_viewshed_is_refused_with_a_message() {
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 12_800.0,
        max_y: 12_800.0,
        width_px: 6400,
        height_px: 6400,
        flip_x: false,
        flip_z: false,
        height_min_m: -204.78,
        height_max_m: 375.53,
    };

    let shipped = params(6400.0, 6400.0, Some(64.0), VIEWSHED_DEFAULT_RADIUS_M, 8.0);
    let g = viewshed_grid(&m, shipped);
    assert_eq!((g.cols, g.rows), (501, 501));
    assert_eq!(g.cell_count(), 251_001);
    assert!(
        g.cap_check().is_ok(),
        "the shipped 2000 m / 8 m default must NOT be refused"
    );
    assert!(ViewshedJob::new(&m, shipped, 0).is_ok());

    let huge = params(6400.0, 6400.0, Some(64.0), VIEWSHED_DEFAULT_RADIUS_M, 1.0);
    let hg = viewshed_grid(&m, huge);
    assert_eq!(hg.cell_count(), 4001 * 4001);
    let err = hg.cap_check().expect_err("16 M cells is over the cap");
    assert_eq!(err.cap, "terrain viewshed cells");
    assert!((err.limit - MAX_VIEWSHED_CELLS as f64).abs() < 1e-9);
    assert!((err.measured - (4001.0 * 4001.0)).abs() < 1e-9);
    let msg = err.to_string();
    assert!(
        msg.contains("terrain viewshed cells")
            && msg.contains("16008001")
            && msg.contains("300000"),
        "the refusal must name the cap AND the measured value: {msg}"
    );
    assert_eq!(
        ViewshedJob::new(&m, huge, 0).expect_err("job refused").cap,
        "terrain viewshed cells"
    );

    let refused = compute_viewshed(&m, huge, |_, _| Some(0.0));
    assert_eq!(
        (refused.cols, refused.rows, refused.cells.len()),
        (0, 0, 0),
        "an over-cap compute_viewshed is refused, not run"
    );
}
