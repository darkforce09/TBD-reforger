//! Role: model.
//! Position: `terrain/satellite/streamer` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::Deserialize;

/// Canonical magic value.
pub(super) const MAGIC: u32 = 0x5344_4254;

/// Canonical v1 value.
pub(super) const V1: u16 = 1;

/// Canonical v2 value.
pub(super) const V2: u16 = 2;

/// Canonical format webp value.
pub(super) const FORMAT_WEBP: u8 = 0;

/// Tbd sat tile.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatTile {
    /// X.
    pub x: u32,

    /// Y.
    pub y: u32,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Offset.
    pub offset: u64,

    /// Length.
    pub length: u64,
}

/// Tbd sat mip.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatMip {
    /// Level.
    pub level: u32,

    /// Width.
    pub width: u32,

    /// Height.
    pub height: u32,

    /// Tiles.
    pub tiles: Vec<TbdSatTile>,
}

/// Tbd sat index.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TbdSatIndex {
    /// Which TBDS container this index came out of: 1 or 2. **Not a wire field on either side** — `serde(skip)` keeps it out of the v1 JSON and the parser stamps it, so `validate_index` can hold the index's own declared `format_version` against the container it was actually read from instead of against a hardcoded 1.
    #[serde(skip)]
    pub container_version: u32,

    /// Format version.
    pub format_version: u32,

    /// Terrain id.
    #[allow(dead_code)]
    pub terrain_id: Option<String>,

    /// World bounds.
    #[allow(dead_code)]
    pub world_bounds: Option<[f64; 4]>,

    /// Base width px.
    pub base_width_px: u32,

    /// Base height px.
    pub base_height_px: u32,

    /// Mip count.
    pub mip_count: u32,

    /// Mips.
    pub mips: Vec<TbdSatMip>,
}

/// Tbd sat error.
#[derive(Debug)]
pub enum TbdSatError {
    /// Too small.
    TooSmall,

    /// Bad magic.
    BadMagic,

    /// Unsupported version.
    UnsupportedVersion(u32),

    /// Json overrun.
    JsonOverrun,

    /// Json.
    Json(String),

    /// Structure.
    Structure(String),

    /// Archive.
    Archive(String),
}

impl std::fmt::Display for TbdSatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall => write!(f, "tbd-sat: file too small for header"),
            Self::BadMagic => write!(f, "tbd-sat: bad magic (expected TBDS)"),
            Self::UnsupportedVersion(v) => write!(f, "tbd-sat: unsupported formatVersion {v}"),
            Self::JsonOverrun => write!(f, "tbd-sat: JSON index overruns file"),
            Self::Json(e) => write!(f, "tbd-sat: JSON index unparseable: {e}"),
            Self::Structure(e) => write!(f, "tbd-sat: {e}"),
            Self::Archive(e) => write!(f, "tbd-sat: v2 index: {e}"),
        }
    }
}
