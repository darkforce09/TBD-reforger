//! **Role:** the fire-mission overlay's geometry — gun, target and burst glyph quads, the
//! gun→target lines and the dispersion ellipses of a solved fire mission, each assigned to the
//! existing overlay lane it paints in.
//! **Position:** `overlay_instances`. The mortar page's map picker hands in world
//! positions from the ballistics solution; the render host uploads the output into
//! [`FIRE_MISSION_GLYPH_LANE`], [`FIRE_MISSION_LINE_LANE`] and [`FIRE_MISSION_DISPERSION_LANE`]
//! through the map engine's frame builder like every other lane.
//! **Signals & state:** none; pure functions from one [`FireMissionPlot`] to one
//! [`FireMissionMarks`].
//! **Invariants:** the overlay owns no GPU resource and builds no buffer. Every emitted
//! coordinate is finite: a gun, target, burst or ellipse with a non-finite or non-positive input
//! is skipped rather than drawn. Positions leave anchor-relative in f32 (world metres minus the
//! caller's anchor), matching [`LineVertex`] and the rest of the lane geometry. A gun→target line
//! stops at the edge of both glyphs, so the line lane, which paints above the glyph lane, never
//! overprints a glyph it joins. The paint order is the lane order: dispersion fill, then glyphs,
//! then lines and ellipse outlines.

use map_draw_lanes::lane_roles::LaneRole;
use render_primitives::draw::geometry::LineVertex;

/// The lane of the gun, target and burst glyph quads.
pub const FIRE_MISSION_GLYPH_LANE: LaneRole = LaneRole::MissionMarkers;

/// The lane of the gun→target lines and the dispersion-ellipse outlines (one `LineList`).
pub const FIRE_MISSION_LINE_LANE: LaneRole = LaneRole::MissionConnections;

/// The lane of the dispersion-ellipse fill polygons, below the glyphs they surround.
pub const FIRE_MISSION_DISPERSION_LANE: LaneRole = LaneRole::MissionZones;

/// Vertices on one dispersion-ellipse ring. 64 keeps the chord error under 0.13 % of the major
/// semi-axis, invisible at any zoom the ellipse is legible at.
pub const DISPERSION_RING_VERTICES: usize = 64;

/// Gun glyph tint: friendly blue, normalised RGBA.
pub const GUN_COLOR: [f32; 4] = [0.24, 0.52, 0.96, 1.0];

/// Target glyph tint: hostile red, normalised RGBA.
pub const TARGET_COLOR: [f32; 4] = [0.93, 0.26, 0.21, 1.0];

/// Burst glyph tint: amber, normalised RGBA.
pub const BURST_COLOR: [f32; 4] = [0.98, 0.70, 0.13, 1.0];

/// Gun→target line tint, normalised RGBA.
pub const GUN_TARGET_LINE_COLOR: [f32; 4] = [0.95, 0.95, 0.95, 0.85];

/// Dispersion-ellipse outline tint, normalised RGBA.
pub const DISPERSION_OUTLINE_COLOR: [f32; 4] = [0.98, 0.70, 0.13, 0.9];

/// Which symbol a glyph quad carries; the render host picks the sprite from it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FireMissionGlyph {
    /// A gun (mortar tube) position.
    Gun,
    /// The target point.
    Target,
    /// The predicted burst or mean point of impact.
    Burst,
}

impl FireMissionGlyph {
    /// The glyph's tint: [`GUN_COLOR`], [`TARGET_COLOR`] or [`BURST_COLOR`].
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Gun => GUN_COLOR,
            Self::Target => TARGET_COLOR,
            Self::Burst => BURST_COLOR,
        }
    }
}

/// A dispersion ellipse on the ground, world metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DispersionEllipse {
    /// Centre `(x east, y north)` in world metres.
    pub centre: [f64; 2],
    /// Semi-axis along [`Self::major_axis_azimuth_rad`], metres (> 0).
    pub major_semi_axis_m: f64,
    /// Semi-axis perpendicular to it, metres (> 0).
    pub minor_semi_axis_m: f64,
    /// Azimuth of the major axis, radians clockwise from north (the line of fire for a range
    /// ellipse).
    pub major_axis_azimuth_rad: f64,
}

