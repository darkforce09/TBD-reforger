//! Mission Creator editor (/missions/:id/edit) — the wgpu boundary collapse (T-159.15).
//!
//! **T-159.15.0 (foundation):** the Leptos app owns `RenderEngine` DIRECTLY as plain Rust — created
//! from a canvas `NodeRef` via `spawn_local`, no `map-engine-wasm` shim, one wasm module / one
//! linear memory. That slice mounted the canvas and rendered a single frame.
//!
//! **T-159.15.1 (this slice):** a damage-driven continuous render loop + wheel-zoom + resize, engine
//! owned directly (D5). Two things unblock the loop on the second `render()` submit (which panicked
//! `wgpu` "Buffer is already mapped" on the 15.0 foundation):
//!   1. `disable_frame_timing()` — drops the `GpuTimer` timestamp-readback lane. Headless is
//!      **WebGPU/Dawn** (not WebGL2 as first assumed), where that lane's `map_async` double-maps its
//!      16-byte buffer on the 2nd submit. The editor has no fps/GPU-time HUD, so the lane is pure
//!      overhead — dropping it removes the offending map. This is the actual fix.
//!   2. `engine.poll()` per frame after `render()` — drains readback `map_async` callbacks for the
//!      WebGL2-fallback path and the cull-counter lane that later world slices add. A no-op on
//!      real-browser WebGPU (the event loop resolves maps).
//! A `window.__selfChecks` bridge exposes the byte-exact GPU readback gate (calibration + texture)
//! the headless driver awaits — under `?force=webgl`, since `self_check`'s polled readback only
//! resolves on WebGL2 headless.
//!
//! The full Eden docked shell (Top Command Strip, Left Outliner, Right Asset Palette, Bottom
//! Toolbelt, doc host) lands across T-159.16–.22. Route is `chromeless` + `full_bleed` (AppLayout
//! hides the platform nav). Verified by GPU readback (not DOM diff) as the map lane grows.
#![allow(dead_code)]
use leptos::prelude::*;

