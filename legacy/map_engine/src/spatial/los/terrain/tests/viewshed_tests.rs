//! **Role:** Unit tests of the one-call terrain viewshed: every cell classified, flat ground all
//! visible, a ridge casting dead ground, the eye anchored at the observer, the off-coverage
//! observer, the raster-backed sampler, the default radius timing and the cell cap refusal.
//! **Position:** Test-only child of `crate::spatial::los::terrain`, exercising
//! [`crate::spatial::los::terrain::viewshed`] (and the cap check of
//! [`crate::spatial::los::terrain::scheduler::ViewshedJob`]); inputs from `viewshed_fixtures.rs`.
//! **Signals & state:** none; pure functions.
//! **Invariants:** an off-coverage cell is `Unknown`, never a fabricated `Visible`; a raster over
//! `MAX_VIEWSHED_CELLS` is refused while the 2000 m / 8 m default is not.

use super::viewshed_fixtures::{flat_world, params};
use crate::spatial::los::terrain::scheduler::ViewshedJob;
use crate::spatial::los::terrain::viewshed::{
    MAX_VIEWSHED_CELLS, VIEWSHED_DEFAULT_RADIUS_M, Viewshed, Visibility, compute_viewshed,
    viewshed_grid,
};
use crate::world::terrain::dem::manifest::DemManifest;
use crate::world::terrain::dem::sampling::{sample_elevation_meters, uint16_to_meters};

fn disc_counts(vs: &Viewshed, radius: f64) -> (usize, usize, usize) {
    let (mut v, mut h, mut u) = (0usize, 0usize, 0usize);
    for r in 0..vs.rows {
        for c in 0..vs.cols {
            let x = vs.min_x + c as f64 * 8.0;
            let y = vs.min_y + r as f64 * 8.0;
            if ((x - vs.obs_x).powi(2) + (y - vs.obs_y).powi(2)).sqrt() > radius {
                continue;
            }
            match vs.at(c, r) {
                Visibility::Visible => v += 1,
                Visibility::Hidden => h += 1,
                Visibility::Unknown => u += 1,
            }
        }
    }
    (v, h, u)
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
