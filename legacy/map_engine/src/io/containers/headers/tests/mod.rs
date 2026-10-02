//! Role: Module boundary for formats/containers/headers/tests.
//! Position: `io/containers/headers/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[repr(align(4))]
struct AlignedBuf([u8; 128]);

fn framed<H: ContainerHeader>(h: &H, payload: &[u8]) -> AlignedBuf {
    let mut b = AlignedBuf([0; 128]);
    b.0[..HEADER_BYTES].copy_from_slice(&h.to_header_bytes());
    b.0[HEADER_BYTES..HEADER_BYTES + payload.len()].copy_from_slice(payload);
    b
}

mod cases_1;
