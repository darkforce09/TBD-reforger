//! Role: png.
//! Position: `terrain/dem` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::dem::sampling::meters_cache;

/// Decoded DEM: the meters cache + raster dims.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedDem {
    /// Meters.
    pub meters: Vec<f32>,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,
}

/// PNG decode failure (the TS throws; the caller degrades to flat mode).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PngError {
    /// Decode.
    Decode(String),

    /// Not a 16-bit grayscale (colorType 0/4) PNG — the DEM export contract.
    NotGray16,
}

impl core::fmt::Display for PngError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PngError::Decode(m) => write!(f, "PNG decode: {m}"),
            PngError::NotGray16 => write!(f, "PNG is not 16-bit grayscale"),
        }
    }
}

impl std::error::Error for PngError {}

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
