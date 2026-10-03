//! The 16-bit grayscale PNG decoder of the elevation model.
//!
//! **Role:** decodes a terrain's height PNG into row-major `u16` samples
//! ([`decode_png_gray16`]) or straight into the `f32` metres cache ([`decode_png_to_meters`]).
//! **Position:** called by the map engine's terrain boot when no raw grid is declared, and by the
//! developer tools' label and alignment checks; converts through [`crate::sampling::meters_cache`].
//! **Signals & state:** none; pure functions over a byte slice.
//! **Invariants:** only a 16-bit image is accepted, and channel 0 is read big-endian whatever the
//! colour type; anything else is a [`PngError`], never a guessed raster.

use crate::sampling::meters_cache;

/// Decoded DEM: the meters cache + raster dims.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedDem {
    /// Elevation in metres per pixel, row-major `width * height`, decoded through
    /// [`meters_cache`] from the caller's height range.
    pub meters: Vec<f32>,

    /// Raster width in pixels (samples per row), from the PNG header.
    pub width: u32,

    /// Raster height in pixels (number of rows), from the PNG header.
    pub height: u32,
}

/// Why a height PNG is refused; the terrain boot degrades to flat ground on either.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PngError {
    /// The bytes are not a readable PNG; the decoder's message.
    #[error("PNG decode: {0}")]
    Decode(String),

    /// Not a 16-bit PNG, or an indexed one; the DEM export writes 16-bit grayscale.
    #[error("PNG is not 16-bit grayscale")]
    NotGray16,
}

/// Decode a 16-bit grayscale PNG → row-major `u16` gray samples (channel 0) + dims. Mirror of `decodeDemPng` + `rasterFromPngjs`.
pub fn decode_png_gray16(bytes: &[u8]) -> Result<(Vec<u16>, u32, u32), PngError> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder
        .read_info()
        .map_err(|e| PngError::Decode(e.to_string()))?;
    let info = reader.info();
    if info.bit_depth != png::BitDepth::Sixteen {
        return Err(PngError::NotGray16);
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1usize,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(PngError::NotGray16),
    };
    let (width, height) = (info.width, info.height);
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buf)
        .map_err(|e| PngError::Decode(e.to_string()))?;
    let out = &buf[..frame.buffer_size()];
    let n = width as usize * height as usize;
    let mut raster = vec![0u16; n];

    for (i, slot) in raster.iter_mut().enumerate() {
        let o = i * channels * 2;
        *slot = u16::from_be_bytes([out[o], out[o + 1]]);
    }
    Ok((raster, width, height))
}

/// Decode a 16-bit grayscale DEM PNG straight to the `f32` meters cache. Mirror of `decodeDemPng` → `buildMetersCache`.
pub fn decode_png_to_meters(bytes: &[u8], min_m: f64, max_m: f64) -> Result<DecodedDem, PngError> {
    let (raster, width, height) = decode_png_gray16(bytes)?;
    Ok(DecodedDem {
        meters: meters_cache(&raster, min_m, max_m),
        width,
        height,
    })
}

#[cfg(test)]
#[path = "tests/png_tests.rs"]
mod tests;
