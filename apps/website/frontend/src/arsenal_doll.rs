//! T-172 B10 — the 3D arsenal doll mount (SoldierModel3D.tsx port onto the intact T-154
//! `map_engine_render::DollEngine`, owned as plain Rust — no shim, no three.js).
//!
//! Dumb per D5: sizes the canvas (device px BEFORE create), forwards pointer deltas (drag =
//! turn character) and sub-threshold clicks (pick), pushes the 14-byte region state array
//! (RAIL order), and drives a damage-driven rAF loop — ALL scene/camera/pick/anchor policy
//! lives in Rust (`map_engine_core::doll` / `DollEngine`). The allowed DOM layer (T-154.1):
//! a cursor tooltip for the hovered part and a pinned name chip + leader line for the ACTIVE
//! part; anchor px comes from Rust and positions are mutated directly in the rAF loop — no
//! per-frame reactive renders. On create failure the caller swaps in the SVG `paper_doll`
//! fallback (the T-154 contract).
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::arsenal_rules::{LOADOUT_ROWS, RAIL_REGIONS};

const CLICK_SLOP_PX: f64 = 4.0; // same bar as the map's drag threshold
const CALLOUT_DX: f64 = 52.0; // chip offset from the anchor (up-right)
const CALLOUT_DY: f64 = -44.0;

type EngineHandle = Rc<RefCell<Option<map_engine_render::DollEngine>>>;

/// Region label for tooltips/callout — `LOADOUT_ROWS` carries the display labels.
fn region_label(key: &str) -> &'static str {
    LOADOUT_ROWS
        .iter()
        .find(|r| r.key == key)
        .map_or("", |r| r.label)
}

fn set_style(el: &web_sys::HtmlElement, prop: &str, value: &str) {
    let _ = el.style().set_property(prop, value);
}

