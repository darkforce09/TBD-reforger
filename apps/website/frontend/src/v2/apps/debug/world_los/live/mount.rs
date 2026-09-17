//! World line-of-sight bench mount and browser input wiring.

use super::*;

/// Reads the run out of the URL, boots the render engine on the bench canvas, streams the
/// catalogue window in, and wires pointer and wheel handling to the probe and the camera.
pub fn mount(s: Signals) {
    let center = [
        query_f64("x").unwrap_or(DEFAULT_CENTER[0]),
        query_f64("y").unwrap_or(DEFAULT_CENTER[1]),
    ];
    let radius = query_f64("r")
        .unwrap_or(DEFAULT_RADIUS_M)
        .clamp(20.0, 600.0);
    let eye_m = query_f64("eye").unwrap_or(DEFAULT_EYE_M);
    let force_webgl = query("force").as_deref() == Some("webgl");
    let engine: EngineHandle = Rc::new(RefCell::new(None));
    let bench: Rc<RefCell<Option<Bench>>> = Rc::new(RefCell::new(None));
    let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let css = RwSignal::new((0.0f64, 0.0f64));
    let cam = RwSignal::new((center[0], center[1], 2.0f64));
    let a = RwSignal::new(query_point("a"));
    let b = RwSignal::new(query_point("b"));
    let next_is_a = RwSignal::new(true);
    let engine_ready = RwSignal::new(false);
    let loaded = RwSignal::new(false);
    let s = Rc::new(s);

    let sync_cam = move |e: &RenderEngine| cam.set((e.target_x(), e.target_y(), e.zoom()));

    // Engine mount once the canvas exists.
    Effect::new({
        let engine = engine.clone();
        let disposed = disposed.clone();
        let s = s.clone();
        move |_| {
            let Some(canvas) = s.canvas_ref.get() else {
                return;
            };
            if engine.borrow().is_some() {
                return;
            }
            let canvas: web_sys::HtmlCanvasElement = canvas;
            let rect = canvas.get_bounding_client_rect();
            let (cw, ch) = (rect.width().max(64.0), rect.height().max(64.0));
            let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                canvas.set_width(((cw * dpr + 0.5).floor().max(1.0)) as u32);
                canvas.set_height(((ch * dpr + 0.5).floor().max(1.0)) as u32);
            }
            css.set((cw, ch));
            let engine = engine.clone();
            let disposed = disposed.clone();
            let s = s.clone();
            leptos::task::spawn_local(async move {
                match RenderEngine::create(canvas, force_webgl).await {
                    Ok(mut e) => {
                        let _ = e.resize(cw, ch, dpr);
                        e.set_camera_bounds(
                            center[0] - 4.0 * radius,
                            center[1] - 4.0 * radius,
                            center[0] + 4.0 * radius,
                            center[1] + 4.0 * radius,
                        );
                        let zoom = (cw.min(ch) / (2.4 * radius))
                            .log2()
                            .min(website_map_engine::camera::ortho::state::MAX_ZOOM);
                        e.set_view(center[0], center[1], zoom);
                        e.hide_calibration();
                        e.disable_frame_timing();
                        e.set_continuous_render(false);
                        e.set_clear_color(0.043, 0.055, 0.075);
                        sync_cam(&e);
                        *engine.borrow_mut() = Some(e);
                        // The shared frame pump, with no per-frame hook: this page's stats
                        // are written from the LOS solve, not from the frame, so render →
                        // poll → repeat until dispose is the whole loop.
                        RafPump::new(engine.clone(), disposed.clone()).start();
                        engine_ready.set(true);
                    }
                    Err(err) => s
                        .engine_err
                        .set(Some(format!("engine create failed: {err:?}"))),
                }
            });
        }
    });

    // The world: chunks + descriptors + BLAS, then the static lanes and the default ray.
    {
        let bench = bench.clone();
        let s = s.clone();
        leptos::task::spawn_local(async move {
            match load(center, radius, eye_m, s.status).await {
                Ok(b0) => {
                    let eye_y = b0.ground_y + b0.eye_m;
                    if a.get_untracked().is_none() {
                        a.set(Some([center[0] - 40.0, eye_y, center[1]]));
                    }
                    if b.get_untracked().is_none() {
                        b.set(Some([center[0] + 40.0, eye_y, center[1]]));
                    }
                    *bench.borrow_mut() = Some(b0);
                    loaded.set(true);
                }
                Err(e) => s.status.set(format!("load failed: {e}")),
            }
        });
    }

    // Static lanes once both the engine and the world are up.
    Effect::new({
        let engine = engine.clone();
        let bench = bench.clone();
        let s = s.clone();
        move |_| {
            if !engine_ready.get() || !loaded.get() {
                return;
            }
            let t0 = js_sys::Date::now();
            let guard = bench.borrow();
            let Some(b0) = guard.as_ref() else {
                return;
            };
            let eye_y = b0.ground_y + b0.eye_m;
            let (fps, cuts, cut_buildings) =
                scene_of(b0.host.occluder(), b0.center, b0.radius, eye_y);
            let lanes = build_bench_lanes(&fps, &cuts);
            let proxies = fps.iter().filter(|f| f.proxy).count();
            if let Ok(mut g) = engine.try_borrow_mut() {
                if let Some(e) = g.as_mut() {
                    upload_lanes(e, &lanes);
                }
            }
            s.stats.set(format!(
                "{} footprints ({} proxies) · {} buildings cut at y {:.1} m ({} segments) · lanes built in {:.0} ms · occluder {} chunks / {} expanded / {} BLAS / {:.1} MB",
                fps.len(),
                proxies,
                cut_buildings,
                eye_y,
                cuts.len(),
                js_sys::Date::now() - t0,
                b0.host.occluder().chunk_count(),
                b0.host.occluder().expanded_count(),
                b0.host.occluder().blas_count(),
                b0.host.occluder().memory_bytes() as f64 / 1_048_576.0
            ));
        }
    });

    // The ray: re-probed whenever A or B moves.
    Effect::new({
        let engine = engine.clone();
        let bench = bench.clone();
        let s = s.clone();
        move |_| {
            let (Some(pa), Some(pb)) = (a.get(), b.get()) else {
                return;
            };
            if !engine_ready.get() || !loaded.get() {
                return;
            }
            if let Some(b0) = bench.borrow().as_ref() {
                probe(&engine, b0, pa, pb, &s);
            }
        }
    });

    // Pointer: drag pans, a click (no movement) places A then B; wheel zooms.
    Effect::new({
        let engine = engine.clone();
        let bench = bench.clone();
        let s = s.clone();
        move |_| {
            let Some(canvas) = s.canvas_ref.get() else {
                return;
            };
            let canvas: web_sys::HtmlCanvasElement = canvas;
            let dragging = Rc::new(std::cell::Cell::new(false));
            let moved = Rc::new(std::cell::Cell::new(0.0f64));
            let last = Rc::new(std::cell::Cell::new((0.0f64, 0.0f64)));
            let down = {
                let dragging = dragging.clone();
                let moved = moved.clone();
                let last = last.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    if ev.button() != 0 {
                        return;
                    }
                    dragging.set(true);
                    moved.set(0.0);
                    last.set((f64::from(ev.client_x()), f64::from(ev.client_y())));
                }) as Box<dyn FnMut(_)>)
            };
            canvas.set_onpointerdown(Some(down.as_ref().unchecked_ref()));
            down.forget();
            let wheel = {
                let engine = engine.clone();
                Closure::wrap(Box::new(move |ev: web_sys::WheelEvent| {
                    ev.prevent_default();
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            e.zoom_at(
                                -ev.delta_y() * 0.0015,
                                ev.offset_x().into(),
                                ev.offset_y().into(),
                            );
                            sync_cam(e);
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
            wheel.forget();
            let Some(win) = web_sys::window() else { return };
            let mv = {
                let engine = engine.clone();
                let dragging = dragging.clone();
                let moved = moved.clone();
                let last = last.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    if !dragging.get() {
                        return;
                    }
                    let (px, py) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                    let (lx, ly) = last.get();
                    let (dx, dy) = (px - lx, py - ly);
                    last.set((px, py));
                    moved.set(moved.get() + dx.abs() + dy.abs());
                    if let Ok(mut guard) = engine.try_borrow_mut() {
                        if let Some(e) = guard.as_mut() {
                            e.pan(dx, dy);
                            sync_cam(e);
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };
            let _ =
                win.add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref());
            mv.forget();
            let up = {
                let dragging = dragging.clone();
                let moved = moved.clone();
                let bench = bench.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    if !dragging.get() {
                        return;
                    }
                    dragging.set(false);
                    if moved.get() > 3.0 {
                        return;
                    }
                    let (tx, ty, zoom) = cam.get_untracked();
                    let w = screen_to_world(
                        [f64::from(ev.client_x()), f64::from(ev.client_y())],
                        tx,
                        ty,
                        zoom,
                        css.get_untracked(),
                    );
                    let eye_y = bench
                        .borrow()
                        .as_ref()
                        .map_or(0.0, |b0| b0.ground_y + b0.eye_m);
                    let p = [w[0], eye_y, w[1]];
                    if next_is_a.get_untracked() {
                        a.set(Some(p));
                    } else {
                        b.set(Some(p));
                    }
                    next_is_a.update(|v| *v = !*v);
                }) as Box<dyn FnMut(_)>)
            };
            let _ = win.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
            up.forget();
        }
    });
}
