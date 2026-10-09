//! Unit tests of the `TBDD` density codec.
//!
//! **Role:** proves round trips, the on-disk header layout and the refusal paths, and that the
//! in-place decoder matches the byte-by-byte reference on synthetic shapes and on an unaligned
//! payload.
//! **Position:** test-only child of [`crate::density::tbdd`].
//! **Signals & state:** none; pure functions over synthetic grids.
//! **Invariants:** the production and reference decoders agree on every grid and every refusal.

use crate::density::tbdd::*;

fn encode(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
    encode_tbdd(cell_m, cols, rows, channels)
}

/// A 65 × 65, two-channel, 8 m tile whose cells are not all zero — the shape of a committed
/// density tile.
fn synthetic_tile_with_signal() -> (&'static str, Vec<u8>) {
    let first: Vec<u16> = (0..65 * 65).map(|i| (i % 251) as u16).collect();
    let second: Vec<u16> = (0..65 * 65).map(|i| (i * 7 % 509) as u16).collect();
    ("the synthetic tile", encode(8, 65, 65, &[&first, &second]))
}

#[test]
fn decode_matches_the_old_loop_on_synthetic_shapes() {
    let mut cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![1, 2, 3],
        encode(32, 2, 2, &[&[1, 2, 3, 4]]),
        encode(8, 1, 1, &[&[7]]),
        encode(8, 0, 0, &[]),
        encode(8, 65, 65, &[&vec![9u16; 4225], &vec![3u16; 4225]]),
    ];

    let mut zero_plane = encode(8, 0, 0, &[]);
    zero_plane[12] = 2;
    cases.push(zero_plane);
    let mut bad = encode(32, 2, 2, &[&[0, 0, 0, 0]]);
    bad[0] = b'X';
    cases.push(bad);
    let full = encode(32, 2, 2, &[&[1, 2, 3, 4]]);
    cases.push(full[..full.len() - 2].to_vec());
    cases.push(full[..TBDD_HEADER_BYTES].to_vec());

    let mut hostile = encode(8, 1, 1, &[&[0]]);
    hostile[8] = 0xFF;
    hostile[9] = 0xFF;
    hostile[10] = 0xFF;
    hostile[11] = 0xFF;
    hostile[12] = 0xFF;
    cases.push(hostile);

    for (i, buf) in cases.iter().enumerate() {
        assert_eq!(
            decode_tbdd(buf),
            parity_reference::decode_tbdd(buf),
            "T-935.5 Class-R: synthetic case {i} ({} B) disagrees",
            buf.len()
        );
    }

    assert!(cases.iter().any(|b| decode_tbdd(b).is_ok()));
    assert!(cases.iter().any(|b| decode_tbdd(b).is_err()));
}

#[test]
fn unaligned_payload_decodes_identically() {
    let (path, bytes) = synthetic_tile_with_signal();
    let mut shifted = Vec::with_capacity(bytes.len() + 1);
    shifted.push(0u8);
    shifted.extend_from_slice(&bytes);
    let unaligned = &shifted[1..];
    assert_eq!(
        unaligned.as_ptr() as usize % 2,
        1,
        "the buffer under test is 2-aligned, so this test never reached the aligned-copy \
             branch it exists to cover"
    );
    assert!(
        bytemuck::try_cast_slice::<u8, u16>(&unaligned[TBDD_HEADER_BYTES..]).is_err(),
        "the payload cast succeeded on an odd address — the copy branch was not exercised"
    );
    let got = decode_tbdd(unaligned).expect("odd-addressed tile must decode");

    assert!(
        got.channels.iter().flatten().any(|v| *v != 0),
        "{} decoded to all zeros — the copy branch was graded against nothing",
        path
    );
    assert_eq!(Ok(got.clone()), decode_tbdd(&bytes));
    assert_eq!(Ok(got), parity_reference::decode_tbdd(&bytes));
}

#[test]
fn short_payloads_are_err_never_panic() {
    let (_, bytes) = synthetic_tile_with_signal();
    for cut in [0usize, 1, 4, 12, 15, 16, 17, 100, 16_915] {
        let short = &bytes[..cut];
        let got = decode_tbdd(short);
        assert!(got.is_err(), "{cut} B decoded as Ok: {got:?}");
        assert_eq!(got, parity_reference::decode_tbdd(short), "{cut} B");
    }
    assert!(
        decode_tbdd(&bytes).is_ok(),
        "the full tile must still decode"
    );
}

#[test]
fn header_pod_is_the_on_disk_header() {
    let (_, bytes) = synthetic_tile_with_signal();
    let head: TbddHeader = bytemuck::pod_read_unaligned(&bytes[..TBDD_HEADER_BYTES]);
    assert_eq!(size_of::<TbddHeader>(), TBDD_HEADER_BYTES);
    assert_eq!(align_of::<TbddHeader>(), 2);
    assert_eq!(head.magic, TBDD_MAGIC);
    assert_eq!((head.version, head.cell_m), (1, 8));
    assert_eq!((head.cols, head.rows), (65, 65));
    assert_eq!(head.channel_count, 2);
    assert_eq!(head._pad, [0, 0, 0]);

    assert_eq!(bytes[4..6], head.version.to_le_bytes());
    assert_eq!(bytes[8..10], head.cols.to_le_bytes());
}

#[test]
fn round_trip() {
    let tree: Vec<u16> = (0..4).collect();
    let rock: Vec<u16> = vec![9, 8, 7, 6];
    let buf = encode(32, 2, 2, &[&tree, &rock]);
    let g = decode_tbdd(&buf).unwrap();
    assert_eq!(g.cell_m, 32);
    assert_eq!((g.cols, g.rows), (2, 2));
    assert_eq!(g.channels.len(), 2);
    assert_eq!(g.channels[0], tree);
    assert_eq!(g.channels[1], rock);
}

#[test]
fn bad_magic() {
    let mut buf = encode(32, 2, 2, &[&[0, 0, 0, 0]]);
    buf[0] = b'X';
    assert_eq!(decode_tbdd(&buf), Err(TbddError::BadMagic));
}

#[test]
fn truncated() {
    let buf = encode(32, 2, 2, &[&[1, 2, 3, 4]]);
    let short = &buf[..buf.len() - 2];
    assert!(matches!(
        decode_tbdd(short),
        Err(TbddError::Truncated { .. })
    ));
}

#[test]
fn short_header() {
    assert!(matches!(
        decode_tbdd(&[1, 2, 3]),
        Err(TbddError::Short { .. })
    ));
}
