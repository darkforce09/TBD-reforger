//! Role: raw tests.
//! Position: `world/terrain/dem/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::world::terrain::dem::raw::*;

const EXACT_MIN_M: f32 = -256.0;
const EXACT_MAX_M: f32 = EXACT_MIN_M + 65535.0 / 128.0;

const EVERON_MIN_M: f32 = -204.78;
const EVERON_MAX_M: f32 = 375.53;

const GRID_W: u32 = 4;
const GRID_H: u32 = 3;
fn grid() -> Vec<u16> {
    vec![
        0, 1, 65535, 32768, 40000, 7, 60000, 100, 12345, 2, 511, 65534,
    ]
}

fn framed(w: u32, h: u32, min_m: f32, max_m: f32, samples: &[u16]) -> Vec<u8> {
    to_bytes(&TbdeHeader::new(w, h, min_m, max_m), samples)
}

#[test]
fn parse_round_trips_the_grid_and_the_header() {
    let s = grid();
    let dem = RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("parse");
    assert_eq!((dem.width(), dem.height()), (GRID_W, GRID_H));
    assert_eq!(dem.samples, s);
    assert_eq!(dem.header.flags, 0);
    assert_eq!(dem.header.reserved, [0_u8; 8]);
    assert_eq!(dem.header.offset_m, EXACT_MIN_M);
    assert_eq!(dem.header.scale_m, (EXACT_MAX_M - EXACT_MIN_M) / 65535.0);
    assert_eq!(
        dem.header.scale_m,
        1.0 / 128.0,
        "EXACT_* must quantise to an exact f32 step — the metres-parity tests rest on it"
    );
}

#[test]
fn sample_u16_indexes_row_major_and_bounds_check() {
    let s = grid();
    let dem = RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("parse");
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            assert_eq!(
                dem.sample_u16(x, y),
                Some(s[(y * GRID_W + x) as usize]),
                "({x},{y})"
            );
        }
    }
    assert_eq!(dem.sample_u16(GRID_W, 0), None);
    assert_eq!(dem.sample_u16(0, GRID_H), None);
    assert_eq!(dem.metres(GRID_W, GRID_H), None);
}

#[test]
fn streamed_in_any_chunk_size_matches_the_whole_buffer() {
    let s = grid();
    let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    let whole = RawDem::parse(&bytes).expect("parse");
    for step in [1_usize, 2, 3, 5, 7, 11, 31, 32, 33, 47, bytes.len()] {
        let mut sink = RawDemSink::new(bytes.len() as u64);
        for c in bytes.chunks(step) {
            sink.push(c).unwrap_or_else(|e| panic!("step {step}: {e}"));
        }
        let got = sink.finish().unwrap_or_else(|e| panic!("step {step}: {e}"));
        assert_eq!(got, whole, "chunk size {step}");
    }
}

#[test]
fn misaligned_payload_decodes_identically_to_the_aligned_one() {
    #[repr(align(2))]
    struct Align2([u8; 64]);

    let s = grid();
    let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    let payload = &bytes[HEADER_BYTES..];
    let head = &bytes[..HEADER_BYTES];

    let mut even = Align2([0; 64]);
    even.0[..payload.len()].copy_from_slice(payload);
    let even_payload = &even.0[..payload.len()];
    let mut odd = Align2([0; 64]);
    odd.0[1..=payload.len()].copy_from_slice(payload);
    let odd_payload = &odd.0[1..=payload.len()];

    assert!(
        bytemuck::try_cast_slice::<u8, u16>(even_payload).is_ok(),
        "the even buffer must take the cast_slice branch"
    );
    assert!(
        bytemuck::try_cast_slice::<u8, u16>(odd_payload).is_err(),
        "the odd buffer must take the byte-pair branch"
    );

    let decode = |p: &[u8]| {
        let mut sink = RawDemSink::new(bytes.len() as u64);
        sink.push(head).expect("head");
        sink.push(p).expect("payload");
        sink.finish().expect("finish")
    };
    let from_even = decode(even_payload);
    assert_eq!(from_even.samples, s);
    assert_eq!(decode(odd_payload), from_even);
}

