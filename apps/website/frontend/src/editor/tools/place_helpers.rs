//! T-645 — Placement helpers: the pure math behind the Placement Tools (the single highest-yield
//! borrow from the 3den Enhanced catalogue). Patterns (Circular / Line / Grid / Fill Area), the 6
//! align commands, the 3 space-equally commands, the 6 orient commands, and the drag-to-garrison
//! firing-position kernel.
//!
//! ── Module shape (mirrors `ruler_tool` / `mission_editor::transform`) ────────────────────────────
//! Everything here is a **pure function** over plain `[Pt]` / `[f64]` — no `web_sys`, no engine, no
//! doc — so a native `cargo test -p website-frontend` proves every pattern's geometry with no
//! browser. This file is UNGATED (declared `mod place_helpers;` with no `#[cfg(target_arch =
//! "wasm32")]` in `main.rs`), exactly so those goldens run on the same command CI uses. The wasm
//! wiring that reads the live selection, confirms, and commits per-entity lives in `editor_ops.rs`
//! (`apply_pattern_to_selection` / `align_selection` / `space_selection` / `orient_selection`); it
//! calls straight into here for the numbers and rides the existing per-field position writes
//! (`update_slot_position` for slots, `set_vehicle_position` for vehicles — the T-648
//! `rotate_selection_to_face` precedent), never a new core mutator.
//!
//! ── The bearing convention (shared, cited) ──────────────────────────────────────────────────────
//! Orientation math reuses the document convention proved in `ruler_tool::bearing_deg` and
//! `mission_editor::transform::bearing_to_face` (T-648): **yaw clockwise from north**, world +Y =
//! north (north-up, `flipY:false`), +X = east, so a bearing is `atan2(east, north) = atan2(dx, dy)`
//! wrapped to `[0, 360)`. The four cardinals fall out exactly — N=0, E=90, S=180, W=270. This module
//! re-derives it in [`bearing_from_to`] rather than importing, so the pure layer has no `wasm32`-gated
//! dependency and the goldens pin the number here directly; `editor_ops` reuses
//! `transform::bearing_to_face` at the call site for the identical result on the face commands.
//!
//! ── Position precision ───────────────────────────────────────────────────────────────────────────
//! All math is f64. Current slot positions are read back through the `SlotSoa` `f32` columns
//! (`Math.fround` store boundary), so the wasm layer widens `f32 → f64` on read and the commit
//! re-normalises via `update_slot_position`. The goldens below work in exact f64 for a DOM-free fixture.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A world-space point in metres (the `SlotSoa` `xs`/`ys` pair, widened to f64). `x` = east, `y` =
/// north — the document convention every helper here shares with `ruler_tool` and the spawn export.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// The four placement patterns — the selector shared by the menu descriptor
/// (`eden_top_strip::MENUS`) and the dispatch (`editor_ops::apply_pattern_to_selection`). Lives here
/// in the UNGATED pure module (not in the wasm-only `editor_ops`) so the menu enum — which compiles
/// on native too — can name it; `place_helpers` exposes one function per pattern, this enum only
/// selects between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Circular,
    Line,
    Grid,
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

/// **The destructive threshold (the corpus rule, kept simple).** An op that moves **more than 10**
/// entities is "destructive/large" and must confirm via `confirm_with_message` (the T-666 idiom) at
/// the wasm call site; ≤10 applies straight through. Pattern application always previews in the sense
/// that the operator sees the result and can Ctrl+Z it — but the ticket's simplification is the flat
/// count gate, so that is exactly what this is. Stated as a constant + predicate so the goldens can
/// pin the boundary (`10 → false`, `11 → true`) without re-reading a magic number at three call sites.
pub const DESTRUCTIVE_MOVE_THRESHOLD: usize = 10;

/// Does an op moving `n` entities need the confirm? `n > 10` (strictly greater — exactly 10 is fine).
/// Boundary pinned by `confirm_threshold_boundary` below.
#[must_use]
pub fn needs_confirm(n: usize) -> bool {
    n > DESTRUCTIVE_MOVE_THRESHOLD
}

/// The centroid (arithmetic mean) of a point set. `(0,0)` for an empty set — callers gate on empty
/// before use, so the value is never consumed for `n == 0`; defined so the function is total. The sum
/// is left-to-right f64 (the same reduce order `paste_slots` uses for its centroid translate — a
/// property the pattern/align commands share so a "circular then undo then circular" is stable).
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

/// Bearing `from → to` in **degrees clockwise from north**, wrapped to `[0, 360)`
/// (`atan2(dx, dy)`). This is the shared document convention (see the module header + T-648). A
/// zero-length aim (`from == to`) returns `0.0` — a degenerate direction; the orient callers that
/// must decline a meaningless rotation check the degeneracy themselves before calling.
#[must_use]
pub fn bearing_from_to(from: Pt, to: Pt) -> f64 {
    let dx = to.x - from.x; // east
    let dy = to.y - from.y; // north
    dx.atan2(dy).to_degrees().rem_euclid(360.0)
}

/// The maximum spread of a point set = the largest distance from the centroid to any point (the
/// pattern "radius = max current spread" input). `0.0` for `< 2` points. Used by [`pattern_circular`]
/// (clamped to a 5 m floor there).
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

/// The axis-aligned bounding box `(min_x, min_y, max_x, max_y)` of a point set. `(0,0,0,0)` for an
/// empty set (callers gate on empty). Backs the align/space commands (which snap to box edges).
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

/* ═══════════════════════════════════ Patterns ═══════════════════════════════════════════════════ */
//
// Each pattern returns a NEW target position per input, index-aligned with `pts`. The caller keeps
// each entity's identity/rotation and only writes x/y (a pattern is a re-arrangement, not a re-orient).
// A pattern of `< 2` entities is a no-op (returns the inputs unchanged) — there is nothing to arrange.

