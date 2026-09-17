//! Role: tbdd.
//! Position: `io/density` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};

/// Canonical tbdd header bytes value.
pub const TBDD_HEADER_BYTES: usize = 16;

/// Channel order: index 0 = tree, 1 = rock (`DENSITY_CHANNEL_NAMES`).
pub const DENSITY_CHANNEL_NAMES: [&str; 2] = ["tree", "rock"];

/// File magic, first four bytes of every TBDD buffer.
pub const TBDD_MAGIC: [u8; 4] = *b"TBDD";

/// The 16-byte TBDD file header, exactly as it sits on disk.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbddHeader {
    /// Magic.
    pub magic: [u8; 4],

    /// Version.
    pub version: u16,

    /// Cell m.
    pub cell_m: u16,

    /// Cols.
    pub cols: u16,

    /// Rows.
    pub rows: u16,

    /// Channel count.
    pub channel_count: u8,

    /// Always written as zero. Present so the struct is exactly 16 B with no compiler padding a `Pod` cast would expose as uninitialised bytes.
    pub _pad: [u8; 3],
}

const _: () = assert!(
    core::mem::size_of::<TbddHeader>() == TBDD_HEADER_BYTES,
    "TbddHeader is the TBDD wire header and MUST be exactly 16 bytes: 625 committed everon density \
     tiles, `developer_tools::density::TBDD_FILE_BYTES` and every payload offset are computed from it."
);
const _: () = assert!(
    core::mem::align_of::<TbddHeader>() == 2,
    "TbddHeader must stay 2-aligned: the header is 16 B, so a 2-aligned buffer puts the u16 cell \
     payload at a 2-aligned offset and `cast_slice` succeeds without a copy."
);
const _: () = assert!(
    cfg!(target_endian = "little"),
    "TBDD is little-endian on disk and bytemuck casts are NATIVE-endian, so a big-endian build \
     would read every cell byte-swapped while reporting success. Add explicit byte-swapping to \
     `decode_tbdd` before enabling such a target."
);

/// Decoded TBDD density grid (one export chunk).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TbddGrid {
    /// Version.
    pub version: u16,

    /// Cell m.
    pub cell_m: u16,

    /// Cols.
    pub cols: u16,

    /// Rows.
    pub rows: u16,

    /// Per-channel corner counts, `DENSITY_CHANNEL_NAMES` order.
    pub channels: Vec<Vec<u16>>,
}

/// Decode failure (the TS throws; the worker maps a throw to "no density for this chunk").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TbddError {
    /// Short.
    Short { len: usize },

    /// Bad magic.
    BadMagic,

    /// Truncated.
    Truncated { len: usize, want: usize },
}

impl core::fmt::Display for TbddError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TbddError::Short { len } => write!(f, "TBDD: short buffer ({len} B)"),
            TbddError::BadMagic => write!(f, "TBDD: bad magic"),
            TbddError::Truncated { len, want } => {
                write!(f, "TBDD: truncated ({len} B, want {want})")
            }
        }
    }
}

impl std::error::Error for TbddError {}

fn aligned_cells(payload: &[u8]) -> Cow<'_, [u16]> {
    if payload.is_empty() {
        return Cow::Borrowed(&[]);
    }
    match bytemuck::try_cast_slice::<u8, u16>(payload) {
        Ok(cells) => Cow::Borrowed(cells),
        Err(_) => {
            let mut owned = vec![0u16; payload.len() / 2];
            bytemuck::cast_slice_mut::<u16, u8>(&mut owned).copy_from_slice(payload);
            Cow::Owned(owned)
        }
    }
}

/// Decode one TBDD buffer. Mirror of `decodeTBDD` (`forestMass.ts:38`).
pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {
    let Some(head_bytes) = bytes.get(..TBDD_HEADER_BYTES) else {
        return Err(TbddError::Short { len: bytes.len() });
    };
    let head: TbddHeader = bytemuck::pod_read_unaligned(head_bytes);
    if head.magic != TBDD_MAGIC {
        return Err(TbddError::BadMagic);
    }

    let plane = head.cols as usize * head.rows as usize;
    let want = TBDD_HEADER_BYTES as u64 + u64::from(head.channel_count) * plane as u64 * 2;
    if (bytes.len() as u64) < want {
        return Err(TbddError::Truncated {
            len: bytes.len(),
            want: usize::try_from(want).unwrap_or(usize::MAX),
        });
    }
    let cell_bytes = head.channel_count as usize * plane * 2;
    let payload = &bytes[TBDD_HEADER_BYTES..TBDD_HEADER_BYTES + cell_bytes];
    let cells = aligned_cells(payload);
    let channels = if plane == 0 {
        vec![Vec::new(); head.channel_count as usize]
    } else {
        cells.chunks_exact(plane).map(<[u16]>::to_vec).collect()
    };
    Ok(TbddGrid {
        version: head.version,
        cell_m: head.cell_m,
        cols: head.cols,
        rows: head.rows,
        channels,
    })
}

/// Encode tbdd.
#[must_use]
pub fn encode_tbdd(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
    let cells = cols as usize * rows as usize;
    let mut b = Vec::with_capacity(16 + channels.len() * cells * 2);
    b.extend_from_slice(b"TBDD");
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&cell_m.to_le_bytes());
    b.extend_from_slice(&cols.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.push(u8::try_from(channels.len()).expect("<=255 channels"));
    b.extend_from_slice(&[0, 0, 0]);
    for (c, ch) in channels.iter().enumerate() {
        assert!(
            ch.len() == cells,
            "encode_tbdd: channel {c} has {} values, want {cells}",
            ch.len()
        );
        for &v in *ch {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b
}

#[cfg(test)]
#[path = "tests/tbdd_parity_reference.rs"]
mod parity_reference;

#[cfg(test)]
#[path = "tests/tbdd_class_r_scrub.rs"]
mod class_r_scrub;

#[cfg(test)]
#[path = "tests/tbdd_tests.rs"]
mod tests;
