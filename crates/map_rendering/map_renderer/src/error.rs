//! The errors of the map renderer.
//!
//! **Role:** the one error type of the crate: why the render engine could not be created, resized
//! or render a frame, or why a typed layer or the text atlas refused an upload.
//! **Position:** returned by `RenderEngine::create`, `resize`, `render`, `ensure_text_atlas` and
//! `upload_text_atlas`; the asset sink maps a typed layer's refusal into the streaming model's
//! error; the Mission Creator and the debug benches log it or show its message.
//! **Signals & state:** none.
//! **Invariants:** each message starts with a stable kebab-case code: the renderer's own
//! (`resize-nonpositive`), or the code of the GPU device, GPU frame, symbology layer or world layer
//! error it carries, unchanged.

/// Why the render engine refused a call.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum Error {
    /// The canvas's GPU could not be created, resized or hand out a swapchain image.
    #[error(transparent)]
    Gpu(#[from] gpu_device::Error),

    /// A resize asked for a CSS size or a device-pixel ratio that is not positive.
    #[error("resize-nonpositive: {css_width}x{css_height} css px at {device_pixel_ratio} dpr")]
    NonPositiveResize {
        /// The requested width in CSS pixels.
        css_width: f64,
        /// The requested height in CSS pixels.
        css_height: f64,
        /// The requested device-pixel ratio.
        device_pixel_ratio: f64,
    },

    /// The text cell atlas could not be built.
    #[error(transparent)]
    TextAtlas(#[from] gpu_frame::Error),

    /// A symbology layer refused an atlas upload.
    #[error(transparent)]
    SymbologyLayer(#[from] symbology_layers_gpu::Error),

    /// A world layer refused a raster or texture write.
    #[error(transparent)]
    WorldLayer(#[from] world_layers_gpu::Error),
}

/// The result of a render engine call.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
#[path = "tests/error_tests.rs"]
mod tests;
