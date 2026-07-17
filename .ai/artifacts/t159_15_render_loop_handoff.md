# T-159.15.1 — Mission Creator render loop: handoff + blocker

**Status:** attempted, reverted to keep the branch green at **15.0** (`3066f14c`). The camera + rAF
render loop + resize + wheel-zoom code below is CORRECT for production; it hit a headless-WebGL2 wgpu
buffer-lifecycle blocker in the smoke. Solve the blocker, paste the code back, verify, commit as 15.1.

## The blocker (precise)

A continuous `render()` loop panics:

```
wgpu-29.0.4/src/api/buffer.rs:572: assertion `left == right` failed: Buffer is already mapped
  left: 0..16   right: 0..0
```

The 16-byte buffer is a **readback buffer** — `GpuTimer` (`engine.rs:322/328`, timestamp read) and/or
the icon-cull GPU counter (`engine.rs` `kick_readback`, called in `render()` at ~1748/1752 after
`queue.submit` + `frame.present`). `kick_readback` does `read_buf.slice(..).map_async(Read, cb)`
where `cb` runs `buf.unmap()` + clears an `in_flight` guard.

**Root cause:** on headless **WebGL2 (SwiftShader)** the `map_async` callback only fires when
`device.poll()` is pumped. The normal render-submit path does NOT poll (only the self-check readbacks
— `map_read_4` @ ~2059 — call `device.poll(Poll)`). So the buffer stays mapped, `in_flight` never
clears, and the next submit's `kick_readback` double-maps → panic.
**WebGPU (lavapipe/browser) auto-resolves `map_async` via the event loop → no poll needed → no panic.**
So this is very likely a *headless-WebGL2-only* issue; production (real-browser WebGPU) is probably fine.

A single `render()` (the 15.0 foundation) is fine — the panic needs a *second* submit (idle frame
under `continuous=true`, or a wheel-driven damage submit under `continuous=false`).

## Fix options (do with the wgpu-headless-gpu-verify harness — see [[wgpu-headless-gpu-verify]])

**IMPORTANT:** the harness `launch()` in `cdp.mjs` ALREADY passes `--enable-unsafe-webgpu` +
`--use-angle=swiftshader` — the panicking smokes ran WITH WebGPU enabled. So the engine either fell
back to WebGL2 anyway, or headless-lavapipe WebGPU also needs the poll. Option 2 is therefore the
real fix, not a launch-flag change.

1. **★ RECOMMENDED — add an engine `pub fn poll(&self)`** that calls
   `let _ = self.device.poll(wgpu::PollType::Poll);` (the engine already holds `self.device`; it
   polls this way internally in `map_read_4` @ ~2075). Call `engine.poll()` once per frame in the
   render loop (right after `render()`). This lets the readback `map_async` callbacks fire →
   `unmap` → the `in_flight` guard clears → no double-map, on both headless backends AND real
   WebGL2-fallback users. Minimal, non-behavioral engine change. **Real-browser WebGPU auto-resolves
   `map_async` via the event loop, so prod is likely already fine — this mainly unblocks the headless
   verify + hardens WebGL2.**
2. **Log `engine.backend()`** in the smoke (expose it via a tiny JS shim or a `data-` attr the engine
   sets) to confirm which backend `create(canvas, false)` picks headless — informs whether the poll
   is needed for WebGPU too.
3. **Gate the readback** — a `set_counters_enabled(false)` on the engine to skip `kick_readback`
   until the readback harness drives poll. Loses the fps/cull HUD counters (DEV-only anyway). A
   fallback if the poll hook is undesirable.
4. **Upload a basemap first** — the empty scene may route `render()` through the compute/cull path
   differently; the basemap controller (`.15.2`) uploads real data. Cheap to check, least likely.

**Plan:** add `poll()` (option 1) → paste the render-loop code back → re-run the smoke (screenshot
changes after wheel, no panic) → wire a GPU readback self-check as the real gate → commit 15.1.

## Verification recipe

- Oracle: `apps/website/frontend/dist` (React prod build). Leptos: `trunk build --release`.
- Smoke scripts in the session scratchpad (`smoke2.mjs` = screenshot→wheel→screenshot diff;
  `smoke3.mjs` = panic capture). Add WebGPU launch flags + `engine.backend()` logging.
- Map-lane gate is **GPU readback self-checks** (`texture_self_check`, `readback_rgba` at fixed
  pixels), NOT DOM diff — the editor DOM is just the canvas until the Eden shell lands.
- The camera app code is deterministic: initial view Everon `[6400,6400]@-2`, bounds `0,0,12800,12800`,
  wheel `zoom_at(-deltaY/500, cx-rect.left, cy-rect.top)` (engine self-clamps zoom to [-6,6]).

## Reference (React)

`apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx` — main render effect ~432–531
(size→create→`set_camera_bounds`→`set_view`→`hide_calibration`→`set_continuous_render(DEV)`→rAF
`render()` loop + ResizeObserver); wheel effect ~550–598. `set_continuous_render` takes
`import.meta.env.DEV` → the prod oracle runs **damage-driven (false)**.