#[test]
fn metres_is_offset_plus_sample_times_scale() {
    let s = grid();
    let dem =
        RawDem::parse(&framed(GRID_W, GRID_H, EVERON_MIN_M, EVERON_MAX_M, &s)).expect("parse");
    let scale = (EVERON_MAX_M - EVERON_MIN_M) / 65535.0;
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            let v = s[(y * GRID_W + x) as usize];
            assert_eq!(dem.metres(x, y), Some(EVERON_MIN_M + f32::from(v) * scale));
        }
    }
    assert_eq!(dem.metres_grid().len(), s.len());
    assert_eq!(dem.metres_grid()[0], dem.metres(0, 0).expect("in range"));
}

#[test]
fn bad_magic_and_version_are_reported_not_guessed() {
    let s = grid();
    let mut bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    bytes[0] = b'X';
    assert!(matches!(
        RawDem::parse(&bytes),
        Err(BinaryError::BadMagic { .. })
    ));
    let mut bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    bytes[4..6].copy_from_slice(&9_u16.to_le_bytes());
    assert!(matches!(
        RawDem::parse(&bytes),
        Err(BinaryError::UnsupportedVersion { .. })
    ));

    assert!(matches!(
        RawDem::parse(b"version https://git-lfs.github.com/spec/v1\n"),
        Err(BinaryError::BadMagic { .. })
    ));
}

#[test]
fn truncated_header_and_short_payload_are_errors() {
    let s = grid();
    let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    assert!(matches!(
        RawDem::parse(&bytes[..HEADER_BYTES - 1]),
        Err(BinaryError::Truncated { .. })
    ));

    assert!(matches!(
        RawDem::parse(&bytes[..bytes.len() - 2]),
        Err(BinaryError::LengthMismatch { .. })
    ));

    let mut sink = RawDemSink::new(bytes.len() as u64);
    sink.push(&bytes[..bytes.len() - 2]).expect("prefix");
    assert!(matches!(
        sink.finish(),
        Err(BinaryError::LengthMismatch { .. })
    ));

    let mut sink = RawDemSink::new(bytes.len() as u64);
    sink.push(&bytes[..bytes.len() - 1]).expect("prefix");
    assert!(matches!(
        sink.finish(),
        Err(BinaryError::LengthMismatch { .. })
    ));
}

#[test]
fn payload_overrun_is_an_error() {
    let s = grid();
    let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    let mut sink = RawDemSink::new(bytes.len() as u64);
    sink.push(&bytes).expect("whole file");
    assert!(matches!(
        sink.push(&[1, 2, 3, 4]),
        Err(BinaryError::LengthMismatch { .. })
    ));
}

#[test]
fn the_sample_vector_is_allocated_once_and_never_moves() {
    let s = grid();
    let bytes = framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s);
    let mut sink = RawDemSink::new(bytes.len() as u64);
    assert_eq!(
        sink.samples.capacity(),
        0,
        "nothing may be allocated before the header has been validated against the file length"
    );

    let mut byte_at_a_time = bytes.chunks(1);
    for c in byte_at_a_time.by_ref().take(HEADER_BYTES) {
        sink.push(c).expect("header byte");
    }
    let (ptr, cap) = (sink.samples.as_ptr(), sink.samples.capacity());
    assert_eq!(
        sink.samples.len(),
        s.len(),
        "the header must size the buffer to the whole grid at once, not incrementally"
    );

    for c in byte_at_a_time {
        sink.push(c).expect("payload byte");
        assert_eq!(
            sink.samples.as_ptr(),
            ptr,
            "the sample buffer moved: the payload decode reallocated"
        );
        assert_eq!(sink.samples.capacity(), cap, "the sample buffer regrew");
    }

    let dem = sink.finish().expect("finish");
    assert_eq!(
        dem.samples.as_ptr(),
        ptr,
        "`finish` must move the buffer out, not copy it into a second allocation"
    );
    assert_eq!(dem.samples, s);
}