/// Round CSS px → device-pixel backing size (≥1), matching the React oracle's `deviceSize`.
#[cfg(target_arch = "wasm32")]
fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // The engine is created + owned on the wasm target only (wgpu is wasm32-gated). Native builds
    // (cargo check) compile the shell without touching the GPU stack.
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::task::spawn_local;
        use std::cell::Cell;
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        // T-159.17 — read the `:id` route param for the persistence key.
        use leptos_router::hooks::use_params_map;

        const TERRAIN_W: f64 = 12_800.0;
        const TERRAIN_H: f64 = 12_800.0;
        const INITIAL_TARGET: (f64, f64) = (6_400.0, 6_400.0);
        const INITIAL_ZOOM: f64 = -2.0;
        const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;

        // T-159.17 — mission id from the `:id` route param (`/missions/:id/edit`; `smoke` on the gate
        // route). One-shot untracked read at mount (id is static per route mount). Fallback `draft`
        // mirrors the React `missionId ?? 'draft'` persistence key.
        let mission_id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string());

        canvas_ref.on_load(move |canvas: web_sys::HtmlCanvasElement| {
            let Some(container) = container_ref.get_untracked() else {
                return;
            };
            let container: web_sys::HtmlDivElement = container;
            let win = web_sys::window().expect("window");

            // Backend override for the headless readback gate: `?force=webgl` → WebGL2/SwiftShader,
            // where the byte-exact self_check readback resolves via `device.poll` (on webgpu/Dawn
            // headless the offscreen map never fires). Default (no query) = prefer WebGPU, matching
            // prod. Mirrors the React `WgpuCanvas` spike's `?force=webgl`.
            let force_webgl = win
                .location()
                .search()
                .map(|s| s.contains("force=webgl"))
                .unwrap_or(false);

            // Size the backing store BEFORE create (the engine reads canvas.width/height).
            let dpr0 = win.device_pixel_ratio();
            let rect0 = container.get_bounding_client_rect();
            let (dw, dh) = device_size(rect0.width(), rect0.height(), dpr0);
            canvas.set_width(dw);
            canvas.set_height(dh);

            let engine: Rc<RefCell<Option<map_engine_render::RenderEngine>>> =
                Rc::new(RefCell::new(None));
            let disposed = Arc::new(AtomicBool::new(false));

            // T-159.16 — MissionDoc host. Built + seeded + bridged synchronously (before the async
            // engine create), so the `window.__missionDoc` Class R gate does not depend on the wgpu
            // engine coming up. The doc leaks on route-leave like the engine (`!Send` `Rc`, and
            // `on_cleanup` is `Send`-bound) — no double-free (plain Rust `Drop`). The optional
            // doc→engine bind (D5) happens below once the engine is `Some`.
            let doc = crate::mission_doc::new_seeded_doc();
            let doc_ver = Rc::new(Cell::new(1u32));
            crate::mission_doc::register_mission_doc(doc.clone(), doc_ver);

            // T-159.18 — LMB select foundation. Selection is app-side state (NOT the Y.Doc — it never
            // lived in the document, matching React's Zustand), held in the editor's leaked-handle
            // idiom so the `window.__editorSelection` smoke bridge (peer of __missionDoc) never reads
            // reactive-owner state a route change could dispose. `left` carries the in-flight LMB
            // gesture (T-159.19 `LeftGesture`: Pending → Move | Marquee — a frozen ortho camera copied
            // at the press drives every unproject) between pointerdown/move/up. Registered
            // synchronously (engine still `None` here — `probe()` reads it lazily; `pick_selfcheck()`
            // needs only the synchronously-seeded doc).
            let selection: crate::select_tool::SelectionHandle = Rc::new(RefCell::new(Vec::new()));
            let left: Rc<RefCell<Option<crate::select_tool::LeftGesture>>> =
                Rc::new(RefCell::new(None));
            crate::select_tool::register_editor_selection(
                selection.clone(),
                doc.clone(),
                engine.clone(),
                container.clone(),
            );

            // T-159.17 — persistence layer (additive; the SYNCHRONOUS seed above keeps the doc smoke
            // synchronous — `smoke_doc_editor` still sees 8 slots immediately on its own cold origin).
            // The `window.__missionPersist` bridge is installed synchronously (so the gate can wait on
            // it); the IDB load / initial-persist / warm-mark run async below and flip `ready` last.
            let persist_ready = Rc::new(Cell::new(false));
            let persist_loaded = Rc::new(Cell::new(false));
            crate::yrs_persist::register_mission_persist(
                doc.clone(),
                mission_id.clone(),
                persist_ready.clone(),
                persist_loaded.clone(),
            );
            spawn_local({
                let doc = doc.clone();
                let id = mission_id.clone();
                let ready = persist_ready.clone();
                let loaded = persist_loaded.clone();
                async move {
                    // 1. Restore from IDB if a blob exists — SWAP a fresh core (mirrors React's
                    //    empty-shell + apply; rests on the tested fresh-peer path + persist_roundtrip_ok,
                    //    NOT on reapply-idempotence). The swap is a synchronous block: no `borrow`/
                    //    `borrow_mut` is ever held across an `.await` (the engine task shares this `Rc`).
                    if let Some(blob) = crate::yrs_persist::load_state(&id).await {
                        if !blob.is_empty() {
                            let fresh = map_engine_core::doc::MissionDocCore::new();
                            fresh.set_origin_init(true);
                            let ok = fresh.apply_update(&blob).is_ok();
                            fresh.set_origin_init(false);
                            if ok {
                                *doc.borrow_mut() = Some(fresh);
                                loaded.set(true);
                            }
                        }
                    }
                    // 2. Initial persist through the debounced writer (get_bytes read at write time;
                    //    cancel when the doc Option is cleared). No mutator hook exists yet, so this
                    //    post-seed/post-load encode is the writer's trigger this slice.
                    {
                        let doc_get = doc.clone();
                        let doc_cancel = doc.clone();
                        crate::yrs_persist::save_state_debounced(
                            &id,
                            Box::new(move || {
                                doc_get
                                    .borrow()
                                    .as_ref()
                                    .map(|c| c.encode_state())
                                    .unwrap_or_default()
                            }),
                            Box::new(move || doc_cancel.borrow().is_none()),
                            crate::yrs_persist::debounce_ms(),
                        );
                    }
                    // 3. Warm-session marker after the doc is ready.
                    let n = doc
                        .borrow()
                        .as_ref()
                        .map(|c| c.slot_count() as u32)
                        .unwrap_or(0);
                    crate::editor_session::mark_ready(&id, n, None);
                    // 4. Flush-on-hide listeners (visibilitychange/hidden + pagehide).
                    crate::yrs_persist::register_flush_on_hide(id.clone());
                    // 5. Ready LAST — the gate waits on this before asserting.
                    ready.set(true);
                }
            });

            spawn_local({
                let engine = engine.clone();
                let disposed = disposed.clone();
                let doc = doc.clone();
                let canvas = canvas.clone();
                let (cw, ch) = (rect0.width(), rect0.height());
                async move {
                    match map_engine_render::RenderEngine::create(canvas, force_webgl).await {
                        Ok(mut eng) => {
                            if disposed.load(Ordering::Relaxed) {
                                return;
                            }
                            let _ = eng.resize(cw, ch, dpr0);
                            eng.set_camera_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
                            eng.set_view(INITIAL_TARGET.0, INITIAL_TARGET.1, INITIAL_ZOOM);
                            eng.hide_calibration();
                            // Drop the GpuTimer readback lane: no fps HUD in the editor yet, and on
                            // headless WebGPU its map_async double-maps the 16-byte buffer on the 2nd
                            // submit ("Buffer is already mapped"). `poll()` (below, per frame) keeps
                            // the WebGL2-fallback + future cull-counter readback honest.
                            eng.disable_frame_timing();
                            eng.set_continuous_render(false); // damage-driven, matches the prod oracle
                            *engine.borrow_mut() = Some(eng);
                            register_self_checks(engine.clone());
                            register_editor_cam(engine.clone());
                            // T-159.16 — optional doc→engine bind (D5). `slots_bind_soa` early-returns
                            // while the slot atlas is unuploaded (the editor uploads none yet), so this
                            // is a safe cache write that proves the SoA wire compiles + runs; the tiny
                            // seeded slot set renders nothing until a later slice uploads the atlas.
                            let soa = doc.borrow().as_ref().map(|c| c.materialize());
                            if let (Some(soa), Some(e)) =
                                (soa.as_ref(), engine.borrow_mut().as_mut())
                            {
                                e.slots_bind_soa(soa.ids.clone(), &soa.xy);
                            }
                            start_raf(engine.clone(), disposed.clone());
                        }
                        Err(e) => leptos::logging::error!("RenderEngine::create: {e:?}"),
                    }
                }
            });

            // T-159.15.2 — pan gesture state: `Some((last_client_x, last_client_y))` while an
            // MMB/RMB drag-pan is in flight, else `None`. The pan feeds INCREMENTAL client-px deltas
            // to `engine.pan` (the camera does `target -= dΧ/scale` at the LIVE scale — Rust owns the
            // ortho math; this mirrors the `WgpuCanvas` oracle, NOT the Deck frozen-viewport path
            // that `useSelectTool` uses and the language gate forbids here). `(f64, f64)` is `Copy`,
            // so a `Cell` suffices (no `RefCell`); JS is single-threaded, so these pointer handlers
            // never reenter the rAF loop's `borrow_mut`.
            let pan_px: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));

            // Wheel → zoom_at (engine self-clamps zoom to [-6, 6]). Capture + non-passive so we can
            // preventDefault and beat any child handler. CSS origin = the container rect (same basis
            // as the pan/pick math).
            let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
                let engine = engine.clone();
                let container = container.clone();
                let pan_px = pan_px.clone();
                move |ev: web_sys::WheelEvent| {
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        ev.prevent_default();
                        let rect = container.get_bounding_client_rect();
                        e.zoom_at(
                            -ev.delta_y() * WHEEL_ZOOM_PER_PX,
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        // P5 mid-pan rebase (T-151.11.6): keep an in-flight pan alive across a
                        // mid-pan zoom. Under the single-pointer invariant a `pointermove` precedes
                        // any `wheel`, so `wheel.client == last_px`; this refresh is a provable no-op
                        // that also defensively re-syncs the start px. The next incremental
                        // `engine.pan` then rides the LIVE post-zoom scale, so panning continues
                        // seamlessly with no re-press. (The incremental model has no frozen zoom to
                        // go stale — the Deck bug T-151.11.6 fixed does not exist here.)
                        if pan_px.get().is_some() {
                            pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                        }
                    }
                }
            });
            let wheel_opts = web_sys::AddEventListenerOptions::new();
            wheel_opts.set_passive(false);
            wheel_opts.set_capture(true);
            let _ = container.add_event_listener_with_callback_and_add_event_listener_options(
                "wheel",
                onwheel.as_ref().unchecked_ref(),
                &wheel_opts,
            );

            // T-159.15.2 — MMB/RMB drag-pan (LMB deferred to the doc host / .16: no marquee / slot
            // move yet). Pointer capture keeps deltas flowing if the drag leaves the div; the
            // contextmenu is suppressed so an RMB-drag isn't interrupted by the browser menu (P3).
            // All five closures leak like the wheel/resize ones above (the engine leaks too;
            // `on_cleanup` only stops the loop — a `!Send` drop handle is later polish).
            let onpointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                move |ev: web_sys::PointerEvent| {
                    // Middle (1) or right (2) button starts a pan.
                    if ev.button() == 1 || ev.button() == 2 {
                        ev.prevent_default();
                        let _ = container.set_pointer_capture(ev.pointer_id());
                        pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                    } else if ev.button() == 0 {
                        // T-159.18/.19 — LMB pending-left: freeze the ortho camera at press (X-05: the
                        // live engine unproject is deleted; a live unproject would feedback-loop
                        // mid-pan). No pointer capture yet — a sub-threshold release is a click; the
                        // first past-threshold `pointermove` (T-159.19) promotes to Move|Marquee and
                        // captures then. `engine.borrow()` is safe: JS is single-threaded, so this never
                        // reenters the rAF loop's `borrow_mut`.
                        if let Some(e) = engine.borrow().as_ref() {
                            let rect = container.get_bounding_client_rect();
                            let cam = crate::select_tool::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            );
                            *left.borrow_mut() = Some(crate::select_tool::LeftGesture::Pending(
                                crate::select_tool::PendingLeft {
                                    start_x: ev.client_x() as f64 - rect.left(),
                                    start_y: ev.client_y() as f64 - rect.top(),
                                    cam,
                                },
                            ));
                        }
                    }
                }
            });
            let onpointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let container = container.clone();
                move |ev: web_sys::PointerEvent| {
                    if let Some((lx, ly)) = pan_px.get() {
                        let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.pan(cx - lx, cy - ly);
                        }
                        pan_px.set(Some((cx, cy)));
                        return;
                    }
                    // T-159.19 — LMB drag gesture. Own the gesture across the update (take → compute →
                    // put back) so a Pending→Move/Marquee transition never aliases a `&mut`, and so no
                    // `left` borrow is held across the inner `left.borrow_mut()` put-back (the `if let`
                    // temporary-lifetime footgun). Frozen cam (M2/X-05 — no live unproject). Live preview
                    // via `engine.set_drag` (drag) / `engine.upload_marquee` (marquee rect).
                    let taken = left.borrow_mut().take();
                    let Some(g0) = taken else { return };
                    use crate::select_tool::{self as st, LeftGesture as LG};
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    // Promote a Pending press once it clears the threshold; else keep the active drag.
                    let active = match g0 {
                        LG::Pending(p) => {
                            let moved = ((px - p.start_x).powi(2) + (py - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                *left.borrow_mut() = Some(LG::Pending(p));
                                return;
                            }
                            // Real drag now: capture so it survives leaving the canvas (React :200).
                            let _ = container.set_pointer_capture(ev.pointer_id());
                            let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                            let hit = doc.borrow().as_ref().and_then(|c| {
                                st::pick(&p.cam, &c.materialize(), p.start_x, p.start_y)
                            });
                            match hit {
                                Some(id) => {
                                    // Drag an already-selected slot → move the whole selection; else
                                    // replace the selection with the dragged slot (React :204).
                                    let cur = selection.borrow().clone();
                                    let ids = st::compute_move_ids(&id, &cur);
                                    if !cur.iter().any(|s| *s == id) {
                                        *selection.borrow_mut() = ids.clone();
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            e.set_selection(ids.clone());
                                        }
                                    }
                                    LG::Move {
                                        ids,
                                        start_wx: sw[0],
                                        start_wy: sw[1],
                                        cam: p.cam,
                                        dx: 0.0,
                                        dy: 0.0,
                                    }
                                }
                                None => LG::Marquee {
                                    start_x: p.start_x,
                                    start_y: p.start_y,
                                    start_wx: sw[0],
                                    start_wy: sw[1],
                                    cam: p.cam,
                                },
                            }
                        }
                        other => other,
                    };
                    // Drive the live preview for the (possibly just-promoted) state, coalescing the
                    // world delta / marquee rect into `active` for the pointerup commit.
                    let next = match active {
                        LG::Move {
                            ids,
                            start_wx,
                            start_wy,
                            cam,
                            ..
                        } => {
                            let (dx, dy) = st::drag_delta(&cam, start_wx, start_wy, px, py);
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(ids.clone(), dx as f32, dy as f32);
                            }
                            LG::Move {
                                ids,
                                start_wx,
                                start_wy,
                                cam,
                                dx,
                                dy,
                            }
                        }
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            let end = cam.unproject_xy(px, py);
                            if end[0].is_finite() && end[1].is_finite() {
                                let (min_x, max_x) = (start_wx.min(end[0]), start_wx.max(end[0]));
                                let (min_y, max_y) = (start_wy.min(end[1]), start_wy.max(end[1]));
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.upload_marquee(min_x, min_y, max_x, max_y, true);
                                }
                            }
                            LG::Marquee {
                                start_x,
                                start_y,
                                start_wx,
                                start_wy,
                                cam,
                            }
                        }
                        LG::Pending(p) => LG::Pending(p),
                    };
                    *left.borrow_mut() = Some(next);
                }
            });
            let onpointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let mission_id = mission_id.clone();
                move |ev: web_sys::PointerEvent| {
                    // Pan end (MMB/RMB).
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    // LMB gesture end. `take()` into a `let` first so the RefMut drops before the
                    // per-branch re-borrows below (the `if let` temporary-lifetime footgun). If a pan
                    // just ended, `left` is None ⇒ this returns.
                    let taken = left.borrow_mut().take();
                    let Some(g) = taken else { return };
                    use crate::select_tool::{self as st, LeftGesture as LG};
                    let rect = container.get_bounding_client_rect();
                    let up_x = ev.client_x() as f64 - rect.left();
                    let up_y = ev.client_y() as f64 - rect.top();
                    match g {
                        // T-159.18/.53 — sub-threshold press = a click: pick against the FROZEN press
                        // camera (X-05) and toggle/replace/clear the selection.
                        LG::Pending(p) => {
                            let moved =
                                ((up_x - p.start_x).powi(2) + (up_y - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                let additive = ev.ctrl_key() || ev.meta_key();
                                let hit = doc.borrow().as_ref().and_then(|c| {
                                    st::pick(&p.cam, &c.materialize(), p.start_x, p.start_y)
                                });
                                {
                                    let mut sel = selection.borrow_mut();
                                    st::apply_click(&mut sel, hit, additive);
                                }
                                let ids = selection.borrow().clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.set_selection(ids); // tint lane (no-op until an atlas uploads)
                                }
                            }
                        }
                        // T-159.19 M4/M5 — drag-move commit. Release capture; if it actually moved,
                        // commit ONE `move_entities` txn (one undo step), re-bind the moved glyphs, keep
                        // the moved slots selected, and schedule the first edit-driven persist.
                        LG::Move { ids, dx, dy, .. } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if dx != 0.0 || dy != 0.0 {
                                // `move_entities(&self)` (mut txn) then `materialize()` (read txn) are
                                // sequential — the mut txn drops before the read txn opens. `zs = 0`
                                // is the DEM-not-ready byte-parity case (React `terrainZ` on flat map).
                                let soa = {
                                    let guard = doc.borrow();
                                    let Some(core) = guard.as_ref() else {
                                        return;
                                    };
                                    core.move_entities(ids.clone(), dx, dy, vec![0.0; ids.len()]);
                                    core.materialize()
                                };
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.set_drag(Vec::new(), 0.0, 0.0); // clear the drag overlay
                                    e.slots_bind_soa(soa.ids.clone(), &soa.xy);
                                    e.set_selection(ids.clone());
                                }
                                crate::yrs_persist::schedule_edit_persist(doc.clone(), &mission_id);
                            } else if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(Vec::new(), 0.0, 0.0); // no move → just clear the overlay
                            }
                        }
                        // T-159.19 M3 — marquee commit. Release capture; a ≥1×1 px box replaces the
                        // selection with the enclosed slots (`pick_rect` over the frozen-cam world AABB);
                        // hide the rect.
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if (up_x - start_x).abs() >= 1.0 && (up_y - start_y).abs() >= 1.0 {
                                let ids = doc
                                    .borrow()
                                    .as_ref()
                                    .map(|c| {
                                        st::marquee_ids(
                                            &cam,
                                            &c.materialize(),
                                            start_wx,
                                            start_wy,
                                            up_x,
                                            up_y,
                                        )
                                    })
                                    .unwrap_or_default();
                                *selection.borrow_mut() = ids.clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.set_selection(ids);
                                }
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false); // hide
                            }
                        }
                    }
                }
            });
            let oncontextmenu = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(
                move |ev: web_sys::MouseEvent| ev.prevent_default(),
            );
            // T-159.18/.19 — pointercancel ends BOTH a pan and any LMB gesture, but (unlike pointerup)
            // is NOT a commit: it drops the gesture without picking / moving / selecting, and clears any
            // live preview (drag overlay / marquee rect) + releases capture.
            let onpointercancel = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let left = left.clone();
                let engine = engine.clone();
                move |ev: web_sys::PointerEvent| {
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    use crate::select_tool::LeftGesture as LG;
                    let taken = left.borrow_mut().take();
                    match taken {
                        Some(LG::Move { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(Vec::new(), 0.0, 0.0);
                            }
                        }
                        Some(LG::Marquee { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                            }
                        }
                        _ => {}
                    }
                }
            });
            let _ = container.add_event_listener_with_callback(
                "pointerdown",
                onpointerdown.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointermove",
                onpointermove.as_ref().unchecked_ref(),
            );
            let _ = container
                .add_event_listener_with_callback("pointerup", onpointerup.as_ref().unchecked_ref());
            // pointercancel ends the pan + a pending LMB without a click (T-159.18).
            let _ = container.add_event_listener_with_callback(
                "pointercancel",
                onpointercancel.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "contextmenu",
                oncontextmenu.as_ref().unchecked_ref(),
            );

            let onresize = Closure::<dyn FnMut()>::new({
                let engine = engine.clone();
                let canvas = canvas.clone();
                let container = container.clone();
                move || {
                    let dpr = web_sys::window()
                        .map(|w| w.device_pixel_ratio())
                        .unwrap_or(1.0);
                    let rect = container.get_bounding_client_rect();
                    let (dw, dh) = device_size(rect.width(), rect.height(), dpr);
                    canvas.set_width(dw);
                    canvas.set_height(dh);
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        let _ = e.resize(rect.width(), rect.height(), dpr);
                    }
                }
            });
            let _ =
                win.add_event_listener_with_callback("resize", onresize.as_ref().unchecked_ref());

            // The engine + these listeners intentionally leak on route-leave: `on_cleanup` is
            // `Send`-bound and can't hold the `!Send` engine, so we only move `disposed` (Send) into
            // it. Stopping the loop is what prevents a runaway render; a proper `!Send` drop handle
            // is a later polish.
            onwheel.forget();
            onresize.forget();
            onpointerdown.forget();
            onpointermove.forget();
            onpointerup.forget();
            onpointercancel.forget();
            oncontextmenu.forget();
            on_cleanup(move || disposed.store(true, Ordering::Relaxed));
        });
    }

    view! {
        <div
            node_ref=container_ref
            class="relative h-screen w-screen overflow-hidden bg-background"
        >
            <canvas node_ref=canvas_ref class="absolute inset-0 block h-full w-full"></canvas>
        </div>
    }
}

