//! Role: wash.
//! Position: `architecture/los/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::architecture::blueprint::model::tests::room_blueprint;
use crate::architecture::blueprint::model::tests::room_sidecar;
use crate::architecture::blueprint::model::tests::slab;
use crate::architecture::los::wash::*;
use crate::spatial::bvh::tree::tests::Scene;

fn params() -> WashParams {
    WashParams {
        radius_m: 8.0,
        ..WashParams::default()
    }
}

fn washes_for(obs: [f64; 3], extra: &[Scene]) -> Vec<LevelWash> {
    let bp = room_blueprint();
    let sc = room_sidecar(extra);
    level_washes(&bp, &sc, obs, &params())
}

fn ceiling_with_stairwell() -> Vec<Scene> {
    vec![
        slab([0.0, 6.0], [2.95, 3.05], [0.0, 1.0]),
        slab([0.0, 6.0], [2.95, 3.05], [2.0, 6.0]),
        slab([0.0, 1.0], [2.95, 3.05], [1.0, 2.0]),
        slab([2.0, 6.0], [2.95, 3.05], [1.0, 2.0]),
    ]
}

fn interior_lit(w: &LevelWash) -> (usize, usize) {
    let (mut vis, mut total) = (0usize, 0usize);
    for row in 0..w.rows {
        for col in 0..w.cols {
            let c = w.cell_center(col, row);
            if c[0] > 0.2 && c[0] < 5.8 && c[1] > 0.2 && c[1] < 5.8 {
                total += 1;
                if w.at(col, row) == Visibility::Visible {
                    vis += 1;
                }
            }
        }
    }
    (vis, total)
}

#[test]
fn grid_is_an_observer_centred_square_rows_north_first() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    assert_eq!(w.len(), 2);
    for (i, lw) in w.iter().enumerate() {
        assert_eq!(lw.level_index, i);
        assert_eq!((lw.cols, lw.rows), (64, 64));
        assert_eq!(lw.cells.len(), 64 * 64);
        assert!((lw.min_x - -5.0).abs() < 1e-9 && (lw.min_z - -5.0).abs() < 1e-9);
        assert!((lw.max_x - 11.0).abs() < 1e-9 && (lw.max_z - 11.0).abs() < 1e-9);
        assert!((lw.cell_m - 0.25).abs() < 1e-12);
        assert_eq!(lw.obs, [3.0, 1.4, 3.0]);
        assert!((lw.radius_m - 8.0).abs() < 1e-12);
    }
    assert!((w[0].eye_y - 1.0).abs() < 1e-9 && (w[1].eye_y - 4.0).abs() < 1e-9);

    let c00 = w[0].cell_center(0, 0);
    assert!(
        (c00[0] - -4.875).abs() < 1e-9 && (c00[1] - 10.875).abs() < 1e-9,
        "{c00:?}"
    );
    let c_last = w[0].cell_center(63, 63);
    assert!((c_last[0] - 10.875).abs() < 1e-9 && (c_last[1] - -4.875).abs() < 1e-9);

    for row in 0..64 {
        for col in 0..64 {
            let c = w[0].cell_center(col, row);
            assert_eq!(w[0].cell_at(c[0], c[1]), Some((col, row)));
        }
    }
    assert_eq!(w[0].cell_at(-5.1, 0.0), None);
    assert_eq!(w[0].visibility_at(-5.1, 0.0), Visibility::Unknown);
    assert_eq!(w[0].at(64, 0), Visibility::Unknown);
}

#[test]
fn cells_outside_the_radius_are_unknown() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    let g = &w[0];

    assert_eq!(g.at(0, 0), Visibility::Unknown);
    assert_eq!(g.at(63, 63), Visibility::Unknown);

    assert_ne!(g.visibility_at(10.5, 3.0), Visibility::Unknown);

    assert_eq!(g.visibility_at(9.5, 9.5), Visibility::Unknown);
    let (v, h, u) = g.class_counts();
    assert_eq!(v + h + u, g.cells.len());

    assert!(
        u * 100 > g.cells.len() * 15 && u * 100 < g.cells.len() * 30,
        "unknown {u}"
    );
}

#[test]
fn ground_wash_reads_the_room() {
    let w = washes_for([3.0, 1.4, 3.0], &[]);
    let g = &w[0];
    assert_eq!(g.visibility_at(1.5, 1.5), Visibility::Visible, "open room");
    assert_eq!(g.visibility_at(3.0, 7.5), Visibility::Hidden, "north wall");
    assert_eq!(
        g.visibility_at(3.0, -3.0),
        Visibility::Visible,
        "through the window hole (ray crosses the wall at y ≈ 1.2)"
    );
    assert_eq!(
        g.visibility_at(1.0, -3.0),
        Visibility::Hidden,
        "south wall beside the hole"
    );
    assert_eq!(g.visibility_at(-1.0, 3.0), Visibility::Hidden, "west wall");
    let (v, h, _) = g.class_counts();
    assert!(v > 0 && h > 0);
}

