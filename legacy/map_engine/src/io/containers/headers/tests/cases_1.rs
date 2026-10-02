//! Role: Domain regression cases.
//! Position: `io/containers/headers/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::BinaryError;

use crate::io::pod::instance::{self as pod, ObjectInstancePod};

use super::*;

#[test]
fn all_headers_are_thirty_two_bytes() {
    assert_eq!(size_of::<TbdcHeader>(), HEADER_BYTES);
    assert_eq!(size_of::<TbdeHeader>(), HEADER_BYTES);
    assert_eq!(size_of::<TbdbHeader>(), HEADER_BYTES);
    assert_eq!(size_of::<TbdsHeader>(), HEADER_BYTES);
}

#[test]
fn header_fields_sum_to_thirty_two() {
    assert_eq!(4 + 2 + 2 + 4 + 2 + 2 + 16, HEADER_BYTES);
    assert_eq!(4 + 2 + 2 + 4 + 4 + 4 + 4 + 8, HEADER_BYTES);
    assert_eq!(4 + 2 + 2 + 4 + 4 + 4 + 12, HEADER_BYTES);
    assert_eq!(4 + 2 + 2 + 4 + 20, HEADER_BYTES);
}

#[test]
fn tbdc_round_trips_and_reads_its_instances() {
    let rows = [ObjectInstancePod::identity(), ObjectInstancePod::identity()];
    let head = TbdcHeader::new(-7, 12, 2);
    let buf = framed(&head, pod::instances_to_bytes(&rows));
    let file = &buf.0[..head.file_bytes().expect("no overflow in a fixture")];

    let (h, payload) = TbdcHeader::parse(file).expect("aligned parse");
    assert_eq!(*h, head);
    assert_eq!(h.cx, -7);
    assert_eq!(h.cy, 12);
    assert_eq!(h.file_bytes(), Some(HEADER_BYTES + 64));
    assert_eq!(h.instances(payload).expect("cast"), &rows[..]);

    let (owned, payload2) = TbdcHeader::read(file).expect("unaligned-safe read");
    assert_eq!(owned, head);
    assert_eq!(payload2.len(), 64);
}

#[test]
fn tbdc_header_bytes_are_the_wire_layout() {
    let b = TbdcHeader::new(-7, 12, 258).to_header_bytes();
    assert_eq!(&b[0..4], b"TBDC");
    assert_eq!(&b[4..6], &1_u16.to_le_bytes());
    assert_eq!(&b[6..8], &0_u16.to_le_bytes());
    assert_eq!(&b[8..12], &258_u32.to_le_bytes());
    assert_eq!(&b[12..14], &(-7_i16).to_le_bytes());
    assert_eq!(&b[14..16], &12_i16.to_le_bytes());
    assert_eq!(&b[16..32], &[0_u8; 16]);
}

#[test]
fn tbde_round_trips_and_decodes_metres() {
    let head = TbdeHeader::new(4, 2, -204.78, 375.53);
    let samples: [u16; 8] = [0, 1, 2, 3, 4, 5, 6, u16::MAX];
    let buf = framed(&head, bytemuck::cast_slice(&samples));
    let file = &buf.0[..HEADER_BYTES + head.payload_bytes().expect("no overflow in a fixture")];

    let (h, payload) = TbdeHeader::parse(file).expect("aligned parse");
    assert_eq!(*h, head);
    assert_eq!(h.sample_count(), Some(8));
    assert_eq!(h.samples(payload).expect("cast"), &samples[..]);
    assert!((h.metres(0) - (-204.78)).abs() < 1e-2);
    assert!((h.metres(u16::MAX) - 375.53).abs() < 1e-2);
}

#[test]
fn tbde_header_bytes_are_the_wire_layout() {
    let head = TbdeHeader::new(6400, 6400, -204.78, 375.53);
    let b = head.to_header_bytes();
    assert_eq!(&b[0..4], b"TBDE");
    assert_eq!(&b[4..6], &1_u16.to_le_bytes());
    assert_eq!(&b[8..12], &6400_u32.to_le_bytes());
    assert_eq!(&b[12..16], &6400_u32.to_le_bytes());
    assert_eq!(&b[16..20], &head.scale_m.to_le_bytes());
    assert_eq!(&b[20..24], &(-204.78_f32).to_le_bytes());
    assert_eq!(&b[24..32], &[0_u8; 8]);
}