#[component]
pub fn ArsenalDoll(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    /// Click on a picked region → select it in the rail/list (never mutates the loadout).
    on_select: Callback<String>,
    /// resource_name → display_name (tooltip/callout item names).
    names: StoredValue<HashMap<String, String>>,
    /// Set on create failure — the caller swaps in the SVG fallback.
    unavailable: RwSignal<bool>,
) -> impl IntoView {
    let container_ref = NodeRef::<html::Div>::new();
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let leader_ref = NodeRef::<html::Div>::new();
    let callout_ref = NodeRef::<html::Div>::new();
    let tooltip_ref = NodeRef::<html::Div>::new();

    let engine: EngineHandle = Rc::new(RefCell::new(None));
    // Arc<AtomicBool>, not Rc<Cell>: `on_cleanup` is Send+Sync-bound (the mission_editor
    // precedent), and this flag is what lets the rAF loop drop the engine after unmount.
    let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready = RwSignal::new(false);
    let hover_idx = RwSignal::new(-1i32);

    // Mount: size backing store → create → rAF loop (render + callout placement).
    {
        let engine = engine.clone();
        let disposed = disposed.clone();
        Effect::new(move |ran: Option<()>| {
            if ran.is_some() {
                return;
            }
            let (Some(container), Some(canvas)) = (container_ref.get(), canvas_ref.get()) else {
                return;
            };
            let container: web_sys::HtmlElement = container.unchecked_into();
            let canvas: web_sys::HtmlCanvasElement = canvas.unchecked_into();
            let Some(win) = web_sys::window() else {
                return;
            };
            let force_webgl = win
                .location()
                .search()
                .map(|s| s.contains("force=webgl"))
                .unwrap_or(false);
            // Backing store BEFORE create — the engine reads canvas.width/height.
            let dpr0 = win.device_pixel_ratio();
            let rect0 = container.get_bounding_client_rect();
            let scale = |v: f64| ((v.max(1.0) * dpr0).round() as u32).max(1);
            canvas.set_width(scale(rect0.width()));
            canvas.set_height(scale(rect0.height()));

            let engine = engine.clone();
            let disposed = disposed.clone();
            leptos::task::spawn_local(async move {
                match map_engine_render::DollEngine::create(canvas.clone(), force_webgl).await {
                    Ok(mut eng) => {
                        if disposed.load(std::sync::atomic::Ordering::Relaxed) {
                            return; // effect died while create was in flight — drop frees
                        }
                        let rect = container.get_bounding_client_rect();
                        eng.resize(rect.width().max(1.0), rect.height().max(1.0), dpr0);
                        *engine.borrow_mut() = Some(eng);
                        register_doll_hooks(&engine);
                        ready.set(true);

                        // Damage-driven rAF: render + active-callout placement each frame.
                        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> =
                            Rc::new(RefCell::new(None));
                        let g = f.clone();
                        let mut last_dpr = dpr0;
                        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                            if disposed.load(std::sync::atomic::Ordering::Relaxed) {
                                f.borrow_mut().take();
                                engine.borrow_mut().take(); // drop frees the GPU context
                                return;
                            }
                            let rect = container.get_bounding_client_rect();
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                // DPR / layout resize follow-up (ResizeObserver stand-in: the
                                // modal is fixed-size, so polling the rect each frame is cheap
                                // and catches both).
                                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                                let want_w = ((rect.width().max(1.0) * dpr).round() as u32).max(1);
                                if dpr != last_dpr || want_w != canvas.width() {
                                    last_dpr = dpr;
                                    canvas.set_width(want_w);
                                    canvas.set_height(
                                        ((rect.height().max(1.0) * dpr).round() as u32).max(1),
                                    );
                                    e.resize(rect.width().max(1.0), rect.height().max(1.0), dpr);
                                }
                                let _ = e.render(); // damage-driven: Rust no-ops idle frames
                                                    // T-154.1 — the active-part callout tracks its Rust-projected
                                                    // anchor every frame (direct style mutation).
                                let idx = RAIL_REGIONS
                                    .iter()
                                    .position(|r| r.key == active_key.get_untracked())
                                    .map_or(-1, |i| i as i32);
                                let anchor = e.anchor_px(idx);
                                if let (Some(chip), Some(leader)) =
                                    (callout_ref.get_untracked(), leader_ref.get_untracked())
                                {
                                    let chip: web_sys::HtmlElement = chip.unchecked_into();
                                    let leader: web_sys::HtmlElement = leader.unchecked_into();
                                    if anchor.len() != 2 {
                                        set_style(&chip, "display", "none");
                                        set_style(&leader, "display", "none");
                                    } else {
                                        let (ax, ay) = (anchor[0], anchor[1]);
                                        let cx = (ax + CALLOUT_DX)
                                            .clamp(8.0, (rect.width() - 8.0).max(8.0));
                                        let cy = (ay + CALLOUT_DY)
                                            .clamp(8.0, (rect.height() - 8.0).max(8.0));
                                        set_style(&chip, "display", "block");
                                        set_style(
                                            &chip,
                                            "transform",
                                            &format!("translate({cx}px, {cy}px)"),
                                        );
                                        let (dx, dy) = (cx - ax, cy - ay);
                                        let len = dx.hypot(dy);
                                        set_style(&leader, "display", "block");
                                        set_style(&leader, "width", &format!("{len}px"));
                                        set_style(
                                            &leader,
                                            "transform",
                                            &format!(
                                                "translate({ax}px, {ay}px) rotate({}rad)",
                                                dy.atan2(dx)
                                            ),
                                        );
                                    }
                                }
                            }
                            let cb_ref = f.borrow();
                            if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
                            }
                        })
                            as Box<dyn FnMut()>));
                        let cb_ref = g.borrow();
                        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
                        }
                    }
                    Err(e) => {
                        leptos::logging::error!("DollEngine::create: {e:?}");
                        unavailable.set(true);
                    }
                }
            });
        });
    }
    // Unmount (modal close / tab switch): flag the loop, which drops the engine on its next
    // tick — repeated Attributes opens must not leak GPU contexts.
    {
        let disposed = disposed.clone();
        on_cleanup(move || disposed.store(true, std::sync::atomic::Ordering::Relaxed));
    }

    // Region states, RAIL order: 2 = active, 1 = equipped, 0 = empty.
    {
        let engine = engine.clone();
        Effect::new(move |_| {
            let map = picks.get();
            let active = active_key.get();
            if !ready.get() {
                return;
            }
            let states: Vec<u8> = RAIL_REGIONS
                .iter()
                .map(|r| {
                    if r.key == active {
                        2
                    } else {
                        u8::from(map.get(r.key).is_some_and(|v| !v.is_empty()))
                    }
                })
                .collect();
            if let Some(e) = engine.borrow_mut().as_mut() {
                let _ = e.set_states(&states);
            }
        });
    }

    let set_hover = {
        let engine = engine.clone();
        move |idx: i32| {
            if hover_idx.get_untracked() == idx {
                return;
            }
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_hover(idx);
            }
            hover_idx.set(idx);
        }
    };

    // Drag state: Some((last_x, start_x, start_y, moved)) while the LMB is down.
    let drag: Rc<Cell<Option<(f64, f64, f64, bool)>>> = Rc::new(Cell::new(None));

    let name_of = move |key: &str| -> String {
        picks.get().get(key).filter(|v| !v.is_empty()).map_or_else(
            || "empty".to_string(),
            |rn| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.clone())),
        )
    };

    let on_pointer_down = {
        let drag = drag.clone();
        move |ev: leptos::ev::PointerEvent| {
            if ev.button() != 0 {
                return;
            }
            drag.set(Some((
                f64::from(ev.client_x()),
                f64::from(ev.client_x()),
                f64::from(ev.client_y()),
                false,
            )));
            if let Some(t) = ev
                .target()
                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            {
                let _ = t.set_pointer_capture(ev.pointer_id());
            }
        }
    };
    let on_pointer_move = {
        let drag = drag.clone();
        let engine = engine.clone();
        let set_hover = set_hover.clone();
        move |ev: leptos::ev::PointerEvent| {
            let (cx, cy) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
            if let Some((last_x, sx, sy, moved)) = drag.get() {
                let mut moved = moved;
                if !moved && (cx - sx).abs() + (cy - sy).abs() > CLICK_SLOP_PX {
                    moved = true;
                    set_hover(-1); // rotating — drop the hover highlight
                }
                let dx = cx - last_x;
                if moved && dx != 0.0 {
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.rotate(dx);
                    }
                }
                drag.set(Some((cx, sx, sy, moved)));
                return;
            }
            let Some(container) = container_ref.get_untracked() else {
                return;
            };
            let container: web_sys::HtmlElement = container.unchecked_into();
            let rect = container.get_bounding_client_rect();
            let (x, y) = (cx - rect.left(), cy - rect.top());
            let idx = engine.borrow().as_ref().map_or(-1, |e| e.pick_region(x, y));
            set_hover(idx);
            // Cursor tooltip follows the pointer (clamped inside the container).
            if let Some(tip) = tooltip_ref.get_untracked() {
                let tip: web_sys::HtmlElement = tip.unchecked_into();
                set_style(
                    &tip,
                    "transform",
                    &format!(
                        "translate({}px, {}px)",
                        (x + 14.0).min((rect.width() - 8.0).max(8.0)),
                        (y - 26.0).max(4.0)
                    ),
                );
            }
        }
    };
    let on_pointer_up = {
        let drag = drag.clone();
        let engine = engine.clone();
        move |ev: leptos::ev::PointerEvent| {
            let Some((_, _, _, moved)) = drag.take() else {
                return;
            };
            if moved {
                return;
            }
            let Some(container) = container_ref.get_untracked() else {
                return;
            };
            let container: web_sys::HtmlElement = container.unchecked_into();
            let rect = container.get_bounding_client_rect();
            let idx = engine.borrow().as_ref().map_or(-1, |e| {
                e.pick_region(
                    f64::from(ev.client_x()) - rect.left(),
                    f64::from(ev.client_y()) - rect.top(),
                )
            });
            if idx >= 0 && (idx as usize) < RAIL_REGIONS.len() {
                on_select.run(RAIL_REGIONS[idx as usize].key.to_string());
            }
        }
    };

    view! {
        <div
            node_ref=container_ref
            data-arsenal-doll
            role="img"
            aria-label="Soldier loadout — drag to rotate, click a part to select"
            class=move || {
                if hover_idx.get() >= 0 {
                    "relative h-full w-full touch-none select-none cursor-pointer"
                } else {
                    "relative h-full w-full touch-none select-none cursor-grab active:cursor-grabbing"
                }
            }
            on:pointerdown=on_pointer_down
            on:pointermove=on_pointer_move
            on:pointerup=on_pointer_up
            on:pointercancel={
                let drag = drag.clone();
                move |_| drag.set(None)
            }
            on:pointerleave={
                let set_hover = set_hover.clone();
                move |_| set_hover(-1)
            }
        >
            <canvas node_ref=canvas_ref class="absolute inset-0 h-full w-full rounded-lg"></canvas>
            // Leader line: origin at the anchor, rotated toward the chip (rAF-positioned).
            <div
                node_ref=leader_ref
                class="pointer-events-none absolute top-0 left-0 hidden h-px origin-left bg-primary/70"
            ></div>
            // Active-part callout chip (rAF-positioned at anchor + offset).
            <div
                node_ref=callout_ref
                data-doll-callout
                class="pointer-events-none absolute top-0 left-0 hidden rounded-md border border-primary/40 bg-surface-container-lowest/90 px-2 py-1 text-label-sm whitespace-nowrap text-on-surface shadow-lg"
            >
                <span class="text-primary">
                    {move || region_label(&active_key.get())}
                </span>
                <span class="normal-case text-on-surface-variant">
                    {move || format!(" — {}", name_of(&active_key.get()))}
                </span>
            </div>
            // Hover tooltip follows the cursor.
            {move || {
                let idx = hover_idx.get();
                (idx >= 0)
                    .then(|| {
                        let key = RAIL_REGIONS[idx as usize].key;
                        view! {
                            <div
                                node_ref=tooltip_ref
                                class="pointer-events-none absolute top-0 left-0 rounded bg-surface-container-lowest/90 px-1.5 py-0.5 text-label-sm whitespace-nowrap text-on-surface shadow"
                            >
                                {region_label(key)}
                                <span class="normal-case text-on-surface-variant">
                                    {format!(" — {}", name_of(key))}
                                </span>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

/// `window.__arsenalDoll` — the smoke's proof surface: backend string, active-anchor px, and a
/// CPU pick at css px (all straight off the live engine).
fn register_doll_hooks(engine: &EngineHandle) {
    let obj = js_sys::Object::new();
    let backend = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| JsValue::from_str(&e.backend()))
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut() -> JsValue>);
    let anchor = Closure::wrap(Box::new({
        let engine = engine.clone();
        move |idx: i32| -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| {
                    let v = e.anchor_px(idx);
                    let arr = js_sys::Array::new();
                    for x in v {
                        arr.push(&JsValue::from_f64(x));
                    }
                    arr.into()
                })
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut(i32) -> JsValue>);
    let pick = Closure::wrap(Box::new({
        let engine = engine.clone();
        move |x: f64, y: f64| -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| JsValue::from_f64(f64::from(e.pick_region(x, y))))
                .unwrap_or(JsValue::NULL)
        }
    }) as Box<dyn FnMut(f64, f64) -> JsValue>);
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("backend"), backend.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("anchor"), anchor.as_ref());
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("pick"), pick.as_ref());
    backend.forget();
    anchor.forget();
    pick.forget();
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__arsenalDoll"), &obj);
    }
}