/// The world positions of one fire mission to draw.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FireMissionPlot {
    /// Gun positions `(x, y)`, world metres; each is joined to the target.
    pub guns: Vec<[f64; 2]>,
    /// The target point, world metres; `None` draws the guns alone.
    pub target: Option<[f64; 2]>,
    /// The burst point, world metres, when it differs from the target.
    pub burst: Option<[f64; 2]>,
    /// One dispersion ellipse per solved gun (or one for the battery).
    pub dispersion: Vec<DispersionEllipse>,
}

/// One glyph quad, anchor-relative world metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FireMissionGlyphQuad {
    /// The symbol it carries.
    pub glyph: FireMissionGlyph,
    /// The quad's centre.
    pub centre: [f32; 2],
    /// Corners counter-clockwise from bottom-left: `(-h, -h)`, `(h, -h)`, `(h, h)`, `(-h, h)`
    /// around the centre, with `h` the glyph half-size.
    pub corners: [[f32; 2]; 4],
    /// Tint, normalised RGBA.
    pub color: [f32; 4],
}

/// The fire-mission overlay, split by the lane each part paints in.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct FireMissionMarks {
    /// Glyph quads for [`FIRE_MISSION_GLYPH_LANE`]: guns first, then the target, then the burst.
    pub glyph_quads: Vec<FireMissionGlyphQuad>,
    /// A `LineList` for [`FIRE_MISSION_LINE_LANE`]: one segment per drawable gun→target line,
    /// then [`DISPERSION_RING_VERTICES`] segments per ellipse outline.
    pub line_vertices: Vec<LineVertex>,
    /// Closed-by-convention polygon rings for [`FIRE_MISSION_DISPERSION_LANE`], world metres
    /// (the first vertex is not repeated), ready for
    /// `render_primitives::draw::triangulate::triangulate_simple`.
    pub dispersion_rings: Vec<Vec<[f64; 2]>>,
}

fn is_finite_point(p: [f64; 2]) -> bool {
    p[0].is_finite() && p[1].is_finite()
}

fn relative(anchor: [f64; 2], p: [f64; 2]) -> [f32; 2] {
    [(p[0] - anchor[0]) as f32, (p[1] - anchor[1]) as f32]
}

fn glyph_quad(
    glyph: FireMissionGlyph,
    world: [f64; 2],
    half_size_m: f64,
    anchor: [f64; 2],
) -> FireMissionGlyphQuad {
    let corner = |dx: f64, dy: f64| relative(anchor, [world[0] + dx, world[1] + dy]);
    let h = half_size_m;
    FireMissionGlyphQuad {
        glyph,
        centre: relative(anchor, world),
        corners: [corner(-h, -h), corner(h, -h), corner(h, h), corner(-h, h)],
        color: glyph.color(),
    }
}

/// The gun→target segment shortened by `trim_m` at both ends, or `None` when the glyphs touch
/// or overlap and no line is left to draw.
fn trimmed_segment(from: [f64; 2], to: [f64; 2], trim_m: f64) -> Option<([f64; 2], [f64; 2])> {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 2.0 * trim_m {
        return None;
    }
    let (ux, uy) = (dx / length, dy / length);
    Some((
        [from[0] + ux * trim_m, from[1] + uy * trim_m],
        [to[0] - ux * trim_m, to[1] - uy * trim_m],
    ))
}

