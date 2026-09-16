//! Role: alignment.
//! Position: `doc/operations/placement` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Pt;
use super::bearing_from_to;
use super::bounds;
use super::centroid;
use super::principal_axis;

/// The six align edges: the 4 bounding-box edges + the 2 centre axes. Left/right/top/bottom snap every entity's matching coordinate to that box edge; centre-h/centre-v snap to the box mid-line. (Top = north = max y; bottom = south = min y — the north-up convention.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignEdge {
    /// Domain representation of left.
    Left,

    /// Domain representation of right.
    Right,

    /// Domain representation of top.
    Top,

    /// Domain representation of bottom.
    Bottom,

    /// Horizontal centres — every entity to the same **x** (the vertical mid-line): a vertical stack.
    CentreH,

    /// Vertical centres — every entity to the same **y** (the horizontal mid-line): a horizontal row.
    CentreV,
}

/// Align every point to `edge` of the selection's own bounding box. Only the affected axis moves; the other coordinate is preserved (Left sets x=min_x, keeps y; Top sets y=max_y, keeps x; CentreH sets x to the box mid-x, keeps y; …). `< 2` points is a no-op. The box is the selection's current bounds, so an align never moves the group as a whole off where it sits — it only collapses one axis.
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

/// The three space-equally axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceAxis {
    /// Distribute along **x** (horizontal): equal x-gaps between the extreme-x entities; y preserved.
    Horizontal,

    /// Distribute along **y** (vertical): equal y-gaps between the extreme-y entities; x preserved.
    Vertical,

    /// Distribute **along the selection's principal axis** (like [`pattern_line`], but each entity keeps its perpendicular offset from the axis — a "space along the line" that respects a slightly-off-axis scatter rather than collapsing it to a strict line).
    AlongLine,
}

/// Space the selection equally along `axis`. Horizontal/Vertical keep the two extreme entities fixed and redistribute the interior ones to equal gaps between them (the standard "distribute" — the span does not change, only the interior spacing evens out). `< 3` points is a no-op for Horizontal/Vertical (with 2, they are already "equally spaced"; with 1/0 there is nothing to do).
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

/// Shared body for Horizontal (`horizontal = true`, spaces x) / Vertical (spaces y). Keeps the two extremes on the spaced axis fixed; the other coordinate is untouched.
pub fn space_axis_aligned(pts: &[Pt], horizontal: bool) -> Vec<Pt> {
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

/// AlongLine — even the along-axis coordinate on the principal axis while preserving each entity's perpendicular offset from that axis (so a slightly-scattered near-line spaces out but keeps its character). Ordered by projection, extremes fixed.
pub fn space_along_line(pts: &[Pt]) -> Vec<Pt> {
    let n = pts.len();
    let c = centroid(pts);
    let (ux, uy) = principal_axis(pts);

    let (px, py) = (-uy, ux);

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

        out[orig_i] = Pt::new(c.x + ux * t + px * s, c.y + uy * t + py * s);
    }
    out
}

/// The six orient commands → a per-entity target yaw (degrees clockwise from north). The four cardinals are ABSOLUTE headings; face-centre / face-away are bearings relative to a pivot (the selection centroid), computed per entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orient {
    /// Domain representation of north.
    North,

    /// Domain representation of east.
    East,

    /// Domain representation of south.
    South,

    /// Domain representation of west.
    West,

    /// Every entity turns to FACE the selection centroid.
    FaceCentre,

    /// Every entity turns to face directly AWAY from the selection centroid (centre bearing + 180°).
    FaceAway,
}

impl Orient {
    /// The fixed cardinal heading for the four absolute commands; `None` for the two face commands (which are per-entity, resolved by [`orient_yaw`]).
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

/// Orient yaw using the supplied domain data.
#[must_use]
pub fn orient_yaw(cmd: Orient, pos: Pt, pivot: Pt) -> Option<f64> {
    if let Some(deg) = cmd.cardinal_deg() {
        return Some(deg);
    }

    if pos.x == pivot.x && pos.y == pivot.y {
        return None;
    }
    let to_centre = bearing_from_to(pos, pivot);
    Some(match cmd {
        Orient::FaceAway => (to_centre + 180.0).rem_euclid(360.0),
        _ => to_centre,
    })
}