/// **Circular** — the selection spaced equally on a circle at the centroid. Radius = `max current
/// spread` with a **5 m minimum** (a tightly-clustered selection still opens to a readable ring).
/// Entities are placed at equal angular steps starting due **north** (bearing 0) and proceeding
/// **clockwise** (the document's positive rotation sense), in input order — so the mapping is
/// deterministic and a re-apply is idempotent up to the ring already being circular.
///
/// The angle for index `i` of `n` is `θ_i = 2π·i/n`, and the world offset uses the SAME
/// bearing→offset convention as the rest of the module: bearing β clockwise-from-north →
/// `(dx, dy) = (r·sin β, r·cos β)`. So `i=0` lands due north of the centroid, `i=n/4` due east, etc.
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

/// **Line** — the selection strung along its **principal axis** through the centroid, equally spaced.
/// The principal axis is the dominant eigenvector of the 2×2 position covariance (the direction the
/// selection is most spread along); the spacing is the current axis extent divided into `n-1` gaps,
/// so the line spans the same length the selection already occupied along that axis (no surprise
/// growth). Entities are ordered by their projection onto the axis, so the leftmost-along-axis input
/// stays leftmost — the arrangement is stable, not a shuffle.
///
/// Falls back to a **horizontal** (due-east) axis when the covariance is degenerate (all points
/// coincident, or a perfectly isotropic cluster) so the result is still a clean line rather than NaN.
#[must_use]
pub fn pattern_line(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let c = centroid(pts);
    let (ux, uy) = principal_axis(pts);
    // Project each input onto the axis (signed distance from centroid along (ux,uy)).
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
    // Order entities by projection so the along-axis order is preserved (stable in `i` on ties).
    proj.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let step = if n > 1 { span / (n as f64 - 1.0) } else { 0.0 };
    let mut out = vec![Pt::new(0.0, 0.0); n];
    for (rank, &(orig_i, _)) in proj.iter().enumerate() {
        let t = min_t + step * rank as f64;
        out[orig_i] = Pt::new(c.x + ux * t, c.y + uy * t);
    }
    out
}

/// **Grid** — the selection packed into the **nearest-square** grid (`cols = ceil(sqrt(n))`), row-major
/// from the top-left of a block centred on the centroid, at a **5 m default cell**. Row-major fill in
/// input order (input 0 → top-left cell), so the mapping is deterministic. "Top-left" is
/// min-x/max-y (north-west), and rows step south (−y) — the reading order on a north-up map.
///
/// The block is centred on the centroid so a grid-then-undo-then-grid does not drift: the centre is a
/// function of the selection, not of a corner.
#[must_use]
pub fn pattern_grid(pts: &[Pt]) -> Vec<Pt> {
    pattern_grid_cell(pts, 5.0)
}

/// [`pattern_grid`] with an explicit cell size (metres). Exposed so a future UI cell-size control and
/// the goldens can drive it; `pattern_grid` pins the 5 m default the ticket names.
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
    // Centre the whole block on the centroid: the block is `(cols-1)*cell` wide, `(rows-1)*cell` tall.
    let x0 = c.x - (cols as f64 - 1.0) * cell / 2.0;
    let y0 = c.y + (rows as f64 - 1.0) * cell / 2.0; // top row is the NORTH-most (max y)
    (0..n)
        .map(|i| {
            let col = i % cols;
            let row = i / cols;
            Pt::new(x0 + col as f64 * cell, y0 - row as f64 * cell)
        })
        .collect()
}

/// **Fill Area (scatter)** — scatter the selection at random-looking but **fully deterministic**
/// positions inside the selection's own convex hull. The seed is a hash of the selection ids
/// (`seed_from_ids`), so the SAME selection scatters the SAME way every time — reproducible, no
/// `Math.random` equivalent (the ticket's explicit requirement). Rejection-samples the hull's bbox
/// until a point lands inside the hull ([`point_in_convex_hull`]); a hull that is a point or a line
/// (all/most points collinear) degenerates gracefully to the bbox sample.
///
/// The i-th entity consumes the i-th pair of the deterministic stream, so the mapping input→scatter
/// is stable per id-set. A hull with `< 3` distinct vertices can't contain area, so those inputs
/// scatter within the (possibly zero-area) bbox — still deterministic, never NaN.
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
            // Up to a bounded number of rejection tries; fall back to the last bbox sample so the
            // function is total and deterministic even for a degenerate (line/point) hull.
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

/* ═══════════════════════════════════ Align (6 commands) ═════════════════════════════════════════ */

/// The six align edges: the 4 bounding-box edges + the 2 centre axes. Left/right/top/bottom snap
/// every entity's matching coordinate to that box edge; centre-h/centre-v snap to the box mid-line.
/// (Top = north = max y; bottom = south = min y — the north-up convention.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignEdge {
    Left,
    Right,
    Top,
    Bottom,
    /// Horizontal centres — every entity to the same **x** (the vertical mid-line): a vertical stack.
    CentreH,
    /// Vertical centres — every entity to the same **y** (the horizontal mid-line): a horizontal row.
    CentreV,
}

/// Align every point to `edge` of the selection's own bounding box. Only the affected axis moves; the
/// other coordinate is preserved (Left sets x=min_x, keeps y; Top sets y=max_y, keeps x; CentreH sets
/// x to the box mid-x, keeps y; …). `< 2` points is a no-op. The box is the selection's current
/// bounds, so an align never moves the group as a whole off where it sits — it only collapses one axis.
#[must_use]
pub fn align_edge(pts: &[Pt], edge: AlignEdge) -> Vec<Pt> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let (min_x, min_y, max_x, max_y) = bounds(pts);
    let mid_x = (min_x + max_x) / 2.0;
    let mid_y = (min_y + max_y) / 2.0;
    pts.iter()
        .map(|p| match edge {
            AlignEdge::Left => Pt::new(min_x, p.y),
            AlignEdge::Right => Pt::new(max_x, p.y),
            AlignEdge::Top => Pt::new(p.x, max_y),
            AlignEdge::Bottom => Pt::new(p.x, min_y),
            AlignEdge::CentreH => Pt::new(mid_x, p.y),
            AlignEdge::CentreV => Pt::new(p.x, mid_y),
        })
        .collect()
}

