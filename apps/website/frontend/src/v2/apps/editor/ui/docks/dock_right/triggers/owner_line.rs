//! Right dock triggers behavior.

use super::*;

/// Draws the selected trigger's link to its owner while both endpoints exist.
#[cfg(target_arch = "wasm32")]
#[component]
pub(super) fn TriggerOwnerLine(
    selected: RwSignal<Option<String>>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    use leptos::portal::Portal;

    let tick = RwSignal::new(0u64);
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let disposed = Arc::new(AtomicBool::new(false));
        #[allow(clippy::type_complexity)]
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = f.clone();
        {
            let disposed = disposed.clone();
            *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                if disposed.load(Ordering::Relaxed) {
                    f.borrow_mut().take(); // drop the loop closure — no further frames
                    return;
                }
                if selected.get_untracked().is_some() {
                    tick.update(|n| *n = n.wrapping_add(1));
                }
                let cb_ref = f.borrow();
                if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                    let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }) as Box<dyn FnMut()>));
        }
        let cb_ref = g.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
        on_cleanup(move || disposed.store(true, Ordering::Relaxed));
    }

    let projected =
        move || -> Option<crate::v2::apps::editor::ui::inspector::zones_panel::ProjectedOwnerLine> {
            let _ = doc_tick.get();
            let _ = tick.get();
            let sel = selected.get();
            let (world_a, world_b) = engine_ops::owner_line_world(sel.as_deref())?;
            let (tx, ty, zoom) = website_map_engine::streaming::host::camera_snapshot()?;
            let win = web_sys::window()?;
            let vw = win.inner_width().ok().and_then(|v| v.as_f64())?;
            let vh = win.inner_height().ok().and_then(|v| v.as_f64())?;
            if vw <= 0.0 || vh <= 0.0 {
                return None;
            }
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
            let project = move |x: f64, y: f64| {
                let p = cam.project([x, y, 0.0]);
                (p[0], p[1])
            };
            Some(
                crate::v2::apps::editor::ui::inspector::zones_panel::project_owner_line(
                    world_a, world_b, project,
                ),
            )
        };

    view! {
        <Portal>
            <svg
                data-trigger-owner-line
                class="pointer-events-none fixed inset-0 z-10"
                width="100%"
                height="100%"
            >
                {move || {
                    projected().map(|l| {
                        view! {
                            <line
                                x1=format!("{:.1}", l.x1)
                                y1=format!("{:.1}", l.y1)
                                x2=format!("{:.1}", l.x2)
                                y2=format!("{:.1}", l.y2)
                                class="stroke-primary/80"
                                stroke-width="1.5"
                                stroke-dasharray="5 3"
                            />
                        }
                    })
                }}
            </svg>
        </Portal>
    }
}
