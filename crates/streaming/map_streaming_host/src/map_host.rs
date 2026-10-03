//! **Role:** the state one mounted map keeps: [`MapHost`] with its loaders, statistics bridge,
//! relief grid and settle timers, and the shared host and elevation-grid handles.
//! **Position:** `map_host` in `map_streaming_host`; [`crate::bootstrap::bootstrap`] builds the
//! host and lands it in the caller's [`HostHandle`], the settle passes and queries read it.
//! **Signals & state:** one `MapHost` per mounted map behind an `Rc<RefCell<Option<_>>>`; the
//! settle timer and deadline cells it shares with the scheduled settle callback.
//! **Invariants:** a viewport pass takes the host out of its handle and puts it back, so a reader
//! during the pass finds no host.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use map_asset_loading::asset_statistics::{BridgeHandle, new_bridge};
use map_asset_loading::browser_asset_sink::BrowserAssetSinkHandle;
use map_asset_loading::environment::forest_mass_loader::ForestMassHost;
use map_asset_loading::terrain::relief::dem_vectors::DemVectors;
use map_asset_loading::world_loader::WorldHost;

use crate::view_preferences::swap_basemap;

/// Shared host handle for camera-settle refresh.
pub type HostHandle = Rc<RefCell<Option<MapHost>>>;

/// Dem grid handle.
pub type DemGridHandle = Rc<RefCell<Option<Rc<terrain_elevation::grid::DemVectorGrid>>>>;

/// New dem grid handle.
pub fn new_dem_grid_handle() -> DemGridHandle {
    Rc::new(RefCell::new(None))
}

/// Map host.
pub struct MapHost {
    /// Preferences.
    pub(crate) preferences: map_streaming_model::host_preferences::HostPreferences,

    /// Bridge.
    pub(crate) bridge: BridgeHandle,

    /// World.
    pub(crate) world: WorldHost,

    /// Forest.
    pub(crate) forest: ForestMassHost,

    /// Dem.
    pub(crate) dem: DemVectors,

    /// Settle timer.
    pub(crate) settle_timer: Rc<Cell<Option<i32>>>,

    /// Settle deadline.
    pub(crate) settle_deadline: Rc<Cell<f64>>,

    /// Terrain.
    pub(crate) terrain: String,

    /// Labels.
    pub(crate) labels: map_asset_loading::environment::location_labels::loader::LabelHost,

    /// Water.
    pub(crate) water: map_asset_loading::terrain::water::loader::WaterHost,
}

impl MapHost {
    /// New.
    pub(crate) fn new(preferences: map_streaming_model::host_preferences::HostPreferences) -> Self {
        Self {
            preferences,
            bridge: new_bridge(),
            world: WorldHost::new(preferences.world_layers),
            forest: ForestMassHost::new(),
            dem: DemVectors::new(),
            settle_timer: Rc::new(Cell::new(None)),
            settle_deadline: Rc::new(Cell::new(0.0)),
            terrain: String::new(),
            labels: map_asset_loading::environment::location_labels::loader::LabelHost::new(),
            water: map_asset_loading::terrain::water::loader::WaterHost::new(),
        }
    }
}

impl MapHost {
    /// Set basemap view.
    pub(crate) async fn set_basemap_view(&self, engine: &BrowserAssetSinkHandle, view: &str) {
        swap_basemap(engine, &self.terrain, view).await;
    }
}

/// New host handle.
pub fn new_host_handle() -> HostHandle {
    Rc::new(RefCell::new(None))
}
