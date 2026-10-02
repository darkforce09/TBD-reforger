//! Role: satellite.
//! Position: `io/archives` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

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
