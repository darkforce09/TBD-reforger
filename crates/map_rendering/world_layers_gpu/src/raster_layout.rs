//! **Role:** the layout rule of a single-bitmap raster (the forest density and viewshed rasters)
//! that a layer checks before it creates a texture.
//! **Position:** called by the forest layer's density upload and the terrain line of sight
//! overlay's viewshed upload before either touches the GPU.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the checks run in a fixed order (dimensions, row pitch, size, length) and the
//! first that fails names the refusal; a row holds at least four bytes per texel and is a multiple
//! of 256 bytes, the GPU copy alignment.

use crate::error::{Error, LayerCall, Result};

/// The GPU's copy row alignment, in bytes.
const ROW_ALIGNMENT: u32 = 256;

/// Check that a `width`×`height` RGBA raster of `byte_len` bytes with `bytes_per_row` bytes per
/// row can be copied into a texture as it stands.
///
/// # Errors
/// A zero dimension, a row shorter than `width × 4` bytes or not 256-aligned, a size overflow, or
/// `byte_len` not `bytes_per_row × height`; each names `call`.
pub(crate) fn check_raster(
    call: LayerCall,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    byte_len: usize,
) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(Error::ZeroTextureDimensions { call });
    }
    if u64::from(bytes_per_row) < u64::from(width) * 4
        || !bytes_per_row.is_multiple_of(ROW_ALIGNMENT)
    {
        return Err(Error::RasterRowPitch {
            call,
            bytes_per_row,
            width,
        });
    }
    let expected = (bytes_per_row as usize)
        .checked_mul(height as usize)
        .ok_or(Error::RasterSizeOverflow { call })?;
    if byte_len != expected {
        return Err(Error::RasterByteLength {
            call,
            expected,
            actual: byte_len,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/raster_layout_tests.rs"]
mod tests;
