//! TBDD density-grid decode — **Class R** (bit-identical to `decodeTBDD`, `forestMass.ts:38`).
//! Little-endian: 16 B header (u32 magic `TBDD`, u16 version, u16 cellM, u16 cols, u16 rows,
//! u8 channelCount, 3 B pad), then per channel `u16[cols·rows]` corner counts, row-major.

pub const TBDD_HEADER_BYTES: usize = 16;
/// Channel order: index 0 = tree, 1 = rock (`DENSITY_CHANNEL_NAMES`).
pub const DENSITY_CHANNEL_NAMES: [&str; 2] = ["tree", "rock"];

/// Decoded TBDD density grid (one export chunk).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TbddGrid {
    pub version: u16,
    pub cell_m: u16,
    pub cols: u16,
    pub rows: u16,
    /// Per-channel corner counts, `DENSITY_CHANNEL_NAMES` order.
    pub channels: Vec<Vec<u16>>,
}

/// Decode failure (the TS throws; the worker maps a throw to "no density for this chunk").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TbddError {
    Short { len: usize },
    BadMagic,
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

#[inline]
fn u16_le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// Decode one TBDD buffer. Mirror of `decodeTBDD` (`forestMass.ts:38`).
///
/// # Errors
/// Returns [`TbddError`] on a short buffer, bad magic, or a truncated channel block.
pub fn decode_tbdd(bytes: &[u8]) -> Result<TbddGrid, TbddError> {
    if bytes.len() < TBDD_HEADER_BYTES {
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
    let need = TBDD_HEADER_BYTES + channel_count * plane * 2;
    if bytes.len() < need {
        return Err(TbddError::Truncated {
            len: bytes.len(),
            want: need,
        });
    }
    let mut channels = Vec::with_capacity(channel_count);
    for c in 0..channel_count {
        let base = TBDD_HEADER_BYTES + c * plane * 2;
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

/// Encode a TBDD buffer (the exporter/tooling side of [`decode_tbdd`]; T-165.4 — promoted from
/// the test-private helper so the Rust world-export pipeline shares one codec with the engine).
/// Layout (locked, little-endian): 16-byte header (`TBDD`, u16 version=1, u16 cell_m, u16 cols,
/// u16 rows, u8 channel_count, 3B zero pad) then per-channel `u16[cols*rows]` row-major counts.
///
/// # Panics
/// Panics if a channel's length ≠ `cols * rows` (caller bug — mirrors the .mjs throw).
#[must_use]
pub fn encode_tbdd(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
    let cells = cols as usize * rows as usize;
    let mut b = Vec::with_capacity(16 + channels.len() * cells * 2);
    b.extend_from_slice(b"TBDD");
    b.extend_from_slice(&1u16.to_le_bytes()); // version
    b.extend_from_slice(&cell_m.to_le_bytes());
    b.extend_from_slice(&cols.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.push(u8::try_from(channels.len()).expect("<=255 channels"));
    b.extend_from_slice(&[0, 0, 0]); // pad
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
mod tests {
    use super::*;

    fn encode(cell_m: u16, cols: u16, rows: u16, channels: &[&[u16]]) -> Vec<u8> {
        encode_tbdd(cell_m, cols, rows, channels)
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
}