#[test]
fn hostile_dimensions_are_rejected_before_allocating() {
    let bytes = framed(65535, 65535, EXACT_MIN_M, EXACT_MAX_M, &[0_u16; 4]);
    assert_eq!(bytes.len(), HEADER_BYTES + 8);
    match RawDem::parse(&bytes) {
        Err(BinaryError::LengthMismatch {
            expected, actual, ..
        }) => {
            let declared: usize = HEADER_BYTES + 65535_usize * 65535 * 2;
            assert_eq!(expected, declared);
            assert_eq!(actual, bytes.len());
        }
        other => panic!("expected LengthMismatch, got {other:?}"),
    }
}

#[test]
fn zero_by_zero_grid_is_empty_not_an_error() {
    let dem = RawDem::parse(&framed(0, 0, EXACT_MIN_M, EXACT_MAX_M, &[])).expect("parse");
    assert!(dem.samples.is_empty());
    assert_eq!(dem.sample_u16(0, 0), None);
}

#[cfg(feature = "world")]
#[test]
fn dem_and_png_decode_to_the_same_grid_and_metres() {
    use crate::world::terrain::dem::png::decode_png_gray16;
    use crate::world::terrain::dem::sampling::uint16_to_meters;

    let s = grid();
    let mut png_bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png_bytes, GRID_W, GRID_H);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Sixteen);
        let mut w = enc.write_header().expect("png header");
        let be: Vec<u8> = s.iter().flat_map(|v| v.to_be_bytes()).collect();
        w.write_image_data(&be).expect("png data");
    }
    let (png_raster, pw, ph) = decode_png_gray16(&png_bytes).expect("png decode");
    let dem = RawDem::parse(&framed(GRID_W, GRID_H, EXACT_MIN_M, EXACT_MAX_M, &s)).expect("dem");

    assert_eq!((dem.width(), dem.height()), (pw, ph), "dims");
    assert_eq!(dem.samples, png_raster, "u16 grid");
    for y in 0..ph {
        for x in 0..pw {
            let png_m = uint16_to_meters(
                f64::from(png_raster[(y * pw + x) as usize]),
                f64::from(EXACT_MIN_M),
                f64::from(EXACT_MAX_M),
            ) as f32;
            let raw_m = dem.metres(x, y).expect("in range");
            assert!(
                (f64::from(raw_m) - f64::from(png_m)).abs() <= 1e-6,
                "({x},{y}): raw {raw_m} vs png {png_m}"
            );
        }
    }
}

#[cfg(feature = "world")]
#[test]
fn everon_range_keeps_the_grid_exact_and_metres_within_f32_rounding() {
    use crate::world::terrain::dem::sampling::uint16_to_meters;

    let s: Vec<u16> = (0..=65535_u32).step_by(97).map(|v| v as u16).collect();
    let w = s.len() as u32;
    let dem = RawDem::parse(&framed(w, 1, EVERON_MIN_M, EVERON_MAX_M, &s)).expect("dem");
    assert_eq!(
        dem.samples, s,
        "u16 grid must be bit-exact regardless of range"
    );
    let mut worst = 0.0_f64;
    for (x, &v) in s.iter().enumerate() {
        let png_m = uint16_to_meters(
            f64::from(v),
            f64::from(EVERON_MIN_M),
            f64::from(EVERON_MAX_M),
        ) as f32;
        let raw_m = dem.metres(x as u32, 0).expect("in range");
        worst = worst.max((f64::from(raw_m) - f64::from(png_m)).abs());
    }
    assert!(
        worst <= 1e-4,
        "worst metre delta {worst} over everon's range"
    );
}
