//! The shared handles one mounted map view is made of.
//!
//! **Role:** bundles the render engine slot, the map host slot, the vector elevation grid, the
//! full-resolution heights and the disposal flag a map mount threads through its tasks and
//! listeners.
//! **Position:** created once per mount by the Mission Creator's canvas mount or by
//! [`super::mount::mount_map_view`]; every other part of [`super`] takes it or its fields.
//! **Signals & state:** `Rc` slots filled asynchronously (the engine after creation, the host and
//! the grids after the terrain boot) and an `Arc<AtomicBool>` set when the owning view unmounts.
//! **Invariants:** a clone shares every slot; once `disposed` is set no pump, listener or boot
//! step writes to the engine again.

use super::camera_fit::ViewState;
use super::terrain_height::TerrainHeights;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use website_map_engine::frame::EngineHandle;
use website_map_engine::streaming::host::{
    new_dem_grid_handle, new_host_handle, DemGridHandle, HostHandle,
};

/// Every shared slot of one mounted map view.
#[derive(Clone)]
pub struct MapViewHandles {
    /// The render engine, present once creation succeeded.
    pub engine: EngineHandle,

    /// The map host, present once the terrain boot finished.
    pub map_host: HostHandle,

    /// The box-averaged elevation grid (contours, line of sight).
    pub dem_grid: DemGridHandle,

    /// The full-resolution heights, filled only by a terrain-and-imagery boot.
    pub heights: TerrainHeights,

    /// Set when the owning view unmounts.
    pub disposed: Arc<AtomicBool>,
}

impl MapViewHandles {
    /// Fresh, empty handles for a new mount.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: std::rc::Rc::new(std::cell::RefCell::new(None)),
            map_host: new_host_handle(),
            dem_grid: new_dem_grid_handle(),
            heights: TerrainHeights::new(),
            disposed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ground height in metres at map position `(x, y)` from the full-resolution raster; `None`
    /// before it loads, outside it, or when the boot scope does not keep it.
    #[must_use]
    pub fn height_at(&self, x: f64, y: f64) -> Option<f64> {
        self.heights.height_at(x, y)
    }

    /// The engine's current camera view, once the engine exists.
    #[must_use]
    pub fn view_state(&self) -> Option<ViewState> {
        self.engine.borrow().as_ref().map(|e| ViewState {
            target_x: e.target_x(),
            target_y: e.target_y(),
            zoom: e.zoom(),
        })
    }

    /// Whether the owning view has unmounted.
    #[must_use]
    pub fn is_disposed(&self) -> bool {
        self.disposed.load(Ordering::Relaxed)
    }

    /// Mark the view unmounted; pumps and listeners stop at their next turn.
    pub fn dispose(&self) {
        self.disposed.store(true, Ordering::Relaxed);
    }
}

impl Default for MapViewHandles {
    fn default() -> Self {
        Self::new()
    }
}
