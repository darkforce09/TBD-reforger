//! Role: tbds.
//! Position: `io/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::io::archives::codec::BinaryError;
use crate::io::containers::header::ContainerHeader;
use crate::io::containers::header::HEADER_BYTES;
use bytemuck::Pod;
use bytemuck::Zeroable;

/// `satellite/{terrain}-sat.tbd-sat`.
pub const TBDS_MAGIC: [u8; 4] = *b"TBDS";

/// Canonical tbds version v2 value.
pub const TBDS_VERSION_V2: u16 = 2;

/// `satellite/{terrain}-sat.tbd-sat` **version 2** — a fixed 32-byte header, then `index_len` bytes of rkyv [`TbdSatIndexV2`](super::archives::TbdSatIndexV2), then the tile bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct TbdsHeader {
    /// Magic.
    pub magic: [u8; 4],

    /// Version.
    pub version: u16,

    /// Flags.
    pub flags: u16,

    /// Bytes of rkyv index immediately after this header.
    pub index_len: u32,

    /// Reserved.
    pub reserved: [u8; 20],
}

const _: () = assert!(size_of::<TbdsHeader>() == HEADER_BYTES);

impl ContainerHeader for TbdsHeader {
    const MAGIC: [u8; 4] = TBDS_MAGIC;
    const VERSION: u16 = TBDS_VERSION_V2;
    const NAME: &'static str = "TBDS";
    fn magic_bytes(&self) -> [u8; 4] {
        self.magic
    }
    fn format_version(&self) -> u16 {
        self.version
    }
}

impl TbdsHeader {
    /// A valid v2 header for an index of `index_len` bytes.
    #[must_use]
    pub fn new(index_len: u32) -> Self {
        Self {
            magic: TBDS_MAGIC,
            version: TBDS_VERSION_V2,
            flags: 0,
            index_len,
            reserved: [0; 20],
        }
    }
}

impl TbdsHeader {
    /// File offset where tile bytes begin — the origin every `Tile::offset` is measured from.
    #[must_use]
    pub fn tiles_offset(&self) -> usize {
        HEADER_BYTES + self.index_len as usize
    }
}

impl TbdsHeader {
    /// The rkyv index bytes out of a payload (a payload being everything after the header), ready for [`access_checked`](super::access_checked).
    pub fn index<'a>(&self, payload: &'a [u8]) -> Result<&'a [u8], BinaryError> {
        let n = self.index_len as usize;
        payload.get(..n).ok_or(BinaryError::Truncated {
            what: Self::NAME,
            expected: n,
            actual: payload.len(),
        })
    }
}
