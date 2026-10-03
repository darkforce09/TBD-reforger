//! **Role:** `BuildingLayerGpu`, the building typed layer: the world building footprints, their
//! outlines and the fence strips uploaded as lanes, and the counters the statistics report reads.
//! **Position:** the map renderer holds one as a field and its asset sink forwards the world
//! loader's building, outline and fence uploads here with the renderer's lanes lent as a
//! `renderer_core::lane_sink::LaneSink`.
//! **Signals & state:** the building upload count and the world chunks the last footprint upload
//! drew.
//! **Invariants:** packed rows are anchor-relative (`map_coordinates::terrain_frames::ANCHOR`)
//! and a payload whose length is not a whole number of rows removes its lane; an empty footprint
//! or fence payload removes its lane only when the lane is hidden.

use gpu_frame::draw::{lines as line_buffers, polygons};
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer};
use map_coordinates::terrain_frames::ANCHOR;
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use render_primitives::draw::geometry::LineVertex;
use renderer_core::lane_sink::LaneSink;
use renderer_core::packet_bindings::{PIPE_LINE, PIPE_ORIENTED_QUAD, PIPE_POLYGON};

/// The world building footprints, outlines and fence strips, and their counters.
#[derive(Default)]
pub struct BuildingLayerGpu {
    /// Footprint and outline uploads since the engine started.
    uploads: u64,

    /// The world chunks the last footprint upload drew.
    chunks_drawn: u32,
}

impl BuildingLayerGpu {
    /// No upload yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Footprint and outline uploads since the engine started.
    #[must_use]
    pub fn uploads(&self) -> u64 {
        self.uploads
    }

    /// The world chunks the last footprint upload drew.
    #[must_use]
    pub fn chunks_drawn(&self) -> u32 {
        self.chunks_drawn
    }

    /// Upload the world building footprints: `fill` packs ten `f32` per building (centre x and y
    /// in world metres, half extents, basis, RGBA), drawn as oriented quads.
    pub fn upload_buildings<T>(
        &mut self,
        lanes: &mut dyn LaneSink<T>,
        fill: &[f32],
        chunk_count: u32,
        visible: bool,
    ) {
        self.uploads += 1;
        const STRIDE: usize = 10;

        if fill.is_empty() {
            if !visible {
                lanes.remove_lane_batch(lane_id(LaneRole::WorldBuildings));
                self.chunks_drawn = 0;
            }
            return;
        }
        if !fill.len().is_multiple_of(STRIDE) {
            lanes.remove_lane_batch(lane_id(LaneRole::WorldBuildings));
            self.chunks_drawn = chunk_count;
            return;
        }
        let mut instances = Vec::with_capacity(fill.len() / STRIDE);
        for c in fill.chunks_exact(STRIDE) {
            instances.push(render_primitives::draw::instances::BuildingInstance {
                center: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                half: [c[2], c[3]],
                basis: [c[4], c[5]],
                color: [c[6], c[7], c[8], c[9]],
            });
        }
        use wgpu::util::DeviceExt;
        let buf =
            lanes
                .layer_context()
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("world-buildings"),
                    contents: bytemuck::cast_slice(&instances),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        #[allow(clippy::cast_possible_truncation)]
        let count = instances.len() as u32;
        self.chunks_drawn = chunk_count;
        lanes.upsert_lane_batch(DrawBatch {
            lane: lane_id(LaneRole::WorldBuildings),
            visible,
            pipeline: PIPE_ORIENTED_QUAD,
            payload: DrawPayload::OrientedQuads(InstanceBuffer::whole(buf, 40, count)),
        });
    }

    /// Upload the world building outlines: `lines` packs six `f32` per hairline segment end.
    pub fn upload_outlines<T>(
        &mut self,
        lanes: &mut dyn LaneSink<T>,
        lines: &[f32],
        visible: bool,
    ) {
        self.uploads += 1;
        const STRIDE: usize = 6;
        if lines.is_empty() || !lines.len().is_multiple_of(STRIDE) {
            lanes.remove_lane_batch(lane_id(LaneRole::WorldBuildingsOutline));
            return;
        }
        let stream = line_buffers::upload_hairlines(
            lanes.layer_context().device(),
            "world-buildings-outline",
            ANCHOR,
            lines,
        );
        lanes.upsert_lane_batch(DrawBatch {
            lane: lane_id(LaneRole::WorldBuildingsOutline),
            visible,
            pipeline: PIPE_LINE,
            payload: DrawPayload::Lines(stream),
        });
    }

    /// Upload the world fence strips: `packed` holds six `f32` per triangle-list vertex (world x
    /// and y, RGBA), drawn as an indexed polygon mesh. `strip_lane_uploads` is the engine's strip
    /// upload counter, which every strip lane shares.
    pub fn upload_fence_strips<T>(
        &mut self,
        lanes: &mut dyn LaneSink<T>,
        strip_lane_uploads: &mut u64,
        packed: &[f32],
        item_count: u32,
        visible: bool,
    ) {
        *strip_lane_uploads += 1;
        const STRIDE: usize = 6;
        if packed.is_empty() {
            if !visible {
                lanes.remove_lane_batch(lane_id(LaneRole::WorldFences));
            }
            return;
        }
        if !packed.len().is_multiple_of(STRIDE) {
            lanes.remove_lane_batch(lane_id(LaneRole::WorldFences));
            return;
        }
        let n_verts = packed.len() / STRIDE;
        let mut verts = Vec::with_capacity(n_verts);
        for c in packed.chunks_exact(STRIDE) {
            verts.push(LineVertex {
                pos: [
                    (f64::from(c[0]) - ANCHOR[0]) as f32,
                    (f64::from(c[1]) - ANCHOR[1]) as f32,
                ],
                color: [c[2], c[3], c[4], c[5]],
            });
        }
        #[allow(clippy::cast_possible_truncation)]
        let indices: Vec<u32> = (0..n_verts as u32).collect();
        let mesh = polygons::upload_indexed_mesh(
            lanes.layer_context().device(),
            "world-fences",
            "world-fences-indices",
            &verts,
            &indices,
            item_count,
        );
        lanes.upsert_lane_batch(DrawBatch {
            lane: lane_id(LaneRole::WorldFences),
            visible,
            pipeline: PIPE_POLYGON,
            payload: DrawPayload::Indexed(mesh),
        });
    }
}
