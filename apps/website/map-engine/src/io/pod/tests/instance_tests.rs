//! Role: instance tests.
//! Position: `io/pod/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::pod::instance::*;

fn sample() -> ObjectInstancePod {
    ObjectInstancePod {
        x: 4096.5,
        y: 8192.25,
        z: -12.125,
        yaw: 271.5,
        pitch: -3.25,
        roll: 0.5,
        scale: 1.75,
        prefab_id: 1623,
        class_code: 7,
        _pad: 0,
    }
}

#[test]
fn pod_is_thirty_two_bytes_and_four_aligned() {
    assert_eq!(core::mem::size_of::<ObjectInstancePod>(), 32);
    assert_eq!(core::mem::align_of::<ObjectInstancePod>(), 4);
    assert_eq!(POD_BYTES, 32);
}

#[test]
fn every_byte_is_a_named_field() {
    let named = 7 * size_of::<f32>() + size_of::<u16>() + 2 * size_of::<u8>();
    assert_eq!(named, POD_BYTES);
}

#[test]
fn round_trips_through_bytes_byte_for_byte() {
    let rows = [sample(), ObjectInstancePod::identity()];
    let bytes = instances_to_bytes(&rows);
    assert_eq!(bytes.len(), 2 * POD_BYTES);
    let back = instances_from_bytes(bytes).expect("aligned cast");
    assert_eq!(back, &rows[..]);
}

#[test]
fn field_offsets_are_the_wire_layout() {
    let rows = [sample()];
    let b = instances_to_bytes(&rows);
    assert_eq!(&b[0..4], &4096.5_f32.to_le_bytes());
    assert_eq!(&b[4..8], &8192.25_f32.to_le_bytes());
    assert_eq!(&b[8..12], &(-12.125_f32).to_le_bytes());
    assert_eq!(&b[12..16], &271.5_f32.to_le_bytes());
    assert_eq!(&b[16..20], &(-3.25_f32).to_le_bytes());
    assert_eq!(&b[20..24], &0.5_f32.to_le_bytes());
    assert_eq!(&b[24..28], &1.75_f32.to_le_bytes());
    assert_eq!(&b[28..30], &1623_u16.to_le_bytes());
    assert_eq!(b[30], 7);
    assert_eq!(b[31], 0);
}

#[test]
fn identity_scale_is_one_and_default_is_not() {
    assert_eq!(ObjectInstancePod::identity().scale, 1.0);
    assert_eq!(ObjectInstancePod::default().scale, 0.0);
}

#[test]
fn truncated_payload_is_err_not_panic() {
    let rows = [sample()];
    let bytes = instances_to_bytes(&rows);
    let err = instances_from_bytes(&bytes[..31]).expect_err("31 B is not a whole row");
    assert!(matches!(err, BinaryError::LengthMismatch { .. }), "{err}");
    assert!(instances_from_bytes(&bytes[..1]).is_err());
}

#[repr(align(4))]
struct AlignedBuf([u8; 1 + POD_BYTES]);

#[test]
fn misaligned_slice_is_err_not_panic() {
    let mut buf = AlignedBuf([0; 1 + POD_BYTES]);
    buf.0[1..].copy_from_slice(bytemuck::bytes_of(&sample()));
    let err = instances_from_bytes(&buf.0[1..]).expect_err("odd address cannot cast");
    assert!(matches!(err, BinaryError::Misaligned { .. }), "{err}");
}

#[test]
fn empty_payload_is_zero_rows_not_an_error() {
    assert_eq!(instances_from_bytes(&[]).expect("0 rows"), &[]);
}
