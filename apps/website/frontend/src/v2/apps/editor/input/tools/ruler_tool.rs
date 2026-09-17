//! Role: the ruler overlay — the DOM/SVG surface the engine's ruler tool draws through — and the
//! install that ties every tool seam in this cluster to its mount.
//! Position: `editor/input/tools` in the frontend editor.
//! Signals & state: two host heartbeats (pan cursor, zoom sampler) plus a mutation tick, and the
//! leaked chain this module hands to the engine's registry.
//! Invariants: every decidable thing — the leg quantities, the readout shapes, the capture machine
//! and the projection — belongs to `website_map_engine::editing::tools::ruler`. What lives here is
//! the drawing and the ownership.
//!
//! The overlay re-runs off the same cursor and zoom heartbeats the LoS overlay and scale bar use —
//! never a frame loop of its own. Native builds render an empty overlay (no engine, no `window`).
#![allow(dead_code)] // the wasm host wires the live path; on native this overlay draws nothing.

use leptos::prelude::*;

use website_map_engine::editing::tools::ruler::chain::RulerChain;
use website_map_engine::editing::tools::ruler::host_registry::{
    read_registered_chain, RULER_CHAIN,
};
use website_map_engine::editing::tools::ruler::projection::{
    project_legs, project_vertices, ProjectedLeg, ProjectedVertex,
};
use website_map_engine::editing::tools::selection;

pub(crate) use crate::v2::apps::editor::ui::inspector::validation_panel::install_seam;

/// Hand the host's leaked ruler chain to the engine's registry so [`RulerOverlay`] can read it.
///
/// **This is an INSTALL** ([`install_seam`]): the chain is unregistered when the owner that
/// registered it is cleaned up, and a remount's newer chain is not clobbered by the old owner's
/// cleanup. Without that, the registry would keep returning a dead page's polyline as though the
/// ruler were still live.
pub fn register_ruler_chain(chain: std::rc::Rc<std::cell::RefCell<RulerChain>>) {
    install_seam(&RULER_CHAIN, chain);
}

/// the ruler overlay. ONE absolutely-positioned, `pointer-events-none` SVG spanning the
/// viewport, drawing the placed polyline + per-leg on-line labels (Decision 1) + vertex dots + the
/// live rubber-band leg to the cursor while drawing. It reads the live camera off the registered
/// engine (`world_assets::camera_snapshot`, the same seam the scale bar / grid refs use) and re-runs
/// off the `cursor` (pan) + `debug_hud` (~1 Hz zoom) heartbeats — NO new rAF loop. `tick` is bumped
/// by the host on every chain mutation (press / Esc / dbl-click) so a click that does not move the
/// pointer still repaints.
///
/// The chain itself is read via [`read_registered_chain`] — a cheap clone of the leaked host
/// `RulerChain` (session-local overlay state — Decision 4). Native builds render an empty overlay
/// (no engine, no `window`); the geometry is proven by `project_legs`/`world_key` above.
#[component]
pub fn RulerOverlay(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-projection). Also
    /// the live cursor world point the rubber-band leg draws to while the chain is `drawing`.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler (a wheel-zoom with a still pointer still
    /// re-projects within a second). `Option` so the mount can forward `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped by the host on every chain mutation so a click repaints even with a still pointer.
    tick: RwSignal<u64>,
) -> impl IntoView {
    #[allow(clippy::type_complexity)]
    let projected =
        move || -> (Vec<ProjectedLeg>, Vec<ProjectedVertex>, Vec<(f64, f64, f64, f64)>) {
            let cur = cursor.get();
            if let Some(h) = debug_hud {
                let _ = h.get();
            }
            let _ = tick.get();
            let chain = read_registered_chain();
            if chain.is_empty() {
                return (Vec::new(), Vec::new(), Vec::new());
            }
            #[cfg(target_arch = "wasm32")]
            {
                let Some((tx, ty, zoom)) = website_map_engine::streaming::host::camera_snapshot() else {
                    return (Vec::new(), Vec::new(), Vec::new());
                };
                let Some(win) = web_sys::window() else {
                    return (Vec::new(), Vec::new(), Vec::new());
                };
                let vw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                if vw <= 0.0 || vh <= 0.0 {
                    return (Vec::new(), Vec::new(), Vec::new());
                }
                let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
                let project = move |x: f64, y: f64| {
                    let p = cam.project([x, y, 0.0]);
                    (p[0], p[1])
                };
                let mut rubber = Vec::new();
                if chain.drawing {
                    if let (Some(last), Some((cwx, cwy, _))) = (chain.points.last().copied(), cur) {
                        let (x1, y1) = project(last.x, last.y);
                        let (x2, y2) = project(cwx, cwy);
                        rubber.push((x1, y1, x2, y2));
                    }
                }
                (
                    project_legs(&chain, &project),
                    project_vertices(&chain, &project),
                    rubber,
                )
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (chain, cur);
                (Vec::new(), Vec::new(), Vec::new())
            }
        };
    view! {
        <svg
            data-ruler-overlay
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <line
                    x1=move || format!("{:.1}", l.x1)
                    y1=move || format!("{:.1}", l.y1)
                    x2=move || format!("{:.1}", l.x2)
                    y2=move || format!("{:.1}", l.y2)
                    class="stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            <For
                each=move || projected().2
                key=|r| format!("{:.0}:{:.0}:{:.0}:{:.0}", r.0, r.1, r.2, r.3)
                let:r
            >
                <line
                    x1=move || format!("{:.1}", r.0)
                    y1=move || format!("{:.1}", r.1)
                    x2=move || format!("{:.1}", r.2)
                    y2=move || format!("{:.1}", r.3)
                    class="stroke-primary/60"
                    stroke-width="1.5"
                    stroke-dasharray="4 4"
                />
            </For>
            <For
                each=move || projected().1
                key=|v| v.key.clone()
                let:v
            >
                <circle
                    cx=move || format!("{:.1}", v.px)
                    cy=move || format!("{:.1}", v.py)
                    r="3"
                    class="fill-surface-container-lowest stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <text
                    x=move || format!("{:.1}", l.mid_x)
                    y=move || format!("{:.1}", l.mid_y - 4.0)
                    text-anchor="middle"
                    class="fill-primary font-mono text-code-md"
                    stroke="rgba(0,0,0,0.55)"
                    stroke-width="3"
                    style="paint-order:stroke"
                >
                    {l.label.clone()}
                </text>
            </For>
        </svg>
    }
}

#[cfg(test)]
#[path = "tests/ruler_tool/seam_lifecycle_and_render_context.rs"]
mod t778_seam_lifecycle;
