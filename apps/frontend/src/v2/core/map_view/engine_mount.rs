//! Canvas sizing and render engine creation for a map view.
//!
//! **Role:** sizes a canvas's backing store to its container, reads the `force=webgl` switch,
//! and creates a damage-driven render engine with its camera bounds and first view applied.
//! **Position:** the first step of every map mount: the Mission Creator's boot tasks and
//! [`super::mount::mount_map_view`] call [`size_canvas`] then [`create_engine`], then add their
//! own lanes to the engine.
//! **Signals & state:** writes the canvas element's `width` and `height`; owns no state.
//! **Invariants:** the canvas carries its device-pixel size before the engine reads it
//! ([`super::device_size::device_size`] matches the engine's rounding); a created engine renders
//! only on damage, with the calibration overlay hidden and frame timing off; a creation failure
//! comes back as a readable reason, never a panic.

use super::camera_fit::{ViewState, WorldBounds};
use super::device_size::device_size;
use map_engine::frame::engine::RenderEngine;

/// A container's CSS size and the device pixel ratio it was measured at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSize {
    /// CSS width.
    pub css_w: f64,

    /// CSS height.
    pub css_h: f64,

    /// Device pixel ratio.
    pub dpr: f64,
}

/// Measure `container` and give `canvas` the matching device-pixel backing size.
pub fn size_canvas(
    container: &web_sys::Element,
    canvas: &web_sys::HtmlCanvasElement,
) -> CanvasSize {
    let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
    let rect = container.get_bounding_client_rect();
    let (dw, dh) = device_size(rect.width(), rect.height(), dpr);
    canvas.set_width(dw);
    canvas.set_height(dh);
    CanvasSize {
        css_w: rect.width(),
        css_h: rect.height(),
        dpr,
    }
}

/// Whether the page URL asks for the WebGL backend (`force=webgl` in the query string).
#[must_use]
pub fn force_webgl_from_location() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|s| s.contains("force=webgl"))
}

/// What a new engine starts with.
#[derive(Clone, Copy, Debug)]
pub struct EngineStartup {
    /// Use the WebGL backend even where WebGPU is available.
    pub force_webgl: bool,

    /// The container size the canvas was sized for.
    pub size: CanvasSize,

    /// The camera's pan bounds.
    pub bounds: WorldBounds,

    /// The first view.
    pub view: ViewState,
}

/// Create a damage-driven render engine on `canvas` (already sized by [`size_canvas`]) with the
/// startup camera applied. The error is the engine's own message, or a generic reason when it
/// carries none.
pub async fn create_engine(
    canvas: web_sys::HtmlCanvasElement,
    startup: EngineStartup,
) -> Result<RenderEngine, String> {
    match RenderEngine::create(canvas, startup.force_webgl).await {
        Ok(mut engine) => {
            let size = startup.size;
            let _ = engine.resize(size.css_w, size.css_h, size.dpr);
            let b = startup.bounds;
            engine.set_camera_bounds(b.min_x, b.min_y, b.max_x, b.max_y);
            let v = startup.view;
            engine.set_view(v.target_x, v.target_y, v.zoom);
            engine.hide_calibration();
            engine.disable_frame_timing();
            engine.set_continuous_render(false);
            Ok(engine)
        }
        Err(e) => Err(js_sys::Error::from(wasm_bindgen::JsValue::from(e))
            .message()
            .as_string()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "the render engine failed to start".to_string())),
    }
}
