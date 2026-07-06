//! 16-bit grayscale PNG decode → meters cache — **Class R**. Replaces the pngjs + `buffer`
//! polyfill decode path (`DemTexture.ts` `decodeDemPng` + `rasterFromPngjs` + `buildMetersCache`).
//! PNG 16-bit samples are big-endian (per spec); the `png` crate yields them unswapped, so channel
//! 0 reads via `u16::from_be_bytes` — the same gray value pngjs exposes as channel 0 with
//! `{ skipRescale: true }`. Gated behind the `png` cargo feature.

use super::sample::meters_cache;

/// Decoded DEM: the meters cache + raster dims.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedDem {
    pub meters: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

/// PNG decode failure (the TS throws; the caller degrades to flat mode).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PngError {
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

/// Decode a 16-bit grayscale PNG → row-major `u16` gray samples (channel 0) + dims. Mirror of
/// `decodeDemPng` + `rasterFromPngjs`.
///
/// # Errors
/// [`PngError::Decode`] on a malformed PNG; [`PngError::NotGray16`] if the bit depth isn't 16.
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
    // Channel 0 (gray), 16-bit big-endian, interleaved `channels` samples per pixel.
    for (i, slot) in raster.iter_mut().enumerate() {
        let o = i * channels * 2;
        *slot = u16::from_be_bytes([out[o], out[o + 1]]);
    }
    Ok((raster, width, height))
}

/// Decode a 16-bit grayscale DEM PNG straight to the `f32` meters cache. Mirror of
/// `decodeDemPng` → `buildMetersCache`.
///
/// # Errors
/// Propagates [`decode_png_gray16`] failures.
pub fn decode_png_to_meters(bytes: &[u8], min_m: f64, max_m: f64) -> Result<DecodedDem, PngError> {
    let (raster, width, height) = decode_png_gray16(bytes)?;
    Ok(DecodedDem {
        meters: meters_cache(&raster, min_m, max_m),
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a 16-bit grayscale PNG with the `png` crate (big-endian samples).
    fn encode_gray16(width: u32, height: u32, samples: &[u16]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, width, height);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Sixteen);
            let mut writer = enc.write_header().unwrap();
            let mut bytes = Vec::with_capacity(samples.len() * 2);
            for &s in samples {
                bytes.extend_from_slice(&s.to_be_bytes());
            }
            writer.write_image_data(&bytes).unwrap();
        }
        out
    }

    #[test]
    fn round_trip_gray16() {
        let samples: Vec<u16> = vec![0, 1, 12345, 65535, 32768, 40000, 7, 60000, 100];
        let png_bytes = encode_gray16(3, 3, &samples);
        let (raster, w, h) = decode_png_gray16(&png_bytes).unwrap();
        assert_eq!((w, h), (3, 3));
        assert_eq!(raster, samples);
    }

    #[test]
    fn decode_to_meters_matches_scalar() {
        let samples: Vec<u16> = vec![0, 65535, 32768, 100];
        let png_bytes = encode_gray16(2, 2, &samples);
        let dem = decode_png_to_meters(&png_bytes, -204.78, 375.53).unwrap();
        assert_eq!((dem.width, dem.height), (2, 2));
        for (i, &s) in samples.iter().enumerate() {
            assert_eq!(
                dem.meters[i],
                super::super::sample::uint16_to_meters(f64::from(s), -204.78, 375.53) as f32
            );
        }
    }

    #[test]
    fn rejects_non_png() {
        assert!(matches!(
            decode_png_gray16(b"not a png"),
            Err(PngError::Decode(_))
        ));
    }
}
