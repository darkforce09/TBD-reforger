//! **Role:** the second half of a frame: resolve the timestamp query pair when the caller
//! samples one, submit the command buffer and present the swapchain image; and the CPU frame-cost
//! average.
//! **Position:** `frame` in the GPU frame crate. The first half, the swapchain acquire, is
//! `gpu_device`'s `context::frame_acquire::acquire_frame` (or `GpuContext::acquire`), which hands
//! out the image [`crate::frame::present::submit`] presents.
//! **Signals & state:** the swapchain image and the queued command buffer, both moved in; none
//! kept.
//! **Invariants:** the caller's own encode step runs between the acquire and [`crate::frame::present::submit`], so the
//! body of a frame arrives as two calls rather than one: `wgpu::Surface` is not `Clone`, and the
//! caller's encode borrows the same object mutably that the acquire borrows, so one function
//! taking a closure could not satisfy both.

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
