//! **Role:** the engine's symbology typed layers at work: the split borrow that lends the slot
//! symbology, the glyph atlas and the icon cull the engine's lanes, camera and text atlas, the
//! accessor the Mission Creator drives the slot symbology through, and the frame hook that keeps
//! the symbology in step with the camera.
//! **Position:** the map renderer; the Mission Creator calls `RenderEngine::with_symbology`, the
//! asset sink forwards the glyph atlas and world icon uploads to the layers through
//! `RenderEngine::symbology_parts`, the camera runs the frame hooks.
//! **Signals & state:** none of its own; `SymbologyParts` borrows disjoint engine fields.
//! **Invariants:** a layer reaches the engine only through the parts lent here (the lane sink, the
//! camera, the shared text atlas and the counters it reports); the engine names the symbology
//! layers' types, never the reverse.

use crate::engine::RenderEngine;
use crate::lane_sinks::untextured_lanes::UntexturedLanes;
use crate::upload::text_atlas::TextAtlasSlot;
use camera_math::ortho::state::OrthoCamera;
use renderer_core::frame_hook::FrameHook;
use symbology_layers_gpu::glyph_atlas_gpu::GlyphAtlasGpu;
use symbology_layers_gpu::icon_cull_gpu::IconCullGpu;
use symbology_layers_gpu::slot_symbology::{SlotSymbology, SlotSymbologyGpu};

/// The engine split into the symbology layers and the parts they write through.
pub(crate) struct SymbologyParts<'a> {
    /// The engine's lanes.
    pub(crate) lanes: UntexturedLanes<'a>,

    /// The slot symbology.
    pub(crate) slot_symbology: &'a mut SlotSymbologyGpu,

    /// The glyph atlas.
    pub(crate) glyph_atlas: &'a mut GlyphAtlasGpu,

    /// The compute cull of the icon lanes.
    pub(crate) icon_cull: &'a mut IconCullGpu,

    /// The shared text atlas.
    pub(crate) text_atlas: TextAtlasSlot<'a>,

    /// The camera.
    pub(crate) camera: &'a OrthoCamera,

    /// The uniform bytes written for the last frame.
    pub(crate) uniform_bytes_last_frame: &'a mut u32,

    /// The world icon lane upload counter.
    pub(crate) icon_lane_uploads: &'a mut u64,
}

impl RenderEngine {
    /// The engine split into the symbology layers and the parts they write through.
    pub(crate) fn symbology_parts(&mut self) -> SymbologyParts<'_> {
        let Self {
            gpu,
            text_bind_group_layout,
            icon_sampler,
            camera,
            glyph_atlas,
            text_atlas,
            slot_symbology,
            batches,
            frame_pipelines,
            frame_bind_groups,
            tex_lanes,
            uniform_bytes_last_frame,
            icon_lane_uploads,
            render_stats,
            damage,
            icon_cull,
            ..
        } = self;
        let device = gpu.device();
        let queue = gpu.queue();
        SymbologyParts {
            lanes: UntexturedLanes {
                batches,
                tex_lanes,
                damage,
                device,
                queue,
                surface_format: gpu.surface_format(),
                frame_pipelines,
                frame_bind_groups,
                render_stats,
            },
            slot_symbology,
            glyph_atlas,
            icon_cull,
            text_atlas: TextAtlasSlot {
                device,
                queue,
                layout: text_bind_group_layout,
                sampler: icon_sampler,
                atlas: text_atlas,
            },
            camera,
            uniform_bytes_last_frame,
            icon_lane_uploads,
        }
    }

    /// Run `work` on the slot symbology, lent the engine's lanes, icon cull, camera and text
    /// atlas: the one door the Mission Creator binds, selects, drags and previews through.
    pub fn with_symbology<R>(&mut self, work: impl FnOnce(&mut SlotSymbology<'_>) -> R) -> R {
        let mut parts = self.symbology_parts();
        let mut symbology = parts.slot_symbology.at_work(
            parts.icon_cull,
            &mut parts.lanes,
            parts.camera,
            &mut parts.text_atlas,
            parts.uniform_bytes_last_frame,
        );
        work(&mut symbology)
    }
}

/// The frame hook that keeps the slot symbology in step with the camera: the pixels-to-metres
/// uniform, the detailed / disc threshold, the cluster gate and the cluster markers.
pub(crate) struct SlotSymbologyCameraSync;

impl FrameHook<RenderEngine> for SlotSymbologyCameraSync {
    fn camera_changed(&mut self, engine: &mut RenderEngine) {
        engine.with_symbology(|symbology| symbology.camera_changed());
    }
}
