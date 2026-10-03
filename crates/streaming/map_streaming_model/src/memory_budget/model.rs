//! The memory budget's vocabulary: the assets, the decisions, the rows and the ledger type.
//!
//! **Role:** [`Asset`], [`Decision`], [`Entry`] and [`Ledger`], with [`MIB`] and
//! [`DEFAULT_BUDGET_MB`]; the ledger's methods are in `ledger.rs` and `hud_suffix.rs`.
//! **Position:** the model the map engine's live ledger and the satellite floor walk work on.
//! **Signals & state:** none; plain values.
//! **Invariants:** [`Asset::ALL`], [`Asset::index`], [`Asset::name`] and the ledger's seven-row
//! entry array list the assets in the same order.

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

    /// Measured wasm linear memory (the page's heap size, as the map engine's live ledger reads it)
    /// claimed while this asset was loading. A lower bound, never added to `held`.
    pub growth: u64,
}

/// The budget itself.
#[derive(Clone, Debug)]
pub struct Ledger {
    /// The ceiling, in bytes.
    pub(super) budget: u64,

    /// One row per asset, in [`Asset::ALL`] order.
    pub(super) entries: [Entry; 7],

    /// High-water mark of the held bytes across every asset.
    pub(super) total_peak: u64,

    /// The satellite mip level the budget settled on, once chosen.
    pub(super) sat_floor: Option<usize>,

    /// How many levels the budget raised the satellite floor by.
    pub(super) sat_raised: u32,
}