## The reverted render-loop code (ready to paste into `src/mission_editor.rs`)

Needs these `web-sys` features in `apps/website-leptos/Cargo.toml` (15.0 has only `HtmlCanvasElement`):
`"HtmlDivElement", "Element", "DomRect", "WheelEvent", "EventTarget", "AddEventListenerOptions"`.

```rust
//! Mission Creator editor (/missions/:id/edit) — the wgpu boundary collapse (T-159.15).
//! T-159.15.0 linked map-engine-render + mounted the canvas. T-159.15.1: camera + continuous render
//! loop + resize + wheel-zoom, engine owned directly (D5). Wheel → zoom_at (engine self-clamps).
#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    #[cfg(target_arch = "wasm32")]
    {
        use leptos::task::spawn_local;
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        const TERRAIN_W: f64 = 12_800.0;
        const TERRAIN_H: f64 = 12_800.0;
        const INITIAL_TARGET: (f64, f64) = (6_400.0, 6_400.0);
        const INITIAL_ZOOM: f64 = -2.0;
        const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;

        canvas_ref.on_load(move |canvas: web_sys::HtmlCanvasElement| {
            let Some(container) = container_ref.get_untracked() else { return };
            let container: web_sys::HtmlDivElement = container;
            let win = web_sys::window().expect("window");

            let dpr0 = win.device_pixel_ratio();
            let rect0 = container.get_bounding_client_rect();
            let (dw, dh) = device_size(rect0.width(), rect0.height(), dpr0);
            canvas.set_width(dw);
            canvas.set_height(dh);

            let engine: Rc<RefCell<Option<map_engine_render::RenderEngine>>> = Rc::new(RefCell::new(None));
            let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

            spawn_local({
                let engine = engine.clone();
                let disposed = disposed.clone();
                let canvas = canvas.clone();
                let (cw, ch) = (rect0.width(), rect0.height());
                async move {
                    match map_engine_render::RenderEngine::create(canvas, false).await {
                        Ok(mut eng) => {
                            if disposed.load(std::sync::atomic::Ordering::Relaxed) { return; }
                            let _ = eng.resize(cw, ch, dpr0);
                            eng.set_camera_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
                            eng.set_view(INITIAL_TARGET.0, INITIAL_TARGET.1, INITIAL_ZOOM);
                            eng.hide_calibration();
                            eng.set_continuous_render(false); // damage-driven, matches prod oracle
                            // FIX HERE: on WebGL2, pump device.poll each frame (engine hook) OR run on WebGPU.
                            *engine.borrow_mut() = Some(eng);
                            start_raf(engine.clone(), disposed.clone());
                        }
                        Err(e) => leptos::logging::error!("RenderEngine::create: {e:?}"),
                    }
                }
            });

            let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
                let engine = engine.clone();
                let container = container.clone();
                move |ev: web_sys::WheelEvent| {
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        ev.prevent_default();
                        let rect = container.get_bounding_client_rect();
                        e.zoom_at(-ev.delta_y() * WHEEL_ZOOM_PER_PX,
                                  ev.client_x() as f64 - rect.left(), ev.client_y() as f64 - rect.top());
                    }
                }
            });
            let wheel_opts = web_sys::AddEventListenerOptions::new();
            wheel_opts.set_passive(false);
            wheel_opts.set_capture(true);
            let _ = container.add_event_listener_with_callback_and_add_event_listener_options(
                "wheel", onwheel.as_ref().unchecked_ref(), &wheel_opts);

            let onresize = Closure::<dyn FnMut()>::new({
                let engine = engine.clone();
                let canvas = canvas.clone();
                let container = container.clone();
                move || {
                    let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
                    let rect = container.get_bounding_client_rect();
                    let (dw, dh) = device_size(rect.width(), rect.height(), dpr);
                    canvas.set_width(dw);
                    canvas.set_height(dh);
                    if let Some(e) = engine.borrow_mut().as_mut() { let _ = e.resize(rect.width(), rect.height(), dpr); }
                }
            });
            let _ = win.add_event_listener_with_callback("resize", onresize.as_ref().unchecked_ref());

            onwheel.forget();
            onresize.forget();
            // NOTE: engine + listeners leak on route-leave (on_cleanup is Send-bound, can't hold the
            // !Send engine). Proper drop = a StoredValue::new_local handle. The loop stopping is what
            // matters (no runaway render).
            on_cleanup(move || { disposed.store(true, std::sync::atomic::Ordering::Relaxed); });
        });
    }

    view! {
        <div node_ref=container_ref class="relative h-screen w-screen overflow-hidden bg-background">
            <canvas node_ref=canvas_ref class="absolute inset-0 block h-full w-full"></canvas>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if disposed.load(std::sync::atomic::Ordering::Relaxed) { f.borrow_mut().take(); return; }
        if let Some(e) = engine.borrow_mut().as_mut() { let _ = e.render(); }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}
```
