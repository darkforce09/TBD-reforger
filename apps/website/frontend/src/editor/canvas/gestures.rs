//! T-934.13 — the Mission Creator canvas GESTURE closures, moved verbatim out of
//! `MissionEditorPage`'s `on_load` block (`mission_editor.rs`). Six closures live here — wheel
//! zoom, pointerdown/move/up (pan + the LMB Pending→Move/Marquee/Ruler/Rotate machine + the armed
//! place), contextmenu and dblclick — bundled behind [`EditorGestureContext`], the struct that
//! carries every `!Send` handle and `Copy` signal they capture. The page builds the context once
//! its handles exist and calls [`attach_canvas_gestures`]; the closure BODIES are byte-identical
//! to their pre-move text (the Class-S pins that used to grep `mission_editor.rs` for them now
//! grep this file), and the capture preambles clone from locals mirroring the page's, so the
//! capture semantics are unchanged.
//!
//! Deliberately NOT here: `onkeydown` (T-934.14's lane), `onpointercancel` / `onpointerleave` /
//! `onresize` (they stay page-side with the boot tasks — the plan's six-closure scope), and the
//! view template.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Everything below comes through `mission_editor`'s re-export hub (the T-934.10/.11 surface), not
// straight from `render_sync`/`overlays`: the closures moved but the page file stays the one
// place those `pub(crate) use` lists are declared, and importing through them keeps the wasm half
// of that surface load-bearing (an unused re-export there would otherwise start to warn).
use crate::editor::mission_editor::{
    armed_place, comment_drag_lane_xy, comment_lane_ids, comment_lane_xy, comment_points,
    dragged_comment_points, hover_due, hover_hit, hover_next, hover_suppressed,
    live_connection_segments, map_render_slot_soa, pick_comment, pick_connection,
    read_widget_pivot, set_map_cursor, transform, HoverPoints, HoverState, COMMENT_PICK_PX,
    CONN_PICK_PX,
};
use crate::editor::state::history as mission_history;
use crate::editor::state::operations as editor_ops;

/// Every handle the six gesture closures capture, bundled so the page hands them over in one
/// `attach_canvas_gestures(&ctx)` call. `Rc`/element handles clone (shared ownership with the
/// page, which keeps using the same cells for its remaining closures — keydown, pointercancel,
/// pointerleave, the boot tasks); `RwSignal`s are `Copy`.
#[derive(Clone)]
pub(crate) struct EditorGestureContext {
    /// The gesture container div (the element every closure measures + captures pointers on).
    pub(crate) container: web_sys::HtmlDivElement,
    /// The map canvas — the hover cursor writes its CSS `cursor` claim here (T-802).
    pub(crate) canvas: web_sys::HtmlCanvasElement,
    pub(crate) engine: crate::editor::tools::select_tool::EngineHandle,
    pub(crate) doc: crate::editor::state::doc_host::DocHandle,
    pub(crate) selection: crate::editor::tools::select_tool::SelectionHandle,
    /// The in-flight LMB gesture (T-159.19 `LeftGesture`: Pending → Move | Marquee | Ruler | Rotate).
    pub(crate) left: Rc<RefCell<Option<crate::editor::tools::select_tool::LeftGesture>>>,
    /// `Some((last_client_x, last_client_y))` while an MMB drag-pan is in flight (T-159.15.2).
    pub(crate) pan_px: Rc<Cell<Option<(f64, f64)>>>,
    pub(crate) map_host: crate::editor::world_assets::HostHandle,
    pub(crate) dem_grid: crate::editor::world_assets::DemGridHandle,
    /// T-642 — the persistent ruler polyline (session-local overlay state, NOT the Y.Doc).
    pub(crate) ruler: Rc<RefCell<crate::editor::tools::ruler_tool::RulerChain>>,
    /// T-643 — the LoS two-click capture (peer of the ruler chain).
    pub(crate) los: Rc<RefCell<crate::editor::tools::los_tool::LosState>>,
    /// T-644 — the viewshed observer + raster (the GPU wash lane's session state).
    pub(crate) viewshed: Rc<RefCell<crate::editor::tools::los_tool::ViewshedState>>,
    /// T-802 — hover throttle clock + pickable claim + hysteresis anchor (`Copy`, so a `Cell`).
    pub(crate) hover_state: Rc<Cell<HoverState>>,
    /// T-802 — the hover pick's point sets, cached per `doc_tick`.
    pub(crate) hover_points: Rc<RefCell<Option<HoverPoints>>>,
    /// T-159.21 — the CUR read-out world point (fed by the pointer-move unproject).
    pub(crate) cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// T-642/T-643 — the active editor tool (Select ⇆ Ruler ⇆ LoS).
    pub(crate) tool_mode: RwSignal<crate::editor::tools::ruler_tool::EditorTool>,
    /// T-644 — the LoS sub-mode (Ray ⇆ Viewshed).
    pub(crate) los_mode: RwSignal<crate::editor::tools::los_tool::LosMode>,
    /// T-648 — the snap-grid state (the rotate commit reads the effective rotation rung).
    pub(crate) snap: RwSignal<transform::SnapState>,
    /// T-648/T-795 — the transform-widget variant (the ring hit-test gates on Rotate).
    pub(crate) widget_variant: RwSignal<transform::WidgetVariant>,
    /// T-780 — the connection edge selected on the map, if any.
    pub(crate) selected_connection: RwSignal<Option<String>>,
    /// The doc-change tick `editor_ops::refresh_docks` bumps (keys the hover point cache).
    pub(crate) doc_tick: RwSignal<u64>,
    /// T-642 — the ruler's status-bar readout (`sync_ruler` writes it).
    pub(crate) ruler_status: RwSignal<Option<String>>,
    /// T-642 — ruler repaint tick (`sync_ruler` bumps it).
    pub(crate) ruler_tick: RwSignal<u64>,
    /// T-643 — LoS repaint tick (`sync_los` bumps it).
    pub(crate) los_tick: RwSignal<u64>,
}

/// The page's `sync_ruler` rebuilt from the context: push the chain's current summary onto the
/// reactive surface (status bar + repaint tick). Byte-for-byte the body the page's own copy runs
/// (its Effect + Esc arm keep that copy), reading the SAME `Rc` + signals, so the two can never
/// disagree about what a sync does.
fn make_sync_ruler(ctx: &EditorGestureContext) -> impl Fn() + Clone {
    let ruler = ctx.ruler.clone();
    let ruler_status = ctx.ruler_status;
    let ruler_tick = ctx.ruler_tick;
    move || {
        ruler_status.set(ruler.borrow().status_readout());
        ruler_tick.update(|t| *t = t.wrapping_add(1));
    }
}

/// The page's `sync_los`, same contract as [`make_sync_ruler`]: bump the LoS repaint tick.
/// `Copy` (the page's own copy is too — it captures one `Copy` signal), which is what lets the
/// pointerup preamble's `let sync_los = sync_los;` keep its original by-value form.
fn make_sync_los(ctx: &EditorGestureContext) -> impl Fn() + Copy {
    let los_tick = ctx.los_tick;
    move || {
        los_tick.update(|t| *t = t.wrapping_add(1));
    }
}

const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;
/// T-159.22 — matches the chrome host div in the view below (and thus every panel inside
/// it), for the wheel guard's `closest()`. A `data-` attribute, not a class: the class list
/// is a styling contract that a Tailwind edit could silently change under the guard.
const CHROME_SEL: &str = "[data-eden-chrome]";

