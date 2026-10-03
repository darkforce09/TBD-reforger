//! The camera reads and writes the map host makes through the asset sink.
//!
//! **Role:** [`MapViewport`]: the zoom, the camera target and the visible world rectangle the
//! loaders size their work by, and the two camera writes the host's fly-to issues.
//! **Position:** the supertrait of [`crate::asset_sink::MapAssetSink`]; the renderer implements
//! it over its camera, the map host and its loaders call it.
//! **Signals & state:** none here; the implementation owns the camera.
//! **Invariants:** a read never moves the camera; `set_view` is followed by `on_camera_changed`
//! before the next frame reads the view.

/// The camera as the map host sees it.
pub trait MapViewport {
    /// The map zoom: `log2` of screen pixels per world metre.
    fn zoom(&self) -> f64;

    /// World `x` of the camera target, in metres.
    fn target_x(&self) -> f64;

    /// World `y` of the camera target, in metres.
    fn target_y(&self) -> f64;

    /// The visible world rectangle `[min_x, min_y, max_x, max_y]`, in metres, or `None` while the
    /// camera has none.
    fn visible_bounds(&self) -> Option<[f64; 4]>;

    /// Move the camera to `target_x`, `target_y` at `zoom`.
    fn set_view(&mut self, target_x: f64, target_y: f64, zoom: f64);

    /// Recompute what depends on the camera after it moved.
    fn on_camera_changed(&mut self);
}