/* ═══════════════════════════════════ Space equally (3 commands) ═════════════════════════════════ */

/// The three space-equally axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceAxis {
    /// Distribute along **x** (horizontal): equal x-gaps between the extreme-x entities; y preserved.
    Horizontal,
    /// Distribute along **y** (vertical): equal y-gaps between the extreme-y entities; x preserved.
    Vertical,
    /// Distribute **along the selection's principal axis** (like [`pattern_line`], but each entity
    /// keeps its perpendicular offset from the axis — a "space along the line" that respects a
    /// slightly-off-axis scatter rather than collapsing it to a strict line).
    AlongLine,
}

/// Space the selection equally along `axis`. Horizontal/Vertical keep the two extreme entities fixed
/// and redistribute the interior ones to equal gaps between them (the standard "distribute" — the
/// span does not change, only the interior spacing evens out). `< 3` points is a no-op for
/// Horizontal/Vertical (with 2, they are already "equally spaced"; with 1/0 there is nothing to do).
///
/// AlongLine projects onto the principal axis, evens the along-axis coordinate, and **restores each
/// entity's perpendicular offset** so a near-line stays near-line rather than snapping dead straight.
#[must_use]
pub fn space_equally(pts: &[Pt], axis: SpaceAxis) -> Vec<Pt> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    match axis {
        SpaceAxis::Horizontal => space_axis_aligned(pts, true),
        SpaceAxis::Vertical => space_axis_aligned(pts, false),
        SpaceAxis::AlongLine => space_along_line(pts),
    }
}

/// Shared body for Horizontal (`horizontal = true`, spaces x) / Vertical (spaces y). Keeps the two
/// extremes on the spaced axis fixed; the other coordinate is untouched.
fn space_axis_aligned(pts: &[Pt], horizontal: bool) -> Vec<Pt> {
    let n = pts.len();
    let key = |p: &Pt| if horizontal { p.x } else { p.y };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        key(&pts[a])
            .partial_cmp(&key(&pts[b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let lo = key(&pts[order[0]]);
    let hi = key(&pts[order[n - 1]]);
    let step = (hi - lo) / (n as f64 - 1.0);
    let mut out = pts.to_vec();
    for (rank, &orig_i) in order.iter().enumerate() {
        let v = lo + step * rank as f64;
        if horizontal {
            out[orig_i].x = v;
        } else {
            out[orig_i].y = v;
        }
    }
    out
}

/// AlongLine — even the along-axis coordinate on the principal axis while preserving each entity's
/// perpendicular offset from that axis (so a slightly-scattered near-line spaces out but keeps its
/// character). Ordered by projection, extremes fixed.
fn space_along_line(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    let c = centroid(pts);
    let (ux, uy) = principal_axis(pts);
    // Perpendicular unit vector (rotate axis +90°): (−uy, ux).
    let (px, py) = (-uy, ux);
    // (index, along-projection t, perpendicular-projection s) relative to centroid.
    let mut proj: Vec<(usize, f64, f64)> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let dx = p.x - c.x;
            let dy = p.y - c.y;
            (i, dx * ux + dy * uy, dx * px + dy * py)
        })
        .collect();
    proj.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let min_t = proj[0].1;
    let max_t = proj[n - 1].1;
    let step = (max_t - min_t) / (n as f64 - 1.0);
    let mut out = vec![Pt::new(0.0, 0.0); n];
    for (rank, &(orig_i, _, s)) in proj.iter().enumerate() {
        let t = min_t + step * rank as f64;
        // Reconstruct: centroid + t·axis + s·perp (perpendicular offset preserved).
        out[orig_i] = Pt::new(c.x + ux * t + px * s, c.y + uy * t + py * s);
    }
    out
}

/* ═══════════════════════════════════ Orient (6 commands) ════════════════════════════════════════ */

/// The six orient commands → a per-entity target yaw (degrees clockwise from north). The four
/// cardinals are ABSOLUTE headings; face-centre / face-away are bearings relative to a pivot (the
/// selection centroid), computed per entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orient {
    North,
    East,
    South,
    West,
    /// Every entity turns to FACE the selection centroid.
    FaceCentre,
    /// Every entity turns to face directly AWAY from the selection centroid (centre bearing + 180°).
    FaceAway,
}

impl Orient {
    /// The fixed cardinal heading for the four absolute commands; `None` for the two face commands
    /// (which are per-entity, resolved by [`orient_yaw`]).
    #[must_use]
    pub fn cardinal_deg(self) -> Option<f64> {
        match self {
            Orient::North => Some(0.0),
            Orient::East => Some(90.0),
            Orient::South => Some(180.0),
            Orient::West => Some(270.0),
            Orient::FaceCentre | Orient::FaceAway => None,
        }
    }
}

/// The target yaw for one entity at `pos` under command `cmd`, given the selection `pivot` (the
/// centroid). Cardinals ignore `pos`/`pivot`; face-centre returns the bearing `pos → pivot`;
/// face-away returns that + 180°. Returns `None` for a face command when the entity sits exactly on
/// the pivot (a degenerate aim — the caller leaves the rotation unchanged, matching
/// `bearing_to_face`'s `None` contract). Cardinals never return `None`.
#[must_use]
pub fn orient_yaw(cmd: Orient, pos: Pt, pivot: Pt) -> Option<f64> {
    if let Some(deg) = cmd.cardinal_deg() {
        return Some(deg);
    }
    // Face commands: degenerate when the entity is on the pivot.
    if pos.x == pivot.x && pos.y == pivot.y {
        return None;
    }
    let to_centre = bearing_from_to(pos, pivot);
    Some(match cmd {
        Orient::FaceAway => (to_centre + 180.0).rem_euclid(360.0),
        _ => to_centre, // FaceCentre (cardinals handled above)
    })
}