#[test]
fn tbdb_level_spans_are_header_computable() {
    let head = TbdbHeader::new(4, 4, 3, 0.5);
    assert_eq!(head.level_dims(0), Some((4, 4)));
    assert_eq!(head.level_dims(1), Some((2, 2)));
    assert_eq!(head.level_dims(2), Some((1, 1)));
    assert_eq!(head.level_dims(3), None);

    let l0 = head.level_span(0).expect("L0");
    assert_eq!(
        (
            l0.depth_offset,
            l0.depth_bytes,
            l0.mask_offset,
            l0.mask_bytes
        ),
        (0, 32, 32, 16)
    );
    assert_eq!(l0.stride, 48);

    let l1 = head.level_span(1).expect("L1");
    assert_eq!((l1.depth_offset, l1.stride), (48, 12));

    let l2 = head.level_span(2).expect("L2");
    assert_eq!((l2.depth_offset, l2.depth_bytes, l2.mask_bytes), (60, 2, 1));
    assert_eq!(l2.stride, 4);
    assert_eq!(head.level_span(3), None);
    assert_eq!(head.payload_bytes(), 48 + 12 + 4);
    assert!((head.metres(10) - 5.0).abs() < 1e-6);
}

#[test]
fn a_count_that_cannot_fit_the_address_space_is_refused_not_wrapped() {
    let head = TbdcHeader::new(0, 0, 0x0800_0000);
    let bytes = head.payload_bytes();
    println!("── usize {} bits ── payload_bytes = {bytes:?}", usize::BITS);
    if usize::BITS == 32 {
        assert_eq!(bytes, None, "the product does not fit and must not wrap");
    } else {
        assert_eq!(bytes, Some(0x1_0000_0000));
    }

    let err = head
        .instances(&[])
        .expect_err("an empty payload cannot satisfy 2^27 instances");
    println!("── instances(&[]) ── {err}");
    assert!(matches!(err, BinaryError::LengthMismatch { .. }));
}

#[test]
fn a_level_past_the_shift_width_is_none_not_a_panic() {
    let head = TbdbHeader::new(12_800, 12_800, 40, 0.1);

    assert_eq!(head.level_dims(0), Some((12_800, 12_800)));
    assert!(head.level_dims(31).is_some(), "31 is the last legal shift");

    for level in [32_u16, 33, 39] {
        println!("── level {level} ── {:?}", head.level_dims(level));
        assert_eq!(
            head.level_dims(level),
            None,
            "level {level} shifts a u32 by >= 32"
        );
        assert_eq!(head.level_span(level), None, "and its span goes with it");
    }

    assert_eq!(head.level_dims(40), None);
}

#[test]
fn tbdb_levels_clamp_at_one_texel() {
    let head = TbdbHeader::new(3, 1, 3, 1.0);
    assert_eq!(head.level_dims(0), Some((3, 1)));
    assert_eq!(head.level_dims(1), Some((1, 1)));
    assert_eq!(head.level_dims(2), Some((1, 1)));
    assert_eq!(head.payload_bytes(), 12 + 4 + 4);
}

#[test]
fn tbdb_round_trips() {
    let head = TbdbHeader::new(2, 2, 1, 0.25);
    let payload = vec![0_u8; head.payload_bytes()];
    let buf = framed(&head, &payload);
    let (h, p) = TbdbHeader::parse(&buf.0[..HEADER_BYTES + payload.len()]).expect("parse");
    assert_eq!(*h, head);
    assert_eq!(p.len(), head.payload_bytes());
}

