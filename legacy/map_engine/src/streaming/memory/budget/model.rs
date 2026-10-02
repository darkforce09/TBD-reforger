//! Role: model.
//! Position: `streaming/memory/budget` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Bytes in a mebibyte — the unit the budget, the query param and the HUD are all spelled in.
pub const MIB: u64 = 1_048_576;

/// The default ceiling, in MiB.
pub const DEFAULT_BUDGET_MB: u64 = 1536;

/// The world assets the ledger keeps separate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Asset {
    /// The DEM: the encoded body and the `Vec<f32>` of metres it decodes into.
    Dem,

    /// The hillshade RGBA raster built from the DEM, alive until it is uploaded.
    Hillshade,

    /// The unified satellite basemap: decoded RGBA per uploaded tile plus the bodies it decodes.
    Satellite,

    /// World chunks — roads, buildings, landcover residency.
    World,

    /// The 8 m forest-mass density bins.
    Forest,

    /// Town / road / height text labels.
    Labels,

    /// The bathymetry mask and water vectors.
    Water,
}

impl Asset {
    /// Every asset, in ledger order. `ALL[a.index()] == a` for all `a`.
    pub const ALL: [Asset; 7] = [
        Asset::Dem,
        Asset::Hillshade,
        Asset::Satellite,
        Asset::World,
        Asset::Forest,
        Asset::Labels,
        Asset::Water,
    ];
}

impl Asset {
    /// Position in [`Asset::ALL`] and in [`Ledger`]'s entry array.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Asset::Dem => 0,
            Asset::Hillshade => 1,
            Asset::Satellite => 2,
            Asset::World => 3,
            Asset::Forest => 4,
            Asset::Labels => 5,
            Asset::Water => 6,
        }
    }
}

impl Asset {
    /// Stable key for the HUD and for `window.__t9386`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Asset::Dem => "dem",
            Asset::Hillshade => "hillshade",
            Asset::Satellite => "satellite",
            Asset::World => "world",
            Asset::Forest => "forest",
            Asset::Labels => "labels",
            Asset::Water => "water",
        }
    }
}

/// What the budget says about one prospective allocation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// Fits alongside everything currently held.
    Ok,

    /// Does not fit what is left, but would fit an empty budget — shrink and ask again.
    Degrade,

    /// Larger than the entire budget — no arrangement of the ledger can hold it.
    Refuse,
}

/// One asset's row.
#[derive(Clone, Copy, Default, Debug)]
pub struct Entry {
    /// Declared bytes currently reserved and not yet released.
    pub held: u64,

    /// The high-water mark of [`Entry::held`].
    pub peak: u64,

    /// Measured wasm linear memory (see [`heap_bytes`]) claimed while this asset was loading. A lower bound, never added to `held`.
    pub growth: u64,
}

/// The budget itself.
#[derive(Clone, Debug)]
pub struct Ledger {
    /// Budget.
    pub(super) budget: u64,

    /// Entries.
    pub(super) entries: [Entry; 7],

    /// Total peak.
    pub(super) total_peak: u64,

    /// Sat floor.
    pub(super) sat_floor: Option<usize>,

    /// Sat raised.
    pub(super) sat_raised: u32,
}
