//! The fire-mission overlay of the map picker: what is drawn and the buffers the render engine
//! takes.
//!
//! **Role:** turns the position drafts and the last solution into the map engine's
//! [`FireMissionPlot`], builds it with
//! [`map_engine::overlay::fire_mission_marks::build_fire_mission_marks`], and packs the
//! result into the three lane uploads: glyphs for the marker lane, the gun→target lines and
//! ellipse outlines for the connection lane, the dispersion fills for the zone lane.
//! **Position:** pure half of the map picker; the browser half (`super::engine_overlay`) hands
//! the packed buffers to the engine.
//! **Signals & state:** none; pure functions over plain values.
//! **Invariants:** the overlay is built with a zero anchor, so every packed coordinate is in world
//! metres, the frame the engine's upload calls take; glyph captions and tints line up one to one
//! with the glyphs (guns in battery order, then the target); a dispersion ellipse is drawn only
//! for a gun whose solution carries one, centred on the target it was solved for; a draft whose
//! grid does not parse draws nothing.

use super::picking::{placed_markers, Placement};
use crate::v2::pages::field_tools::mortar::inputs::battery::GunDraft;
use crate::v2::pages::field_tools::mortar::inputs::positions::PositionDraft;
use crate::v2::pages::field_tools::mortar::solve_bridge::SolvedMission;
use map_engine::overlay::fire_mission_marks::{
    build_fire_mission_marks, DispersionEllipse, FireMissionGlyph, FireMissionMarks,
    FireMissionPlot, BURST_COLOR,
};

/// Half the side of a glyph, CSS pixels; lines stop this far from a glyph's centre.
pub(crate) const GLYPH_HALF_SIZE_PX: f64 = 12.0;

/// Opacity of the dispersion-ellipse fill.
pub(crate) const DISPERSION_FILL_ALPHA: f32 = 0.22;

/// What one redraw of the overlay shows: the plot and one caption per plotted glyph.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct OverlayScene {
    /// Guns, target and dispersion ellipses in world metres.
    pub(crate) plot: FireMissionPlot,
    /// Gun labels in battery order, then "Target" when the target is plotted.
    pub(crate) captions: Vec<String>,
}

/// The scene of the current drafts with the ellipses of `solved`.
pub(crate) fn overlay_scene(
    target: &PositionDraft,
    guns: &[GunDraft],
    solved: Option<&SolvedMission>,
) -> OverlayScene {
    let mut scene = OverlayScene::default();
    for (placement, at) in placed_markers(target, guns) {
        match placement {
            Placement::Gun(key) => {
                scene.plot.guns.push(at);
                let label = guns.iter().find(|g| g.key == key).map(|g| g.label.trim());
                scene.captions.push(label.unwrap_or_default().to_string());
            }
            Placement::Target => scene.plot.target = Some(at),
        }
    }
    if scene.plot.target.is_some() {
        scene.captions.push("Target".to_string());
    }
    if let Some(solved) = solved {
        let centre = [solved.inputs.target.x, solved.inputs.target.y];
        scene.plot.dispersion = solved
            .solution
            .guns
            .iter()
            .filter_map(|gun| gun.dispersion.as_ref())
            .map(|d| DispersionEllipse {
                centre,
                major_semi_axis_m: d.ellipse_semi_major_m,
                minor_semi_axis_m: d.ellipse_semi_minor_m,
                major_axis_azimuth_rad: d.ellipse_orientation_deg.to_radians(),
            })
            .collect();
    }
    scene
}

/// The engine icon alias of a glyph.
pub(crate) fn glyph_icon(glyph: FireMissionGlyph) -> &'static str {
    match glyph {
        FireMissionGlyph::Gun => "defend",
        FireMissionGlyph::Target => "target",
        FireMissionGlyph::Burst => "destroy",
    }
}

/// The three lane uploads of one overlay, world metres.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct LaneUploads {
    /// Glyph centres, `x, y` pairs.
    pub(crate) glyph_xy: Vec<f32>,
    /// Glyph tints, RGBA bytes.
    pub(crate) glyph_rgba: Vec<u8>,
    /// Glyph icon aliases.
    pub(crate) glyph_icons: Vec<String>,
    /// Glyph captions.
    pub(crate) glyph_captions: Vec<String>,
    /// Line-list vertices, `x, y, r, g, b, a` each.
    pub(crate) line_packed: Vec<f32>,
    /// Line segments in `line_packed`.
    pub(crate) line_segments: u32,
    /// Fill vertex positions, `x, y` pairs.
    pub(crate) fill_positions: Vec<f32>,
    /// Fill vertex colours, RGBA each.
    pub(crate) fill_colors: Vec<f32>,
    /// Fill triangle indices.
    pub(crate) fill_indices: Vec<u32>,
    /// Filled ellipses.
    pub(crate) fill_count: u32,
}

fn tint_bytes(color: [f32; 4]) -> [u8; 4] {
    color.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Packs `scene` built at `glyph_half_size_m` into the lane uploads.
pub(crate) fn lane_uploads(scene: &OverlayScene, glyph_half_size_m: f64) -> LaneUploads {
    let marks: FireMissionMarks =
        build_fire_mission_marks(&scene.plot, glyph_half_size_m, [0.0; 2]);
    let mut out = LaneUploads::default();
    for (quad, caption) in marks.glyph_quads.iter().zip(&scene.captions) {
        out.glyph_xy.extend_from_slice(&quad.centre);
        out.glyph_rgba.extend_from_slice(&tint_bytes(quad.color));
        out.glyph_icons.push(glyph_icon(quad.glyph).to_string());
        out.glyph_captions.push(caption.clone());
    }
    for vertex in &marks.line_vertices {
        out.line_packed.extend_from_slice(&vertex.pos);
        out.line_packed.extend_from_slice(&vertex.color);
    }
    out.line_segments = (marks.line_vertices.len() / 2) as u32;
    let fill = [
        BURST_COLOR[0],
        BURST_COLOR[1],
        BURST_COLOR[2],
        DISPERSION_FILL_ALPHA,
    ];
    for ring in &marks.dispersion_rings {
        // An ellipse ring is convex, so a fan from its first vertex triangulates it exactly.
        let base = (out.fill_positions.len() / 2) as u32;
        for point in ring {
            out.fill_positions.push(point[0] as f32);
            out.fill_positions.push(point[1] as f32);
            out.fill_colors.extend_from_slice(&fill);
        }
        for i in 1..ring.len().saturating_sub(1) as u32 {
            out.fill_indices
                .extend_from_slice(&[base, base + i, base + i + 1]);
        }
        out.fill_count += 1;
    }
    out
}