#[test]
fn tbds_v2_frames_its_index() {
    let head = TbdsHeader::new(9);
    let payload = [1_u8, 2, 3, 4, 5, 6, 7, 8, 9, 99, 99];
    let buf = framed(&head, &payload);
    let (h, p) = TbdsHeader::parse(&buf.0[..HEADER_BYTES + payload.len()]).expect("parse");
    assert_eq!(*h, head);
    assert_eq!(h.version, TBDS_VERSION_V2);
    assert_eq!(h.tiles_offset(), HEADER_BYTES + 9);
    assert_eq!(h.index(p).expect("index"), &payload[..9]);
    assert!(
        TbdsHeader::new(64).index(p).is_err(),
        "index longer than payload must be Err"
    );
}

#[test]
fn tbds_v1_is_rejected_by_the_v2_header_but_peekable() {
    let mut b = AlignedBuf([0; 128]);
    b.0[..4].copy_from_slice(&TBDS_MAGIC);
    b.0[4..6].copy_from_slice(&1_u16.to_le_bytes());
    assert_eq!(peek_version(&b.0).expect("peek"), (TBDS_MAGIC, 1));
    let err = TbdsHeader::parse(&b.0[..HEADER_BYTES]).expect_err("v1 is not v2");
    assert_eq!(
        err,
        BinaryError::UnsupportedVersion {
            what: "TBDS",
            expected: 2,
            actual: 1
        }
    );
}

#[test]
fn truncated_buffer_is_err_not_panic() {
    let head = TbdcHeader::new(0, 0, 0);
    let buf = framed(&head, &[]);
    for n in 0..HEADER_BYTES {
        let err = TbdcHeader::parse(&buf.0[..n]).expect_err("short header");
        assert!(matches!(err, BinaryError::Truncated { .. }), "{n}: {err}");
        assert!(TbdcHeader::read(&buf.0[..n]).is_err(), "{n}");
    }
    assert!(peek_version(&buf.0[..5]).is_err());
}

#[test]
fn wrong_magic_is_err_not_panic() {
    let mut buf = framed(&TbdcHeader::new(0, 0, 0), &[]);
    buf.0[..4].copy_from_slice(b"vers");
    let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("wrong magic");
    assert_eq!(
        err,
        BinaryError::BadMagic {
            what: "TBDC",
            expected: TBDC_MAGIC,
            actual: *b"vers"
        }
    );
    assert!(err.to_string().contains("b\"vers\""), "{err}");
}

#[test]
fn magic_is_checked_before_version() {
    let buf = framed(&TbdeHeader::new(1, 1, 0.0, 1.0), &[]);
    let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("TBDE is not TBDC");
    assert!(matches!(err, BinaryError::BadMagic { .. }), "{err}");
}

#[test]
fn wrong_version_is_err_not_panic() {
    let mut head = TbdcHeader::new(0, 0, 0);
    head.version = 99;
    let buf = framed(&head, &[]);
    let err = TbdcHeader::parse(&buf.0[..HEADER_BYTES]).expect_err("v99");
    assert_eq!(
        err,
        BinaryError::UnsupportedVersion {
            what: "TBDC",
            expected: 1,
            actual: 99
        }
    );
}

#[test]
fn count_payload_mismatch_is_err() {
    let head = TbdcHeader::new(0, 0, 4);
    let rows = [ObjectInstancePod::identity()];
    let err = head
        .instances(pod::instances_to_bytes(&rows))
        .expect_err("1 row for a count of 4");
    assert_eq!(
        err,
        BinaryError::LengthMismatch {
            what: "TBDC",
            expected: 128,
            actual: 32
        }
    );
    let short = TbdeHeader::new(4, 4, 0.0, 1.0);
    assert!(short.samples(&[0_u8; 4]).is_err());
}

#[test]
fn misaligned_buffer_parses_via_read_only() {
    let head = TbdcHeader::new(3, 4, 0);

    let mut v = AlignedBuf([0; 128]);
    v.0[1..=HEADER_BYTES].copy_from_slice(&head.to_header_bytes());
    let err = TbdcHeader::parse(&v.0[1..=HEADER_BYTES]).expect_err("odd address");
    assert!(matches!(err, BinaryError::Misaligned { .. }), "{err}");
    let (owned, payload) = TbdcHeader::read(&v.0[1..=HEADER_BYTES]).expect("read copies");
    assert_eq!(owned, head);
    assert!(payload.is_empty());
}
