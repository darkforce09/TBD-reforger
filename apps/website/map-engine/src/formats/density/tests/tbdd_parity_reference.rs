//! Role: tbdd parity reference.
//! Position: `formats/density/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::formats::density::tbdd::TbddError;
use crate::formats::density::tbdd::TbddGrid;

const REF_HEADER_BYTES: usize = 16;

#[inline]
fn u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// Decode tbdd.
pub(crate) fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {
    if bytes.len() < REF_HEADER_BYTES {
        return Err(TbddError::Short { len: bytes.len() });
    }
    if &bytes[0..4] != b"TBDD" {
        return Err(TbddError::BadMagic);
    }
    let version = u16_le(bytes, 4);
    let cell_m = u16_le(bytes, 6);
    let cols = u16_le(bytes, 8);
    let rows = u16_le(bytes, 10);
    let channel_count = bytes[12] as usize;
    let plane = cols as usize * rows as usize;
    let need = REF_HEADER_BYTES + channel_count * plane * 2;
    if bytes.len() < need {
        return Err(TbddError::Truncated {
            len: bytes.len(),
            want: need,
        });
    }
    let mut channels = Vec::with_capacity(channel_count);
    for c in 0..channel_count {
        let base = REF_HEADER_BYTES + c * plane * 2;
        let mut ch = vec![0u16; plane];
        for (k, slot) in ch.iter_mut().enumerate() {
            *slot = u16_le(bytes, base + 2 * k);
        }
        channels.push(ch);
    }
    Ok(TbddGrid {
        version,
        cell_m,
        cols,
        rows,
        channels,
    })
}
