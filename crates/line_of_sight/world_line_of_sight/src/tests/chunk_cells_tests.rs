//! Unit tests of the chunk walk against its brute-force rasteriser.
//!
//! **Role:** checks [`crate::chunk_cells::cells_on_segment`] against a rasteriser that tests every
//! cell of the grid, over seeded random segments and the axis, point, off-terrain and diagonal
//! cases.
//! **Position:** test-only child of [`crate::chunk_cells`].
//! **Signals & state:** none; a seeded generator.
//! **Invariants:** the walk and the rasteriser agree cell for cell and in order.

use crate::chunk_cells::cells_on_segment;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

/// The brute-force rasteriser: every cell whose square the segment touches, ordered by the parametric entry of the segment into the cell (ties by `(cx, cy)`).
fn cells_on_segment_reference(
    a: [f64; 2],
    b: [f64; 2],
    cell_m: f64,
    cols: i64,
    rows: i64,
) -> Vec<(i64, i64)> {
    let mut hits: Vec<(f64, i64, i64)> = Vec::new();
    for cy in 0..rows {
        for cx in 0..cols {
            let lo = [cx as f64 * cell_m, cy as f64 * cell_m];
            let hi = [(cx + 1) as f64 * cell_m, (cy + 1) as f64 * cell_m];

            let mut t0 = 0.0f64;
            let mut t1 = 1.0f64;
            let mut miss = false;
            for k in 0..2 {
                let d = b[k] - a[k];
                if d.abs() < 1e-15 {
                    if a[k] < lo[k] || a[k] >= hi[k] {
                        miss = true;
                    }
                    continue;
                }
                let (mut ta, mut tb) = ((lo[k] - a[k]) / d, (hi[k] - a[k]) / d);
                if ta > tb {
                    core::mem::swap(&mut ta, &mut tb);
                }
                t0 = t0.max(ta);
                t1 = t1.min(tb);
            }
            if miss || t0 > t1 {
                continue;
            }

            let mid_t = 0.5 * (t0 + t1);
            let p = [a[0] + mid_t * (b[0] - a[0]), a[1] + mid_t * (b[1] - a[1])];
            if p[0] < lo[0] || p[0] >= hi[0] || p[1] < lo[1] || p[1] >= hi[1] {
                continue;
            }
            hits.push((t0, cx, cy));
        }
    }
    hits.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    hits.into_iter().map(|(_, cx, cy)| (cx, cy)).collect()
}

#[test]
fn dda_matches_the_brute_force_rasteriser() {
    let mut rng = Lcg(11);
    for i in 0..400 {
        let a = [rng.range(-100.0, 1636.0), rng.range(-100.0, 1636.0)];
        let b = [rng.range(-100.0, 1636.0), rng.range(-100.0, 1636.0)];
        let fast = cells_on_segment(a, b, 512.0, 3, 3);
        let slow = cells_on_segment_reference(a, b, 512.0, 3, 3);
        assert_eq!(fast, slow, "segment {i}: {a:?} → {b:?}");
    }

    assert_eq!(
        cells_on_segment([10.0, 700.0], [1500.0, 700.0], 512.0, 3, 3),
        vec![(0, 1), (1, 1), (2, 1)]
    );
    assert_eq!(
        cells_on_segment([700.0, 1500.0], [700.0, 10.0], 512.0, 3, 3),
        vec![(1, 2), (1, 1), (1, 0)]
    );
    assert_eq!(
        cells_on_segment([700.0, 700.0], [700.0, 700.0], 512.0, 3, 3),
        vec![(1, 1)]
    );
    assert_eq!(
        cells_on_segment([-600.0, 100.0], [-10.0, 100.0], 512.0, 3, 3),
        Vec::<(i64, i64)>::new()
    );
    let diag = cells_on_segment([100.0, 100.0], [1400.0, 1400.0], 512.0, 3, 3);
    assert_eq!(diag.first(), Some(&(0, 0)));
    assert_eq!(diag.last(), Some(&(2, 2)));
    assert!(diag.contains(&(1, 1)));
    assert!(diag.len() <= 5, "{diag:?}");
}
