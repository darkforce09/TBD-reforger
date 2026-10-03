//! **Role:** the world icon lanes: the streamed world's trees, props and building badges, moved
//! to the scene anchor and uploaded as glyph-atlas sprite lanes, or handed to the compute cull.
//! **Position:** `symbology_layers_gpu`; the renderer's asset sink forwards the world
//! loader's icon uploads here with its lane sink, its icon cull and its upload counter.
//! **Signals & state:** the three world icon lanes, the compute cull's sources and tree icons.
//! **Invariants:** a lane is 20-byte instances; an empty upload removes the lane and its cull
//! source; a byte count off the stride drops the lane; every call counts one upload.

use crate::icon_cull_gpu::IconCullGpu;
use crate::icon_uniforms::{convert_icon_world_to_anchor, sprite_atlas_for};
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::lane_sink::LaneSink;
use renderer_core::packet_bindings;

/// Upload world icon lane `kind` (0 trees, 1 props, 2 badges; any other kind is ignored) from
/// `bytes` (20-byte instances in world metres) through `lanes`, or into `icon_cull` when it is in
/// use; `uploads` counts the call.
pub fn upload_world_icon_lane<T>(
    lanes: &mut dyn LaneSink<T>,
    icon_cull: &mut IconCullGpu,
    uploads: &mut u64,
    kind: u32,
    bytes: &[u8],
    visible: bool,
) {
    *uploads += 1;
    let role = match kind {
        0 => LaneRole::WorldTrees,
        1 => LaneRole::WorldProps,
        2 => LaneRole::WorldBadges,
        _ => return,
    };
    const STRIDE: usize = 20;
    if bytes.is_empty() {
        if role == LaneRole::WorldTrees {
            icon_cull.clear_tree_icons();
        }
        let context = lanes.layer_context();
        icon_cull.clear_lane(context.device(), context.queue(), role as u32);

        lanes.remove_lane_batch(lane_id(role));
        lanes.mark_damage();
        return;
    }
    if !bytes.len().is_multiple_of(STRIDE) {
        lanes.remove_lane_batch(lane_id(role));
        return;
    }

    let mut converted = bytes.to_vec();
    convert_icon_world_to_anchor(&mut converted);

    if icon_cull.enabled() {
        if role == LaneRole::WorldTrees {
            icon_cull.set_tree_icons(converted.clone());
        }
        let context = lanes.layer_context();
        icon_cull.upload_lane(context.device(), context.queue(), role as u32, &converted);
        lanes.remove_lane_batch(lane_id(role));
        lanes.mark_damage();
        return;
    }

    use wgpu::util::DeviceExt;
    let buf =
        lanes
            .layer_context()
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("icon-lane"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
    #[allow(clippy::cast_possible_truncation)]
    let count = (converted.len() / STRIDE) as u32;
    lanes.upsert_lane_batch(DrawBatch {
        lane: lane_id(role),
        visible,
        pipeline: packet_bindings::PIPE_ICON,
        #[allow(clippy::cast_possible_truncation)]
        payload: DrawPayload::Sprites {
            instances: InstanceBuffer::whole(buf, STRIDE as u32, count),
            atlas: sprite_atlas_for(role),
        },
    });
}
