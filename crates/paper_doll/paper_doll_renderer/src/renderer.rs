//! The paper doll renderer: its GPU, its buffers and the state the Arsenal drives it with.
//!
//! **Role:** [`PaperDollRenderer`] creates the GPU of a canvas through
//! [`gpu_device::GpuContext`], builds the doll's program, pipeline, uniform, meshes, instance
//! buffer and depth target on it, and takes the Arsenal's resizes, turns, hover, region states,
//! picks and anchor queries; its frame is drawn by `frame_render`, its self-check by
//! `self_check`.
//! **Position:** the type the Arsenal host (`crates/frontend/workspaces/mission_creator_arsenal/src/doll.rs`)
//! creates once per mounted canvas, drives from pointer events and an animation-frame loop, and
//! drops on unmount.
//! **Signals & state:** the GPU context and buffers, the yaw, the CSS size, the 14 region
//! states, the hovered region and the dirty and continuous flags; single-threaded, owned by the
//! host.
//! **Invariants:** the GPU device is labelled `doll-engine` and never asks for timestamp queries;
//! a resize clamps each side to at least one pixel and recreates the depth target; every call
//! that changes what is drawn sets the dirty flag, and only that flag (or continuous rendering)
//! lets a frame acquire the surface.

use crate::doll_draw::DollMeshes;
use crate::error::{Error, Result};
use crate::instance_packing::pack_instances;
use crate::pipeline::{DollProgram, create_depth};
use gpu_device::GpuContext;
use paper_doll_scene::region_picking::{anchor_px, pick};
use paper_doll_scene::soldier_parts::{REGION_COUNT, STATE_EMPTY};

/// The label of the doll's GPU device in driver messages.
const DEVICE_LABEL: &str = "doll-engine";

/// The yaw change, in radians, of one CSS pixel of horizontal drag.
const YAW_RADIANS_PER_PIXEL: f64 = 0.012;

/// The Arsenal paper doll's renderer: owns its GPU context, canvas surface, camera yaw and region
/// states. Created with [`PaperDollRenderer::create`]; its host holds it and drops it exactly
/// once, which frees the GPU context.
pub struct PaperDollRenderer {
    /// The canvas's instance, surface, device, queue and surface configuration.
    pub(crate) gpu: GpuContext,

    /// The compiled shader and its layouts.
    pub(crate) program: DollProgram,

    /// The pipeline writing the surface format.
    pub(crate) pipeline: wgpu::RenderPipeline,

    /// The bind group of [`Self::uniform_buffer`].
    pub(crate) bind_group: wgpu::BindGroup,

    /// The camera matrix and params uniform, rewritten every drawn frame.
    pub(crate) uniform_buffer: wgpu::Buffer,

    /// The unit cube and cylinder.
    pub(crate) meshes: DollMeshes,

    /// The packed instance stream, repacked on every state or hover change.
    pub(crate) instance_buffer: wgpu::Buffer,

    /// The cube parts at the start of the instance stream.
    pub(crate) cube_instances: u32,

    /// The cylinder parts after the cubes.
    pub(crate) cylinder_instances: u32,

    /// The depth target, recreated on resize.
    pub(crate) depth: wgpu::TextureView,

    /// The canvas width in CSS pixels, at least 1.
    pub(crate) css_width: f64,

    /// The canvas height in CSS pixels, at least 1.
    pub(crate) css_height: f64,

    /// The camera's turn about the soldier, in radians.
    pub(crate) yaw: f64,

    /// One state byte per region, in `REGION_KEYS` order.
    pub(crate) states: [u8; REGION_COUNT],

    /// The hovered region, or -1.
    pub(crate) hover: i32,

    /// Something drawn changed since the last frame.
    pub(crate) dirty: bool,

    /// Draw every frame, changed or not.
    pub(crate) continuous: bool,
}

