//! One-call mount of a navigable terrain map view.
//!
//! **Role:** mounts a complete map view on a container and canvas: sizes the canvas, fits the
//! camera to the terrain manifest's `worldBounds`, creates the engine, starts the damage-driven
//! pump, keeps the canvas sized, attaches drag-pan, wheel-zoom and click-to-pick, and boots the
//! terrain with the caller's preferences.
//! **Position:** the entry point for map views outside the Mission Creator (the fire-planning map
//! picker). The Mission Creator composes the same parts itself because its boot interleaves the
//! mission document.
//! **Signals & state:** fills the slots of the caller's [`MapViewHandles`]; progress goes to the
//! caller's report callback.
//! **Invariants:** the engine is published into the handles only when the view is still mounted;
//! a missing or malformed manifest or a failed engine creation ends the mount with a typed
//! [`MapViewError`] before any listener is attached.

use super::camera_fit::{fit_view, WorldBounds};
use super::engine_mount::{create_engine, size_canvas, EngineStartup};
use super::frame_pump::start_plain_frame_pump;
use super::handles::MapViewHandles;
use super::navigation::{attach_navigation, MapClick};
use super::resize::observe_container_resize;
use map_engine::streaming::bridge::host_preferences::HostPreferences;
use map_engine::streaming::bridge::progress::ProgressFn;
use std::rc::Rc;

/// Everything a map view mount needs from its page.
pub struct MapViewMount {
    /// The element the canvas fills; pointer and wheel listeners attach here.
    pub container: web_sys::HtmlElement,

    /// The canvas the engine draws on.
    pub canvas: web_sys::HtmlCanvasElement,

    /// Terrain id, the folder under `/map-assets/`.
    pub terrain: String,

    /// Use the WebGL backend even where WebGPU is available.
    pub force_webgl: bool,

    /// Boot scope and preference readers for the terrain boot.
    pub preferences: HostPreferences,

    /// Boot progress sink.
    pub report: ProgressFn,

    /// Called with the map position of every click.
    pub on_click: Rc<dyn Fn(MapClick)>,
}

/// Why a map view could not mount.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapViewError {
    /// `/map-assets/<terrain>/manifest.json` was unreachable or had no usable `worldBounds`.
    ManifestUnavailable,

    /// The render engine could not start; the engine's reason.
    EngineFailed(String),
}

/// Mount a map view into `handles` and boot its terrain; resolves once the boot finished (or
/// immediately when the view unmounted while the engine was being created).
pub async fn mount_map_view(
    mount: MapViewMount,
    handles: MapViewHandles,
) -> Result<(), MapViewError> {
    let MapViewMount {
        container,
        canvas,
        terrain,
        force_webgl,
        preferences,
        report,
        on_click,
    } = mount;
    let size = size_canvas(&container, &canvas);
    let manifest_url = format!("/map-assets/{terrain}/manifest.json");
    let bounds = browser_platform::fetch::fetch_bytes(&manifest_url)
        .await
        .as_deref()
        .and_then(WorldBounds::from_manifest_json)
        .ok_or(MapViewError::ManifestUnavailable)?;
    let startup = EngineStartup {
        force_webgl,
        size,
        bounds,
        view: fit_view(bounds, size.css_w, size.css_h),
    };
    let engine = create_engine(canvas.clone(), startup)
        .await
        .map_err(MapViewError::EngineFailed)?;
    if handles.is_disposed() {
        return Ok(());
    }
    *handles.engine.borrow_mut() = Some(engine);
    start_plain_frame_pump(handles.engine.clone(), handles.disposed.clone());
    observe_container_resize(&container, &canvas, &handles);
    attach_navigation(&container, &handles, on_click);
    map_engine::streaming::host::bootstrap(
        handles.engine.clone(),
        terrain,
        handles.map_host.clone(),
        handles.dem_grid.clone(),
        handles.heights.handle(),
        report,
        preferences,
    )
    .await;
    Ok(())
}