#[test]
fn upstairs_wash_visible_only_through_stairwell() {
    let obs = [1.5, 1.4, 1.5];
    let with = washes_for(obs, &ceiling_with_stairwell());
    let without = washes_for(obs, &[]);
    let up = &with[1];
    assert_eq!(
        up.visibility_at(1.5, 1.5),
        Visibility::Visible,
        "straight up the stairwell"
    );
    assert_eq!(
        up.visibility_at(5.0, 5.0),
        Visibility::Hidden,
        "ceiling between"
    );
    let (vis, total) = interior_lit(up);
    assert!(vis > 0, "nothing lit through the stairwell");
    assert!(
        vis * 100 < total * 15,
        "{vis}/{total} lit — the ceiling is not occluding"
    );
    assert_eq!(with[0].cells, without[0].cells);
    let (vis, total) = interior_lit(&without[1]);
    assert!(vis * 100 > total * 90, "{vis}/{total} lit with no ceiling");
}

#[test]
fn observer_cell_and_open_air_are_visible() {
    let obs = [-4.0, 1.4, 3.0];
    let w = washes_for(obs, &[]);
    let g = &w[0];
    assert_eq!(
        g.visibility_at(-4.0, 3.0),
        Visibility::Visible,
        "the observer's own cell"
    );
    assert_eq!(g.visibility_at(-3.0, 3.0), Visibility::Visible, "open air");
    assert_eq!(
        g.visibility_at(3.0, 3.0),
        Visibility::Hidden,
        "the west wall between"
    );

    let (col, row) = g.cell_at(-4.0, 3.0).expect("observer cell");
    let c = g.cell_center(col, row);
    let w2 = washes_for([c[0], 1.0, c[1]], &[]);
    assert_eq!(w2[0].visibility_at(c[0], c[1]), Visibility::Visible);
}

#[test]
fn oversized_radius_coarsens_the_cell_to_the_cap() {
    let (min_x, min_z, n, cell) = grid_rect([0.0, 0.0], 1000.0, WASH_CELL_M);
    assert!((min_x - -1000.0).abs() < 1e-9 && (min_z - -1000.0).abs() < 1e-9);
    assert_eq!(n, MAX_WASH_DIM, "the side fills the cap exactly");
    assert!(cell > WASH_CELL_M, "cell {cell}");
    assert!(min_x + n as f64 * cell >= 1000.0 - 1e-9);
    let (_, _, n, cell) = grid_rect([3.0, 3.0], 8.0, WASH_CELL_M);
    assert_eq!(n, 64);
    assert!((cell - WASH_CELL_M).abs() < 1e-12);
}

#[test]
fn level_wash_picks_one_level() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let obs = [3.0, 1.4, 3.0];
    let up = level_wash(&bp, &sc, obs, 1, &params()).expect("level 1");
    assert_eq!(up.level_index, 1);
    assert!((up.eye_y - 4.0).abs() < 1e-9);
    assert!(level_wash(&bp, &sc, obs, 7, &params()).is_none());
    let mut none = room_blueprint();
    none.levels.clear();
    assert!(level_washes(&none, &sc, obs, &params()).is_empty());
}

fn drain(job: &mut WashJob, budget_ms: f64, blocked: &dyn Fn([f64; 3], [f64; 3]) -> bool) -> usize {
    let t = std::cell::Cell::new(0.0f64);
    let now = || {
        t.set(t.get() + 1.0);
        t.get()
    };
    let mut batches = 0usize;
    while !job.done {
        assert!(
            job.step(blocked, budget_ms, &now),
            "a live job must decide cells"
        );
        batches += 1;
        assert!(batches < 1_000_000, "job never finished");
    }
    batches
}

#[test]
fn sliced_wash_is_bit_identical_to_the_sync_path() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let p = params();
    let obs = [1.5, 1.4, 1.5];
    let blocked = |a: [f64; 3], b: [f64; 3]| {
        sc.bvh
            .any_hit(&sc.verts, &sc.tris, a, b, 0.0, 1.0)
            .is_some()
    };
    let lvl = &bp.levels[1];
    let eye_y = lvl.elevation_range[0] + p.eye_m;
    let sync = wash_band(lvl.level_index, eye_y, obs, &p, blocked);

    let (v, h, u) = sync.class_counts();
    assert!(v > 0 && h > 0 && u > 0, "fixture classes: {v}/{h}/{u}");
    for (i, budget) in [0.0, 3.0, 1e9].into_iter().enumerate() {
        let mut job = WashJob::new(lvl.level_index, eye_y, obs, &p, 3).expect("under the cap");
        let batches = drain(&mut job, budget, &blocked);
        assert_eq!(
            job.wash(),
            &sync,
            "budget {budget}: the sliced wash must equal the synchronous one ({batches} batches)"
        );
        let (done, total) = job.progress();
        assert_eq!(done, total, "budget {budget}: every cell decided");
        assert_eq!(total, sync.cols * sync.rows);
        if i == 0 {
            assert_eq!(
                batches,
                total.div_ceil(WASH_BATCH_CELLS),
                "a zero budget slices to one WASH_BATCH_CELLS batch per step"
            );
        }
    }
}