/* ═══════════════════════════════════ Garrison (firing positions) ════════════════════════════════ */
//
// SCOPE NOTE (T-645 / T-732 sibling): the PURE KERNEL below — perimeter firing positions of a
// building OBB — is in-scope and lands here, natively tested. The LIVE drag-to-garrison WIRING
// (resolve which building a dropped group landed on, read that building's live OBB) is a SCOPE
// DISCOVERY: `crates/map-engine-core/src/world/{store,residency}.rs` expose buildings only as
// pre-tessellated GPU fill/outline vertex buffers and a single `last_chunk` — there is no
// building-at-point query returning `(centre, half-extents, rotation)`, and adding one touches the
// unowned world store/host (obb.rs is READ-ONLY per the ticket). See the final report. Shipping the
// kernel now means the wiring is a thin lookup once that accessor exists.

/// A firing position on a building perimeter: the world point plus the outward-facing yaw (degrees
/// clockwise from north) an occupant would face — out through the nearest wall.
///
/// `allow(dead_code)`: this kernel is SHIPPED and golden-tested, but its production caller is a SCOPE
/// DISCOVERY — the live drag-to-garrison wiring needs a building-at-point accessor on the (unowned)
/// world store that does not exist (see the garrison scope note above + the T-645 report). The kernel
/// lands now so that wiring is a thin lookup once the accessor is added; until then it has no
/// non-test caller by design, not by oversight.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiringPosition {
    pub pos: Pt,
    pub yaw_deg: f64,
}

/// Garrison firing positions for one building OBB, capped at `count` (the group size). The building
/// is given by its centre `(cx, cy)`, half-extents `(half_x, half_y)` and `rotation_deg` (the same
/// OBB parameterisation `world::obb::obb_corners` uses — `0° = north (+y)`, clockwise-positive).
///
/// Positions are the perimeter of the footprint **inset 1 m** (an occupant stands just inside the
/// wall, not on it), distributed evenly around the inset rectangle's circumference starting at the
/// north-west corner and proceeding clockwise, each **facing outward** (the local wall-normal, in
/// world bearing). `count == 0` or a footprint that fully collapses under the 1 m inset (half-extent
/// ≤ 1 m on an axis) yields an empty list — a hut too small to garrison gets no ghost positions
/// rather than degenerate ones.
///
/// Pure and DOM-free: the OBB params are the only input, so this is fully golden-tested. The live
/// path (once a building-at-point accessor exists on the world store — see the scope note above)
/// reads those params off the resolved building and drives one `set_slot_position` / `move` per
/// position, capped by the dropped group's size.
///
/// `allow(dead_code)`: shipped kernel, wiring is a scope discovery — see [`FiringPosition`].
#[allow(dead_code)]
#[must_use]
pub fn garrison_firing_positions(
    cx: f64,
    cy: f64,
    half_x: f64,
    half_y: f64,
    rotation_deg: f64,
    count: usize,
) -> Vec<FiringPosition> {
    let inset = 1.0;
    let hx = half_x - inset;
    let hy = half_y - inset;
    if count == 0 || hx <= 0.0 || hy <= 0.0 {
        return Vec::new();
    }
    let rad = rotation_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    // Local (building-frame) → world, matching `obb_corners`: world = centre + local rotated.
    // (dx along local east/+x, dy along local north/+y.)
    let to_world = |dx: f64, dy: f64| Pt::new(cx + dx * cos + dy * sin, cy - dx * sin + dy * cos);
    // Distribute `count` points evenly by PERIMETER arc-length of the inset rectangle, so a long thin
    // building gets more points along its long walls (natural firing-line spacing). Walk the perimeter
    // clockwise from the NW corner (−hx, +hy): NW→NE (east, +x) → NE→SE (south, −y) → SE→SW → SW→NW.
    let perim = 2.0 * (2.0 * hx + 2.0 * hy);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        // Arc-length position along the perimeter for this point (evenly spaced, not repeating the
        // start point for the last one — `i/count`, not `i/(count-1)`).
        let s = perim * (i as f64) / (count as f64);
        let (dx, dy, out_normal_local) = perimeter_point(hx, hy, s);
        let world = to_world(dx, dy);
        // Outward normal in the building's local frame → world bearing. A local direction
        // (nx, ny) has world bearing = base rotation + atan2(nx, ny) (same +x=east/+y=north basis).
        let local_bearing = (out_normal_local.0).atan2(out_normal_local.1).to_degrees();
        let yaw = (rotation_deg + local_bearing).rem_euclid(360.0);
        out.push(FiringPosition {
            pos: world,
            yaw_deg: yaw,
        });
    }
    out
}

/// Given the inset half-extents and an arc-length `s` measured clockwise from the NW corner, return
/// the local `(dx, dy)` on the perimeter and the outward wall normal `(nx, ny)` in the local frame.
/// The four sides in clockwise order from NW `(−hx, +hy)`:
///   1. top wall  NW→NE  (moving +x), outward normal +y (north)
///   2. right wall NE→SE  (moving −y), outward normal +x (east)
///   3. bottom wall SE→SW (moving −x), outward normal −y (south)
///   4. left wall  SW→NW  (moving +y), outward normal −x (west)
///
/// `allow(dead_code)`: only [`garrison_firing_positions`] calls it — same scope-discovery status.
#[allow(dead_code)]
fn perimeter_point(hx: f64, hy: f64, s: f64) -> (f64, f64, (f64, f64)) {
    let top = 2.0 * hx;
    let right = 2.0 * hy;
    let bottom = 2.0 * hx;
    let mut r = s;
    if r < top {
        return (-hx + r, hy, (0.0, 1.0)); // top wall, normal north
    }
    r -= top;
    if r < right {
        return (hx, hy - r, (1.0, 0.0)); // right wall, normal east
    }
    r -= right;
    if r < bottom {
        return (hx - r, -hy, (0.0, -1.0)); // bottom wall, normal south
    }
    r -= bottom;
    (-hx, -hy + r, (-1.0, 0.0)) // left wall, normal west
}

/* ═══════════════════════════════════ Geometry primitives ════════════════════════════════════════ */

