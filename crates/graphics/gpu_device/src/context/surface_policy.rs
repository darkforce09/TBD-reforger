//! The GPU-free decisions of a canvas's GPU bootstrap.
//!
//! **Role:** the size checks of a create and a resize, the surface format pick, the backend kind
//! and whether the device asks for timestamp queries.
//! **Position:** `crate::context::gpu_context::GpuContext` applies these on `wasm32`; the native
//! tests drive them directly.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a zero width or height is an error, never clamped here (the caller owns its
//! clamp and rounding policy); the surface format is the first non-sRGB one the surface offers,
//! because the renderers write linear colour; timestamps are requested only when the caller wants
//! them and the adapter supports them.

use crate::error::{Error, Result};

/// The browser graphics API a context runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// WebGPU.
    WebGpu,
    /// The WebGL2 fallback, chosen when WebGPU is absent or the caller forces it.
    WebGl2,
}

impl BackendKind {
    /// The kind of an adapter whose backend is (`true`) or is not (`false`) GL.
    #[must_use]
    pub fn from_is_gl(is_gl: bool) -> Self {
        if is_gl { Self::WebGl2 } else { Self::WebGpu }
    }

    /// Whether this is the WebGL2 fallback, which has no compute shaders.
    #[must_use]
    pub fn is_gl(self) -> bool {
        self == Self::WebGl2
    }

    /// The readout spelling: `"webgpu"` or `"webgl2"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebGpu => "webgpu",
            Self::WebGl2 => "webgl2",
        }
    }
}

/// Checks a canvas's device-pixel backing size before a context is created over it.
///
/// # Errors
/// [`Error::CanvasZeroSize`] when the width or the height is zero.
pub fn checked_canvas_size(width: u32, height: u32) -> Result<(u32, u32)> {
    if width == 0 || height == 0 {
        return Err(Error::CanvasZeroSize { width, height });
    }
    Ok((width, height))
}

/// Checks a resize's device-pixel size; the caller has already applied its own rounding and
/// clamping.
///
/// # Errors
/// [`Error::NonPositiveSurfaceSize`] when the width or the height is zero.
pub fn checked_surface_size(width: u32, height: u32) -> Result<(u32, u32)> {
    if width == 0 || height == 0 {
        return Err(Error::NonPositiveSurfaceSize { width, height });
    }
    Ok((width, height))
}

/// The first format of `formats` that `is_srgb` says is not sRGB.
///
/// # Errors
/// [`Error::SrgbOnlySurface`] when every format is sRGB (or there is none).
pub fn first_linear_format<F: Copy>(formats: &[F], is_srgb: impl Fn(F) -> bool) -> Result<F> {
    formats
        .iter()
        .copied()
        .find(|format| !is_srgb(*format))
        .ok_or(Error::SrgbOnlySurface)
}

/// Whether the device is requested with timestamp queries: only when the caller wants them and
/// the adapter supports them.
#[must_use]
pub fn request_timestamps(wanted: bool, adapter_supports: bool) -> bool {
    wanted && adapter_supports
}

#[cfg(test)]
#[path = "tests/surface_policy_tests.rs"]
mod tests;