#[test]
fn wash_job_cancels_mid_disc() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let p = params();
    let obs = [3.0, 1.4, 3.0];
    let blocked = |a: [f64; 3], b: [f64; 3]| {
        sc.bvh
            .any_hit(&sc.verts, &sc.tris, a, b, 0.0, 1.0)
            .is_some()
    };
    let lvl = &bp.levels[0];
    let mut job = WashJob::new(
        lvl.level_index,
        lvl.elevation_range[0] + p.eye_m,
        obs,
        &p,
        9,
    )
    .expect("under the cap");
    assert_eq!(job.generation, 9);
    let now = || 0.0f64;
    assert!(job.step(&blocked, 0.0, &now), "one batch decided");
    let (before, total) = job.progress();
    assert_eq!(before, WASH_BATCH_CELLS);
    assert!(before < total);
    let partial = job.wash().clone();
    job.cancel();
    assert!(job.done);
    assert!(
        !job.step(&blocked, 1e9, &now),
        "a cancelled job decides nothing"
    );
    assert_eq!(
        job.progress().0,
        before,
        "cursor frozen at the cancel point"
    );
    assert_eq!(job.wash(), &partial, "partial raster preserved");
    assert_eq!(partial.cells.len(), partial.cols * partial.rows);
}

#[test]
fn over_cap_wash_radius_is_refused_with_a_message() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[]);
    let obs = [3.0, 1.4, 3.0];

    for r in [WASH_RADIUS_M, 10f64.hypot(10.0) + 5.0, MAX_WASH_RADIUS_M] {
        assert!(wash_cap_check(r).is_ok(), "radius {r} m must be accepted");
    }
    let err = wash_cap_check(1000.0).expect_err("1000 m is over the 400 m cap");
    assert_eq!(err.cap, "building wash radius (m)");
    assert!((err.limit - MAX_WASH_RADIUS_M).abs() < 1e-12);
    assert!((err.measured - 1000.0).abs() < 1e-12);
    let msg = err.to_string();
    assert!(
        msg.contains("building wash radius (m)") && msg.contains("1000") && msg.contains("400"),
        "the refusal must name the cap AND the measured value: {msg}"
    );

    let over = WashParams {
        radius_m: 1000.0,
        ..WashParams::default()
    };
    assert_eq!(
        WashJob::new(0, 1.0, obs, &over, 0)
            .expect_err("job refused")
            .cap,
        "building wash radius (m)"
    );

    let rays = std::cell::Cell::new(0u32);
    let counting = |_: [f64; 3], _: [f64; 3]| {
        rays.set(rays.get() + 1);
        false
    };
    let refused = wash_band(0, 1.0, obs, &over, counting);
    assert_eq!((refused.cols, refused.rows), (0, 0));
    assert!(refused.cells.is_empty());
    assert_eq!(rays.get(), 0, "an over-cap wash must cast no ray");
    assert_eq!(refused.at(0, 0), Visibility::Unknown);
    assert_eq!(refused.visibility_at(obs[0], obs[2]), Visibility::Unknown);
    for w in level_washes(&bp, &sc, obs, &over) {
        assert_eq!((w.cols, w.rows), (0, 0), "level_washes refuses too");
    }
    assert_eq!(
        level_wash(&bp, &sc, obs, 0, &over)
            .expect("level exists")
            .cols,
        0,
        "level_wash refuses too"
    );
}

#[test]
fn wash_timing_envelope() {
    let bp = room_blueprint();
    let sc = room_sidecar(&ceiling_with_stairwell());
    let t0 = std::time::Instant::now();
    let w = level_washes(&bp, &sc, [3.0, 1.4, 3.0], &params());
    let dt = t0.elapsed();
    let rays: usize = w
        .iter()
        .map(|l| {
            let (v, h, _) = l.class_counts();
            v + h
        })
        .sum();
    eprintln!(
        "wash_timing_envelope: {rays} rays in {:.1} ms ({:.2} µs/ray)",
        dt.as_secs_f64() * 1e3,
        dt.as_secs_f64() * 1e6 / rays.max(1) as f64
    );
}