/// The dominant eigenvector (unit) of the 2×2 position covariance — the "principal axis" the Line
/// and AlongLine commands run along. Returns `(ux, uy)`, a unit vector. Degenerate covariance
/// (coincident/isotropic points) falls back to `(1, 0)` (due east) so downstream math never sees NaN.
///
/// Closed-form 2×2 eigen: for covariance `[[a, b],[b, d]]` the larger eigenvalue's eigenvector is
/// `(b, λ − a)` (or `(λ − d, b)`); we take whichever is well-conditioned and normalise.
#[must_use]
pub fn principal_axis(pts: &[Pt]) -> (f64, f64) {
    let n = pts.len();
    if n < 2 {
        return (1.0, 0.0);
    }
    let c = centroid(pts);
    let (mut a, mut b, mut d) = (0.0, 0.0, 0.0);
    for p in pts {
        let dx = p.x - c.x;
        let dy = p.y - c.y;
        a += dx * dx;
        b += dx * dy;
        d += dy * dy;
    }
    // (No need to divide by n — eigenvectors are scale-invariant.)
    if b.abs() < 1e-12 {
        // Diagonal covariance: axis is x if x-variance dominates, else y.
        return if a >= d { (1.0, 0.0) } else { (0.0, 1.0) };
    }
    let trace = a + d;
    let det = a * d - b * b;
    let disc = (trace * trace / 4.0 - det).max(0.0).sqrt();
    let lambda = trace / 2.0 + disc; // larger eigenvalue
                                     // Eigenvector (b, λ − a); normalise.
    let (vx, vy) = (b, lambda - a);
    let len = (vx * vx + vy * vy).sqrt();
    if len < 1e-12 {
        (1.0, 0.0)
    } else {
        (vx / len, vy / len)
    }
}

