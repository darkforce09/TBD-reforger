//! Browser listeners and engine lifecycle for the building viewer.

use super::*;

/// Boots the render engine on the bench canvas and wires the whole live surface to it: the
/// blueprint, sidecar and compound fetches, the floor rail, the draggable observer and target,
/// the viewshed wash upload, and the pointer, wheel and keyboard handling.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn wire(
    canvas_ref: NodeRef<leptos::html::Canvas>,
    blueprint: RwSignal<Option<BuildingBlueprint>>,
    sidecar: RwSignal<Option<Arc<BvhSidecar>>>,
    sidecar_err: RwSignal<Option<String>>,
    wash: RwSignal<Option<Arc<LevelWash>>>,
    drawing: RwSignal<Option<Arc<BuildingDrawing>>>,
    load_err: RwSignal<Option<String>>,
    engine_err: RwSignal<Option<String>>,
    view_floor: RwSignal<ViewFloor>,
    floors_open: RwSignal<bool>,
    viewshed_on: RwSignal<bool>,
    obs: RwSignal<RayEnd>,
    tgt: RwSignal<RayEnd>,
    los: RwSignal<Option<LosResult>>,
    cam: RwSignal<Cam>,
    css: RwSignal<(f64, f64)>,
    drag: RwSignal<Drag>,
    compound: RwSignal<Option<CompoundBuilding>>,
    compound_err: RwSignal<Option<String>>,
    cuts: RwSignal<Option<Arc<Vec<LevelCuts>>>>,
) {
    let engine: EngineHandle = Rc::new(RefCell::new(None));
    let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    // URL flags select reproducible ray ends; `?force=webgl` = the headless capture
    // backend (the editor's convention — software WebGPU wedges headless Chrome).
    if let Some(a) = ray_end_query("a") {
        obs.set(a);
    }
    if let Some(b) = ray_end_query("b") {
        tgt.set(b);
    }
    let force_webgl = query("force").is_some_and(|v| v == "webgl");
    let doors_open = query("doors").is_some_and(|v| matches!(v.as_str(), "open" | "1"));
    let fitted = Rc::new(std::cell::Cell::new(false));
    // Bumped once the engine lands so the upload effects rerun — without it, a blueprint that
    // arrives before `RenderEngine::create` resolves would never get its first upload.
    let engine_ready = RwSignal::new(false);

    // Blueprint + sidecar fetch (once). The sidecar is the JSON path with `.bvh` in place of
    // `.json`; a missing one (the hand-authored Green/plain assets ship none) is not an error
    // for the plan view — LOS just stays off, and the header says why.
    leptos::task::spawn_local(async move {
        let path = prefab_path();
        match gloo_net::http::Request::get(&path).send().await {
            Ok(resp) if resp.ok() => match resp.text().await {
                Ok(body) => match serde_json::from_str::<BuildingBlueprint>(&body) {
                    Ok(bp) => blueprint.set(Some(bp)),
                    Err(e) => load_err.set(Some(format!("{path}: parse failed — {e}"))),
                },
                Err(e) => load_err.set(Some(format!("{path}: read failed — {e}"))),
            },
            Ok(resp) => load_err.set(Some(format!("{path}: HTTP {}", resp.status()))),
            Err(e) => load_err.set(Some(format!("{path}: {e}"))),
        }

        let Some(url) = path.strip_suffix(".json").map(|s| format!("{s}.bvh")) else {
            sidecar_err.set(Some(format!(
                "{path}: not a .json path, no occlusion sidecar — LOS, viewshed and mesh drawing off (blueprint fallback)"
            )));
            return;
        };
        let outcome = match gloo_net::http::Request::get(&url).send().await {
            Ok(resp) if resp.ok() => match resp.binary().await {
                Ok(bytes) => BvhSidecar::parse(&bytes)
                    .map_err(|e| format!("{url}: sidecar parse failed — {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
                Err(e) => Err(format!("{url}: read failed — {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
            },
            Ok(resp) => Err(format!(
                "{url}: HTTP {} — no occlusion sidecar, LOS, viewshed and mesh drawing off (blueprint fallback)",
                resp.status()
            )),
            Err(e) => Err(format!("{url}: {e} — LOS, viewshed and mesh drawing off (blueprint fallback)")),
        };
        match outcome {
            Ok(sc) => {
                let shell = Arc::new(sc);
                sidecar.set(Some(Arc::clone(&shell)));
                // The instances and BLAS closure extend the shell.
                match load_compound(&path, shell, scene_mode()).await {
                    Ok((mut c, warning)) => {
                        if doors_open {
                            let ids: Vec<String> = c.doors().map(|d| d.record.id.clone()).collect();
                            for id in ids {
                                c.set_door(
                                    &id,
                                    website_map_engine::world::architecture::compound::doors::DoorState::OPEN,
                                );
                            }
                        }
                        compound_err.set(warning);
                        compound.set(Some(c));
                    }
                    Err(msg) => compound_err.set(Some(format!(
                        "{msg} — shell-only bench (no doors, glass, furniture)"
                    ))),
                }
            }
            Err(msg) => sidecar_err.set(Some(msg)),
        }
    });

    // Engine mount once the canvas exists.
    Effect::new({
        let engine = engine.clone();
        let disposed = disposed.clone();
        move |_| {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            if engine.borrow().is_some() || disposed.load(std::sync::atomic::Ordering::Relaxed) {
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
            leptos::task::spawn_local(async move {
                match RenderEngine::create(canvas, force_webgl).await {
                    Ok(mut e) => {
                        let _ = e.resize(cw, ch, dpr);
                        // Camera roam box: a generous pad around the anchor-placed building.
                        e.set_camera_bounds(
                            geom::ANCHOR[0] - 200.0,
                            geom::ANCHOR[1] - 200.0,
                            geom::ANCHOR[0] + 200.0,
                            geom::ANCHOR[1] + 200.0,
                        );
                        e.set_view(geom::ANCHOR[0], geom::ANCHOR[1], 4.5);
                        e.hide_calibration();
                        e.disable_frame_timing();
                        e.set_continuous_render(false);
                        let (r, g, b) = (geom::COL_BG[0], geom::COL_BG[1], geom::COL_BG[2]);
                        e.set_clear_color(r, g, b);
                        sync_cam(&e, cam);
                        *engine.borrow_mut() = Some(e);
                        // The shared frame pump, with no per-frame hook: this page has no
                        // HUD and no signal to publish, so render → poll → repeat until
                        // dispose is the whole loop.
                        RafPump::new(engine.clone(), disposed.clone()).start();
                        engine_ready.set(true);
                    }
                    Err(err) => {
                        engine_err.set(Some(format!("engine create failed: {err:?}")));
                    }
                }
            });
        }
    });

    // Static lanes: (blueprint, view floor) → upload; first arrival also fits the camera.
    Effect::new({
        let engine = engine.clone();
        let fitted = fitted.clone();
        move |_| {
            if !engine_ready.get() {
                return;
            }
            let view = view_floor.get();
            let d = drawing.get();
            let cu = cuts.get();
            blueprint.with(|bp| {
                let Some(bp) = bp.as_ref() else { return };
                if let Ok(mut guard) = engine.try_borrow_mut() {
                    if let Some(e) = guard.as_mut() {
                        if !fitted.get() {
                            let (tx, ty, zoom) = geom::fit_camera(bp, css.get_untracked());
                            e.set_view(tx, ty, zoom);
                            sync_cam(e, cam);
                            fitted.set(true);
                        }
                        compound.with(|c| {
                            upload_static(
                                e,
                                bp,
                                d.as_deref(),
                                c.as_ref(),
                                cu.as_deref().map(Vec::as_slice),
                                view,
                            );
                        });
                    }
                }
            });
        }
    });

    // Ray lane: follows the LOS result, clipped to the viewed floor's band.
    Effect::new({
        let engine = engine.clone();
        move |_| {
            if !engine_ready.get() {
                return;
            }
            let Some(r) = los.get() else { return };
            let view = view_floor.get();
            let (o, t) = (obs.get_untracked(), tgt.get_untracked());
            blueprint.with_untracked(|bp| {
                let Some(bp) = bp.as_ref() else { return };
                let (band, band_last) = view.band(bp);
                if let Ok(mut guard) = engine.try_borrow_mut() {
                    if let Some(e) = guard.as_mut() {
                        upload_ray(e, o, t, &r, band, band_last);
                    }
                }
            });
        }
    });

    // Wash lane: mirrors the page's `wash` signal (already the viewed level's disc; the
    // page recomputes it on observer / floor-rail change, `None` clears).
    Effect::new({
        let engine = engine.clone();
        move |_| {
            if !engine_ready.get() {
                return;
            }
            let w = wash.get();
            if let Ok(mut guard) = engine.try_borrow_mut() {
                if let Some(e) = guard.as_mut() {
                    upload_wash(e, w.as_deref());
                }
            }
        }
    });

    // Pointer + wheel listeners on the canvas; move/up on the window (drag escapes the rect).
    Effect::new({
        let engine = engine.clone();
        move |_| {
            let Some(canvas) = canvas_ref.get() else {
                return;
            };
            let canvas: web_sys::HtmlCanvasElement = canvas;

            let down = {
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    // Alt+LMB: teleport observer A here and light its viewshed; the held
                    // drag keeps moving A (fill follows live).
                    if ev.alt_key() && ev.button() == 0 {
                        ev.prevent_default();
                        let c = cam.get_untracked();
                        let size = css.get_untracked();
                        let w = geom::screen_to_world(
                            [f64::from(ev.client_x()), f64::from(ev.client_y())],
                            c.tx,
                            c.ty,
                            c.zoom,
                            size,
                        );
                        let l = geom::from_world(w);
                        obs.update(|e| {
                            e.x = l[0];
                            e.z = l[1];
                        });
                        viewshed_on.set(true);
                        drag.set(Drag::Observer);
                        return;
                    }
                    if drag.get_untracked() == Drag::None {
                        drag.set(Drag::Pan);
                    }
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
                            sync_cam(e, cam);
                        }
                    }
                }) as Box<dyn FnMut(_)>)
            };
            canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
            wheel.forget();

            let Some(win) = web_sys::window() else { return };
            let last = Rc::new(std::cell::Cell::new((0.0f64, 0.0f64)));
            let moved = Rc::new(std::cell::Cell::new(0.0f64));

            let mv = {
                let engine = engine.clone();
                let last = last.clone();
                let moved = moved.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    let (px, py) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                    let (lx, ly) = last.get();
                    let (dx, dy) = (px - lx, py - ly);
                    last.set((px, py));
                    let mode = drag.get_untracked();
                    if mode == Drag::None {
                        return;
                    }
                    moved.set(moved.get() + dx.abs() + dy.abs());
                    match mode {
                        Drag::Pan => {
                            if let Ok(mut guard) = engine.try_borrow_mut() {
                                if let Some(e) = guard.as_mut() {
                                    e.pan(dx, dy);
                                    sync_cam(e, cam);
                                }
                            }
                        }
                        Drag::Observer | Drag::Target => {
                            let c = cam.get_untracked();
                            let size = css.get_untracked();
                            let w = geom::screen_to_world([px, py], c.tx, c.ty, c.zoom, size);
                            let l = geom::from_world(w);
                            let s = if mode == Drag::Observer { obs } else { tgt };
                            s.update(|e| {
                                e.x = l[0];
                                e.z = l[1];
                            });
                        }
                        _ => {}
                    }
                }) as Box<dyn FnMut(_)>)
            };
            let _ =
                win.add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref());
            mv.forget();

            let up = {
                let moved = moved.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    let mode = drag.get_untracked();
                    drag.set(Drag::None);
                    // A pan that never moved = a CLICK: open the floor selector when it
                    // lands inside the building footprint.
                    if mode == Drag::Pan && moved.get() < 4.0 {
                        let c = cam.get_untracked();
                        let size = css.get_untracked();
                        let w = geom::screen_to_world(
                            [f64::from(ev.client_x()), f64::from(ev.client_y())],
                            c.tx,
                            c.ty,
                            c.zoom,
                            size,
                        );
                        let l = geom::from_world(w);
                        // A click on a door leaf (or its closed footprint) swings it;
                        // LOS, wash, cuts and lanes follow through the compound signal.
                        let toggled = blueprint.with_untracked(|bp| {
                            let Some(bp) = bp.as_ref() else { return false };
                            let (band, _) = view_floor.get_untracked().band(bp);
                            let id = compound.with_untracked(|c| {
                                c.as_ref()
                                    .and_then(|c| building_interior::door_at(c, l, band))
                            });
                            match id {
                                Some(id) => {
                                    compound.update(|c| {
                                        if let Some(c) = c.as_mut() {
                                            if let Some(s) = c.door_state(&id) {
                                                c.set_door(&id, s.toggled());
                                            }
                                        }
                                    });
                                    true
                                }
                                None => false,
                            }
                        });
                        if !toggled {
                            blueprint.with_untracked(|bp| {
                                if let Some(bp) = bp.as_ref() {
                                    if geom::point_in_polygon(l, &bp.overall_footprint.polygon2_d) {
                                        floors_open.set(true);
                                    }
                                }
                            });
                        }
                    }
                    moved.set(0.0);
                }) as Box<dyn FnMut(_)>)
            };
            let _ = win.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
            up.forget();

            // Seed `last` on every pointerdown anywhere (markers set their own drag mode
            // before this bubbles).
            let seed = {
                let last = last.clone();
                let moved = moved.clone();
                Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                    last.set((f64::from(ev.client_x()), f64::from(ev.client_y())));
                    moved.set(0.0);
                }) as Box<dyn FnMut(_)>)
            };
            let _ =
                win.add_event_listener_with_callback("pointerdown", seed.as_ref().unchecked_ref());
            seed.forget();
        }
    });

    // Window resize → engine resize + css mirror.
    Effect::new({
        let engine = engine.clone();
        move |_| {
            let Some(_canvas) = canvas_ref.get() else {
                return;
            };
            let Some(win) = web_sys::window() else { return };
            let engine = engine.clone();
            let resize = Closure::wrap(Box::new(move || {
                let Some(canvas) = canvas_ref.get_untracked() else {
                    return;
                };
                let canvas: web_sys::HtmlCanvasElement = canvas;
                let rect = canvas.get_bounding_client_rect();
                let (cw, ch) = (rect.width().max(64.0), rect.height().max(64.0));
                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                css.set((cw, ch));
                if let Ok(mut guard) = engine.try_borrow_mut() {
                    if let Some(e) = guard.as_mut() {
                        let _ = e.resize(cw, ch, dpr);
                        sync_cam(e, cam);
                    }
                }
            }) as Box<dyn FnMut()>);
            let _ = win.add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref());
            resize.forget();
        }
    });

    // Dispose on unmount.
    on_cleanup(move || disposed.store(true, std::sync::atomic::Ordering::Relaxed));
}
