//! Role: state.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// Shared host handle for camera-settle refresh.
pub type HostHandle = Rc<RefCell<Option<MapHost>>>;

/// Dem grid handle.
pub type DemGridHandle = Rc<RefCell<Option<Rc<crate::world::terrain::dem::grid::DemVectorGrid>>>>;

/// New dem grid handle.
pub fn new_dem_grid_handle() -> DemGridHandle {
    Rc::new(RefCell::new(None))
}

/// Map host.
pub struct MapHost {
    /// Preferences.
    pub(super) preferences: crate::streaming::bridge::host_preferences::HostPreferences,

    /// Bridge.
    pub(super) bridge: BridgeHandle,

    /// World.
    pub(super) world: WorldHost,

    /// Forest.
    pub(super) forest: ForestMassHost,

    /// Dem.
    pub(super) dem: DemVectors,

    /// Settle timer.
    pub(super) settle_timer: Rc<Cell<Option<i32>>>,

    /// Settle deadline.
    pub(super) settle_deadline: Rc<Cell<f64>>,

    /// Terrain.
    pub(super) terrain: String,

    /// Labels.
    pub(super) labels: crate::world::environment::locations::loader::LabelHost,

    /// Water.
    pub(super) water: crate::world::terrain::water::loader::WaterHost,
}

impl MapHost {
    /// New.
    pub(super) fn new(
        preferences: crate::streaming::bridge::host_preferences::HostPreferences,
    ) -> Self {
        Self {
            preferences,
            bridge: new_bridge(),
            world: WorldHost::new(preferences.world_layers),
            forest: ForestMassHost::new(),
            dem: DemVectors::new(),
            settle_timer: Rc::new(Cell::new(None)),
            settle_deadline: Rc::new(Cell::new(0.0)),
            terrain: String::new(),
            labels: crate::world::environment::locations::loader::LabelHost::new(),
            water: crate::world::terrain::water::loader::WaterHost::new(),
        }
    }
}

impl MapHost {
    /// Set basemap view.
    pub(super) async fn set_basemap_view(&self, engine: &EngineHandle, view: &str) {
        swap_basemap(engine, &self.terrain, view).await;
    }
}

/// New host handle.
pub fn new_host_handle() -> HostHandle {
    Rc::new(RefCell::new(None))
}
