//! The parsed tile index and the container reader's error.
//!
//! **Role:** [`TbdSatIndex`] with its levels ([`TbdSatMip`]) and tiles ([`TbdSatTile`]), the
//! shape both container versions parse into; [`TbdSatError`], why a container is refused; and
//! the format constants (magic, versions, the WebP tile format).
//! **Position:** produced by [`crate::header`], checked by [`crate::validation`], read by
//! [`crate::selection`] and by the map engine's satellite loader.
//! **Signals & state:** none; plain data.
//! **Invariants:** the version 1 JSON index deserialises in camelCase; `container_version` is
//! never read from the wire, the parser stamps it.

use serde::Deserialize;
use world_file_formats::ids::TerrainId;

/// Canonical magic value.
pub(super) const MAGIC: u32 = 0x5344_4254;

/// Canonical v1 value.
pub(super) const V1: u16 = 1;

/// Canonical v2 value.
pub(super) const V2: u16 = 2;

/// Canonical format webp value.
pub(super) const FORMAT_WEBP: u8 = 0;

/// One WebP tile of a level: its pixel rectangle on the level and its bytes in the file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatTile {
    /// Left edge of the tile on its level, in pixels (JSON `x`).
    pub x: u32,

    /// Top edge of the tile on its level, in pixels (JSON `y`).
    pub y: u32,

    /// Tile width in pixels (JSON `width`); a last-column tile may be narrower than the rest.
    pub width: u32,

    /// Tile height in pixels (JSON `height`); a last-row tile may be shorter than the rest.
    pub height: u32,

    /// Absolute byte offset of the tile's WebP bytes in the container file (JSON `offset`);
    /// never before the payload start.
    pub offset: u64,

    /// Byte length of the tile's WebP bytes (JSON `length`); `offset + length` stays within
    /// the file.
    pub length: u64,
}

/// One level of the satellite pyramid: its size in pixels and the tiles that cover it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatMip {
    /// Level number, 0 for the full-resolution base, one more per halving (JSON `level`).
    pub level: u32,

    /// Level width in pixels (JSON `width`).
    pub width: u32,

    /// Level height in pixels (JSON `height`).
    pub height: u32,

    /// The tiles covering the level (JSON `tiles`); a version 2 index lists them row-major.
    pub tiles: Vec<TbdSatTile>,
}

/// The tile index of a `.tbd-sat` container: the base image size and every level's tiles,
/// parsed from either container version.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatIndex {
    /// Which TBDS container this index came out of: 1 or 2. **Not a wire field on either side** — `serde(skip)` keeps it out of the v1 JSON and the parser stamps it, so `validate_index` can hold the index's own declared `format_version` against the container it was actually read from instead of against a hardcoded 1.
    #[serde(skip)]
    pub container_version: u32,

    /// Version the index declares (JSON `formatVersion`); must equal `container_version`.
    pub format_version: u32,

    /// The terrain the version 1 JSON index names, such as `everon`; absent in version 2.
    pub terrain_id: Option<TerrainId>,

    /// The world rectangle the version 1 JSON index names; absent in version 2.
    pub world_bounds: Option<[f64; 4]>,

    /// Width in pixels of level 0, the full-resolution image (JSON `baseWidthPx`); at least 1.
    pub base_width_px: u32,

    /// Height in pixels of level 0 (JSON `baseHeightPx`); at least 1.
    pub base_height_px: u32,

    /// Number of levels (JSON `mipCount`); at least 1 and equal to `mips.len()`.
    pub mip_count: u32,

    /// The levels from the base down (JSON `mips`); the strict check requires level `i` at
    /// index `i`.
    pub mips: Vec<TbdSatMip>,
}

/// Why a satellite container is refused.
#[derive(Debug, thiserror::Error)]
pub enum TbdSatError {
    /// The buffer is shorter than the header.
    #[error("tbd-sat: file too small for header")]
    TooSmall,

    /// The first four bytes are not `TBDS`.
    #[error("tbd-sat: bad magic (expected TBDS)")]
    BadMagic,

    /// A container version other than 1 or 2.
    #[error("tbd-sat: unsupported formatVersion {0}")]
    UnsupportedVersion(u32),

    /// The index is empty or over 16 MiB, or the index or the payload runs past the file.
    #[error("tbd-sat: JSON index overruns file")]
    JsonOverrun,

    /// The version 1 JSON index does not parse; the parser's message.
    #[error("tbd-sat: JSON index unparseable: {0}")]
    Json(String),

    /// The index contradicts itself, its container or the file; what is wrong.
    #[error("tbd-sat: {0}")]
    Structure(String),

    /// The version 2 header or archived index does not validate; the reader's message.
    #[error("tbd-sat: v2 index: {0}")]
    Archive(String),
}