/// Attach the six gesture closures (wheel / pointerdown / pointermove / pointerup / contextmenu
/// / dblclick) to the context's container. The local `let` belt below mirrors the page's `on_load`
/// environment name-for-name, so every closure's capture preamble — and its whole body — is
/// byte-identical to the pre-move `mission_editor.rs` text.
pub(crate) fn attach_canvas_gestures(ctx: &EditorGestureContext) {
    let container = ctx.container.clone();
    let canvas = ctx.canvas.clone();
    let engine = ctx.engine.clone();
    let doc = ctx.doc.clone();
    let selection = ctx.selection.clone();
    let left = ctx.left.clone();
    let pan_px = ctx.pan_px.clone();
    let map_host = ctx.map_host.clone();
    let dem_grid = ctx.dem_grid.clone();
    let ruler = ctx.ruler.clone();
    let los = ctx.los.clone();
    let viewshed = ctx.viewshed.clone();
    let hover_state = ctx.hover_state.clone();
    let hover_points = ctx.hover_points.clone();
    let cursor = ctx.cursor;
    let tool_mode = ctx.tool_mode;
    let los_mode = ctx.los_mode;
    let snap = ctx.snap;
    let widget_variant = ctx.widget_variant;
    let selected_connection = ctx.selected_connection;
    let doc_tick = ctx.doc_tick;
    let sync_ruler = make_sync_ruler(ctx);
    let sync_los = make_sync_los(ctx);

    // Wheel → zoom_at (engine self-clamps zoom to [-6, 6]). Capture + non-passive so we can
    // preventDefault and beat any child handler. CSS origin = the container rect (same basis
    // as the pan/pick math).
    let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
        let engine = engine.clone();
        let container = container.clone();
        let pan_px = pan_px.clone();
        let map_host = map_host.clone();
        move |ev: web_sys::WheelEvent| {
            // T-159.22 — the wheel is capture-phase on the CONTAINER, so it fires before any
            // dock could stop it (that is deliberate: it is what lets `prevent_default` beat
            // a child, and the panels are descendants). The chrome therefore can't opt out
            // by listener order — this handler has to look at the target and decline.
            // Returning BEFORE `prevent_default` is the whole point: it leaves the event
            // native, so a dock's `overflow-y-auto` scrolls instead of the map zooming
            // (T-159.21 deferred item #1). A wheel over the free canvas is untouched.
            if ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                .is_some_and(|el| el.closest(CHROME_SEL).ok().flatten().is_some())
            {
                return;
            }
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
                // T-172 H5 — keep the slot ring px→m sizing + cluster gate in step with
                // the camera (never called before; stale once the atlas exists).
                e.on_camera_changed();
                crate::editor::world_assets::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
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

    // T-159.15.2 — MMB drag-pan (LMB deferred to the doc host / .16: no marquee / slot
    // move yet). T-662 narrowed this to the middle button only; RMB is no longer a pan, so
    // the browser context menu is only suppressed (never blanket-eaten) by `oncontextmenu`
    // below, leaving RMB reachable for T-664. Pointer capture keeps deltas flowing if the
    // drag leaves the div. All five closures leak like the wheel/resize ones above (the
    // engine leaks too; `on_cleanup` only stops the loop — a `!Send` drop handle is later
    // polish).
    let onpointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
        let pan_px = pan_px.clone();
        let container = container.clone();
        let engine = engine.clone();
        let left = left.clone();
        move |ev: web_sys::PointerEvent| {
            // T-662 — ONLY the middle button (1) pans. RMB (2) used to pan here too, which
            // ate the right-click before any handler downstream could see it; the button is
            // now free for T-664's context menu (and the six tickets behind it). MMB-pan is
            // unchanged.
            if ev.button() == 1 {
                ev.prevent_default();
                let _ = container.set_pointer_capture(ev.pointer_id());
                pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                // T-176 B2 — mark the pan active so a settle fired mid-drag (incl. by a
                // simultaneous wheel-zoom) defers the heavy zoom-band recompute (DEM
                // contours + 8 m forest mass) until the gesture ends.
                crate::editor::world_assets::set_camera_gesture(true);
            } else if ev.button() == 0 {
                // T-723 — while a place is armed, do NOT open LG::Pending / LG::Ruler.
                // A canvas press under the arm used to latch left; the armed pointerup then
                // returned without take(), stranding Pending (phantom Move) or Ruler/LoS
                // (phantom vertex / observer). `armed_place::open_left_gesture_while_armed`
                // is the one decision; the armed pointerup still take()s as belt-and-braces.
                if editor_ops::has_pending() {
                    return;
                }
                // T-159.18/.19 — LMB pending-left: freeze the ortho camera at press (X-05: the
                // live engine unproject is deleted; a live unproject would feedback-loop
                // mid-pan). No pointer capture yet — a sub-threshold release is a click; the
                // first past-threshold `pointermove` (T-159.19) promotes to Move|Marquee and
                // captures then. `engine.borrow()` is safe: JS is single-threaded, so this never
                // reenters the rAF loop's `borrow_mut`.
                if let Some(e) = engine.borrow().as_ref() {
                    let rect = container.get_bounding_client_rect();
                    let cam = crate::editor::tools::select_tool::frozen_camera(
                        rect.width(),
                        rect.height(),
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                    );
                    let sx = ev.client_x() as f64 - rect.left();
                    let sy = ev.client_y() as f64 - rect.top();
                    // T-642 — TOOL-MODE ARBITRATION (the third mode's entry point). With the
                    // Ruler tool active, an LMB press opens `LG::Ruler` INSTEAD of
                    // `LG::Pending`, so the gesture never enters the Select machine's
                    // pick/marquee/move path and never reaches those doc commits. Constraint
                    // (c) button-0 is enforced by `should_begin_ruler` (this arm is already
                    // button 0, so it always passes here); the constraint matters for the
                    // predicate's other callers. `should_begin_ruler` is false under Select,
                    // so the existing Pending path is byte-for-byte unchanged there.
                    *left.borrow_mut() = Some(
                        if crate::editor::tools::ruler_tool::should_begin_ruler(
                            tool_mode.get_untracked(),
                            ev.button(),
                        ) {
                            crate::editor::tools::select_tool::LeftGesture::Ruler {
                                start_x: sx,
                                start_y: sy,
                                cam,
                            }
                        } else {
                            crate::editor::tools::select_tool::LeftGesture::Pending(
                                crate::editor::tools::select_tool::PendingLeft {
                                    start_x: sx,
                                    start_y: sy,
                                    cam,
                                },
                            )
                        },
                    );
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
        let dem_grid = dem_grid.clone();
        let map_host = map_host.clone();
        // T-802 — the hover cursor's own state: the throttle clock + hysteresis anchor, and
        // the per-`doc_tick` point-set cache. Cloned in like every other handle here; the
        // `pointerleave` closure below clones the state cell (and the canvas) so leaving
        // the map resets the claim. The point CACHE is not reset there on purpose — it is
        // keyed on `doc_tick`, so it is still valid when the pointer comes back.
        let hover_state = hover_state.clone();
        let hover_points = hover_points.clone();
        let canvas = canvas.clone();
        move |ev: web_sys::PointerEvent| {
            use crate::editor::tools::select_tool::{self as st, LeftGesture as LG};
            let rect = container.get_bounding_client_rect();
            let (px, py) = (
                ev.client_x() as f64 - rect.left(),
                ev.client_y() as f64 - rect.top(),
            );
            // T-159.21 — CUR read-out. FIRST: both the pan branch and the no-gesture case
            // below return early, and the cursor must keep tracking through both. Unprojects
            // against the same `frozen_camera` the pick uses, so CUR always names the world
            // point a click would hit. The borrow is scoped — the pan branch takes
            // `borrow_mut` two lines down, and an overlapping borrow would panic.
            // Un-throttled by design: React rAF-throttles because its cursor write
            // re-rendered the page, whereas this feeds two text nodes through Leptos's
            // fine-grained bindings. NaN (singular matrix) reads as off-map.
            //
            // T-802 — the camera is now HELD rather than discarded after the unproject, so
            // the hover hit-test below reads the SAME frozen camera this read-out and the
            // pick already use. One construction per move, not two: keeping it is strictly
            // less work than building a second one, and — the wave-201 lesson — a third
            // copy of the transform would be its own defect class.
            let hover_cam = {
                let g = engine.borrow();
                g.as_ref().map(|e| {
                    st::frozen_camera(
                        rect.width(),
                        rect.height(),
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                    )
                })
            };
            let world = hover_cam.as_ref().map(|c| c.unproject_xy(px, py));
            cursor.set(
                world
                    .filter(|c| c[0].is_finite() && c[1].is_finite())
                    .map(|c| {
                        // T-172 B2 — DEM-fed Z beside X/Y; None (em-dash) until the grid
                        // publishes or when the point is outside DEM coverage.
                        let z = dem_grid.borrow().as_ref().and_then(|g| {
                            map_engine_core::dem::downsample::sample_grid_meters(g, c[0], c[1])
                        });
                        (c[0], c[1], z)
                    }),
            );
            // T-723 — MMB pan must work WHILE a place is armed (wave-106 MAJOR-3). The
            // prior order returned at `has_pending` before the pan branch, so an MMB drag
            // never moved the camera and the armed pointerup then stole the place. Pan
            // first whenever `pan_px` is latched; the place ghost yields for the gesture.
            if let Some((lx, ly)) = pan_px.get() {
                let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                if let Some(e) = engine.borrow_mut().as_mut() {
                    e.pan(cx - lx, cy - ly);
                    e.on_camera_changed();
                }
                pan_px.set(Some((cx, cy)));
                crate::editor::world_assets::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
                return;
            }
            // T-175 B2 — palette place ghost: while an asset is being dragged from the
            // palette (`begin_place` armed `pending`), show a live translucent slot ring at
            // the cursor's world point so the operator sees where it will land (the drop
            // commits at pointerup). Mutually exclusive with a map drag/marquee (`left` is
            // None during a palette place), so this returns before the gesture machine.
            if editor_ops::has_pending() {
                if let Some(c) = world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.set_place_preview(c[0] as f32, c[1] as f32);
                    }
                }
                return;
            }

            // T-802 (O-8) — THE HOVER CURSOR. Strictly READ-ONLY with respect to every
            // gesture on this handler: it borrows `left` immutably to ask one question
            // ("is a gesture in flight?"), drops that borrow on the same line, and owns
            // nothing but its own `hover_state` cell + `hover_points` cache. It sits HERE
            // — after the pan and armed-place early returns, before the gesture machine
            // takes `left` — because those two returns are themselves suppression cases and
            // this is the last point at which "no gesture is running" is still knowable.
            //
            // Cost per pointer move when the throttle is closed (the common case): one
            // `Date::now`, one `Cell` read, one shared `RefCell` borrow. When it is open:
            // one radius query over the cached SoA. Never a document read unless `doc_tick`
            // moved. See the T-802 block above for why each of those matters (T-057).
            {
                let now_ms = js_sys::Date::now();
                let gesture_active = left.borrow().is_some();
                let prev = hover_state.get();
                if hover_suppressed(
                    gesture_active,
                    editor_ops::has_pending(),
                    tool_mode.get_untracked().captures_points(),
                ) {
                    // Drop the claim rather than freeze it: a drag that began on a glyph
                    // must not leave the cursor saying "pickable" over open ground, and the
                    // reset means the first move after release re-tests immediately.
                    if prev.pickable {
                        set_map_cursor(&canvas, false);
                    }
                    hover_state.set(HoverState::default());
                } else if hover_due(prev, now_ms) {
                    let hit = hover_cam.as_ref().is_some_and(|cam| {
                        hover_hit(
                            &mut hover_points.borrow_mut(),
                            doc_tick.get_untracked(),
                            &doc,
                            cam,
                            px,
                            py,
                        )
                    });
                    let next = hover_next(prev, hit, px, py, now_ms);
                    // Write ONLY on a transition — the churn guard. A per-tick write would
                    // be 25 style mutations a second for a value that did not change.
                    if next.pickable != prev.pickable {
                        set_map_cursor(&canvas, next.pickable);
                    }
                    hover_state.set(next);
                }
            }

            // T-159.19 — LMB drag gesture. Own the gesture across the update (take → compute →
            // put back) so a Pending→Move/Marquee transition never aliases a `&mut`, and so no
            // `left` borrow is held across the inner `left.borrow_mut()` put-back (the `if let`
            // temporary-lifetime footgun). Frozen cam (M2/X-05 — no live unproject). Live preview
            // via `engine.set_drag` (drag) / `engine.upload_marquee` (marquee rect).
            let taken = left.borrow_mut().take();
            let Some(g0) = taken else { return };
            // Promote a Pending press once it clears the threshold; else keep the active drag.
            let active = match g0 {
                LG::Pending(p) => {
                    let moved = ((px - p.start_x).powi(2) + (py - p.start_y).powi(2)).sqrt();
                    if moved < st::DRAG_THRESHOLD_PX {
                        *left.borrow_mut() = Some(LG::Pending(p));
                        return;
                    }
                    // T-723 — button-less pointermove must NOT promote a stranded Pending
                    // into Move (wave-106 MAJOR-2). `buttons == 0` drops the gesture.
                    if !st::may_promote_pending(ev.buttons()) {
                        return;
                    }
                    // Real drag now: capture so it survives leaving the canvas (React :200).
                    let _ = container.set_pointer_capture(ev.pointer_id());
                    let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                    // T-795 WIDGET-ROTATE-RING — a drag STARTING on the rotate ring rotates the
                    // selection about its centre, WITHOUT Shift and REGARDLESS of what (if
                    // anything) is under the press. This hit-test runs BEFORE the pick/marquee
                    // arm below so a drag on the ring can never fall through to `None =>
                    // LG::Marquee` and destroy the selection (the F-16 defect: the ring was
                    // pure decoration; dragging its edge started a marquee). Gated on Rotate
                    // mode + a live selection (the ring is only drawn then) and on the press
                    // pixel sitting in the ring band around the PROJECTED pivot — the same
                    // `WIDGET_RADIUS_PX` the overlay draws, so the drawn ring IS the draggable
                    // ring. Commits through the identical `LG::Rotate` arm as Shift+drag: one
                    // txn on release, one undo step, live selection re-read at release.
                    let on_ring = widget_variant.get_untracked().is_rotate()
                        && !selection.borrow().is_empty()
                        && read_widget_pivot()
                            .map(|(wx, wy)| p.cam.project([wx, wy, 0.0]))
                            .filter(|pv| pv[0].is_finite() && pv[1].is_finite())
                            .is_some_and(|pv| {
                                transform::press_on_ring(p.start_x, p.start_y, pv[0], pv[1])
                            });
                    if on_ring {
                        LG::Rotate {
                            start_x: p.start_x,
                            start_y: p.start_y,
                            cam: p.cam,
                        }
                    } else {
                        let hit = doc.borrow().as_ref().and_then(|c| {
                            st::pick_slot_or_vehicle(
                                &p.cam,
                                &map_render_slot_soa(c),
                                &editor_ops::vehicle_points(),
                                p.start_x,
                                p.start_y,
                            )
                        });
                        // T-796 — pick the COMMENT GLYPH so a drag STARTING on a note grabs
                        // it, the same fold the click path (T-784) does. Precedence: the
                        // rotate ring (T-795) short-circuited above, then slot/vehicle wins
                        // its own pixels here, then a comment, then `None => LG::Marquee`. A
                        // note parked on a unit can never steal the unit's drag, and a drag
                        // on a note can never fall through to a marquee that destroys the
                        // selection — the F-16 failure class the ring pin already names,
                        // applied to the note. Against the FROZEN press camera, tolerance
                        // derived by unprojecting two points `COMMENT_PICK_PX` apart, exactly
                        // as the click-path comment pick does — so the drag grabs a note over
                        // the identical hit box the click selects it with.
                        let hit = hit.or_else(|| {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            doc.borrow().as_ref().and_then(|c| {
                                pick_comment(&comment_points(&c.comments_json()), w[0], w[1], tol)
                            })
                        });
                        match hit {
                            // T-648 XFORM-SHIFT-001 — SHIFT + drag grabbing an ALREADY-SELECTED
                            // entity rotates the whole selection to face the cursor instead of
                            // moving it. Shift is free in this drag path (T-053 left it unbound;
                            // the T-073 cancel note confirms it), so this steals no existing
                            // gesture. Gated on the grabbed entity being in the CURRENT selection
                            // so a Shift+drag on empty ground or an unselected entity still falls
                            // through to the normal pick/marquee below (a rotate needs something
                            // to rotate). No pointer preview: the render engine's `set_drag` is a
                            // TRANSLATION lane only, so — like the ruler — the rotate shows its
                            // result on release; the widget ring (mounted in the view) is the
                            // live affordance. `LG::Rotate` carries no ids: the commit re-reads
                            // the live selection at release.
                            Some(ref id)
                                if ev.shift_key() && selection.borrow().iter().any(|s| s == id) =>
                            {
                                LG::Rotate {
                                    start_x: p.start_x,
                                    start_y: p.start_y,
                                    cam: p.cam,
                                }
                            }
                            Some(id) => {
                                // Drag an already-selected slot → move the whole selection; else
                                // replace the selection with the dragged slot (React :204).
                                let cur = selection.borrow().clone();
                                let ids = st::compute_move_ids(&id, &cur);
                                if !cur.iter().any(|s| *s == id) {
                                    *selection.borrow_mut() = ids.clone();
                                    if let Some(e) = engine.borrow_mut().as_mut() {
                                        // Slot tint only — vehicle glyphs have no selection lane.
                                        let slot_ids: Vec<String> = ids
                                            .iter()
                                            .filter(|i| !editor_ops::is_vehicle_id(i))
                                            .cloned()
                                            .collect();
                                        e.set_selection(slot_ids);
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
                    } // end `else` — non-ring drag (T-795 WIDGET-ROTATE-RING short-circuits above)
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
                        // T-573 — preview the WHOLE selection. The T-425 pre-filter fed
                        // `set_drag` slot ids only and nothing previewed the vehicles, so a
                        // mixed drag drew the slots moving and the vehicles standing while
                        // the pointerup commit moved both: an overlay lying about its drop.
                        st::push_drag_preview(e, &ids, &editor_ops::vehicle_points(), dx, dy);
                        // T-796 — the COMMENT half of the preview. A comment is not in the
                        // slot overlay lane (`set_drag`) nor the vehicle re-pack, so
                        // `push_drag_preview` cannot move it; its lane is re-bound here so a
                        // dragged note's glyph FOLLOWS THE CURSOR (the O-7 preview-parity
                        // note). `comment_drag_lane_xy` re-packs EVERY note — the ones not in
                        // the drag stay drawn at rest — with the dragged ids offset by the
                        // live delta. On a mixed drag with no comment in `ids` this re-binds
                        // the lane to its authored positions (all offsets zero), which is a
                        // harmless identity re-upload, so there is no branch to keep in step.
                        //
                        // T-808 — the ids ride along: the lane's selection treatment is
                        // per-row and the engine looks it up BY ID, so a preview bound
                        // without them would strip the ring off a dragged note for the
                        // whole gesture and hand it back on release. `comment_lane_ids`
                        // packs the same `comment_points` list this re-pack offsets, so
                        // ids[i] still names the bubble at xy[2i] mid-drag.
                        let lane = doc.borrow().as_ref().map(|c| {
                            let cj = c.comments_json();
                            (
                                comment_drag_lane_xy(&cj, &ids, dx, dy),
                                comment_lane_ids(&cj),
                            )
                        });
                        if let Some((cxy, cids)) = lane {
                            e.comments_bind_ids(&cxy, cids);
                        }
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
                // T-642 — a ruler press does NOT promote: it stays `Ruler` until release,
                // when a sub-threshold pointerup commits ONE vertex. The rubber-band leg to
                // the cursor is drawn by `RulerOverlay` off the live `cursor` signal (already
                // updated at the top of this handler), so there is nothing to preview via the
                // engine here — the arm just carries itself back. No pointer capture, no GPU
                // upload: a ruler never touches the drag/marquee engine lanes.
                LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                } => LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                },
                // T-648 — a Shift-rotate, like the ruler, does NOT preview through the engine
                // (its `set_drag` is translation-only) and does NOT promote: it stays
                // `Rotate` until release. The live affordance is the widget ring in the view;
                // the rotate itself is applied on pointerup from the release cursor. Carry the
                // arm back unchanged.
                LG::Rotate {
                    start_x,
                    start_y,
                    cam,
                } => LG::Rotate {
                    start_x,
                    start_y,
                    cam,
                },
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
        let map_host = map_host.clone();
        // T-642 — the ruler chain + its reactive sync + the DEM grid (per-vertex Z sample).
        let ruler = ruler.clone();
        let dem_grid = dem_grid.clone();
        let sync_ruler = sync_ruler.clone();
        // T-643 — the LoS capture + its reactive sync. The commit arm below routes a captured
        // click to the ruler OR the LoS state by `tool_mode`, since both tools share the
        // `LG::Ruler` gesture (the "mode field on the ruler arm").
        let los = los.clone();
        let sync_los = sync_los;
        // T-644 — the viewshed state, for the one-shot placement branch of the LoS commit (the
        // `engine` clone above carries the wash upload). `los_mode` is a Copy RwSignal read
        // directly below (`get_untracked`) to route ray vs viewshed.
        let viewshed = viewshed.clone();
        // T-159.21 — no `mission_id` capture: the persist tail now runs inside
        // `mission_history::after_local_edit`, which reads the id from its ctx.
        move |ev: web_sys::PointerEvent| {
            // T-159.22 / T-723 — palette place. FIRST: a place is armed by a palette /
            // picker / composition surface. The ARMED state (`has_pending()`) is checked
            // before any gesture branch below. This branch used to assume `left`/`pan_px`
            // were both None here — that was FALSE (canvas pointerdown while armed still
            // wrote LG::Pending/Ruler; MMB still latched pan_px) and is the root of
            // wave-106 MAJOR-1/2/3. Corrected contract:
            //   * button 0 only places; button 2 disarms; button 1 falls through to pan;
            //   * always `left.take()` so Pending/Ruler cannot strand;
            //   * off-canvas LMB keeps the arm (arming click's own release) — Esc/RMB cancel.
            // The host still stops `pointerdown` only, so a release over a dock bubbles here;
            // the chrome insets decide Place vs KeepArmed.
            if editor_ops::has_pending() {
                // T-723 — clear ANY stranded left gesture before deciding (Pending → phantom
                // Move; Ruler → phantom vertex; LoS capture under the same LG::Ruler arm →
                // phantom observer/target). Belt-and-braces with the pointerdown skip.
                let _ = left.borrow_mut().take();

                let button = ev.button();
                // MMB: do not place — fall through to pan-end cleanup below.
                if button == 1 {
                    // keep armed; pan_px cleanup runs next
                } else if button == 2 {
                    // RMB — Eden stamp-mode cancel
                    editor_ops::cancel_pending();
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.clear_place_preview();
                    }
                    return;
                } else if button != 0 {
                    return;
                } else {
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    // T-638 — the LIVE insets (dock collapse + chrome_hidden folded in).
                    let on_canvas = px >= crate::editor::layout::dock_left_px()
                        && px <= rect.width() - crate::editor::layout::dock_right_px()
                        && py >= crate::editor::layout::strip_top_px()
                        && py <= rect.height() - crate::editor::layout::toolbelt_band_px();
                    let world = if on_canvas {
                        let g = engine.borrow();
                        g.as_ref().map(|e| {
                            crate::editor::tools::select_tool::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            )
                            .unproject_xy(px, py)
                        })
                    } else {
                        None
                    };
                    let world_ok = world.filter(|c| c[0].is_finite() && c[1].is_finite());
                    match armed_place::decide_armed_pointerup(button, world_ok.is_some()) {
                        armed_place::ArmedUp::Place => {
                            // ══════════════════════ T-647 — the Ctrl state machine (arm ↔ Ctrl) ═══════
                            // Ctrl is OVERLOADED across this ticket and its meaning is decided by the
                            // ARMED state, resolved in exactly two places:
                            //   (1) HERE, with a placement ARMED — Ctrl on release = MULTI-PLACE: land
                            //       the entity but KEEP the pending armed so the next click drops another
                            //       (`place_at_keep`). Without Ctrl the arm is one-shot (`place_at`
                            //       take()s it). Eden's Ctrl-stamp behaviour.
                            //   (2) In the LMB drag-commit (pointerup, `LG::Move` below), with NO
                            //       placement armed — Ctrl + drag character→character = REGROUP.
                            // The two can never fire at once: `has_pending()` gates this branch and the
                            // drag branch runs only when it is false. That mutual exclusion is the whole
                            // reason PLACE-004 and CONN-GROUP-001 are one row — see the pin
                            // `t647_ctrl_state_machine`.
                            //
                            // T-647 PLACE-CREW-001 — Alt on release = place an EMPTY vehicle: the
                            // per-gesture override of the DockRight crew toggle (which is the default).
                            // Threaded to `place_at*` as `alt_empty`; for a Vehicle arm it forces
                            // `crewed: false`, for a character/object arm it is inert.
                            let ctrl_multi = ev.ctrl_key() || ev.meta_key();
                            let alt_empty = ev.alt_key();
                            let c = world_ok.expect("Place implies finite world");
                            if ctrl_multi {
                                editor_ops::place_at_keep(c[0], c[1], alt_empty);
                            } else {
                                editor_ops::place_at_alt(c[0], c[1], alt_empty);
                            }
                        }
                        armed_place::ArmedUp::KeepArmed => {
                            // Off-canvas LMB: the arming click's own release (dock /
                            // composition) — do NOT cancel_pending (wave-106 MAJOR-1 /
                            // wave-108 composition tooltip). Esc / RMB disarm.
                        }
                        // FallThroughPan / Disarm / Ignore are handled above by button.
                        armed_place::ArmedUp::FallThroughPan
                        | armed_place::ArmedUp::Disarm
                        | armed_place::ArmedUp::Ignore => {}
                    }
                    // T-175 B2 — drop the ghost after a place attempt or a chrome release.
                    // Ctrl multi-place that KEPT the pending re-shows on the next move.
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.clear_place_preview();
                    }
                    return;
                }
            }
            // Pan end (MMB/RMB).
            if pan_px.get().is_some() {
                pan_px.set(None);
                if container.has_pointer_capture(ev.pointer_id()) {
                    let _ = container.release_pointer_capture(ev.pointer_id());
                }
                // T-176 B2 — pan ended: clear the gesture flag BEFORE scheduling so this
                // settle runs the full zoom-band recompute (contours + forest) once.
                crate::editor::world_assets::set_camera_gesture(false);
                crate::editor::world_assets::schedule_camera_settle(
                    map_host.clone(),
                    engine.clone(),
                );
            }
            // LMB gesture end. `take()` into a `let` first so the RefMut drops before the
            // per-branch re-borrows below (the `if let` temporary-lifetime footgun). If a pan
            // just ended, `left` is None ⇒ this returns.
            let taken = left.borrow_mut().take();
            let Some(g) = taken else { return };
            use crate::editor::tools::select_tool::{self as st, LeftGesture as LG};
            // T-723 — only button 0 commits a left gesture. A phantom Move (stranded
            // Pending promoted after disarm) used to commit on RMB/MMB pointerup and
            // teleport the just-placed entity. Wrong-button releases abandon the gesture.
            if ev.button() != 0 {
                match &g {
                    LG::Move { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            st::clear_drag_preview(e, &editor_ops::vehicle_points());
                            // T-796 — a wrong-button release is never a commit; put a dragged
                            // note's lane back at its authored position too (identity when the
                            // drag held no comment). T-808 — with its ids, or the restore
                            // would drop the selection ring the abandoned drag was carrying.
                            if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                                (
                                    comment_lane_xy(&c.comments_json()),
                                    comment_lane_ids(&c.comments_json()),
                                )
                            }) {
                                e.comments_bind_ids(&cxy, cids);
                            }
                        }
                    }
                    LG::Marquee { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                        }
                    }
                    LG::Rotate { .. } => {
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    _ => {}
                }
                return;
            }
            let rect = container.get_bounding_client_rect();
            let up_x = ev.client_x() as f64 - rect.left();
            let up_y = ev.client_y() as f64 - rect.top();
            match g {
                // T-159.18/.53 — sub-threshold press = a click: pick against the FROZEN press
                // camera (X-05) and toggle/replace/clear the selection.
                LG::Pending(p) => {
                    let moved = ((up_x - p.start_x).powi(2) + (up_y - p.start_y).powi(2)).sqrt();
                    if moved < st::DRAG_THRESHOLD_PX {
                        let additive = ev.ctrl_key() || ev.meta_key();
                        let hit = doc.borrow().as_ref().and_then(|c| {
                            st::pick_slot_or_vehicle(
                                &p.cam,
                                &map_render_slot_soa(c),
                                &editor_ops::vehicle_points(),
                                p.start_x,
                                p.start_y,
                            )
                        });
                        // T-768 — Eden CONN-START-001 final half: when a connect is armed
                        // (RMB ▸ Connect ▸ kind), an LMB pick on a target calls the SAME
                        // `complete_connect` the RMB "Complete Connection" row uses. A miss
                        // keeps the arm (Esc / RMB Cancel / panel Cancel disarm). The arm is
                        // consumed on attempt inside complete_connect — no stranded mode.
                        if editor_ops::pending_connect().is_some() {
                            if let Some(ref id) = hit {
                                let _ = editor_ops::complete_connect(id);
                            }
                        }
                        // ══════════ T-784 — pick the COMMENT GLYPH ══════════════════════
                        //
                        // On an entity MISS only: a slot or vehicle always wins its own
                        // pixels, so a note parked on top of a unit can never steal the
                        // unit's click.
                        //
                        // FOLDED INTO `hit`, not handled beside it. Everything downstream —
                        // `apply_click`'s replace/toggle, the additive Ctrl branch, the SEL
                        // readout, `refresh_selection` — then treats a comment exactly as it
                        // treats a slot, with no new selection route to keep in step. That
                        // is what makes Ctrl+click COMPOSE a comment with entities (the
                        // T-781 capture reads one selection `Vec`) and what makes the map's
                        // edge selection drop: a non-empty entity selection is the condition
                        // `editor_ops::reconcile_connection_selection` already tests inside
                        // `mirror_selection`, so this arm adds no clear of its own.
                        //
                        // DELIBERATELY AFTER the connect arm above: `complete_connect` must
                        // keep seeing entity hits only, or an armed connection would take a
                        // comment as an endpoint — an edge to a thing that never compiles.
                        //
                        // …and DELIBERATELY BEFORE the edge pick below, which is NOT a draw
                        // -order argument (wave 145 F-3: `draw_order.rs` pins
                        // `MissionConnections` ABOVE `MissionComments`, so at an exact
                        // overlap the hairline is the topmost pixel and the note still wins
                        // the click). The reason is the SHAPE of the two targets. A note is
                        // a point with a `COMMENT_PICK_PX` radius — a few pixels, and if a
                        // line crossing it could take the click it would be unclickable at
                        // that spot with no way for the operator to tell why. An edge is a
                        // long segment that stays clickable along its whole length, so it
                        // loses nothing by yielding the handful of pixels around a note.
                        //
                        // Against the FROZEN press camera (X-05), tolerance derived by
                        // unprojecting two points `COMMENT_PICK_PX` apart, exactly as the
                        // connection pick below derives its own. NO affordance is painted on
                        // the strength of this pick and that is on purpose (wave 129): it is
                        // a SELECTION gesture like a slot click, not a route to an
                        // inspector, so there is no "can this be clicked" question here and
                        // therefore no second answer to it.
                        let hit = hit.or_else(|| {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            doc.borrow().as_ref().and_then(|c| {
                                pick_comment(&comment_points(&c.comments_json()), w[0], w[1], tol)
                            })
                        });
                        // ══════ T-780 — pick the CONNECTION line the operator drew ══════
                        //
                        // Runs ONLY on a miss: an entity always wins its own pixels, so an
                        // edge can never steal a click from the slot it ends at. That
                        // ordering is also what keeps the two selections mutually exclusive
                        // — a hit clears the edge, and a plain miss is the only thing that
                        // can set one (`apply_click(None, false)` clears the slot selection
                        // in the very same breath).
                        //
                        // Against the FROZEN press camera `p.cam` (X-05), like the slot pick
                        // above, so a click during an inertial pan tests the geometry the
                        // operator was actually looking at. The tolerance is derived by
                        // unprojecting two points `CONN_PICK_PX` apart rather than reading a
                        // scale off the camera: it stays a constant SCREEN radius at every
                        // zoom without this file learning the projection's internals.
                        //
                        // NO affordance is painted anywhere on the strength of this pick,
                        // and that is on purpose (wave 129). This is a SELECTION gesture,
                        // the same kind a slot click is — not a route to an inspector — so
                        // there is no "can this subject be clicked" question to ask, and
                        // therefore no second answer to it. The tint the picked edge gets is
                        // the CONSEQUENCE of the click, applied after the fact by the lane
                        // Effect, never a promise made before it. Nothing here consults
                        // `route_target` or a hardcoded kind list.
                        if hit.is_some() {
                            selected_connection.set(None);
                        } else if !additive {
                            let w = p.cam.unproject_xy(p.start_x, p.start_y);
                            let w2 = p.cam.unproject_xy(p.start_x + CONN_PICK_PX, p.start_y);
                            let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                            let edge = doc.borrow().as_ref().and_then(|c| {
                                pick_connection(&live_connection_segments(c), w[0], w[1], tol)
                            });
                            selected_connection.set(edge);
                        }
                        {
                            let mut sel = selection.borrow_mut();
                            // T-788 F-27 — a PLAIN click that lands INSIDE the current
                            // multi-selection must NOT collapse it to `[hit]`. Two natural
                            // gestures depend on this: a group drag (the `LG::Move` arm
                            // below moves the whole selection via `compute_move_ids`), and a
                            // double-click to multi-edit (the first click's pointerup used to
                            // re-select the one slot BEFORE `dblclick`/`open_attributes`
                            // fired, so SEL9 became SEL1 and the modal opened single-edit).
                            // Eden keeps the marquee alive on a click within it. A plain
                            // click OUTSIDE the selection still REPLACES (Eden semantics), and
                            // Ctrl/Cmd (`additive`) still toggles — both flow through
                            // `apply_click` untouched, so a lone-slot click is unchanged.
                            let keep_multi = !additive
                                && sel.len() > 1
                                && hit.as_ref().is_some_and(|h| sel.iter().any(|s| s == h));
                            if !keep_multi {
                                st::apply_click(&mut sel, hit, additive);
                            }
                        }
                        let ids = selection.borrow().clone();
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            let slot_ids: Vec<String> = ids
                                .iter()
                                .filter(|i| !editor_ops::is_vehicle_id(i))
                                .cloned()
                                .collect();
                            e.set_selection(slot_ids); // tint lane (slots only)
                        }
                        // T-159.21 — SEL readout only: a click changes the selection, not the
                        // document (no rebind / persist / undo step / tree rebuild).
                        mission_history::refresh_selection();
                    }
                }
                // T-159.19 M4/M5 — drag-move commit. Release capture; if it actually moved,
                // commit ONE `move_entities` txn (one undo step), re-bind the moved glyphs, keep
                // the moved slots selected, and schedule the first edit-driven persist.
                LG::Move {
                    ids, dx, dy, cam, ..
                } => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    // ══════════ T-647 CONN-GROUP-001 (map half) — Ctrl+drag = regroup ══════
                    // The second half of the Ctrl state machine (see the arm ↔ Ctrl block in
                    // the place branch above). This branch runs only with NO placement armed
                    // (`has_pending()` short-circuited the whole pointerup before here), so
                    // Ctrl here can only mean "regroup", never "multi-place". A SINGLE
                    // CHARACTER slot dragged onto ANOTHER character slot moves the dragged
                    // one into the target's squad (`regroup_slot_onto`), and the positional
                    // move is SKIPPED — the drop was a group gesture, not a reposition.
                    // Anything else under Ctrl (a vehicle in the drag, a multi-selection, a
                    // drop onto empty ground or onto a vehicle) falls through to the normal
                    // move, so Ctrl+drag keeps its move meaning everywhere regroup does not
                    // apply. The preview lanes are dropped back either way (regroup commits
                    // no position, so nothing re-binds the glyphs from a move).
                    // T-796 — a comment id can never be a regroup SOURCE: regroup moves a
                    // character slot into another slot's squad (`regroup_slot_onto`), and a
                    // note is in neither the slot SoA nor a squad. Excluding it here (asked
                    // of the document's comment map, the `delete_selection` membership rule —
                    // not a `cmt-` prefix) means a Ctrl+drag of a lone note falls straight
                    // through to the positional move below instead of probing for a phantom
                    // regroup target under the drop.
                    let single_comment_drag = ids.len() == 1
                        && doc.borrow().as_ref().is_some_and(|c| {
                            editor_ops::comment_details(c)
                                .iter()
                                .any(|d| d.id == ids[0])
                        });
                    let regrouped = if (ev.ctrl_key() || ev.meta_key())
                        && ids.len() == 1
                        && !editor_ops::is_vehicle_id(&ids[0])
                        && !single_comment_drag
                    {
                        let target = doc
                            .borrow()
                            .as_ref()
                            .and_then(|c| st::pick(&cam, &map_render_slot_soa(c), up_x, up_y));
                        match target {
                            Some(tid) if tid != ids[0] => {
                                // `regroup_slot_onto` runs the shared dirty tail itself
                                // (via `refile_slot`), so this branch must NOT also call
                                // `after_local_edit` — it only drops the stale drag preview.
                                let ok = editor_ops::regroup_slot_onto(&ids[0], &tid);
                                if ok {
                                    if let Some(e) = engine.borrow_mut().as_mut() {
                                        st::clear_drag_preview(e, &editor_ops::vehicle_points());
                                    }
                                }
                                ok
                            }
                            _ => false,
                        }
                    } else {
                        false
                    };
                    if regrouped {
                        return;
                    }
                    if dx != 0.0 || dy != 0.0 {
                        // T-796 — split the COMMENTS out of the drag FIRST, asked of the
                        // document's own comment map (`comment_details`) exactly as
                        // `delete_selection` asks it — the prefix `cmt-` is a minting
                        // convention, not a document invariant. A note is in neither the slot
                        // SoA nor `vehiclesById`, so left in `ids` it would land in
                        // `slot_ids` and be handed to `move_entities`, which reads the slot
                        // SoA, finds nothing, and moves it NOWHERE — the exact defect O-6
                        // pixel-verified (a 90px drag left the stored position unchanged).
                        let comment_ids: Vec<String> = doc
                            .borrow()
                            .as_ref()
                            .map(|c| {
                                let members: std::collections::HashSet<String> =
                                    editor_ops::comment_details(c)
                                        .into_iter()
                                        .map(|d| d.id)
                                        .collect();
                                ids.iter()
                                    .filter(|id| members.contains(*id))
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                        // T-796 — commit each dragged note to base + delta through
                        // `move_comment`, ONE core transaction each ⇒ one Ctrl+Z. A
                        // SINGLE-comment drag (the O-6 case, and what `compute_move_ids`
                        // produces for a drag of an unselected note) is therefore exactly one
                        // undo step, the ticket's "moving must be ONE step". A drag of
                        // SEVERAL notes is N steps — the SAME accepted per-txn class as the
                        // `delete_selection` comment loop (the one-txn batch would be minted
                        // core-side in `store.rs`, out of this slice), never lost work: undo
                        // restores every moved note. The base x/z is read from the same
                        // `comment_details` list, so the stored position and the previewed
                        // glyph came from ONE read. `z` here is the note's NORTHING (a
                        // comment is `{x, z}`, two horizontals) — `dy` is a plane delta, so
                        // this passes the northing through verbatim and never zeroes an axis.
                        if !comment_ids.is_empty() {
                            let moves: Vec<(String, f64, f64)> = doc
                                .borrow()
                                .as_ref()
                                .map(|c| {
                                    dragged_comment_points(
                                        &comment_points(&c.comments_json()),
                                        &comment_ids,
                                    )
                                    .into_iter()
                                    .map(|p| (p.id, p.x + dx, p.y + dy))
                                    .collect()
                                })
                                .unwrap_or_default();
                            for (id, x, z) in moves {
                                editor_ops::move_comment(id, x, z);
                            }
                        }
                        // T-491 — one LOCAL yrs txn for mixed slot+vehicle drag (T-425 split
                        // `move_entities` then `move_vehicles` needed two Ctrl+Z). The comment
                        // half is already committed above; this partitions the REMAINDER.
                        let (veh_ids, slot_ids): (Vec<String>, Vec<String>) = ids
                            .iter()
                            .filter(|id| !comment_ids.iter().any(|c| c == *id))
                            .cloned()
                            .partition(|id| editor_ops::is_vehicle_id(id));
                        if !slot_ids.is_empty() || !veh_ids.is_empty() {
                            let guard = doc.borrow();
                            let Some(core) = guard.as_ref() else {
                                return;
                            };
                            // wave-127 F-6 — the drag carries each slot's CURRENT z.
                            // `move_entities_in_txn` reads the existing z and DISCARDS it,
                            // writing `zs[i]` verbatim, so the `vec![0.0; n]` that used to
                            // sit here flattened every dragged slot to the deck inside one
                            // txn — while VEHICLES in the same drag kept theirs
                            // (`move_vehicles_in_txn` never touches z). Nothing re-samples
                            // afterwards to hide it: `terrainZ` did not survive the React
                            // deletion, so that `0.0` was the final stored value, not a
                            // placeholder for a DEM lookup. Same defect, and same fix, as
                            // the Attributes tab (F-2) and Align/Distribute (F-5); the z is
                            // resolved through their `keep_z_rows`/`slot_z` pair so there is
                            // one z-resolution vocabulary in the editor, not three.
                            //
                            // ORDER: the core indexes `zs` by each id's position in the
                            // `ids` slice, so `zs[i]` must be `slot_ids[i]`'s z. `zs` is
                            // built by mapping over the very `slot_ids` Vec that is then
                            // passed as `ids` — same length, same order, no re-sort between
                            // the two — so the correspondence is structural, not a
                            // convention two call sites have to agree on.
                            //
                            // `raw_slot_rows` is an O(document) JSON parse, so it is read
                            // ONCE for the whole drag rather than per slot, and not at all
                            // for a vehicle-only drag. `keep_z_rows` is asked with the write
                            // shape a translate always has (x and y written, z absent — the
                            // deltas stand in for the coordinates, since it only asks WHICH
                            // fields are written), so it answers `Some` for every drag.
                            let z_rows = (!slot_ids.is_empty())
                                .then(|| editor_ops::keep_z_rows(core, Some(dx), Some(dy), None))
                                .flatten();
                            let zs: Vec<f64> = slot_ids
                                .iter()
                                .map(|id| {
                                    z_rows
                                        .as_ref()
                                        .and_then(|rows| editor_ops::slot_z(rows, id))
                                        .unwrap_or(0.0)
                                })
                                .collect();
                            core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);
                            drop(guard);
                            mission_history::after_local_edit();
                        }
                    } else if let Some(e) = engine.borrow_mut().as_mut() {
                        // No move ⇒ no commit, so nothing else re-binds: drop BOTH preview
                        // lanes back to the authored positions (T-573 — the vehicle lane is
                        // a live re-pack now, not a passive bind).
                        st::clear_drag_preview(e, &editor_ops::vehicle_points());
                        // T-796 — and the comment lane: a zero-delta release still ran the
                        // preview re-pack above, so re-bind the notes to their authored
                        // positions (no committed move re-binds them here). Identity when the
                        // drag held no note. T-808 — ids ride along (see the preview arm).
                        if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                            (
                                comment_lane_xy(&c.comments_json()),
                                comment_lane_ids(&c.comments_json()),
                            )
                        }) {
                            e.comments_bind_ids(&cxy, cids);
                        }
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
                                st::marquee_ids_with_vehicles(
                                    &cam,
                                    &map_render_slot_soa(c),
                                    &editor_ops::vehicle_points(),
                                    start_wx,
                                    start_wy,
                                    up_x,
                                    up_y,
                                )
                            })
                            .unwrap_or_default();
                        *selection.borrow_mut() = ids.clone();
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            let slot_ids: Vec<String> = ids
                                .iter()
                                .filter(|i| !editor_ops::is_vehicle_id(i))
                                .cloned()
                                .collect();
                            e.set_selection(slot_ids);
                        }
                        // T-159.21 — SEL readout only (selection change, not a doc edit).
                        mission_history::refresh_selection();
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.upload_marquee(0.0, 0.0, 0.0, 0.0, false); // hide
                    }
                }
                // T-642 — RULER vertex commit. This arm is only reached with NO palette place
                // armed (the `has_pending()` branch at the top of pointerup already returned)
                // and no pan in flight — so it deliberately sits OUTSIDE the T-723 armed-place
                // branch (constraint (a)), and because the ruler pointerdown wrote `LG::Ruler`
                // into `left`, this `take()` (constraint (b)) is what clears it. A sub-threshold
                // release is a click → commit ONE point; the tool stays armed for the next.
                // (Past-threshold would be a drag; neither measure tool has a drag gesture, so
                // an accidental micro-drag simply drops without committing.) The point records
                // its DEM elevation at click time (Decision 2) from the SAME grid CUR-Z reads,
                // unprojected against the FROZEN press camera so it lands where CUR pointed.
                //
                // T-643 — BOTH measure tools share this `LG::Ruler` arm (the "mode field on
                // the ruler arm"): a captured click routes by `tool_mode` — a ruler VERTEX
                // (`chain.press`) under Ruler, or a LoS observer/target (`state.click`) under
                // LoS. The unproject + Z-sample + threshold are identical; only the
                // destination differs, so the two tools can never disagree about where a click
                // landed. Neither destination is a doc write (Decision 4 for both).
                LG::Ruler {
                    start_x,
                    start_y,
                    cam,
                } => {
                    let moved = ((up_x - start_x).powi(2) + (up_y - start_y).powi(2)).sqrt();
                    if moved < st::DRAG_THRESHOLD_PX {
                        let w = cam.unproject_xy(start_x, start_y);
                        if w[0].is_finite() && w[1].is_finite() {
                            let z = dem_grid.borrow().as_ref().and_then(|g| {
                                map_engine_core::dem::downsample::sample_grid_meters(g, w[0], w[1])
                            });
                            if tool_mode.get_untracked().is_los() {
                                if los_mode.get_untracked().is_viewshed() {
                                    // T-644 VIEWSHED sub-mode — a SINGLE click places the
                                    // observer and shades the whole disc (one-shot, not a
                                    // drag: this shares the ray's sub-threshold click arm, so
                                    // the T-723 button-0/no-armed-place/take discipline is
                                    // already met). Follows `place_viewshed`'s documented
                                    // host-wiring example: store the observer + click-time Z in
                                    // the session state, then `place_viewshed` (compute the
                                    // raster + stash it for pan re-projection) and upload the
                                    // returned texture to the engine's viewshed lane. Session-
                                    // local overlay state + a GPU wash — never a doc write
                                    // (Decision 4). NO-ENGINE GUARD (mirrors the ray's engine
                                    // guard / Boot-Failed): `place_viewshed` returns `None`
                                    // when no DEM sampler is registered, and the upload only
                                    // runs when the engine is live — a dead map draws nothing.
                                    viewshed.borrow_mut().place(w[0], w[1], z);
                                    if let Some(tex) =
                                        crate::editor::tools::los_tool::place_viewshed(w[0], w[1])
                                    {
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            let _ = e.viewshed_upload(
                                                tex.min_x,
                                                tex.min_y,
                                                tex.max_x,
                                                tex.max_y,
                                                tex.tex_w,
                                                tex.tex_h,
                                                &tex.rgba,
                                                tex.stride_bytes,
                                            );
                                        }
                                    }
                                } else {
                                    // LoS RAY: first click sets the observer, second completes
                                    // the shot (Decision 2's two-click capture). Session-local
                                    // overlay state, never a doc write (Decision 4).
                                    los.borrow_mut().click(w[0], w[1], z);
                                    sync_los();
                                }
                            } else {
                                ruler.borrow_mut().press(w[0], w[1], z);
                                sync_ruler();
                            }
                        }
                    }
                }
                // T-648 XFORM-SHIFT-001 — SHIFT-ROTATE commit. Reached only with NO palette
                // place armed (the `has_pending()` branch at the top of pointerup already
                // returned — the T-723 discipline: this arm sits OUTSIDE that branch) and no
                // pan in flight, and because the promotion wrote `LG::Rotate` into `left`,
                // this `take()` above is what clears it (nothing is left armed). Release the
                // capture the promotion grabbed, then rotate the LIVE selection to face the
                // release cursor (unprojected against the frozen press `cam`), quantised to
                // the effective rotation rung. One history/persist tail via
                // `rotate_selection_to_face`. A drop with no finite aim (cursor off-map, or
                // on the pivot) is a silent no-op inside the commit.
                LG::Rotate { cam, .. } => {
                    if container.has_pointer_capture(ev.pointer_id()) {
                        let _ = container.release_pointer_capture(ev.pointer_id());
                    }
                    let aim = cam.unproject_xy(up_x, up_y);
                    if aim[0].is_finite() && aim[1].is_finite() {
                        let rung = snap.get_untracked().effective_rotate_rung();
                        let acted = editor_ops::rotate_selection_to_face(aim[0], aim[1], rung);
                        if acted {
                            // A rotate changes the doc but not the selection; keep the tint
                            // lane in sync (glyphs re-bind off the history tail) and refresh
                            // the SEL readout, mirroring the Move commit's bookkeeping.
                            mission_history::refresh_selection();
                        }
                    }
                }
            }
        }
    });
    // T-662 → T-664 — RMB no longer pans (see onpointerdown), so the browser menu is only
    // *suppressed* here, never propagation-eaten. `prevent_default` stops the BROWSER's
    // native menu (still the first thing this does, and all it did under T-662); it does NOT
    // `stop_propagation` — that is the invariant the T-662 pin protects, and it holds: this
    // handler attaches to the SAME `contextmenu` event and, having stopped the native menu,
    // opens OUR menu at the event pixel. `prevent_default`'s only meaning is "suppress the
    // browser menu" — it is NOT a "someone handled this" flag (wave-101 verifier note 2), so
    // there is no `default_prevented()` gate here: this handler always acts on the click.
    //
    // Hit-target (T-664, selection-aware): pick the entity under the cursor with a fresh
    // frozen camera at the event px (the same pick the click / dbl-click paths run), then
    // `resolve_target` decides the take — empty ground vs on-entity, retargeting to the hit
    // entity when it is not already selected (Eden's rule). `open` commits any retarget to
    // the live selection and shows the menu. Do not add `stop_propagation` here.
    let oncontextmenu = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
        let container = container.clone();
        let engine = engine.clone();
        let doc = doc.clone();
        let selection = selection.clone();
        move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
            let rect = container.get_bounding_client_rect();
            let (px, py) = (
                ev.client_x() as f64 - rect.left(),
                ev.client_y() as f64 - rect.top(),
            );
            // Frozen camera at the event px (borrow scoped so it drops before the pick's
            // doc borrow; JS is single-threaded so this never reenters the rAF borrow_mut).
            let cam = {
                let g = engine.borrow();
                let Some(e) = g.as_ref() else { return };
                crate::editor::tools::select_tool::frozen_camera(
                    rect.width(),
                    rect.height(),
                    e.target_x(),
                    e.target_y(),
                    e.zoom(),
                )
            };
            // Slot OR vehicle under the cursor — the same pick the left-click uses, so the
            // menu's notion of "the entity here" matches selection's.
            let hit = doc.borrow().as_ref().and_then(|c| {
                crate::editor::tools::select_tool::pick_slot_or_vehicle(
                    &cam,
                    &map_render_slot_soa(c),
                    &editor_ops::vehicle_points(),
                    px,
                    py,
                )
            });
            let sel = selection.borrow().clone();
            // T-651 (`PLACE-COMMENT-001`) — the PLACE GESTURE, and it is deliberately not an
            // armed one. The world point is unprojected HERE, against the same frozen camera
            // the pick above used, and rides `MenuTarget` to the dispatch; "Place Comment"
            // then writes the annotation immediately at that point.
            //
            // Why no arm: comments do not need an arm to be correct. Unlike a palette place,
            // the gesture that chooses the point (the right-click) and the gesture that
            // confirms the action (the menu row) are already two events, so the point is
            // captured once and consumed once, with no in-flight state to strand. (T-723
            // repaired the armed pointerup machine — button filter, left.take(), Esc/RMB
            // disarm — but Place Comment still adds ZERO new state to LeftGesture.)
            let world = cam.unproject_xy(px, py);
            let target = crate::editor::panels::context_menu::resolve_target(hit.as_deref(), &sel)
                .at_world(world[0], world[1]);
            crate::editor::panels::context_menu::open(
                ev.client_x() as f64,
                ev.client_y() as f64,
                target,
            );
        }
    });
    // T-159.26 A1 / T-647 ATTR-OPEN-001 / PLACE-003 — native dblclick, left button only.
    // Picks with a FRESH frozen camera at the event px (the same pick the click / context
    // menu paths use). Two outcomes:
    //   * HIT an entity → open Attributes. The pick is `pick_slot_or_vehicle`, NOT the
    //     slot-only `pick` this handler used before T-647: Attributes must open for a
    //     VEHICLE (and any glyph on the vehicle lane) as well as a slot, which is exactly
    //     the ATTR-OPEN-001 "not just slots" swap. `open_attributes` still owns the
    //     multi-select suppression (>1 selected ⇒ no-op).
    //   * MISS (empty ground) → open the asset PICKER at the world point (PLACE-003).
    //     Picking an asset there arms a place (`begin_place*`), and the very next canvas
    //     click lands it (the click-then-click contract, PLACE-001). This is the LEFT
    //     button; right-click is T-664's context menu, so the two never collide.
    // The chrome subtree stops pointerdown, so a dblclick over a dock never reaches here;
    // and a boot that ended `Failed` has no engine, so the `engine.borrow()` guard below
    // returns before either branch — no engine, no placement (and no picker).
    let ondblclick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
        let container = container.clone();
        let engine = engine.clone();
        let doc = doc.clone();
        // T-642 — the ruler chain + its reactive sync, so a dbl-click can END the chain.
        let ruler = ruler.clone();
        let sync_ruler = sync_ruler.clone();
        move |ev: web_sys::MouseEvent| {
            if ev.button() != 0 {
                return;
            }
            // T-642 — with the Ruler tool active, a double-click ENDS the chain and KEEPS it
            // placed (Decision 3), instead of opening Attributes / the asset picker. The two
            // pointerups of the dbl-click already committed two coincident final vertices, so
            // `dedup_tail` drops the duplicate before `double_click` stops the draw — the kept
            // ruler ends on the real penultimate vertex. Returns before the pick below so a
            // dbl-click in ruler mode never opens an editor dialog. (Select mode is unchanged:
            // this guard is skipped and the pick path runs exactly as before.)
            if tool_mode.get_untracked().is_ruler() {
                let mut r = ruler.borrow_mut();
                // 0.5 m dedupe: far below a click's pixel footprint at any editor zoom, so
                // only the dbl-click's own coincident second vertex is removed.
                r.dedup_tail(0.5);
                r.double_click();
                drop(r);
                sync_ruler();
                return;
            }
            // T-643 — with the LoS tool active, a double-click must NOT open Attributes / the
            // asset picker either. LoS captures TWO single clicks (observer then target); a
            // fast double-click's two pointerups already ran `LosState::click` twice via the
            // shared `LG::Ruler` arm — which is exactly a completed shot — so this handler just
            // swallows the `dblclick` event so it opens no dialog. (Select mode is unchanged:
            // both measure-tool guards are skipped and the pick path runs as before.)
            if tool_mode.get_untracked().is_los() {
                return;
            }
            let rect = container.get_bounding_client_rect();
            let (px, py) = (
                ev.client_x() as f64 - rect.left(),
                ev.client_y() as f64 - rect.top(),
            );
            let cam = {
                let g = engine.borrow();
                let Some(e) = g.as_ref() else { return };
                crate::editor::tools::select_tool::frozen_camera(
                    rect.width(),
                    rect.height(),
                    e.target_x(),
                    e.target_y(),
                    e.zoom(),
                )
            };
            // T-647 ATTR-OPEN-001 — slot OR vehicle under the cursor, matching the click and
            // context-menu picks so "the entity here" means one thing editor-wide.
            let hit = doc.borrow().as_ref().and_then(|c| {
                crate::editor::tools::select_tool::pick_slot_or_vehicle(
                    &cam,
                    &map_render_slot_soa(c),
                    &editor_ops::vehicle_points(),
                    px,
                    py,
                )
            });
            match hit {
                Some(id) => editor_ops::open_attributes(id),
                // T-647 PLACE-003 — empty ground: open the asset picker at the world point
                // the dblclick names (same frozen-cam unproject the place ghost/CUR use, so
                // the picker's eventual drop lands where the dblclick was). A singular
                // unproject (NaN) is off-map and opens nothing.
                None => {
                    let world = cam.unproject_xy(px, py);
                    if world[0].is_finite() && world[1].is_finite() {
                        editor_ops::open_asset_picker(
                            world[0],
                            world[1],
                            ev.client_x() as f64,
                            ev.client_y() as f64,
                        );
                    }
                }
            }
        }
    });
    let _ =
        container.add_event_listener_with_callback("dblclick", ondblclick.as_ref().unchecked_ref());

    // The pointer/contextmenu listeners, registered on the same container the
    // closures measure (the wheel + dblclick registrations sit beside their
    // closures above, keeping the page's original ordering).
    let _ = container
        .add_event_listener_with_callback("pointerdown", onpointerdown.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("pointermove", onpointermove.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("pointerup", onpointerup.as_ref().unchecked_ref());
    let _ = container
        .add_event_listener_with_callback("contextmenu", oncontextmenu.as_ref().unchecked_ref());

    // Leak contract — identical to the page's remaining listeners: `on_cleanup` is
    // `Send`-bound and cannot hold a `!Send` `Closure`, so all six leak on route-
    // leave exactly as they did before the T-934.13 move (the engine leaks too).
    onwheel.forget();
    onpointerdown.forget();
    onpointermove.forget();
    onpointerup.forget();
    oncontextmenu.forget();
    ondblclick.forget();
}
