//! Mission Creator editor (/missions/:id/edit) — the wgpu boundary collapse (T-159.15).
//!
//! **Slice T-159.15.0 (foundation):** the Leptos app owns `RenderEngine` DIRECTLY as plain Rust —
//! created from a canvas `NodeRef` via `spawn_local`, no `map-engine-wasm` shim, one wasm module /
//! one linear memory. This slice proves the engine links + mounts + renders one frame; the full
//! Eden docked shell (Top Command Strip, Left Outliner, Right Asset Palette, Bottom Toolbelt, the
//! doc host + camera/interaction) lands across T-159.15–.22. Route is `chromeless` + `fullBleed`
//! (AppLayout hides the platform nav). Verified by GPU readback (not DOM diff) as the map lane grows.
#![allow(dead_code)]
use leptos::prelude::*;

#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // The engine is created + owned on the wasm target only (wgpu is wasm32-gated). Native builds
    // (cargo test) compile the shell without touching the GPU stack.
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::task::spawn_local;
        // StoredValue::new_local holds the !Send engine for the component's lifetime (wgpu handles
        // are single-thread) — see [[wasm-react-lifecycle]]: own the handle in an effect-local cell,
        // not a memo. Leptos has no StrictMode double-invoke, so a single create is safe.
        let engine = StoredValue::new_local(None::<map_engine_render::RenderEngine>);
        canvas_ref.on_load(move |el: web_sys::HtmlCanvasElement| {
            spawn_local(async move {
                // The canvas device-pixel backing size must be set before create (plan §S4).
                let dpr = web_sys::window()
                    .map(|w| w.device_pixel_ratio())
                    .unwrap_or(1.0);
                let w = ((el.client_width() as f64 * dpr).round() as u32).max(1);
                let h = ((el.client_height() as f64 * dpr).round() as u32).max(1);
                el.set_width(w);
                el.set_height(h);
                match map_engine_render::RenderEngine::create(el, false).await {
                    Ok(eng) => {
                        engine.set_value(Some(eng));
                        engine.update_value(|e| {
                            if let Some(e) = e {
                                let _ = e.render();
                            }
                        });
                    }
                    Err(e) => {
                        leptos::logging::error!("RenderEngine::create failed: {e:?}");
                    }
                }
            });
        });
    }

    view! {
        <div class="relative h-screen w-screen overflow-hidden bg-background">
            <canvas node_ref=canvas_ref class="absolute inset-0 block h-full w-full"></canvas>
        </div>
    }
}
