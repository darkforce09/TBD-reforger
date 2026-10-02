//! Shared inputs of the container header tests.
//!
//! **Role:** frames a header and a payload into one 4-byte-aligned buffer, the shape a reader
//! receives from a fetch.
//! **Position:** test-only child of [`crate::containers`], used by `container_header_tests.rs`;
//! writes the header through [`crate::containers::header::ContainerHeader::to_header_bytes`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** the payload starts exactly [`crate::containers::header::HEADER_BYTES`] after the
//! buffer start.

use crate::containers::header::{ContainerHeader, HEADER_BYTES};

#[repr(align(4))]
pub(super) struct AlignedBuf(pub(super) [u8; 128]);

pub(super) fn framed<H: ContainerHeader>(h: &H, payload: &[u8]) -> AlignedBuf {
    let mut b = AlignedBuf([0; 128]);
    b.0[..HEADER_BYTES].copy_from_slice(&h.to_header_bytes());
    b.0[HEADER_BYTES..HEADER_BYTES + payload.len()].copy_from_slice(payload);
    b
}
