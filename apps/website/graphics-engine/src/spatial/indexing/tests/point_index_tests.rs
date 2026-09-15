//! Role: point index tests.
//! Position: `spatial/indexing/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::spatial::indexing::point_index::*;

fn brute_rect(xs: &[f32], ys: &[f32], r: (f64, f64, f64, f64)) -> Vec<u32> {
    let (min_x, min_y, max_x, max_y) = r;
    (0..xs.len() as u32)
        .filter(|&i| {
            let (x, y) = (f64::from(xs[i as usize]), f64::from(ys[i as usize]));
            x >= min_x && x <= max_x && y >= min_y && y <= max_y
        })
        .collect()
}

fn brute_nearest(xs: &[f32], ys: &[f32], x: f64, y: f64, radius: f64) -> Option<u32> {
    let r2 = radius * radius;
    let mut best: Option<(f64, u32)> = None;
    for i in 0..xs.len() as u32 {
        let dx = f64::from(xs[i as usize]) - x;
        let dy = f64::from(ys[i as usize]) - y;
        let d2 = dx * dx + dy * dy;
        if d2 <= r2 && best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, i));
        }
    }
    best.map(|(_, i)| i)
}

fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v
}

#[test]
fn matches_brute_force_over_pseudorandom_points() {
    let n = 20_000usize;
    let mut xs = vec![0f32; n];
    let mut ys = vec![0f32; n];
    let mut s: u64 = 0x1234_5678;
    for k in 0..n {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        xs[k] = ((s >> 33) as f64 / (1u64 << 31) as f64 * 12800.0) as f32;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ys[k] = ((s >> 33) as f64 / (1u64 << 31) as f64 * 12800.0) as f32;
    }
    let idx = PointIndex::build(xs.clone(), ys.clone(), 256.0);
    assert_eq!(idx.len(), n);

    for r in [
        (1000.0, 2000.0, 3000.0, 5000.0),
        (0.0, 0.0, 12800.0, 12800.0),
        (6000.0, 6000.0, 6100.0, 6100.0),
        (-500.0, -500.0, 100.0, 100.0),
        (12700.0, 0.0, 13000.0, 500.0),
    ] {
        assert_eq!(
            sorted(idx.pick_rect(r.0, r.1, r.2, r.3)),
            sorted(brute_rect(&xs, &ys, r)),
            "pick_rect {r:?}"
        );
    }

    for (x, y, rad) in [
        (6400.0, 6400.0, 500.0),
        (0.0, 0.0, 300.0),
        (12800.0, 12800.0, 1000.0),
        (3333.0, 9999.0, 50.0),
        (100.0, 100.0, 5.0),
    ] {
        assert_eq!(
            idx.pick_nearest(x, y, rad),
            brute_nearest(&xs, &ys, x, y, rad),
            "pick_nearest ({x},{y},{rad})"
        );
    }
}

#[test]
fn empty_index() {
    let idx = PointIndex::build(Vec::new(), Vec::new(), 100.0);
    assert!(idx.is_empty());
    assert!(idx.pick_rect(0.0, 0.0, 1.0, 1.0).is_empty());
    assert_eq!(idx.pick_nearest(0.0, 0.0, 10.0), None);
}
