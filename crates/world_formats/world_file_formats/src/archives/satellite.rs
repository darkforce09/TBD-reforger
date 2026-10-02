//! The satellite tile index inside a `TBDS` container.
//!
//! **Role:** declares [`TbdSatIndexV2`]: the satellite pyramid's base size, tile size and, per
//! level, each tile's byte offset, length and image format.
//! **Position:** written by the developer tools' satellite archive writer after a
//! [`crate::containers::tbds::TbdsHeader`]; read by the map engine's satellite streamer through
//! [`crate::archives::codec::access_checked`].
//! **Signals & state:** none; plain rkyv data types.
//! **Invariants:** tile offsets count from the end of the index
//! ([`crate::containers::tbds::TbdsHeader::tiles_offset`]); the index carries no schema
//! version, the container version names it.

/// Sat tile.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct SatTile {
    /// Offset.
    pub offset: u64,

    /// Len.
    pub len: u32,

    /// Format.
    pub format: u8,
}

/// Sat level.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct SatLevel {
    /// W tiles.
    pub w_tiles: u32,

    /// H tiles.
    pub h_tiles: u32,

    /// Tiles.
    pub tiles: Vec<SatTile>,
}

/// Tbd sat index v2.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
#[rkyv(derive(Debug))]
pub struct TbdSatIndexV2 {
    /// Base w.
    pub base_w: u32,

    /// Base h.
    pub base_h: u32,

    /// Tile px.
    pub tile_px: u16,

    /// Levels.
    pub levels: Vec<SatLevel>,
}
