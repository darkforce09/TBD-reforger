//! **Role:** lane preferences — the string → lane cartography table the world layer toggles
//! speak, the texture lane opacity, and the 1 km grid lane.
//! **Position:** `symbology_layers_gpu`; the renderer's asset sink forwards the streaming
//! host's and the world loader's preference writes here with itself as the lane sink.
//! **Signals & state:** none of its own; the lanes' visibility and the textured lanes' tint live
//! in the renderer's batches.
//! **Invariants:** [`set_world_layer_visible`] maps a layer *name* to its lanes, which makes it
//! cartography, not renderer state; [`set_lane_opacity`] writes a tint at byte offset 16 of a
//! live instance buffer and toggles the lane without rebuilding its texture.

use gpu_frame::draw::lines;
use gpu_frame::frame::{DrawBatch, DrawPayload};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use map_draw_lanes::lane_roles::tex_lane_role_from_u32;
use renderer_core::lane_sink::LaneSink;
use renderer_core::packet_bindings;

/// Build the 1 km grid over a `width`×`height` terrain and upsert it as the grid lane, or remove
/// the lane when the grid is empty.
pub fn set_grid<T>(
    lanes: &mut dyn LaneSink<T>,
    width: f64,
    height: f64,
    over_hillshade: bool,
    visible: bool,
) {
    let verts = grid_lines(width, height, over_hillshade);
    if verts.is_empty() {
        lanes.remove_lane_batch(lane_id(LaneRole::Grid));
        return;
    }
    let stream = lines::upload_line_stream(lanes.layer_context().device(), "grid-lines", &verts);
    lanes.upsert_lane_batch(DrawBatch {
        lane: lane_id(LaneRole::Grid),
        visible,
        pipeline: packet_bindings::PIPE_LINE,
        payload: DrawPayload::Lines(stream),
    });
}

/// Show or hide the lanes of world layer `key` (`roads`, `forest`, `contours`, `sea`,
/// `airfield`, `heights`, `townLabels`, `roadNames`; any other key is ignored), marking damage
/// when a lane changes.
pub fn set_world_layer_visible<T>(lanes: &mut dyn LaneSink<T>, key: &str, visible: bool) {
    let roles: &[LaneRole] = match key {
        "roads" => &[LaneRole::RoadsCasing, LaneRole::Roads],
        "forest" => &[LaneRole::ForestFill, LaneRole::ForestOutline],
        "contours" => &[LaneRole::Contours],
        "sea" => &[LaneRole::Sea],
        "airfield" => &[LaneRole::WorldAirfieldApron],
        "heights" => &[LaneRole::WorldLabels],
        "townLabels" => &[LaneRole::WorldTownLabels],
        "roadNames" => &[LaneRole::WorldRoadLabels],
        _ => return,
    };
    // The world-layer table is cartography — `"roads"` means two lanes, one casing and one
    // surface — so it is resolved to lane ids here, before it ever reaches a batch.
    let mut changed = false;
    for lane in roles.iter().copied().map(lane_id) {
        if let Some(b) = lanes.lane_batch_mut(lane)
            && b.visible != visible
        {
            b.visible = visible;
            changed = true;
        }
    }
    if changed {
        lanes.mark_damage();
    }
}

/// Re-tint texture lane `role`'s opacity in place and set its visibility (`tex_role_id::BASEMAP`
/// or `HILLSHADE`) without rebuilding its texture. The tint alpha is `color[3]` at byte offset 16
/// of the one-instance quad buffer.
pub fn set_lane_opacity<T>(lanes: &mut dyn LaneSink<T>, role: u32, opacity: f32, visible: bool) {
    let Some(want) = tex_lane_role_from_u32(role) else {
        debug_assert!(
            false,
            "set_lane_opacity: role must be 0 (basemap) or 1 (hillshade), got {role} — \
             ignored. Note this is NOT the vector-lane `role_id` namespace."
        );
        return;
    };
    let color = [1.0f32, 1.0, 1.0, opacity.clamp(0.0, 1.0)];
    let want_lane = lane_id(want);
    let target = lanes.lane_batch_mut(want_lane).and_then(|b| {
        b.visible = visible;
        if let DrawPayload::TexturedRect { instances, .. } = &b.payload {
            return Some(instances.buffer.clone());
        }
        None
    });
    if let Some(buf) = target {
        lanes
            .layer_context()
            .queue()
            .write_buffer(&buf, 16, bytemuck::cast_slice(&color));
    }
}

/// Build the procedural 1 km grid as a `LineList` vertex buffer, anchored at `ANCHOR`. The grid
/// is a cartographic overlay with a visibility preference (`over_hillshade`), so it sits with the
/// rest of the lane preferences; the line arithmetic itself is `render_primitives::draw::grid`.
#[must_use]
pub fn grid_lines(
    width: f64,
    height: f64,
    over_hillshade: bool,
) -> Vec<render_primitives::draw::geometry::LineVertex> {
    render_primitives::draw::grid::grid_lines(
        map_coordinates::terrain_frames::ANCHOR,
        width,
        height,
        over_hillshade,
    )
}
