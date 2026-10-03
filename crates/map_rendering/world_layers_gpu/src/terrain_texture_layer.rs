//! **Role:** `TerrainTextureLayerGpu`, the terrain texture typed layer: the satellite basemap
//! (role 0) and the hillshade (role 1), each begun as a pending texture, written region by region
//! from RGBA bytes or browser bitmaps, and committed as a textured lane.
//! **Position:** the map renderer holds one as a field and its asset sink forwards the satellite
//! and relief loads' begin, write and commit calls here with the renderer's lanes lent as a
//! `renderer_core::lane_sink::LaneSink`.
//! **Signals & state:** one pending texture per role between its begin and its commit; the
//! textured bind-group layout and the basemap sampler a committed texture binds with.
//! **Invariants:** a role is 0 or 1; a write or commit without a begin is refused; a commit
//! replaces the role's lane whole and its texture record lives as long as the lane's batch.

use crate::basemap_mode::BasemapMode;
use crate::error::{Error, LayerCall, Result};
use crate::textured_lane::{TexLane, upsert_textured_quad_lane};
use crate::textured_quad::world_rect_rel;
use map_draw_lanes::lane_roles::LaneRole;
use render_primitives::draw::instances::QuadInstance;
use renderer_core::lane_sink::LaneSink;

/// A texture between its begin and its commit: the texture being written, the world rectangle it
/// will cover and the counters its record will report.
struct PendingTex {
    /// The texture being written.
    texture: wgpu::Texture,

    /// The covered rectangle's minimum corner, in world metres.
    world_min: [f64; 2],

    /// The covered rectangle's maximum corner, in world metres.
    world_max: [f64; 2],

    /// How the texture is laid out.
    mode: BasemapMode,

    /// The tiles written so far.
    tiles: u32,

    /// The texture's size in bytes, mip chain included.
    bytes: u64,
}

/// The satellite basemap and hillshade texture lanes.
pub struct TerrainTextureLayerGpu {
    /// The pending texture of each role, between its begin and its commit.
    pending: [Option<PendingTex>; 2],

    /// The textured bind-group layout a committed texture's bind group is built against.
    tex_bind_group_layout: wgpu::BindGroupLayout,

    /// The mipmapped linear sampler a committed texture binds.
    sampler: wgpu::Sampler,
}

impl TerrainTextureLayerGpu {
    /// No pending texture; a committed texture's bind group is built against
    /// `tex_bind_group_layout` with `sampler`.
    #[must_use]
    pub fn new(tex_bind_group_layout: wgpu::BindGroupLayout, sampler: wgpu::Sampler) -> Self {
        Self {
            pending: [None, None],
            tex_bind_group_layout,
            sampler,
        }
    }

    /// Begin `role`'s texture: a `tex_w`×`tex_h` RGBA texture with `mip_count` levels that will
    /// cover the world rectangle `[min_x, min_y]`…`[max_x, max_y]`, laid out as mode code `mode`.
    ///
    /// # Errors
    /// A role other than 0 (basemap) or 1 (hillshade), or a zero dimension or mip count.
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        role: u32,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tex_w: u32,
        tex_h: u32,
        mip_count: u32,
        mode: u32,
    ) -> Result<()> {
        let idx = role as usize;
        if idx > 1 {
            return Err(Error::TextureRole { role });
        }
        if tex_w == 0 || tex_h == 0 || mip_count == 0 {
            return Err(Error::ZeroTextureDimensions {
                call: LayerCall::TextureBegin,
            });
        }
        let texture = lanes
            .layer_context()
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("basemap-lane"),
                size: wgpu::Extent3d {
                    width: tex_w,
                    height: tex_h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,

                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

        let base_bytes = u64::from(tex_w) * u64::from(tex_h) * 4;
        let bytes = if mip_count > 1 {
            base_bytes * 4 / 3
        } else {
            base_bytes
        };
        self.pending[idx] = Some(PendingTex {
            texture,
            world_min: [min_x, min_y],
            world_max: [max_x, max_y],
            mode: BasemapMode::from_u32(mode),
            tiles: 0,
            bytes,
        });
        Ok(())
    }

    /// Copy the browser bitmap `bmp` into `role`'s pending texture at mip `mip`, region
    /// `(x, y, w, h)`.
    ///
    /// # Errors
    /// No pending texture for `role`.
    #[allow(clippy::too_many_arguments)]
    pub fn write_bitmap(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        bmp: web_sys::ImageBitmap,
    ) -> Result<()> {
        let idx = role as usize;
        let context = lanes.layer_context();
        let queue = context.queue();
        let pending =
            self.pending
                .get_mut(idx)
                .and_then(|p| p.as_mut())
                .ok_or(Error::TextureNotBegun {
                    call: LayerCall::TextureWriteBitmap,
                    role,
                })?;
        queue.copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(bmp),
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }

    /// Write the `w`×`h` RGBA bytes `rgba` into `role`'s pending texture at mip `mip`, region
    /// `(x, y, w, h)`.
    ///
    /// # Errors
    /// `rgba` not `w × h × 4` bytes long, or no pending texture for `role`.
    #[allow(clippy::too_many_arguments)]
    pub fn write_rgba(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        role: u32,
        mip: u32,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        rgba: &[u8],
    ) -> Result<()> {
        let idx = role as usize;
        let expected = (w as usize) * (h as usize) * 4;
        if rgba.len() != expected {
            return Err(Error::TileByteLength {
                expected,
                actual: rgba.len(),
            });
        }
        let context = lanes.layer_context();
        let queue = context.queue();
        let pending =
            self.pending
                .get_mut(idx)
                .and_then(|p| p.as_mut())
                .ok_or(Error::TextureNotBegun {
                    call: LayerCall::TextureWriteRgba,
                    role,
                })?;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &pending.texture,
                mip_level: mip,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        pending.tiles += 1;
        Ok(())
    }

    /// Finalize the pending texture for `role` into a drawn lane: a 1-instance world-rect quad
    /// tinted `[1,1,1,opacity]` over the uploaded texture, upserted into the draw list in order.
    ///
    /// # Errors
    /// No pending texture for `role`.
    pub fn commit(
        &mut self,
        lanes: &mut dyn LaneSink<TexLane>,
        role: u32,
        opacity: f32,
        visible: bool,
    ) -> Result<()> {
        let idx = role as usize;
        let pending =
            self.pending
                .get_mut(idx)
                .and_then(Option::take)
                .ok_or(Error::TextureNotBegun {
                    call: LayerCall::TextureCommit,
                    role,
                })?;
        let rect = world_rect_rel(pending.world_min, pending.world_max);
        let inst = QuadInstance {
            min: [rect[0], rect[1]],
            max: [rect[2], rect[3]],
            color: [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)],
        };
        let context = lanes.layer_context();
        let device = context.device();
        use wgpu::util::DeviceExt;
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tex-lane-quad"),
            contents: bytemuck::cast_slice(&[inst]),

            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let view = pending
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tex-lane"),
            layout: &self.tex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let lane = TexLane {
            texture: pending.texture,
            bind_group,
            mode: pending.mode,
            tiles: pending.tiles,
            bytes: pending.bytes,
        };
        let role_enum = if idx == 0 {
            LaneRole::Satellite
        } else {
            LaneRole::Hillshade
        };
        upsert_textured_quad_lane(lanes, role_enum, visible, instances, lane);
        Ok(())
    }
}