/// The polygon ring of a dispersion ellipse: [`DISPERSION_RING_VERTICES`] points, the first on
/// the major axis ahead of the centre, turning from the major axis towards the minor axis's
/// clockwise side. `None` when any input is non-finite or a semi-axis is not positive.
#[must_use]
pub fn dispersion_ring(ellipse: &DispersionEllipse) -> Option<Vec<[f64; 2]>> {
    let DispersionEllipse {
        centre,
        major_semi_axis_m: a,
        minor_semi_axis_m: b,
        major_axis_azimuth_rad: azimuth,
    } = *ellipse;
    if !is_finite_point(centre) || !a.is_finite() || !b.is_finite() || !azimuth.is_finite() {
        return None;
    }
    if a <= 0.0 || b <= 0.0 {
        return None;
    }
    // Azimuth is clockwise from north: the major axis points (sin, cos); its clockwise normal,
    // the minor axis, points (cos, -sin).
    let (sin_az, cos_az) = azimuth.sin_cos();
    let major = [sin_az, cos_az];
    let minor = [cos_az, -sin_az];
    let ring = (0..DISPERSION_RING_VERTICES)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / DISPERSION_RING_VERTICES as f64;
            let (sin_t, cos_t) = t.sin_cos();
            [
                centre[0] + a * cos_t * major[0] + b * sin_t * minor[0],
                centre[1] + a * cos_t * major[1] + b * sin_t * minor[1],
            ]
        })
        .collect();
    Some(ring)
}

/// Builds the fire-mission overlay for `plot`. Glyphs are squares of side `2 × glyph_half_size_m`
/// world metres (the caller converts its on-screen glyph size at the current zoom); positions are
/// relative to `anchor`, the world origin the render host folds coordinates against
/// (`map_coordinates::terrain_frames::ANCHOR` for the served terrains). A non-finite or negative glyph size
/// draws no glyph and untrimmed lines.
#[must_use]
pub fn build_fire_mission_marks(
    plot: &FireMissionPlot,
    glyph_half_size_m: f64,
    anchor: [f64; 2],
) -> FireMissionMarks {
    let glyphs_drawn = glyph_half_size_m.is_finite() && glyph_half_size_m > 0.0;
    let trim_m = if glyphs_drawn { glyph_half_size_m } else { 0.0 };
    let guns: Vec<[f64; 2]> = plot
        .guns
        .iter()
        .copied()
        .filter(|g| is_finite_point(*g))
        .collect();
    let target = plot.target.filter(|t| is_finite_point(*t));
    let burst = plot.burst.filter(|b| is_finite_point(*b));

    let mut marks = FireMissionMarks::default();
    if glyphs_drawn {
        let placed = guns
            .iter()
            .map(|g| (FireMissionGlyph::Gun, *g))
            .chain(target.map(|t| (FireMissionGlyph::Target, t)))
            .chain(burst.map(|b| (FireMissionGlyph::Burst, b)));
        marks.glyph_quads = placed
            .map(|(glyph, world)| glyph_quad(glyph, world, glyph_half_size_m, anchor))
            .collect();
    }

    if let Some(target) = target {
        for gun in &guns {
            if let Some((start, end)) = trimmed_segment(*gun, target, trim_m) {
                marks.line_vertices.push(LineVertex {
                    pos: relative(anchor, start),
                    color: GUN_TARGET_LINE_COLOR,
                });
                marks.line_vertices.push(LineVertex {
                    pos: relative(anchor, end),
                    color: GUN_TARGET_LINE_COLOR,
                });
            }
        }
    }

    for ellipse in &plot.dispersion {
        let Some(ring) = dispersion_ring(ellipse) else {
            continue;
        };
        for (i, point) in ring.iter().enumerate() {
            let next = ring[(i + 1) % ring.len()];
            marks.line_vertices.push(LineVertex {
                pos: relative(anchor, *point),
                color: DISPERSION_OUTLINE_COLOR,
            });
            marks.line_vertices.push(LineVertex {
                pos: relative(anchor, next),
                color: DISPERSION_OUTLINE_COLOR,
            });
        }
        marks.dispersion_rings.push(ring);
    }
    marks
}

#[cfg(test)]
#[path = "tests/fire_mission_marks_tests.rs"]
mod tests;