/// Convex hull (counter-clockwise, no repeated endpoint) via Andrew's monotone chain. Returns the
/// input's distinct points when `< 3` remain after the chain (a point or a line has no area). Used by
/// [`pattern_fill_area`] for the scatter containment test.
#[must_use]
pub fn convex_hull(pts: &[Pt]) -> Vec<Pt> {
    let mut p: Vec<Pt> = pts.to_vec();
    p.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    p.dedup_by(|a, b| a.x == b.x && a.y == b.y);
    let n = p.len();
    if n < 3 {
        return p;
    }
    let cross = |o: Pt, a: Pt, b: Pt| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut hull: Vec<Pt> = Vec::with_capacity(2 * n);
    // Lower hull.
    for &pt in &p {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0 {
            hull.pop();
        }
        hull.push(pt);
    }
    // Upper hull.
    let lower_len = hull.len() + 1;
    for &pt in p.iter().rev().skip(1) {
        while hull.len() >= lower_len
            && cross(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0.0
        {
            hull.pop();
        }
        hull.push(pt);
    }
    hull.pop(); // last point == first point of the other chain
    hull
}

/// Is `pt` inside (or on the boundary of) the CCW convex `hull`? A point/line hull (`< 3`) can't
/// contain area → `false`. Uses the sign of the cross product for every directed edge (all `>= 0`
/// for a CCW hull means inside-or-on).
#[must_use]
pub fn point_in_convex_hull(hull: &[Pt], pt: Pt) -> bool {
    let n = hull.len();
    if n < 3 {
        return false;
    }
    for i in 0..n {
        let a = hull[i];
        let b = hull[(i + 1) % n];
        let cross = (b.x - a.x) * (pt.y - a.y) - (b.y - a.y) * (pt.x - a.x);
        if cross < 0.0 {
            return false; // right of a CCW edge ⇒ outside
        }
    }
    true
}

/// A stable 64-bit seed from the selection ids — the deterministic scatter seed (`pattern_fill_area`).
/// Order-independent so the SAME set of ids (regardless of selection order) scatters identically:
/// each id is hashed and the per-id hashes are XOR-combined. The ticket's "hash, not `Math.random`".
#[must_use]
pub fn seed_from_ids(ids: &[String]) -> u64 {
    let mut acc: u64 = 0;
    for id in ids {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        acc ^= h.finish();
    }
    acc
}

/// SplitMix64 — a tiny, deterministic PRNG for the reproducible scatter. Not cryptographic; its only
/// job is a stable, well-distributed stream from a fixed seed so a given selection scatters the same
/// way on every machine (no platform `Math.random` divergence).
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next value in `[0, 1)` (53-bit mantissa precision).
    fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(f64, f64)]) -> Vec<Pt> {
        v.iter().map(|&(x, y)| Pt::new(x, y)).collect()
    }

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn pt_approx(a: Pt, b: Pt, eps: f64) -> bool {
        approx(a.x, b.x, eps) && approx(a.y, b.y, eps)
    }

    // ── Confirm-threshold pin (the corpus rule) ──────────────────────────────────────────────────
    #[test]
    fn confirm_threshold_boundary() {
        assert!(!needs_confirm(0));
        assert!(!needs_confirm(10), "exactly 10 does NOT confirm");
        assert!(needs_confirm(11), "11 (>10) confirms");
        assert!(needs_confirm(500));
        assert_eq!(DESTRUCTIVE_MOVE_THRESHOLD, 10);
    }

    // ── Bearing convention pin (shared with ruler_tool / transform::bearing_to_face) ─────────────
    #[test]
    fn bearing_cardinals_clockwise_from_north() {
        let o = Pt::new(0.0, 0.0);
        assert!(approx(bearing_from_to(o, Pt::new(0.0, 10.0)), 0.0, 1e-9)); // north
        assert!(approx(bearing_from_to(o, Pt::new(10.0, 0.0)), 90.0, 1e-9)); // east
        assert!(approx(bearing_from_to(o, Pt::new(0.0, -10.0)), 180.0, 1e-9)); // south
        assert!(approx(bearing_from_to(o, Pt::new(-10.0, 0.0)), 270.0, 1e-9)); // west
        assert!(approx(bearing_from_to(o, o), 0.0, 1e-9)); // degenerate → 0
    }

    // ── Centroid / spread / bounds ───────────────────────────────────────────────────────────────
    #[test]
    fn centroid_and_spread_and_bounds() {
        let p = pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
        assert_eq!(centroid(&p), Pt::new(5.0, 5.0));
        // Corner-to-centroid distance of a 10×10 square = sqrt(50).
        assert!(approx(max_spread(&p), 50.0_f64.sqrt(), 1e-9));
        assert_eq!(bounds(&p), (0.0, 0.0, 10.0, 10.0));
        assert_eq!(centroid(&[]), Pt::new(0.0, 0.0));
        assert_eq!(max_spread(&pts(&[(3.0, 3.0)])), 0.0);
    }

    // ══════════════════════ PATTERN GOLDENS (known fixtures) ═════════════════════════════════════

    /// CIRCULAR — 4 entities clustered at the corners of a 2×2 box centred at (100,100). Centroid is
    /// (100,100); max spread = sqrt(2) < 5 so radius clamps to the 5 m floor. i=0 due north, i=1 due
    /// east, i=2 due south, i=3 due west of the centroid.
    #[test]
    fn circular_golden_radius_floor_and_cardinals() {
        let p = pts(&[(99.0, 99.0), (101.0, 99.0), (101.0, 101.0), (99.0, 101.0)]);
        let out = pattern_circular(&p);
        let c = Pt::new(100.0, 100.0);
        let r = 5.0;
        let expected = [
            Pt::new(c.x, c.y + r), // i=0 north
            Pt::new(c.x + r, c.y), // i=1 east
            Pt::new(c.x, c.y - r), // i=2 south
            Pt::new(c.x - r, c.y), // i=3 west
        ];
        for (g, e) in out.iter().zip(expected.iter()) {
            assert!(pt_approx(*g, *e, 1e-9), "circular {g:?} vs {e:?}");
        }
        // Every point is exactly `r` from the centroid.
        for g in &out {
            assert!(approx(
                ((g.x - c.x).powi(2) + (g.y - c.y).powi(2)).sqrt(),
                r,
                1e-9
            ));
        }
    }

    /// CIRCULAR — radius follows max spread when it exceeds 5 m. 2 entities 40 m apart on the x-axis:
    /// centroid mid-point, spread = 20, so radius = 20 (not the floor). i=0 north, i=1 south.
    #[test]
    fn circular_golden_radius_from_spread() {
        let p = pts(&[(0.0, 0.0), (40.0, 0.0)]);
        let out = pattern_circular(&p);
        let c = Pt::new(20.0, 0.0);
        assert!(pt_approx(out[0], Pt::new(c.x, c.y + 20.0), 1e-9));
        assert!(pt_approx(out[1], Pt::new(c.x, c.y - 20.0), 1e-9));
    }

    /// LINE — 3 entities already roughly on the x-axis. Principal axis = east; equal spacing across
    /// the current x-extent [0,20] → 0,10,20 at y=centroid. The middle point (which was off-axis)
    /// collapses onto the line, spaced to the midpoint.
    #[test]
    fn line_golden_horizontal_equal_spacing() {
        let p = pts(&[(0.0, 0.0), (10.0, 3.0), (20.0, 0.0)]);
        let out = pattern_line(&p);
        let cy = 1.0; // centroid y = (0+3+0)/3 = 1
        assert!(pt_approx(out[0], Pt::new(0.0, cy), 1e-9), "{:?}", out[0]);
        assert!(pt_approx(out[1], Pt::new(10.0, cy), 1e-9), "{:?}", out[1]);
        assert!(pt_approx(out[2], Pt::new(20.0, cy), 1e-9), "{:?}", out[2]);
    }

    /// GRID — 4 entities → 2×2 grid, 5 m cell, centred on the centroid (0,0). Row-major from the
    /// NW cell: (−2.5,+2.5) (+2.5,+2.5) / (−2.5,−2.5) (+2.5,−2.5).
    #[test]
    fn grid_golden_two_by_two() {
        let p = pts(&[(1.0, 1.0), (2.0, -1.0), (-1.0, 2.0), (-2.0, -2.0)]);
        // centroid = (0,0)
        let out = pattern_grid(&p);
        let expected = [
            Pt::new(-2.5, 2.5),  // i=0 NW
            Pt::new(2.5, 2.5),   // i=1 NE
            Pt::new(-2.5, -2.5), // i=2 SW
            Pt::new(2.5, -2.5),  // i=3 SE
        ];
        for (g, e) in out.iter().zip(expected.iter()) {
            assert!(pt_approx(*g, *e, 1e-9), "grid {g:?} vs {e:?}");
        }
    }

    /// GRID — 5 entities → 3 cols (ceil(sqrt(5))=3) × 2 rows.
    #[test]
    fn grid_golden_five_is_three_by_two() {
        let p = pts(&[(0.0, 0.0); 5]); // all coincident; centroid (0,0)
        let out = pattern_grid_cell(&p, 10.0);
        // cols=3, rows=2, block 20 wide, 10 tall, centred (0,0): x0=-10, y0=+5.
        let expected = [
            Pt::new(-10.0, 5.0),
            Pt::new(0.0, 5.0),
            Pt::new(10.0, 5.0),
            Pt::new(-10.0, -5.0),
            Pt::new(0.0, -5.0),
        ];
        for (g, e) in out.iter().zip(expected.iter()) {
            assert!(pt_approx(*g, *e, 1e-9), "grid5 {g:?} vs {e:?}");
        }
    }

    /// FILL AREA — determinism pin: the SAME ids scatter identically across two runs, and every
    /// scattered point lands inside the selection hull. A DIFFERENT seed scatters differently.
    #[test]
    fn fill_area_deterministic_and_contained() {
        let p = pts(&[
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (50.0, 50.0),
        ]);
        let ids: Vec<String> = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let seed = seed_from_ids(&ids);
        let run1 = pattern_fill_area(&p, seed);
        let run2 = pattern_fill_area(&p, seed);
        assert_eq!(run1, run2, "same seed → identical scatter (reproducible)");
        let hull = convex_hull(&p);
        for g in &run1 {
            assert!(
                point_in_convex_hull(&hull, *g),
                "scattered point {g:?} must be inside the hull"
            );
        }
        // A different seed gives a different arrangement (overwhelmingly — pin it).
        let run3 = pattern_fill_area(&p, seed.wrapping_add(1));
        assert_ne!(run1, run3, "different seed → different scatter");
    }

    /// FILL AREA — seed is ORDER-INDEPENDENT: the same id SET in a different order scatters the same.
    #[test]
    fn fill_area_seed_order_independent() {
        let a: Vec<String> = ["s1", "s2", "s3"].iter().map(|s| s.to_string()).collect();
        let b: Vec<String> = ["s3", "s1", "s2"].iter().map(|s| s.to_string()).collect();
        assert_eq!(seed_from_ids(&a), seed_from_ids(&b));
    }

    // ══════════════════════ ALIGN GOLDENS ════════════════════════════════════════════════════════
    #[test]
    fn align_golden_all_six_edges() {
        let p = pts(&[(0.0, 0.0), (10.0, 4.0), (6.0, 10.0)]);
        // bounds: min_x=0 max_x=10 min_y=0 max_y=10; mid_x=5 mid_y=5.
        let left = align_edge(&p, AlignEdge::Left);
        assert_eq!(left, pts(&[(0.0, 0.0), (0.0, 4.0), (0.0, 10.0)]));
        let right = align_edge(&p, AlignEdge::Right);
        assert_eq!(right, pts(&[(10.0, 0.0), (10.0, 4.0), (10.0, 10.0)]));
        let top = align_edge(&p, AlignEdge::Top);
        assert_eq!(top, pts(&[(0.0, 10.0), (10.0, 10.0), (6.0, 10.0)]));
        let bottom = align_edge(&p, AlignEdge::Bottom);
        assert_eq!(bottom, pts(&[(0.0, 0.0), (10.0, 0.0), (6.0, 0.0)]));
        let ch = align_edge(&p, AlignEdge::CentreH);
        assert_eq!(ch, pts(&[(5.0, 0.0), (5.0, 4.0), (5.0, 10.0)]));
        let cv = align_edge(&p, AlignEdge::CentreV);
        assert_eq!(cv, pts(&[(0.0, 5.0), (10.0, 5.0), (6.0, 5.0)]));
    }

    // ══════════════════════ SPACE GOLDENS ════════════════════════════════════════════════════════

    /// HORIZONTAL — 3 entities at x=0,3,10 (interior bunched left) → even to 0,5,10; y preserved.
    #[test]
    fn space_golden_horizontal_evens_interior() {
        let p = pts(&[(0.0, 1.0), (3.0, 2.0), (10.0, 3.0)]);
        let out = space_equally(&p, SpaceAxis::Horizontal);
        assert_eq!(out, pts(&[(0.0, 1.0), (5.0, 2.0), (10.0, 3.0)]));
    }

    /// VERTICAL — same idea on y.
    #[test]
    fn space_golden_vertical_evens_interior() {
        let p = pts(&[(1.0, 0.0), (2.0, 8.0), (3.0, 10.0)]);
        let out = space_equally(&p, SpaceAxis::Vertical);
        assert_eq!(out, pts(&[(1.0, 0.0), (2.0, 5.0), (3.0, 10.0)]));
    }

    /// ALONG LINE — 3 entities on the x-axis with the middle bunched, all y=0. Principal axis = east;
    /// along-line spacing evens x to 0,5,10 and the (zero) perpendicular offset is preserved.
    #[test]
    fn space_golden_along_line() {
        let p = pts(&[(0.0, 0.0), (3.0, 0.0), (10.0, 0.0)]);
        let out = space_equally(&p, SpaceAxis::AlongLine);
        assert!(pt_approx(out[0], Pt::new(0.0, 0.0), 1e-9), "{:?}", out[0]);
        assert!(pt_approx(out[1], Pt::new(5.0, 0.0), 1e-9), "{:?}", out[1]);
        assert!(pt_approx(out[2], Pt::new(10.0, 0.0), 1e-9), "{:?}", out[2]);
    }

    /// ALONG LINE preserves a perpendicular offset (unlike a strict Line). One point sits 2 m off the
    /// axis; after spacing it keeps that 2 m offset.
    #[test]
    fn space_along_line_keeps_perpendicular_offset() {
        let p = pts(&[(0.0, 0.0), (5.0, 2.0), (10.0, 0.0)]);
        let out = space_equally(&p, SpaceAxis::AlongLine);
        // Axis is (near) east; the middle keeps ~x=5 and ~y=2 (its perpendicular offset survives).
        assert!(
            approx(out[1].y, 2.0, 1e-6),
            "perp offset preserved: {:?}",
            out[1]
        );
    }

    // ══════════════════════ ORIENT GOLDENS ═══════════════════════════════════════════════════════
    #[test]
    fn orient_golden_cardinals_and_face() {
        let pivot = Pt::new(0.0, 0.0);
        // Cardinals are absolute, position-independent.
        let anywhere = Pt::new(37.0, -12.0);
        assert_eq!(orient_yaw(Orient::North, anywhere, pivot), Some(0.0));
        assert_eq!(orient_yaw(Orient::East, anywhere, pivot), Some(90.0));
        assert_eq!(orient_yaw(Orient::South, anywhere, pivot), Some(180.0));
        assert_eq!(orient_yaw(Orient::West, anywhere, pivot), Some(270.0));
        // Face-centre: an entity due EAST of the pivot faces WEST (270) to look at the centre.
        let east_of = Pt::new(10.0, 0.0);
        assert!(approx(
            orient_yaw(Orient::FaceCentre, east_of, pivot).unwrap(),
            270.0,
            1e-9
        ));
        // Face-away: the same entity faces EAST (90), directly away from the centre.
        assert!(approx(
            orient_yaw(Orient::FaceAway, east_of, pivot).unwrap(),
            90.0,
            1e-9
        ));
        // An entity due NORTH of the pivot faces SOUTH (180) to face-centre.
        let north_of = Pt::new(0.0, 10.0);
        assert!(approx(
            orient_yaw(Orient::FaceCentre, north_of, pivot).unwrap(),
            180.0,
            1e-9
        ));
        // Degenerate: entity ON the pivot declines a face command (None), cardinals still answer.
        assert_eq!(orient_yaw(Orient::FaceCentre, pivot, pivot), None);
        assert_eq!(orient_yaw(Orient::North, pivot, pivot), Some(0.0));
    }

    // ══════════════════════ GARRISON GOLDENS (pure kernel) ═══════════════════════════════════════

    /// An axis-aligned 12×12 building (half 6) centred at origin, rotation 0, 4 firing positions.
    /// Inset 1 m → inset half-extents 5. Perimeter = 40; 4 points at arc-lengths 0,10,20,30 clockwise
    /// from the NW corner (−5,+5): NW corner, NE corner, SE corner, SW corner. Each faces outward.
    #[test]
    fn garrison_golden_square_four_positions() {
        let fp = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 4);
        assert_eq!(fp.len(), 4);
        // s=0 → NW corner (−5,+5); on the top wall so it faces NORTH (0°).
        assert!(
            pt_approx(fp[0].pos, Pt::new(-5.0, 5.0), 1e-9),
            "{:?}",
            fp[0]
        );
        assert!(approx(fp[0].yaw_deg, 0.0, 1e-9));
        // s=10 → NE corner (+5,+5); this arc-length starts the RIGHT wall, facing EAST (90°).
        assert!(pt_approx(fp[1].pos, Pt::new(5.0, 5.0), 1e-9), "{:?}", fp[1]);
        assert!(approx(fp[1].yaw_deg, 90.0, 1e-9));
        // s=20 → SE corner (+5,−5); starts the BOTTOM wall, facing SOUTH (180°).
        assert!(
            pt_approx(fp[2].pos, Pt::new(5.0, -5.0), 1e-9),
            "{:?}",
            fp[2]
        );
        assert!(approx(fp[2].yaw_deg, 180.0, 1e-9));
        // s=30 → SW corner (−5,−5); starts the LEFT wall, facing WEST (270°).
        assert!(
            pt_approx(fp[3].pos, Pt::new(-5.0, -5.0), 1e-9),
            "{:?}",
            fp[3]
        );
        assert!(approx(fp[3].yaw_deg, 270.0, 1e-9));
    }

    /// Cap: the count caps the positions; a too-small building (collapses under the 1 m inset) yields
    /// none; count 0 yields none.
    #[test]
    fn garrison_caps_and_degenerate() {
        assert_eq!(
            garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 0).len(),
            0
        );
        assert_eq!(
            garrison_firing_positions(0.0, 0.0, 1.0, 6.0, 0.0, 4).len(),
            0
        ); // half_x≤1
        assert_eq!(
            garrison_firing_positions(0.0, 0.0, 0.5, 0.5, 0.0, 4).len(),
            0
        );
        assert_eq!(
            garrison_firing_positions(0.0, 0.0, 10.0, 10.0, 0.0, 8).len(),
            8
        );
    }

    /// Rotation carries into both the positions and the outward yaws. A 12×12 building rotated 90°:
    /// the NW-corner point rotates and its outward normal (was north) becomes east → yaw 90°.
    #[test]
    fn garrison_rotation_rotates_positions_and_yaw() {
        let fp0 = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 0.0, 4);
        let fp90 = garrison_firing_positions(0.0, 0.0, 6.0, 6.0, 90.0, 4);
        // First point's yaw was 0 (north wall); rotating the building 90° makes it face 90 (east).
        assert!(approx(fp90[0].yaw_deg, 90.0, 1e-9), "{:?}", fp90[0]);
        // Position rotated 90° clockwise about origin: obb_corners' (−5,+5) at rot 90 → (+5,+5).
        assert!(
            pt_approx(fp90[0].pos, Pt::new(5.0, 5.0), 1e-9),
            "{:?}",
            fp90[0]
        );
        // The set of positions is the same ring, just rotated — same count, all on the inset rect.
        assert_eq!(fp0.len(), fp90.len());
    }

    // ── < 2 selection is a no-op for every pattern (nothing to arrange) ──────────────────────────
    #[test]
    fn patterns_noop_below_two() {
        let one = pts(&[(3.0, 4.0)]);
        assert_eq!(pattern_circular(&one), one);
        assert_eq!(pattern_line(&one), one);
        assert_eq!(pattern_grid(&one), one);
        assert_eq!(pattern_fill_area(&one, 42), one);
        assert_eq!(align_edge(&one, AlignEdge::Left), one);
        assert_eq!(space_equally(&one, SpaceAxis::Horizontal), one);
    }

    // ── principal_axis + convex_hull primitive sanity ───────────────────────────────────────────
    #[test]
    fn principal_axis_picks_dominant_spread() {
        // Spread mostly along x → axis ≈ (±1, 0).
        let p = pts(&[(-10.0, 0.5), (0.0, -0.5), (10.0, 0.5)]);
        let (ux, uy) = principal_axis(&p);
        assert!(ux.abs() > 0.99 && uy.abs() < 0.1, "axis=({ux},{uy})");
        // Spread mostly along y → axis ≈ (0, ±1).
        let q = pts(&[(0.5, -10.0), (-0.5, 0.0), (0.5, 10.0)]);
        let (vx, vy) = principal_axis(&q);
        assert!(vy.abs() > 0.99 && vx.abs() < 0.1, "axis=({vx},{vy})");
    }

    #[test]
    fn convex_hull_of_square_with_interior_point() {
        let p = pts(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (5.0, 5.0),
        ]);
        let hull = convex_hull(&p);
        assert_eq!(hull.len(), 4, "interior point dropped");
        // The interior point is inside; a far point is outside.
        assert!(point_in_convex_hull(&hull, Pt::new(5.0, 5.0)));
        assert!(point_in_convex_hull(&hull, Pt::new(0.0, 0.0))); // on boundary
        assert!(!point_in_convex_hull(&hull, Pt::new(-1.0, 5.0)));
    }
}
