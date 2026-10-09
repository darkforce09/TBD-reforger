//! The streaming PNG writer of the export image lanes.
//!
//! **Role:** [`write_png_rows`] writes a non-interlaced PNG one row at a time: the caller fills a
//! single reusable row buffer per row and the encoder compresses it straight to the file, so a
//! 12800 × 12800 image never sits in memory as a whole; [`PngPixelLayout`] names the four sample
//! layouts it writes.
//! **Position:** called by the water and road export image lanes (`water_export_images`,
//! `road_export_images`) for every image they write; decoded back in their golden tests.
//! **Signals & state:** none held; each call owns its file, its encoder and its row buffer.
//! **Invariants:** rows are filled and written top to bottom, each `width × bytes_per_pixel` bytes;
//! a 16-bit sample is two bytes, most significant first, as PNG stores it; the file holds exactly
//! the IHDR, IDAT and IEND chunks of the layout's colour type and bit depth; a failure at any step
//! is an [`crate::error::Error`] naming the step and the path.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::{Result, ResultExt};

/// The sample layout of one PNG: its colour type and bit depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PngPixelLayout {
    /// One 8-bit grey sample per pixel.
    Gray8,
    /// One 16-bit grey sample per pixel, most significant byte first.
    Gray16,
    /// Three 8-bit samples per pixel: red, green, blue.
    Rgb8,
    /// Four 8-bit samples per pixel: red, green, blue, alpha.
    Rgba8,
}

impl PngPixelLayout {
    /// The bytes one pixel takes in a row.
    pub(crate) const fn bytes_per_pixel(self) -> usize {
        match self {
            PngPixelLayout::Gray8 => 1,
            PngPixelLayout::Gray16 => 2,
            PngPixelLayout::Rgb8 => 3,
            PngPixelLayout::Rgba8 => 4,
        }
    }

    /// The PNG colour type of the layout.
    const fn colour_type(self) -> png::ColorType {
        match self {
            PngPixelLayout::Gray8 | PngPixelLayout::Gray16 => png::ColorType::Grayscale,
            PngPixelLayout::Rgb8 => png::ColorType::Rgb,
            PngPixelLayout::Rgba8 => png::ColorType::Rgba,
        }
    }

    /// The PNG bit depth of the layout's samples.
    const fn bit_depth(self) -> png::BitDepth {
        match self {
            PngPixelLayout::Gray16 => png::BitDepth::Sixteen,
            PngPixelLayout::Gray8 | PngPixelLayout::Rgb8 | PngPixelLayout::Rgba8 => {
                png::BitDepth::Eight
            }
        }
    }
}

/// Writes the `width` × `height` PNG of `layout` at `path`, calling `fill_row(row_index, row)` for
/// each row from the top to fill the row's `width × layout.bytes_per_pixel()` bytes.
///
/// Rows stream through `png`'s stream writer at the default zlib level (6) with no row filter.
/// Measured on 12800 × 12800 images, that writes a sparse RGBA road canvas in about 0.7 s and a
/// smooth RGB depth image in about 2.4 s at a fifth of the size the fast level gives; the `Sub`
/// filter makes the smooth image both larger and almost three times slower.
pub(crate) fn write_png_rows(
    path: &Path,
    width: u32,
    height: u32,
    layout: PngPixelLayout,
    mut fill_row: impl FnMut(usize, &mut [u8]),
) -> Result<()> {
    let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    let mut output = BufWriter::new(file);
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(layout.colour_type());
    encoder.set_depth(layout.bit_depth());
    encoder.set_compression(png::Compression::Default);
    encoder.set_filter(png::FilterType::NoFilter);
    encoder.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);
    let mut writer = encoder
        .write_header()
        .with_context(|| format!("write the PNG header of {}", path.display()))?;
    let mut stream = writer
        .stream_writer()
        .with_context(|| format!("start the PNG rows of {}", path.display()))?;
    let row_length = width as usize * layout.bytes_per_pixel();
    let mut row = vec![0_u8; row_length];
    for row_index in 0..height as usize {
        fill_row(row_index, &mut row);
        stream
            .write_all(&row)
            .with_context(|| format!("write PNG row {row_index} of {}", path.display()))?;
    }
    stream
        .finish()
        .with_context(|| format!("finish the PNG rows of {}", path.display()))?;
    writer
        .finish()
        .with_context(|| format!("finish the PNG {}", path.display()))?;
    output
        .flush()
        .with_context(|| format!("flush {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/png_writing_tests.rs"]
mod tests;