impl PaperDollRenderer {
    /// Create the renderer on `canvas`, whose `width` and `height` already hold the device-pixel
    /// backing size (the host owns the element). `force_webgl` skips WebGPU and uses WebGL2.
    ///
    /// The CSS size starts at the device-pixel size, so the host calls [`Self::resize`] before
    /// its first pick.
    ///
    /// # Errors
    /// [`Error::Gpu`] when the GPU context cannot be created (no backing size, no adapter, no
    /// device, an sRGB-only surface or an unsupported surface).
    pub async fn create(canvas: web_sys::HtmlCanvasElement, force_webgl: bool) -> Result<Self> {
        let gpu = GpuContext::create(canvas, force_webgl, DEVICE_LABEL, false).await?;
        let (device_width, device_height) = gpu.surface_size();
        let device = gpu.device();
        let queue = gpu.queue();

        let program = DollProgram::new(device);
        let pipeline = program.pipeline(device, gpu.surface_format());
        let (uniform_buffer, bind_group) = program.uniforms(device, "doll-uniforms");
        let meshes = DollMeshes::upload(device, queue);

        let states = [STATE_EMPTY; REGION_COUNT];
        let streams = pack_instances(&states, -1);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("doll-instances"),
            size: streams.bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&instance_buffer, 0, &streams.bytes);

        let depth = create_depth(device, device_width, device_height);

        Ok(Self {
            gpu,
            program,
            pipeline,
            bind_group,
            uniform_buffer,
            meshes,
            instance_buffer,
            cube_instances: streams.cube_instances,
            cylinder_instances: streams.cylinder_instances,
            depth,
            css_width: f64::from(device_width),
            css_height: f64::from(device_height),
            yaw: 0.0,
            states,
            hover: -1,
            dirty: true,
            continuous: false,
        })
    }

    /// `"webgpu"` or `"webgl2"`: the backend the GPU context runs on.
    #[must_use]
    pub fn backend(&self) -> &'static str {
        self.gpu.backend_kind().as_str()
    }

    /// Reconfigure for a CSS size and device pixel ratio: the surface takes the rounded device
    /// size, each side clamped to at least one pixel, and the depth target is recreated.
    ///
    /// Never fails: the GPU context refuses only a zero side, which the clamp rules out.
    pub fn resize(&mut self, css_width: f64, css_height: f64, device_pixel_ratio: f64) {
        let width = (css_width * device_pixel_ratio).round().max(1.0) as u32;
        let height = (css_height * device_pixel_ratio).round().max(1.0) as u32;
        self.css_width = css_width.max(1.0);
        self.css_height = css_height.max(1.0);
        self.gpu
            .resize(width, height)
            .expect("a surface size clamped to at least one pixel is positive");
        self.depth = create_depth(self.gpu.device(), width, height);
        self.dirty = true;
    }

    /// Turn the soldier by a horizontal drag of `dx_px` CSS pixels (a drag left turns it left).
    pub fn rotate(&mut self, dx_px: f64) {
        self.yaw -= dx_px * YAW_RADIANS_PER_PIXEL;
        self.dirty = true;
    }

    /// Highlight `region` (or -1 for none). No-op when unchanged; otherwise repacks the instance
    /// stream through the lifted palette.
    pub fn set_hover(&mut self, region: i32) {
        if region == self.hover {
            return;
        }
        self.hover = region;
        self.upload_instances();
    }

    /// The region's callout anchor in CSS pixels, or `None` when the region is unknown or behind
    /// the camera.
    #[must_use]
    pub fn anchor_px(&self, region: i32) -> Option<(f64, f64)> {
        anchor_px(self.yaw, self.css_width, self.css_height, region)
    }

    /// Take one state byte per region in `REGION_KEYS` order (0 empty, 1 equipped, 2 active).
    ///
    /// # Errors
    /// [`Error::RegionStateCount`] when `states` does not hold exactly one byte per region; the
    /// drawn states stay as they were.
    pub fn set_states(&mut self, states: &[u8]) -> Result<()> {
        if states.len() != REGION_COUNT {
            return Err(Error::RegionStateCount {
                expected: REGION_COUNT,
                got: states.len(),
            });
        }
        self.states.copy_from_slice(states);
        self.upload_instances();
        Ok(())
    }

    /// The nearest clickable region under a CSS pixel, or -1, picked with the camera the frame
    /// draws with.
    #[must_use]
    pub fn pick_region(&self, x_css: f64, y_css: f64) -> i32 {
        pick(self.yaw, self.css_width, self.css_height, x_css, y_css)
    }

    /// Force the next frame to draw.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Draw every frame while `on`, for the frame-rate readout during development.
    pub fn set_continuous_render(&mut self, on: bool) {
        self.continuous = on;
    }

    /// Repack the instance stream from the current states and hover, upload it, and mark the
    /// frame dirty.
    fn upload_instances(&mut self) {
        let streams = pack_instances(&self.states, self.hover);
        self.gpu
            .queue()
            .write_buffer(&self.instance_buffer, 0, &streams.bytes);
        self.dirty = true;
    }
}
