//! Role: tbdd tests.
//! Position: `io/density/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::density::tbdd::*;
use std::path::PathBuf;

fn encode(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
    encode_tbdd(cell_m, cols, rows, channels)
}

fn everon_density_tiles() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets_v2/terrains/everon/objects/density");
    let rd = std::fs::read_dir(&dir).unwrap_or_else(|e| {
        panic!(
            "T-935.5: {} could not be read ({e}). The Class-R acceptance is all 625 committed \
                 tiles; a missing corpus is a FAILURE, never a skip.",
            dir.display()
        )
    });
    let mut files: Vec<PathBuf> = rd
        .map(|e| e.expect("density dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "bin"))
        .collect();
    files.sort();

    assert_eq!(
        files.len(),
        625,
        "expected 625 everon density tiles in {}, found {}",
        dir.display(),
        files.len()
    );
    files
}

fn read_tile(path: &PathBuf) -> Vec<u8> {
    let bytes = std::fs::read(path).expect("read density tile");
    assert_eq!(
        bytes.get(..4),
        Some(&TBDD_MAGIC[..]),
        "{} does not start with the TBDD magic — if it starts with `vers` this checkout has \
             an LFS POINTER instead of the payload, which is an environment fault, not a decode \
             fault",
        path.display()
    );
    bytes
}

fn everon_tile_with_signal() -> (PathBuf, Vec<u8>) {
    for path in everon_density_tiles() {
        let bytes = read_tile(&path);
        if bytes[TBDD_HEADER_BYTES..].iter().any(|b| *b != 0) {
            return (path, bytes);
        }
    }
    panic!("all 625 everon density tiles have an all-zero payload — the corpus carries no signal")
}

#[test]
fn everon_tiles_decode_bit_identically_to_the_old_loop() {
    let files = everon_density_tiles();
    let mut nonzero_cells = 0u64;
    let mut compared_cells = 0u64;
    for path in &files {
        let bytes = read_tile(path);
        let got = decode_tbdd(&bytes);
        let want = parity_reference::decode_tbdd(&bytes);
        let (a, b) = match (&got, &want) {
            (Ok(a), Ok(b)) => (a, b),
            _ => panic!(
                "T-935.5 Class-R: {} — cast decode {got:?} vs old loop {want:?}",
                path.display()
            ),
        };
        assert_eq!(
            (a.version, a.cell_m, a.cols, a.rows),
            (b.version, b.cell_m, b.cols, b.rows),
            "T-935.5 Class-R: {} header mismatch",
            path.display()
        );
        assert_eq!(
            a.channels.len(),
            b.channels.len(),
            "T-935.5 Class-R: {} channel count mismatch",
            path.display()
        );
        for (ci, (ac, bc)) in a.channels.iter().zip(&b.channels).enumerate() {
            assert_eq!(
                ac.len(),
                bc.len(),
                "T-935.5 Class-R: {} channel {ci} length mismatch",
                path.display()
            );
            if let Some(at) = ac.iter().zip(bc).position(|(x, y)| x != y) {
                panic!(
                    "T-935.5 Class-R: {} channel {ci} cell {at}: cast decode {} != old loop {}",
                    path.display(),
                    ac[at],
                    bc[at]
                );
            }
            compared_cells += ac.len() as u64;
            nonzero_cells += ac.iter().filter(|v| **v != 0).count() as u64;
        }

        assert_eq!(
            (a.channels.len(), a.cols, a.rows),
            (DENSITY_CHANNEL_NAMES.len(), 65, 65),
            "T-935.5: {} is not a 2-channel 65×65 tile",
            path.display()
        );
    }
    assert_eq!(
        compared_cells,
        625 * 2 * 65 * 65,
        "the parity loop compared {compared_cells} cells, not the whole corpus"
    );
    assert!(
        nonzero_cells > 0,
        "every cell in all 625 tiles decoded to zero — the corpus carries no signal, so the \
             parity above compared nothing that could differ"
    );
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
    let (path, bytes) = everon_tile_with_signal();
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
        path.display()
    );
    assert_eq!(Ok(got.clone()), decode_tbdd(&bytes));
    assert_eq!(Ok(got), parity_reference::decode_tbdd(&bytes));
}

#[test]
fn short_payloads_are_err_never_panic() {
    let (_, bytes) = everon_tile_with_signal();
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
    let (_, bytes) = everon_tile_with_signal();
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
fn production_decode_has_no_per_byte_assembly_loop() {
    const SRC: &str = include_str!("../tbdd.rs");
    let live = class_r_scrub::live_source(SRC);

    assert!(
        live.contains("pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {"),
        "the scrub ate the production decoder"
    );
    assert!(
        live.contains("bytemuck::try_cast_slice::<u8, u16>(payload)"),
        "the production decoder no longer casts the payload"
    );
    assert!(
        live.contains("pub fn encode_tbdd("),
        "the scrub ate the production encoder"
    );

    assert!(
        !live.contains("REF_HEADER_BYTES"),
        "parity_reference survived"
    );
    assert!(
        !live.contains("mod parity_reference"),
        "the cut missed its item"
    );
    assert!(
        !live.contains("fn production_decode_has_no_per_byte_assembly_loop"),
        "the pin is grading itself"
    );

    let kept = live.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(
        kept > 100,
        "the scrub left {kept} non-blank lines — that is not the production half of this file"
    );

    for needle in ["u16_le(", "from_le_bytes(["] {
        assert!(
            !live.contains(needle),
            "T-935.5 acceptance: `{needle}` is back in production TBDD code — the decode is a \
                 per-byte assembly loop again"
        );
    }
}

#[test]
fn the_scrubber_keeps_production_and_cuts_the_test_half() {
    let src = concat!(
        "fn live() { let a = 1; }\n",
        "/// doc naming u16_le( and #[cfg(test)]\n",
        "const S: &str = \"u16_le( in a literal\";\n",
        "#[cfg(test)]\n",
        "mod t {\n    fn dead() { u16_le(b, 0); if x { y } }\n}\n",
        "fn live2() { let c = '{'; }\n",
    );
    let out = class_r_scrub::live_source(src);
    assert_eq!(out.lines().count(), src.lines().count(), "line count moved");
    assert!(out.contains("fn live()"), "production item cut");
    assert!(
        out.contains("fn live2()"),
        "the cut ran past its item's closing brace"
    );
    assert!(
        out.contains("const S: &str ="),
        "the literal's declaration was cut"
    );
    assert!(!out.contains("mod t"), "the test module survived");
    assert!(!out.contains("fn dead"), "the test module's body survived");
    assert!(
        !out.contains("u16_le("),
        "a banned needle survived in a comment or a literal"
    );
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
