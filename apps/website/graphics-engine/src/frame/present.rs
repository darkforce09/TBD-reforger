//! Role: frame present.
//! Position: `frame` in the graphics engine.
//! Signals & state: the swapchain image, the queued command buffer, and the CPU frame cost.
//! Invariants: the caller's own encode step runs BETWEEN [`acquire`] and [`submit`], so the
//! body of a frame arrives here as two halves rather than one. That is forced, not chosen:
//! `wgpu::Surface` is not `Clone`, and the caller's encode borrows the same object mutably
//! that the acquire borrows — one function taking a closure could not satisfy both.

/// What [`acquire`] produced.
pub enum Acquired {
    /// A swapchain image is ready to render into.
    Ready {
        /// The acquired image. Call `present()` on it via [`submit`].
        frame: wgpu::SurfaceTexture,

        /// Its default view.
        view: wgpu::TextureView,
    },

    /// The surface had nothing to hand out. Skip the frame; this is not an error.
    Skip,
}

/// Acquire the next swapchain image, re-configuring once on `Outdated | Lost`.
///
/// `Timeout | Occluded` is a skip, not a failure — the surface is alive and will hand one
/// over next tick. A second failure after the re-configure is real and comes back as `Err`.
pub fn acquire(
    surface: &wgpu::Surface<'static>,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> Result<Acquired, String> {
    use wgpu::CurrentSurfaceTexture as Cst;
    let frame = match surface.get_current_texture() {
        Cst::Success(f) | Cst::Suboptimal(f) => f,
        Cst::Timeout | Cst::Occluded => return Ok(Acquired::Skip),
        Cst::Outdated | Cst::Lost => {
            surface.configure(device, config);
            match surface.get_current_texture() {
                Cst::Success(f) | Cst::Suboptimal(f) => f,
                other => {
                    return Err(format!("surface-acquire-after-reconfigure: {other:?}"));
                }
            }
        }
        other => return Err(format!("surface-acquire: {other:?}")),
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    Ok(Acquired::Ready { frame, view })
}

/// The timestamp query pair to resolve into a readback buffer before submitting.
pub struct TimestampResolve<'a> {
    /// Query set holding the begin/end timestamps.
    pub query_set: &'a wgpu::QuerySet,

    /// Destination for `resolve_query_set`.
    pub resolve_buf: &'a wgpu::Buffer,

    /// Mappable buffer the resolved pair is copied into.
    pub read_buf: &'a wgpu::Buffer,
}

/// Resolve timestamps if asked, submit the encoder, and present the image.
pub fn submit(
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    frame: wgpu::SurfaceTexture,
    resolve: Option<TimestampResolve<'_>>,
) {
    if let Some(t) = resolve {
        encoder.resolve_query_set(t.query_set, 0..2, t.resolve_buf, 0);
        encoder.copy_buffer_to_buffer(t.resolve_buf, 0, t.read_buf, 0, 16);
    }
    queue.submit(Some(encoder.finish()));
    frame.present();
}

/// Exponential moving average of the CPU frame cost: the first sample seeds it, then 9:1.
#[must_use]
pub fn frame_ms_ema(prev: f64, last: f64) -> f64 {
    if prev == 0.0 {
        last
    } else {
        prev * 0.9 + last * 0.1
    }
}
