//! Role: draw encode.
//! Position: `draw` in the graphics engine.
//! Signals & state: one render pass's worth of `set_*` and `draw` calls.
//! Invariants: **this module never asks what a lane is.** Which pipeline to bind and which
//! bind group to sample are fields on the batch, decided by the caller. The only thing the
//! encoder decides for itself is which GROUP index a payload binds at — group 0 is the
//! camera for every draw, group 1 belongs to a textured rect's own texture, group 2 to any
//! atlas — and that is a property of the vertex layout, not of the subject.

use crate::frame::batch::{DrawBatch, DrawPayload};
use crate::frame::buffers::InstanceBuffer;
use crate::frame::ids::BindGroupId;
use crate::frame::packet::FramePacket;
use crate::frame::text::TextRun;

fn group<'a>(packet: &'a FramePacket<'a>, id: BindGroupId) -> Option<&'a wgpu::BindGroup> {
    packet
        .bind_groups
        .get(id.0 as usize)
        .and_then(Option::as_ref)
}

fn sprites<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    packet: &'a FramePacket<'a>,
    camera: &'a wgpu::BindGroup,
    pipeline: &'a wgpu::RenderPipeline,
    instances: &'a InstanceBuffer,
    atlas: &'a wgpu::BindGroup,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, camera, &[]);
    pass.set_bind_group(2, atlas, &[]);
    pass.set_vertex_buffer(0, packet.unit_quad.slice(..));
    pass.set_vertex_buffer(1, instances.buffer.slice(instances.offset..));
    pass.draw(0..4, 0..instances.count);
}

fn text_run<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    packet: &'a FramePacket<'a>,
    camera: &'a wgpu::BindGroup,
    run: &'a TextRun,
) {
    let (Some(pipeline), Some(atlas)) = (
        packet.pipelines.get(run.pipeline.0 as usize),
        group(packet, run.atlas),
    ) else {
        return;
    };
    sprites(pass, packet, camera, pipeline, &run.glyphs, atlas);
}

fn batch<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    packet: &'a FramePacket<'a>,
    camera: &'a wgpu::BindGroup,
    b: &'a DrawBatch,
) {
    let Some(pipeline) = packet.pipelines.get(b.pipeline.0 as usize) else {
        return;
    };
    match &b.payload {
        DrawPayload::Quads(instances) | DrawPayload::OrientedQuads(instances) => {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            pass.set_vertex_buffer(0, packet.unit_quad.slice(..));
            pass.set_vertex_buffer(1, instances.buffer.slice(instances.offset..));
            pass.draw(0..4, 0..instances.count);
        }
        DrawPayload::TexturedRect { instances, texture } => {
            let Some(tex) = group(packet, *texture) else {
                return;
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            pass.set_bind_group(1, tex, &[]);
            pass.set_vertex_buffer(0, packet.unit_quad.slice(..));
            pass.set_vertex_buffer(1, instances.buffer.slice(instances.offset..));
            pass.draw(0..4, 0..instances.count);
        }
        DrawPayload::Lines(stream) => {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            pass.set_vertex_buffer(0, stream.vertices.slice(..));
            pass.draw(0..stream.vertex_count, 0..1);
        }
        DrawPayload::Indexed(mesh) => {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
        DrawPayload::Sprites { instances, atlas } => {
            let Some(bg) = group(packet, *atlas) else {
                return;
            };
            sprites(pass, packet, camera, pipeline, instances, bg);
        }
        DrawPayload::SpritesWithText {
            sprites: instances,
            atlas,
            text,
        } => {
            if let Some(bg) = group(packet, *atlas) {
                sprites(pass, packet, camera, pipeline, instances, bg);
            }
            if let Some(run) = text {
                text_run(pass, packet, camera, run);
            }
        }
        DrawPayload::Text(run) => text_run(pass, packet, camera, run),
    }
}

/// Emit every not-yet-emitted indirect draw whose lane is strictly below `before`.
///
/// `before = None` flushes the remainder. This is the k-way merge that keeps indirect draws
/// interleaved with the batch list by lane instead of appended after it.
fn indirect_below<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    packet: &'a FramePacket<'a>,
    camera: &'a wgpu::BindGroup,
    emitted: &mut [bool],
    before: Option<crate::frame::ids::LaneId>,
) {
    for (i, d) in packet.indirect.iter().enumerate() {
        if emitted[i] {
            continue;
        }
        if let Some(lane) = before
            && lane <= d.lane
        {
            continue;
        }
        emitted[i] = true;
        let (Some(pipeline), Some(atlas)) = (
            packet.pipelines.get(d.pipeline.0 as usize),
            group(packet, d.atlas),
        ) else {
            continue;
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(2, atlas, &[]);
        pass.set_vertex_buffer(0, packet.unit_quad.slice(..));
        pass.set_vertex_buffer(1, d.instances.slice(..));
        pass.draw_indirect(d.indirect, 0);
    }
}

/// Same merge, for the glyph runs the caller chose to address separately.
fn text_below<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    packet: &'a FramePacket<'a>,
    camera: &'a wgpu::BindGroup,
    next: &mut usize,
    before: Option<crate::frame::ids::LaneId>,
) {
    while let Some(run) = packet.text.get(*next) {
        if let Some(lane) = before
            && lane <= run.lane
        {
            return;
        }
        *next += 1;
        text_run(pass, packet, camera, run);
    }
}

/// Encode one frame's draw list into `pass`.
pub fn encode<'a>(pass: &mut wgpu::RenderPass<'a>, packet: &'a FramePacket<'a>) {
    debug_assert!(
        packet.batches_sorted(),
        "FramePacket.batches must be ascending by lane — the caller owns paint order"
    );
    let Some(camera) = group(packet, packet.camera_bind) else {
        return;
    };

    let mut emitted = vec![false; packet.indirect.len()];
    let mut next_text = 0usize;

    for b in packet.batches {
        indirect_below(pass, packet, camera, &mut emitted, Some(b.lane));
        text_below(pass, packet, camera, &mut next_text, Some(b.lane));
        if !b.visible {
            continue;
        }
        batch(pass, packet, camera, b);
    }

    indirect_below(pass, packet, camera, &mut emitted, None);
    text_below(pass, packet, camera, &mut next_text, None);
}
