//! **Role:** unit tests of [`crate::image_operations::png_writing`]: every pixel layout written
//! row by row decodes back to the same bytes, colour type and bit depth.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by `image_operations/png_writing.rs`.
//! **Signals & state:** each case writes one file under the system temporary folder and removes
//! it.
//! **Invariants:** the decoder applies no transformation, so a 16-bit sample reads back most
//! significant byte first, exactly as it was filled.

use std::path::PathBuf;

use super::{PngPixelLayout, write_png_rows};

const WIDTH: u32 = 5;
const HEIGHT: u32 = 3;

/// A temporary file path unique to this process and case.
fn temporary_png(case: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tbd-png-writing-{case}-{}.png", std::process::id()))
}

/// The byte at `index` of row `row`: distinct across rows and columns, and every value of a byte
/// within a few rows.
fn sample_byte(row: usize, index: usize) -> u8 {
    ((row * 97 + index * 31 + 7) % 256) as u8
}

/// Writes `layout` through [`write_png_rows`], decodes it, and returns the decoded colour type,
/// bit depth and bytes beside the bytes that were filled.
fn round_trip(
    layout: PngPixelLayout,
    case: &str,
) -> (png::ColorType, png::BitDepth, Vec<u8>, Vec<u8>) {
    let path = temporary_png(case);
    let mut filled = Vec::new();
    write_png_rows(&path, WIDTH, HEIGHT, layout, |row_index, row| {
        assert_eq!(row.len(), WIDTH as usize * layout.bytes_per_pixel());
        for (index, byte) in row.iter_mut().enumerate() {
            *byte = sample_byte(row_index, index);
        }
        filled.extend_from_slice(row);
    })
    .expect("write the PNG");

    let file = std::fs::File::open(&path).expect("open the PNG");
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().expect("read the PNG header");
    let mut decoded = vec![0_u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut decoded).expect("decode the PNG");
    decoded.truncate(frame.buffer_size());
    let info = reader.info();
    let (colour_type, bit_depth) = (info.color_type, info.bit_depth);
    assert_eq!((info.width, info.height), (WIDTH, HEIGHT));
    assert!(!info.interlaced);
    std::fs::remove_file(&path).expect("remove the PNG");
    (colour_type, bit_depth, filled, decoded)
}

#[test]
fn gray8_rows_round_trip() {
    let (colour_type, bit_depth, filled, decoded) = round_trip(PngPixelLayout::Gray8, "gray8");
    assert_eq!(colour_type, png::ColorType::Grayscale);
    assert_eq!(bit_depth, png::BitDepth::Eight);
    assert_eq!(decoded, filled);
}

#[test]
fn gray16_rows_round_trip_most_significant_byte_first() {
    let (colour_type, bit_depth, filled, decoded) = round_trip(PngPixelLayout::Gray16, "gray16");
    assert_eq!(colour_type, png::ColorType::Grayscale);
    assert_eq!(bit_depth, png::BitDepth::Sixteen);
    assert_eq!(decoded.len(), (WIDTH * HEIGHT * 2) as usize);
    assert_eq!(decoded, filled);
}

#[test]
fn rgb8_rows_round_trip() {
    let (colour_type, bit_depth, filled, decoded) = round_trip(PngPixelLayout::Rgb8, "rgb8");
    assert_eq!(colour_type, png::ColorType::Rgb);
    assert_eq!(bit_depth, png::BitDepth::Eight);
    assert_eq!(decoded, filled);
}

#[test]
fn rgba8_rows_round_trip() {
    let (colour_type, bit_depth, filled, decoded) = round_trip(PngPixelLayout::Rgba8, "rgba8");
    assert_eq!(colour_type, png::ColorType::Rgba);
    assert_eq!(bit_depth, png::BitDepth::Eight);
    assert_eq!(decoded, filled);
}

#[test]
fn a_missing_folder_is_an_error_naming_the_path() {
    let path = std::env::temp_dir()
        .join(format!("tbd-png-writing-missing-{}", std::process::id()))
        .join("image.png");
    let error = write_png_rows(&path, WIDTH, HEIGHT, PngPixelLayout::Gray8, |_, _| {})
        .expect_err("a missing folder cannot hold the file");
    assert!(error.chain_text().contains(&path.display().to_string()));
}
