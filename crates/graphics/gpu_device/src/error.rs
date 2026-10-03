//! The errors of the GPU device.
//!
//! **Role:** the one error type of the crate: why a GPU context could not be created, resized or
//! could not hand out a swapchain image.
//! **Position:** returned by [`crate::context`]; the renderers that create their GPU through this
//! crate map it into their own errors at their boundary.
//! **Signals & state:** none.
//! **Invariants:** each message starts with a stable kebab-case code (`canvas-zero-size`,
//! `create-surface`, `no-adapter`, `no-device`, `srgb-only-surface`,
//! `surface-unsupported-by-adapter`, `surface-size-nonpositive`, `surface-acquire`,
//! `surface-acquire-after-reconfigure`) that logs and readouts match on; the text after the code
//! is the driver's description.

/// Why the GPU of a canvas could not be created, resized or acquired.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The canvas has no device-pixel backing size; its width and height are set before the
    /// context is created.
    #[error(
        "canvas-zero-size: set canvas.width/height before creating the GPU context ({width}x{height})"
    )]
    CanvasZeroSize {
        /// The canvas backing width in device pixels.
        width: u32,
        /// The canvas backing height in device pixels.
        height: u32,
    },

    /// The instance could not create a surface over the canvas.
    #[error("create-surface: {0}")]
    CreateSurface(String),

    /// No adapter can present to the surface.
    #[error("no-adapter: {0}")]
    NoAdapter(String),

    /// The adapter refused the device request.
    #[error("no-device: {0}")]
    NoDevice(String),

    /// The surface offers only sRGB formats; the renderers write linear colour.
    #[error("srgb-only-surface: no non-sRGB surface format")]
    SrgbOnlySurface,

    /// The adapter has no default configuration for the surface.
    #[error("surface-unsupported-by-adapter")]
    SurfaceUnsupportedByAdapter,

    /// A resize asked for a zero width or height; the caller's size policy runs first.
    #[error("surface-size-nonpositive: {width}x{height}")]
    NonPositiveSurfaceSize {
        /// The requested width in device pixels.
        width: u32,
        /// The requested height in device pixels.
        height: u32,
    },

    /// The surface handed out no image.
    #[error("surface-acquire: {0}")]
    SurfaceAcquire(String),

    /// The surface handed out no image after it was reconfigured once.
    #[error("surface-acquire-after-reconfigure: {0}")]
    SurfaceAcquireAfterReconfigure(String),
}

/// The result of a GPU device call.
pub type Result<T, E = Error> = std::result::Result<T, E>;
