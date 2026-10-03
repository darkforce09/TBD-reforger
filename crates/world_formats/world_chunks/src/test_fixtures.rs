//! `TBDC` binary chunk fixtures shared by the container tests and the chunk-ingest tests.
//!
//! **Role:** writes a `WorldChunk` back into `TBDC` bytes and compares two chunks column by
//! column, bit for bit.
//! **Position:** compiled for this crate's tests and, through the `test_fixtures` feature, for the
//! map engine's chunk scheduler tests (`chunk_ingest_chunk_bin_tests.rs`).
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** the bytes follow the container header and `ObjectInstancePod` layout the parser
//! reads; a float column compares by its bits, so `-0.0` and `NaN` payloads are told apart.

use crate::world_chunk::WorldChunk;
use world_file_formats::containers::header::HEADER_BYTES;
use world_file_formats::pod::instance::POD_BYTES;

/// The `TBDC` bytes of `c` filed under tile `(cx, cy)`: the header, then one 32-byte row per instance.
pub fn encode_by_offset(cx: i16, cy: i16, c: &WorldChunk) -> Vec<u8> {
    let n = c.count as usize;
    let mut out = Vec::with_capacity(HEADER_BYTES + POD_BYTES * n);
    out.extend_from_slice(b"TBDC");
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&(n as u32).to_le_bytes());
    out.extend_from_slice(&cx.to_le_bytes());
    out.extend_from_slice(&cy.to_le_bytes());
    out.extend_from_slice(&[0_u8; 16]);
    for i in 0..n {
        out.extend_from_slice(&c.positions[2 * i].to_le_bytes());
        out.extend_from_slice(&c.positions[2 * i + 1].to_le_bytes());
        out.extend_from_slice(&c.z[i].to_le_bytes());
        out.extend_from_slice(&c.rotations[i].to_le_bytes());
        out.extend_from_slice(&c.pitch[i].to_le_bytes());
        out.extend_from_slice(&c.roll[i].to_le_bytes());
        out.extend_from_slice(&c.scale[i].to_le_bytes());
        out.extend_from_slice(&c.prefab_idx[i].to_le_bytes());
        out.push(c.cls_codes[i]);
        out.push(0);
    }
    assert_eq!(out.len(), HEADER_BYTES + POD_BYTES * n);
    out
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

/// Assert two chunks hold bit-identical columns; `ctx` names the comparison in a failure.
pub fn assert_columns_equal(got: &WorldChunk, want: &WorldChunk, ctx: &str) {
    assert_eq!(got.id, want.id, "{ctx}: id");
    assert_eq!(got.cx, want.cx, "{ctx}: cx");
    assert_eq!(got.cy, want.cy, "{ctx}: cy");
    assert_eq!(got.count, want.count, "{ctx}: count");
    assert_eq!(
        bits(&got.positions),
        bits(&want.positions),
        "{ctx}: positions"
    );
    assert_eq!(got.prefab_idx, want.prefab_idx, "{ctx}: prefab_idx");
    assert_eq!(
        bits(&got.rotations),
        bits(&want.rotations),
        "{ctx}: rotations"
    );
    assert_eq!(bits(&got.z), bits(&want.z), "{ctx}: z");
    assert_eq!(bits(&got.pitch), bits(&want.pitch), "{ctx}: pitch");
    assert_eq!(bits(&got.roll), bits(&want.roll), "{ctx}: roll");
    assert_eq!(bits(&got.scale), bits(&want.scale), "{ctx}: scale");
    assert_eq!(got.cls_codes, want.cls_codes, "{ctx}: cls_codes");
    assert_eq!(
        got.rows_by_class, want.rows_by_class,
        "{ctx}: rows_by_class"
    );
}
