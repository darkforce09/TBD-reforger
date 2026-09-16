//! Role: png tests.
//! Position: `terrain/dem/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::dem::png::*;

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
            crate::terrain::dem::sampling::uint16_to_meters(f64::from(s), -204.78, 375.53) as f32
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