/// The rAF render loop. Each frame renders then polls the device (see `RenderEngine::poll`) so
/// readback `map_async` callbacks drain on the WebGL2-fallback + cull-counter path. (The timer
/// double-map that panicked the 15.0 loop is handled upstream by `disable_frame_timing`.) Stops
/// (and drops itself) once `disposed` is set.
#[cfg(target_arch = "wasm32")]
fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::Ordering;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if disposed.load(Ordering::Relaxed) {
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        if let Some(e) = engine.borrow_mut().as_mut() {
            let _ = e.render();
            e.poll(); // ★ T-159.15.1: drain readback map_async so the next submit can't double-map
        }
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

/// Expose the byte-exact GPU readback self-checks on `window.__selfChecks` — the map-lane gate the
/// headless driver awaits (see [[wgpu-headless-gpu-verify]]). Both checks are scene-independent
/// (`self_check` renders its own fixed calibration probe scene; `texture_self_check` a synthetic
/// 2×2 texture) and `&self`: they clone their GPU handles up front, so the shared `borrow()` here is
/// released before the async readback runs — no contention with the rAF loop's `borrow_mut` (JS is
/// single-threaded). Each resolves to a JSON string with a `pass` field.
#[cfg(target_arch = "wasm32")]
fn register_self_checks(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let obj = js_sys::Object::new();

    let calibration = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };
    let texture = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.texture_self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };

    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("calibration"), calibration.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("texture"), texture.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__selfChecks"), &obj);
    }
    // The harness reads these across the page lifetime; leak them (the engine leaks too).
    calibration.forget();
    texture.forget();
}

