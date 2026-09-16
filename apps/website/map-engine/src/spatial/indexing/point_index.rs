//! Role: point index.
//! Position: `spatial/indexing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// A built grid over a slot SoA. Rebuild on bulk change; O(1)-ish cell lookups for queries.
pub struct PointIndex {
    xs: Vec<f32>,
    ys: Vec<f32>,
    min_x: f64,
    min_y: f64,
    cell: f64,
    cols: usize,
    rows: usize,

    cell_start: Vec<u32>,

    items: Vec<u32>,
}

impl PointIndex {
    #[inline]
    fn cell_xy(&self, x: f64, y: f64) -> (usize, usize) {
        let cx = (((x - self.min_x) / self.cell).floor() as isize).clamp(0, self.cols as isize - 1);
        let cy = (((y - self.min_y) / self.cell).floor() as isize).clamp(0, self.rows as isize - 1);
        (cx as usize, cy as usize)
    }

    /// Build the grid over `xs`/`ys` (row-aligned). `cell` is the grid cell size in world units (a few hundred metres works for slots); non-positive falls back to 1.
    #[must_use]
    pub fn build(xs: Vec<f32>, ys: Vec<f32>, cell: f64) -> PointIndex {
        assert_eq!(xs.len(), ys.len(), "xs/ys length mismatch");
        let cell = if cell > 0.0 { cell } else { 1.0 };
        if xs.is_empty() {
            return PointIndex {
                xs,
                ys,
                min_x: 0.0,
                min_y: 0.0,
                cell,
                cols: 1,
                rows: 1,
                cell_start: vec![0, 0],
                items: Vec::new(),
            };
        }
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (&x, &y) in xs.iter().zip(ys.iter()) {
            let (x, y) = (f64::from(x), f64::from(y));
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let cols = (((max_x - min_x) / cell).floor() as usize + 1).max(1);
        let rows = (((max_y - min_y) / cell).floor() as usize + 1).max(1);
        let ncells = cols * rows;

        let mut idx = PointIndex {
            xs,
            ys,
            min_x,
            min_y,
            cell,
            cols,
            rows,
            cell_start: vec![0u32; ncells + 1],
            items: Vec::new(),
        };

        for (&x, &y) in idx.xs.iter().zip(idx.ys.iter()) {
            let (cx, cy) = idx.cell_xy(f64::from(x), f64::from(y));
            idx.cell_start[cy * cols + cx + 1] += 1;
        }
        for c in 0..ncells {
            idx.cell_start[c + 1] += idx.cell_start[c];
        }
        let n = idx.xs.len();
        idx.items = vec![0u32; n];
        let mut cursor = idx.cell_start.clone();
        for i in 0..n {
            let (cx, cy) = idx.cell_xy(f64::from(idx.xs[i]), f64::from(idx.ys[i]));
            let c = cy * cols + cx;
            idx.items[cursor[c] as usize] = i as u32;
            cursor[c] += 1;
        }
        idx
    }

    /// Handles whose point is inside the inclusive bbox `[min_x,max_x] × [min_y,max_y]`. Mirrors rbush `search` over degenerate point boxes.
    #[must_use]
    pub fn pick_rect(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<u32> {
        let mut out = Vec::new();
        if self.items.is_empty() || max_x < min_x || max_y < min_y {
            return out;
        }
        let (cx0, cy0) = self.cell_xy(min_x, min_y);
        let (cx1, cy1) = self.cell_xy(max_x, max_y);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * self.cols + cx;
                let (a, b) = (self.cell_start[c] as usize, self.cell_start[c + 1] as usize);
                for &i in &self.items[a..b] {
                    let (x, y) = (
                        f64::from(self.xs[i as usize]),
                        f64::from(self.ys[i as usize]),
                    );
                    if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                        out.push(i);
                    }
                }
            }
        }
        out
    }

    /// The single nearest handle within `radius` (circular, `dx²+dy² ≤ radius²`), or `None`. Ties keep the first-encountered (strict `<`), matching a single deterministic result for the non-degenerate (no exactly-equal-distance) inputs the probe battery uses.
    #[must_use]
    pub fn pick_nearest(&self, x: f64, y: f64, radius: f64) -> Option<u32> {
        if self.items.is_empty() {
            return None;
        }
        let r = radius.max(0.0);
        let r2 = r * r;
        let (cx0, cy0) = self.cell_xy(x - r, y - r);
        let (cx1, cy1) = self.cell_xy(x + r, y + r);
        let mut best: Option<(f64, u32)> = None;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * self.cols + cx;
                let (a, b) = (self.cell_start[c] as usize, self.cell_start[c + 1] as usize);
                for &i in &self.items[a..b] {
                    let dx = f64::from(self.xs[i as usize]) - x;
                    let dy = f64::from(self.ys[i as usize]) - y;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= r2 && best.is_none_or(|(bd, _)| d2 < bd) {
                        best = Some((d2, i));
                    }
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// Len.
    #[must_use]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    /// Is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }
}

#[cfg(test)]
#[path = "tests/point_index_tests.rs"]
mod tests;
