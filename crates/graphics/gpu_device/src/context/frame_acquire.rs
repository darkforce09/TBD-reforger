//! The swapchain acquire of a frame.
//!
//! **Role:** [`crate::context::frame_acquire::acquire_frame`] hands out the next swapchain image of a surface, reconfiguring the
//! surface once when it is outdated or lost, and says when there is nothing to draw into.
//! **Position:** [`crate::context::gpu_context::GpuContext::acquire`] calls it with its own
//! surface; the caller encodes into the acquired view and submits and presents through
//! `gpu_frame`'s `frame::present::submit`.
//! **Signals & state:** none of its own; the surface's configuration is the caller's.
//! **Invariants:** `Timeout` and `Occluded` are a skip, not a failure, because the surface is
//! alive and hands one over next tick; a second failure after the one reconfigure is an error.
//! The acquire and the submit are two calls because the caller's encode borrows the same object
//! mutably that the acquire borrows, and `wgpu::Surface` is not `Clone`.

use crate::error::{Error, Result};

/// What [`crate::context::frame_acquire::acquire_frame`] produced.
pub enum Acquired {
    /// A swapchain image is ready to render into.
    Ready {
        /// The acquired image; `gpu_frame`'s `frame::present::submit` presents it.
        frame: wgpu::SurfaceTexture,

        /// Its default view.
        view: wgpu::TextureView,
    },

    /// The surface had nothing to hand out. Skip the frame; this is not an error.
    Skip,
}

/// Acquire the next swapchain image of `surface`, re-configuring it with `config` once on
/// `Outdated | Lost`.
///
/// # Errors
/// [`Error::SurfaceAcquire`] when the surface hands out no image for another reason, and
/// [`Error::SurfaceAcquireAfterReconfigure`] when it still hands out none after the reconfigure.
pub fn acquire_frame(
    surface: &wgpu::Surface<'static>,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> Result<Acquired> {
    use wgpu::CurrentSurfaceTexture as Cst;
    let frame = match surface.get_current_texture() {
        Cst::Success(f) | Cst::Suboptimal(f) => f,
        Cst::Timeout | Cst::Occluded => return Ok(Acquired::Skip),
        Cst::Outdated | Cst::Lost => {
            surface.configure(device, config);
            match surface.get_current_texture() {
                Cst::Success(f) | Cst::Suboptimal(f) => f,
                other => {
                    return Err(Error::SurfaceAcquireAfterReconfigure(format!("{other:?}")));
                }
            }
        }
        other => return Err(Error::SurfaceAcquire(format!("{other:?}"))),
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    Ok(Acquired::Ready { frame, view })
}
