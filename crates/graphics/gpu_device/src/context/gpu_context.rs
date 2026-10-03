//! The GPU of one browser canvas.
//!
//! **Role:** [`crate::context::gpu_context::GpuContext`] creates and owns the instance, surface, device, queue and surface
//! configuration of a canvas, keeps the adapter facts a renderer decides on (backend kind,
//! limits, whether timestamp queries are on), resizes the surface and acquires its frames.
//! **Position:** one per canvas, owned by the renderer of that canvas: a map canvas asks for the
//! label `map-engine-render` with timestamps when supported, an Arsenal paper doll canvas for
//! `doll-engine` without; each renderer builds its own shaders, layouts and pipelines on the
//! device and queue handed out here.
//! **Signals & state:** the GPU handles of one canvas; single-threaded, owned by its renderer.
//! **Invariants:** WebGPU is preferred and WebGL2 is the fallback (or the only backend when
//! forced); the WebGL2 device asks for the downlevel WebGL2 limits raised to the adapter's
//! resolution limits; the surface format is linear (never sRGB) and presents with `Fifo`; a resize
//! never clamps (the caller rounds and clamps first) and a zero side is an error.

use crate::context::frame_acquire::{Acquired, acquire_frame};
use crate::context::surface_policy::{
    BackendKind, checked_canvas_size, checked_surface_size, first_linear_format, request_timestamps,
};
use crate::context::web_display::instance_descriptor;
use crate::error::{Error, Result};

/// The instance, surface, adapter facts, device, queue and surface configuration of a canvas.
pub struct GpuContext {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter_info: wgpu::AdapterInfo,
    adapter_limits: wgpu::Limits,
    backend_kind: BackendKind,
    timestamps_enabled: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    /// Create the GPU of `canvas`, whose `width` and `height` already hold the device-pixel
    /// backing size.
    ///
    /// `force_webgl` skips WebGPU detection and uses WebGL2 alone; `label` names the device in
    /// driver messages; `want_timestamps` requests timestamp queries when the adapter supports
    /// them ([`Self::timestamps_enabled`] says whether it did).
    ///
    /// # Errors
    /// [`Error::CanvasZeroSize`], [`Error::CreateSurface`], [`Error::NoAdapter`],
    /// [`Error::NoDevice`], [`Error::SrgbOnlySurface`] or
    /// [`Error::SurfaceUnsupportedByAdapter`], each at the step that failed.
    pub async fn create(
        canvas: web_sys::HtmlCanvasElement,
        force_webgl: bool,
        label: &str,
        want_timestamps: bool,
    ) -> Result<Self> {
        let (width, height) = checked_canvas_size(canvas.width(), canvas.height())?;

        let instance = if force_webgl {
            wgpu::Instance::new(instance_descriptor(wgpu::Backends::GL))
        } else {
            wgpu::util::new_instance_with_webgpu_detection(instance_descriptor(
                wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ))
            .await
        };

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| Error::CreateSurface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..wgpu::RequestAdapterOptions::default()
            })
            .await
            .map_err(|e| Error::NoAdapter(e.to_string()))?;

        let adapter_info = adapter.get_info();
        let backend_kind = BackendKind::from_is_gl(adapter_info.backend == wgpu::Backend::Gl);
        let base_limits = if backend_kind.is_gl() {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };
        let adapter_limits = adapter.limits();
        let timestamps_enabled = request_timestamps(
            want_timestamps,
            adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(label),
                required_features: if timestamps_enabled {
                    wgpu::Features::TIMESTAMP_QUERY
                } else {
                    wgpu::Features::empty()
                },
                required_limits: base_limits.using_resolution(adapter_limits.clone()),
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .map_err(|e| Error::NoDevice(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let format = first_linear_format(&caps.formats, |f: wgpu::TextureFormat| f.is_srgb())?;
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(Error::SurfaceUnsupportedByAdapter)?;
        config.format = format;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        Ok(Self {
            instance,
            surface,
            adapter_info,
            adapter_limits,
            backend_kind,
            timestamps_enabled,
            device,
            queue,
            config,
        })
    }

    /// Reconfigure the surface to `width` × `height` device pixels.
    ///
    /// # Errors
    /// [`Error::NonPositiveSurfaceSize`] when either side is zero; the surface is left as it was.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        let (width, height) = checked_surface_size(width, height)?;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Acquire the next swapchain image; see [`acquire_frame`].
    ///
    /// # Errors
    /// [`Error::SurfaceAcquire`] or [`Error::SurfaceAcquireAfterReconfigure`].
    pub fn acquire(&self) -> Result<Acquired> {
        acquire_frame(&self.surface, &self.device, &self.config)
    }

    /// The instance the surface and adapter came from.
    #[must_use]
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// The canvas surface.
    #[must_use]
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    /// The adapter's name, vendor, device and backend.
    #[must_use]
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// The adapter's own limits (before the device request lowered them for WebGL2).
    #[must_use]
    pub fn adapter_limits(&self) -> &wgpu::Limits {
        &self.adapter_limits
    }

    /// The largest 2D texture side the adapter supports, in texels.
    #[must_use]
    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.adapter_limits.max_texture_dimension_2d
    }

    /// WebGPU or the WebGL2 fallback.
    #[must_use]
    pub fn backend_kind(&self) -> BackendKind {
        self.backend_kind
    }

    /// Whether the backend is WebGL2, which has no compute shaders.
    #[must_use]
    pub fn is_gl(&self) -> bool {
        self.backend_kind.is_gl()
    }

    /// Whether the device was created with timestamp queries.
    #[must_use]
    pub fn timestamps_enabled(&self) -> bool {
        self.timestamps_enabled
    }

    /// The device.
    #[must_use]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue.
    #[must_use]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The surface configuration: format, size, present mode.
    #[must_use]
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    /// The surface's texture format, which every render pipeline targets.
    #[must_use]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// The surface size in device pixels, `(width, height)`.
    #[must_use]
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
}
