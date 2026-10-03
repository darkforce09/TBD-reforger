//! **Role:** the host's thread-local cells: the mounted render context the layer switches and
//! queries reach the map through, and the camera gesture flag the settle passes read.
//! **Position:** `render_context` in `map_streaming_host`; the frontend's world-asset bridge
//! installs and clears [`RENDER_CTX`], the pointer and wheel gestures set the gesture flag through
//! `crate::viewport::set_camera_gesture`.
//! **Signals & state:** both cells are thread-local; the embedding frontend owns the render
//! context's registration and teardown.
//! **Invariants:** an empty render context means no map is mounted, and every reader answers
//! empty instead of waiting.

use std::cell::{Cell, RefCell};

use map_asset_loading::browser_asset_sink::BrowserAssetSinkHandle;

use crate::map_host::HostHandle;

thread_local! {

    /// Mounted render context; the embedding frontend owns registration and teardown.
    pub static RENDER_CTX: RefCell<Option<(BrowserAssetSinkHandle, HostHandle)>> = const { RefCell::new(None) };
}
thread_local! {

    /// Whether a camera gesture (a drag or a pinch) is under way.
    pub(crate) static CAMERA_GESTURE: Cell<bool> = const { Cell::new(false) };
}
