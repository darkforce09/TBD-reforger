//! Role: patterns.
//! Position: `doc/operations/placement` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::SplitMix64;
use super::convex_hull;
use super::point_in_convex_hull;
use super::principal_axis;

/// A world-space point in metres (the `SlotSoa` `xs`/`ys` pair, widened to f64). `x` = east, `y` = north — the document convention every helper here shares with `ruler_tool` and the spawn export.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,
}

impl Pt {
    /// New using the supplied domain data.
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// The four placement patterns — the selector shared by the menu descriptor (`eden_top_strip::MENUS`) and the dispatch (`editor_ops::apply_pattern_to_selection`). Lives here in the UNGATED pure module (not in the wasm-only `editor_ops`) so the menu enum — which compiles on native too — can name it; `place_helpers` exposes one function per pattern, this enum only selects between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternKind {
    /// Domain representation of circular.
    Circular,

    /// Domain representation of line.
    Line,

    /// Domain representation of grid.
    Grid,

    /// Domain representation of fill area.
    FillArea,
}

impl PatternKind {
    /// Human label for the confirm prompt + the menu row.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PatternKind::Circular => "Circular",
            PatternKind::Line => "Line",
            PatternKind::Grid => "Grid",
            PatternKind::FillArea => "Fill Area",
        }
    }
}

/// Canonical destructive move threshold value.
pub const DESTRUCTIVE_MOVE_THRESHOLD: usize = 10;

/// Does an op moving `n` entities need the confirm? `n > 10` (strictly greater — exactly 10 is fine). Boundary pinned by `confirm_threshold_boundary` below.
#[must_use]
pub fn needs_confirm(n: usize) -> bool {
    n > DESTRUCTIVE_MOVE_THRESHOLD
}

/// Centroid using the supplied domain data.
#[must_use]
pub fn centroid(pts: &[Pt]) -> Pt {
    let n = pts.len();
    if n == 0 {
        return Pt::new(0.0, 0.0);
    }
    let mut sx = 0.0;
    let mut sy = 0.0;
    for p in pts {
        sx += p.x;
        sy += p.y;
    }
    Pt::new(sx / n as f64, sy / n as f64)
}

/// Bearing from to using the supplied domain data.
#[must_use]
pub fn bearing_from_to(from: Pt, to: Pt) -> f64 {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    dx.atan2(dy).to_degrees().rem_euclid(360.0)
}

/// The maximum spread of a point set = the largest distance from the centroid to any point (the pattern "radius = max current spread" input). `0.0` for `< 2` points. Used by [`pattern_circular`] (clamped to a 5 m floor there).
#[must_use]
pub fn max_spread(pts: &[Pt]) -> f64 {
    if pts.len() < 2 {
        return 0.0;
    }
    let c = centroid(pts);
    pts.iter()
        .map(|p| ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt())
        .fold(0.0f64, f64::max)
}

/// The axis-aligned bounding box `(min_x, min_y, max_x, max_y)` of a point set. `(0,0,0,0)` for an empty set (callers gate on empty). Backs the align/space commands (which snap to box edges).
#[must_use]
pub fn bounds(pts: &[Pt]) -> (f64, f64, f64, f64) {
    if pts.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in pts {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    (min_x, min_y, max_x, max_y)
}

/// The angle for index `i` of `n` is `θ_i = 2π·i/n`, and the world offset uses the SAME bearing→offset convention as the rest of the module: bearing β clockwise-from-north → `(dx, dy) = (r·sin β, r·cos β)`. So `i=0` lands due north of the centroid, `i=n/4` due east, etc.
#[must_use]
pub fn pattern_circular(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let c = centroid(pts);
    let r = max_spread(pts).max(5.0);
    (0..n)
        .map(|i| {
            let beta = std::f64::consts::TAU * (i as f64) / (n as f64);
            Pt::new(c.x + r * beta.sin(), c.y + r * beta.cos())
        })
        .collect()
}

/// Falls back to a **horizontal** (due-east) axis when the covariance is degenerate (all points coincident, or a perfectly isotropic cluster) so the result is still a clean line rather than NaN.
#[must_use]
pub fn pattern_line(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let c = centroid(pts);
    let (ux, uy) = principal_axis(pts);

    let mut proj: Vec<(usize, f64)> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| (i, (p.x - c.x) * ux + (p.y - c.y) * uy))
        .collect();
    let min_t = proj.iter().map(|&(_, t)| t).fold(f64::INFINITY, f64::min);
    let max_t = proj
        .iter()
        .map(|&(_, t)| t)
        .fold(f64::NEG_INFINITY, f64::max);
    let span = max_t - min_t;

    proj.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let step = if n > 1 { span / (n as f64 - 1.0) } else { 0.0 };
    let mut out = vec![Pt::new(0.0, 0.0); n];
    for (rank, &(orig_i, _)) in proj.iter().enumerate() {
        let t = min_t + step * rank as f64;
        out[orig_i] = Pt::new(c.x + ux * t, c.y + uy * t);
    }
    out
}

/// **Grid** — the selection packed into the **nearest-square** grid (`cols = ceil(sqrt(n))`), row-major from the top-left of a block centred on the centroid, at a **5 m default cell**. Row-major fill in input order (input 0 → top-left cell), so the mapping is deterministic. "Top-left" is min-x/max-y (north-west), and rows step south (−y) — the reading order on a north-up map.
#[must_use]
pub fn pattern_grid(pts: &[Pt]) -> Vec<Pt> {
    pattern_grid_cell(pts, 5.0)
}

/// Pattern grid cell using the supplied domain data.
#[must_use]
pub fn pattern_grid_cell(pts: &[Pt], cell: f64) -> Vec<Pt> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let c = centroid(pts);
    let cols = (n as f64).sqrt().ceil() as usize;
    let cols = cols.max(1);
    let rows = n.div_ceil(cols);

    let x0 = c.x - (cols as f64 - 1.0) * cell / 2.0;
    let y0 = c.y + (rows as f64 - 1.0) * cell / 2.0;
    (0..n)
        .map(|i| {
            let col = i % cols;
            let row = i / cols;
            Pt::new(x0 + col as f64 * cell, y0 - row as f64 * cell)
        })
        .collect()
}

/// The i-th entity consumes the i-th pair of the deterministic stream, so the mapping input→scatter is stable per id-set. A hull with `< 3` distinct vertices can't contain area, so those inputs scatter within the (possibly zero-area) bbox — still deterministic, never NaN.
#[must_use]
pub fn pattern_fill_area(pts: &[Pt], seed: u64) -> Vec<Pt> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let hull = convex_hull(pts);
    let (min_x, min_y, max_x, max_y) = bounds(pts);
    let w = max_x - min_x;
    let h = max_y - min_y;
    let mut rng = SplitMix64::new(seed);
    (0..n)
        .map(|_| {
            let mut candidate = Pt::new(min_x, min_y);
            for _ in 0..32 {
                let cx = min_x + rng.next_unit() * w;
                let cy = min_y + rng.next_unit() * h;
                candidate = Pt::new(cx, cy);
                if hull.len() < 3 || point_in_convex_hull(&hull, candidate) {
                    break;
                }
            }
            candidate
        })
        .collect()
}