/// Expose the camera view-state on `window.__editorCam()` for the headless pan smoke (T-159.15.2 /
/// spec P6): a JSON string `{"tx","ty","z","backend"}` read from the `&self` getters `target_x()` /
/// `target_y()` / `zoom()` / `backend()`. (`#[wasm_bindgen(getter)]` fns are plain method calls from
/// Rust.) All are `&self` behind a shared `borrow()`, released before return — no contention with the
/// rAF loop's `borrow_mut` (JS is single-threaded). Registered once the engine is `Some`; the closure
/// leaks like the self-checks. The smoke drives pan via getter deltas (never `unproject_xy`, X-05).
#[cfg(target_arch = "wasm32")]
fn register_editor_cam(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let cam = Closure::wrap(Box::new(move || -> JsValue {
        engine
            .borrow()
            .as_ref()
            .map(|e| {
                JsValue::from_str(&format!(
                    r#"{{"tx":{},"ty":{},"z":{},"backend":"{}"}}"#,
                    e.target_x(),
                    e.target_y(),
                    e.zoom(),
                    e.backend()
                ))
            })
            .unwrap_or_else(|| JsValue::from_str("null"))
    }) as Box<dyn FnMut() -> JsValue>);

    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCam"), cam.as_ref());
    }
    cam.forget();
}
