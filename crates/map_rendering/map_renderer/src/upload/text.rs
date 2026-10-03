//! **Role:** the cartographic, town and road label lanes, and the device's texture size limits.
//! **Position:** the map renderer's upload belts; the location label loader's uploads arrive
//! through the engine's asset sink; the label lanes sample the text atlas of `text_atlas.rs`.
//! **Signals & state:** the three label lanes and their drawn counts and upload counter.
//! **Invariants:** a label lane is 20-byte glyph instances moved to the scene anchor; an empty or
//! hidden upload clears its drawn count, and a byte count off the stride drops the lane.

use crate::engine::RenderEngine;
use gpu_frame::frame::{DrawBatch, DrawPayload, InstanceBuffer, TextRun};
use map_draw_lanes::lane_roles::LaneRole;
use map_draw_lanes::lane_roles::lane_id;
use renderer_core::packet_bindings;
use symbology_layers_gpu::icon_uniforms::convert_icon_world_to_anchor;

impl RenderEngine {
    /// Upload height / cartographic text labels (20 B instances, `WorldLabels` lane).
    pub fn upload_text_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.text_labels_drawn = 0;
            if !visible {
                self.remove_lane(LaneRole::WorldLabels);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .gpu
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.text_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldLabels);
        self.upsert_lane(
            LaneRole::WorldLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: packet_bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: packet_bindings::BIND_TEXT_ATLAS,
                    pipeline: packet_bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

impl RenderEngine {
    /// Upload town name labels (20 B instances, `WorldTownLabels` lane — above height labels).
    pub fn upload_town_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.town_labels_drawn = 0;
            self.remove_lane(LaneRole::WorldTownLabels);
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldTownLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .gpu
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("town-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.town_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldTownLabels);
        self.upsert_lane(
            LaneRole::WorldTownLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: packet_bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: packet_bindings::BIND_TEXT_ATLAS,
                    pipeline: packet_bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

impl RenderEngine {
    /// Upload road name labels (20 B instances, `WorldRoadLabels` — below town labels).
    pub fn upload_road_labels(&mut self, bytes: &[u8], visible: bool) {
        self.text_label_uploads += 1;
        use wgpu::util::DeviceExt;
        const STRIDE: usize = 20;
        const STRIDE_U32: u32 = 20;
        if bytes.is_empty() || !visible {
            self.road_labels_drawn = 0;
            if !visible {
                self.remove_lane(LaneRole::WorldRoadLabels);
            }
            return;
        }
        if !bytes.len().is_multiple_of(STRIDE) {
            self.remove_lane(LaneRole::WorldRoadLabels);
            return;
        }
        let _ = self.ensure_text_atlas();
        let mut converted = bytes.to_vec();
        convert_icon_world_to_anchor(&mut converted);
        let count = (converted.len() / STRIDE) as u32;
        let buf = self
            .gpu
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("road-labels"),
                contents: &converted,
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.road_labels_drawn = count;
        let lane = lane_id(LaneRole::WorldRoadLabels);
        self.upsert_lane(
            LaneRole::WorldRoadLabels,
            DrawBatch {
                lane,
                visible: true,
                pipeline: packet_bindings::PIPE_TEXT,
                payload: DrawPayload::Text(TextRun {
                    lane,
                    glyphs: InstanceBuffer::whole(buf, STRIDE_U32, count),
                    atlas: packet_bindings::BIND_TEXT_ATLAS,
                    pipeline: packet_bindings::PIPE_TEXT,
                }),
            },
        );
    }
}

impl RenderEngine {
    /// The largest 2D texture side the device was created with, in texels.
    #[must_use]
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.gpu.device().limits().max_texture_dimension_2d
    }
}

impl RenderEngine {
    /// Read-only. It exists so a half-resolution basemap can be attributed: if this equals [`Self::max_texture_dimension_2d`] the GPU genuinely cannot do better, and if it is larger the device request lost the resolution somewhere.
    #[must_use]
    pub fn adapter_max_texture_dimension_2d(&self) -> u32 {
        self.gpu.max_texture_dimension_2d()
    }
}
